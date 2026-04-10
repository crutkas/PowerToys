//! Integration test: Load the Rust Awake module DLL and exercise its exports.
//!
//! This simulates what the PowerToys runner does:
//! 1. LoadLibrary the DLL
//! 2. GetProcAddress("powertoy_create")
//! 3. Call it → get PowertoyModuleIface* (opaque, vtable-dispatched)
//! 4. Exercise the module through the Rust function table API
//!
//! We test BOTH entry points:
//! - `powertoy_create` — the C++ vtable adapter (what the real runner calls)
//! - `rust_module_create` — the raw Rust function table (for direct Rust consumers)

#[cfg(test)]
mod tests {
    use powertoys_module_ffi::*;
    use std::ffi::c_int;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows_sys::Win32::Foundation::FreeLibrary;

    /// Find the built DLL in the target directory
    fn find_awake_dll() -> String {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let target_base = format!("{}\\..\\..\\target", manifest);
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };

        // Try target-specific path first (when built with --target)
        let candidates = [
            format!("{}\\x86_64-pc-windows-msvc\\{}\\awake_module_interface.dll", target_base, profile),
            format!("{}\\aarch64-pc-windows-msvc\\{}\\awake_module_interface.dll", target_base, profile),
            format!("{}\\{}\\awake_module_interface.dll", target_base, profile),
        ];

        for path in &candidates {
            if let Ok(canonical) = std::fs::canonicalize(path) {
                return canonical.to_string_lossy().to_string();
            }
        }

        panic!(
            "DLL not found. Tried:\n{}",
            candidates.join("\n")
        );
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn wide_to_string(ptr: *const u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        unsafe {
            let mut len = 0;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
        }
    }

    #[allow(dead_code)]
    fn load_dll() -> isize {
        let dll_path = find_awake_dll();
        let wide_path = to_wide(&dll_path);
        let handle = unsafe { LoadLibraryW(wide_path.as_ptr()) };
        assert!(!handle.is_null(), "LoadLibraryW failed for: {}", dll_path);
        handle as isize
    }

    // ── Tests for rust_module_create (direct Rust FFI) ─────────────────────

    #[test]
    fn test_load_dll_has_all_exports() {
        let dll_path = find_awake_dll();
        let wide_path = to_wide(&dll_path);

        unsafe {
            let handle = LoadLibraryW(wide_path.as_ptr());
            assert!(!handle.is_null(), "LoadLibraryW failed");

            // Check all three expected exports
            for name in [
                "rust_module_create\0",
                "rust_module_destroy\0",
                "powertoy_create\0",
            ] {
                let addr = GetProcAddress(handle, name.as_ptr());
                assert!(addr.is_some(), "Export '{}' not found", name.trim_end_matches('\0'));
            }

            FreeLibrary(handle);
        }
    }

    #[test]
    fn test_rust_module_create_lifecycle() {
        let dll_path = find_awake_dll();
        let wide_path = to_wide(&dll_path);

        unsafe {
            let handle = LoadLibraryW(wide_path.as_ptr());
            assert!(!handle.is_null());

            type CreateFn = unsafe extern "C" fn() -> *mut ModuleFunctionTable;
            let create: CreateFn = std::mem::transmute(
                GetProcAddress(handle, b"rust_module_create\0".as_ptr()).unwrap(),
            );

            let table_ptr = create();
            assert!(!table_ptr.is_null());
            let table = &*table_ptr;

            // Identity
            assert_eq!(wide_to_string((table.get_name)(table.context)), "Awake");
            assert_eq!(wide_to_string((table.get_key)(table.context)), "Awake");

            // Lifecycle
            assert!(!(table.is_enabled)(table.context));
            (table.enable)(table.context);
            assert!((table.is_enabled)(table.context));
            (table.disable)(table.context);
            assert!(!(table.is_enabled)(table.context));

            // Config roundtrip
            let mut size: c_int = 0;
            (table.get_config)(table.context, std::ptr::null_mut(), &mut size);
            assert!(size > 0);

            let mut buffer = vec![0u16; size as usize];
            let mut buf_size = size;
            let ok = (table.get_config)(table.context, buffer.as_mut_ptr(), &mut buf_size);
            assert!(ok);

            let config_json = wide_to_string(buffer.as_ptr());
            assert!(config_json.contains("Awake"));
            assert!(config_json.contains("awake_mode"));

            // Set config
            let custom = r#"{"name":"Awake","version":"1.0","properties":{"awake_keep_display_on":{"value":false},"awake_mode":{"value":2},"awake_hours":{"value":3},"awake_minutes":{"value":45}}}"#;
            let wide_config = to_wide(custom);
            (table.set_config)(table.context, wide_config.as_ptr());

            // Read back
            let mut size2: c_int = 0;
            (table.get_config)(table.context, std::ptr::null_mut(), &mut size2);
            let mut buf2 = vec![0u16; size2 as usize];
            let mut bs2 = size2;
            (table.get_config)(table.context, buf2.as_mut_ptr(), &mut bs2);
            let result = wide_to_string(buf2.as_ptr());
            assert!(result.contains("\"value\":2"));
            assert!(result.contains("\"value\":45"));

            // GPO default
            let gpo = (table.gpo_policy_enabled_configuration)(table.context);
            assert_eq!(gpo, GpoRuleConfigured::NotConfigured);

            // Hotkeys
            assert_eq!((table.get_hotkeys)(table.context, std::ptr::null_mut(), 0), 0);
            assert!(!(table.on_hotkey)(table.context, 0));

            // Enabled by default
            assert!((table.is_enabled_by_default)(table.context));

            // Destroy
            (table.destroy)(table.context);
            let _ = Box::from_raw(table_ptr);

            FreeLibrary(handle);
        }
    }

    // ── Tests for powertoy_create (C++ vtable — what the runner calls) ─────

    /// Opaque pointer representing PowertoyModuleIface*.
    /// We can't call virtual methods from Rust directly, but we CAN verify
    /// the pointer is non-null and that the export resolves.
    #[test]
    fn test_powertoy_create_returns_nonnull() {
        let dll_path = find_awake_dll();
        let wide_path = to_wide(&dll_path);

        unsafe {
            let handle = LoadLibraryW(wide_path.as_ptr());
            assert!(!handle.is_null());

            // This is what the runner calls
            type PowertoyCreateFn = unsafe extern "C" fn() -> *mut std::ffi::c_void;
            let create: PowertoyCreateFn = std::mem::transmute(
                GetProcAddress(handle, b"powertoy_create\0".as_ptr()).unwrap(),
            );

            let iface_ptr = create();
            assert!(!iface_ptr.is_null(), "powertoy_create returned null!");

            // We can't call vtable methods from Rust (that's the whole point
            // of the C++ adapter), but we've proven:
            // 1. DLL loads ✓
            // 2. powertoy_create resolves ✓
            // 3. It returns a valid, non-null pointer ✓
            //
            // The C++ runner would now call iface_ptr->get_key(), etc.
            // through the MSVC vtable. That works because the adapter's
            // vtable delegates to our Rust function table.

            // We need to call destroy through the vtable, but since we can't
            // invoke C++ virtual methods from Rust, we'll just leak this.
            // In a real test harness (C++ or mixed), you'd call:
            //   static_cast<PowertoyModuleIface*>(iface_ptr)->destroy();

            FreeLibrary(handle);
        }
    }

    /// Prove the C++ vtable works by reading it manually.
    /// MSVC vtable layout: first 8 bytes at the object pointer = vptr,
    /// which points to an array of function pointers.
    #[test]
    fn test_powertoy_create_vtable_is_valid() {
        let dll_path = find_awake_dll();
        let wide_path = to_wide(&dll_path);

        unsafe {
            let handle = LoadLibraryW(wide_path.as_ptr());
            assert!(!handle.is_null());

            type PowertoyCreateFn = unsafe extern "C" fn() -> *mut std::ffi::c_void;
            let create: PowertoyCreateFn = std::mem::transmute(
                GetProcAddress(handle, b"powertoy_create\0".as_ptr()).unwrap(),
            );

            let obj_ptr = create();
            assert!(!obj_ptr.is_null());

            // In MSVC C++ ABI, the first pointer-sized value at the object
            // is the vptr (pointer to the vtable).
            let vptr = *(obj_ptr as *const *const usize);
            assert!(!vptr.is_null(), "vtable pointer should be non-null");

            // The vtable should contain function pointers.
            // First entry = get_name virtual method.
            let first_entry = *vptr;
            assert!(first_entry != 0, "First vtable entry should be a valid function pointer");

            // Call get_name() through the vtable manually!
            // MSVC calling convention for virtual methods:
            //   rcx = this pointer, returns wchar_t*
            type VirtualGetName = unsafe extern "C" fn(*mut std::ffi::c_void) -> *const u16;
            let get_name: VirtualGetName = std::mem::transmute(first_entry);
            let name = get_name(obj_ptr);
            assert!(!name.is_null());
            let name_str = wide_to_string(name);
            assert_eq!(name_str, "Awake", "get_name() via vtable should return 'Awake'");

            // Second vtable entry = get_key()
            let second_entry = *vptr.add(1);
            let get_key: VirtualGetName = std::mem::transmute(second_entry);
            let key = get_key(obj_ptr);
            let key_str = wide_to_string(key);
            assert_eq!(key_str, "Awake", "get_key() via vtable should return 'Awake'");

            FreeLibrary(handle);
        }
    }
}
