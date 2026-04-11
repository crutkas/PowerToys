//! MouseJump Module Interface — Rust implementation
//!
//! This is the PowerToys module DLL for MouseJump.
//! It launches `PowerToys.MouseJumpUI.exe` when enabled and handles
//! a hotkey (default: Win+Shift+D) to invoke the jump preview.
//!
//! is_enabled_by_default returns false, matching the C++ original.

use powertoys_module_ffi::*;
use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

static MODULE_WIDE: std::sync::LazyLock<&'static [u16]> = std::sync::LazyLock::new(|| {
    Box::leak(to_wide("MouseJump").into_boxed_slice())
});

pub struct Module {
    enabled: AtomicBool,
    hotkey: Hotkey,
    invoke_event: *mut std::ffi::c_void,
    process_handle: Option<*mut std::ffi::c_void>,
}

unsafe impl Send for Module {}

impl Module {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            hotkey: Hotkey {
                win: true,
                ctrl: false,
                shift: true,
                alt: false,
                key: b'D',
                id: 0,
                is_shown: true,
            },
            invoke_event: create_event("PowerToys_MouseJump_ShowPreview"),
            process_handle: None,
        }
    }

    fn launch_process(&mut self) {
        use windows_sys::Win32::UI::Shell::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let pid = unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() };
        let exe = to_wide("PowerToys.MouseJumpUI.exe");
        let params = to_wide(&pid.to_string());

        unsafe {
            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI;
            sei.lpFile = exe.as_ptr();
            sei.nShow = SW_SHOWNORMAL;
            sei.lpParameters = params.as_ptr();
            if ShellExecuteExW(&mut sei) != 0 {
                self.process_handle = Some(sei.hProcess);
            }
        }
    }

    fn is_process_running(&self) -> bool {
        if let Some(h) = self.process_handle {
            unsafe {
                windows_sys::Win32::System::Threading::WaitForSingleObject(h, 0) == 0x00000102
                // WAIT_TIMEOUT
            }
        } else {
            false
        }
    }
}

impl PowerToyModule for Module {
    fn get_name(&self) -> *const u16 {
        MODULE_WIDE.as_ptr()
    }

    fn get_key(&self) -> *const u16 {
        MODULE_WIDE.as_ptr()
    }

    fn enable(&mut self) {
        self.enabled.store(true, Ordering::SeqCst);
        unsafe {
            windows_sys::Win32::System::Threading::ResetEvent(self.invoke_event);
        }
        self.launch_process();
    }

    fn disable(&mut self) {
        if self.enabled.load(Ordering::SeqCst) {
            unsafe {
                windows_sys::Win32::System::Threading::ResetEvent(self.invoke_event);
            }
            if let Some(h) = self.process_handle.take() {
                unsafe {
                    windows_sys::Win32::System::Threading::TerminateProcess(h, 1);
                    windows_sys::Win32::Foundation::CloseHandle(h);
                }
            }
        }
        self.enabled.store(false, Ordering::SeqCst);
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    fn get_config(&self, _buf: *mut u16, sz: *mut c_int) -> bool {
        if !sz.is_null() {
            unsafe { *sz = 0; }
        }
        false
    }

    fn set_config(&mut self, _config: *const u16) {}

    fn destroy(&mut self) {
        self.disable();
    }

    fn get_hotkeys(&self, buffer: *mut Hotkey, buffer_size: usize) -> usize {
        if self.hotkey.key != 0 {
            if !buffer.is_null() && buffer_size >= 1 {
                unsafe { *buffer = self.hotkey; }
            }
            1
        } else {
            0
        }
    }

    fn on_hotkey(&mut self, _hotkey_id: usize) -> bool {
        if !self.enabled.load(Ordering::SeqCst) {
            return false;
        }

        if !self.is_process_running() {
            self.launch_process();
        }

        unsafe {
            windows_sys::Win32::System::Threading::SetEvent(self.invoke_event);
        }
        true
    }

    fn is_enabled_by_default(&self) -> bool {
        false
    }

    fn gpo_policy_enabled_configuration(&self) -> GpoRuleConfigured {
        check_gpo("ConfigureEnabledUtilityMouseJump")
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn create_event(name: &str) -> *mut std::ffi::c_void {
    let wide = to_wide(name);
    unsafe {
        let mut sa: windows_sys::Win32::Security::SECURITY_ATTRIBUTES = std::mem::zeroed();
        sa.nLength =
            std::mem::size_of::<windows_sys::Win32::Security::SECURITY_ATTRIBUTES>() as u32;
        windows_sys::Win32::System::Threading::CreateEventW(&sa, 0, 0, wide.as_ptr())
    }
}

fn check_gpo(value_name: &str) -> GpoRuleConfigured {
    use windows_sys::Win32::System::Registry::*;

    let subkey = to_wide("SOFTWARE\\Policies\\PowerToys");
    let val = to_wide(value_name);

    unsafe {
        let mut hkey = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return GpoRuleConfigured::NotConfigured;
        }
        let mut data: u32 = 0;
        let mut sz = 4u32;
        let mut dt: u32 = 0;
        let r = RegQueryValueExW(
            hkey,
            val.as_ptr(),
            std::ptr::null(),
            &mut dt,
            &mut data as *mut u32 as *mut u8,
            &mut sz,
        );
        RegCloseKey(hkey);
        if r != 0 || dt != REG_DWORD {
            return GpoRuleConfigured::NotConfigured;
        }
        match data {
            1 => GpoRuleConfigured::Enabled,
            0 => GpoRuleConfigured::Disabled,
            _ => GpoRuleConfigured::NotConfigured,
        }
    }
}

powertoys_module_ffi::register_module!(Module::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creates() {
        let module = Module::new();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_get_name() {
        let module = Module::new();
        let name = wide_ptr_to_string(module.get_name());
        assert_eq!(name, "MouseJump");
    }

    #[test]
    fn test_get_key() {
        let module = Module::new();
        let key = wide_ptr_to_string(module.get_key());
        assert_eq!(key, "MouseJump");
    }

    #[test]
    fn test_gpo_default() {
        let module = Module::new();
        assert_eq!(
            module.gpo_policy_enabled_configuration(),
            GpoRuleConfigured::NotConfigured
        );
    }

    #[test]
    fn test_register_macro_exports() {
        let table_ptr = rust_module_create();
        assert!(!table_ptr.is_null());
        unsafe {
            let table = &*table_ptr;
            let name = wide_ptr_to_string((table.get_name)(table.context));
            assert_eq!(name, "MouseJump");
            (table.destroy)(table.context);
            let _ = Box::from_raw(table_ptr);
        }
    }

    fn wide_ptr_to_string(ptr: *const u16) -> String {
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
}
