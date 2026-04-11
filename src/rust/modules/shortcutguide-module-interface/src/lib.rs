use powertoys_module_ffi::*;
use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

static MODULE_WIDE: std::sync::LazyLock<&'static [u16]> = std::sync::LazyLock::new(|| {
    Box::leak(to_wide("Shortcut Guide").into_boxed_slice())
});

pub struct Module {
    enabled: AtomicBool,
    process_handle: Option<*mut std::ffi::c_void>,
    terminate_event: *mut std::ffi::c_void,
    use_legacy_win_key: bool,
    press_time_ms: u32,
    hotkey: HotkeyEx,
}
unsafe impl Send for Module {}

impl Module {
    pub fn new() -> Self {
        let mut m = Self {
            enabled: AtomicBool::new(false),
            process_handle: None,
            terminate_event: create_event("Local\\ShortcutGuide-ExitEvent-35697cdd-a3d2-47d6-a246-34efcc73eac0"),
            use_legacy_win_key: true,
            press_time_ms: 900,
            hotkey: HotkeyEx { modifiers_mask: 0, vk_code: 0, id: 0 },
        };
        m.load_settings();
        m
    }

    fn load_settings(&mut self) {
        let path = match powertoys_win32::settings::module_settings_path("Shortcut Guide") {
            Some(p) => p,
            None => return,
        };
        let json_str = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return,
        };
        let root: serde_json::Value = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(_) => return,
        };
        let props = match root.get("properties") {
            Some(p) => p,
            None => return,
        };

        // Legacy Win key press behavior
        if let Some(v) = props.get("use_legacy_press_win_key_behavior").and_then(|o| o.get("value")).and_then(|v| v.as_bool()) {
            self.use_legacy_win_key = v;
        }

        // Press time
        if let Some(v) = props.get("press_time").and_then(|o| o.get("value")).and_then(|v| v.as_i64()) {
            if v >= 0 { self.press_time_ms = v as u32; }
        }

        // Custom hotkey
        if let Some(hk) = props.get("open_shortcutguide") {
            let mut mask: u16 = 0;
            if hk.get("win").and_then(|v| v.as_bool()).unwrap_or(false) { mask |= 0x0008; } // MOD_WIN
            if hk.get("ctrl").and_then(|v| v.as_bool()).unwrap_or(false) { mask |= 0x0002; } // MOD_CONTROL
            if hk.get("shift").and_then(|v| v.as_bool()).unwrap_or(false) { mask |= 0x0004; } // MOD_SHIFT
            if hk.get("alt").and_then(|v| v.as_bool()).unwrap_or(false) { mask |= 0x0001; } // MOD_ALT
            let code = hk.get("code").and_then(|v| v.as_i64()).unwrap_or(0) as u16;
            self.hotkey = HotkeyEx { modifiers_mask: mask, vk_code: code, id: 0 };
        }
    }

    fn launch_process(&mut self) {
        use windows_sys::Win32::UI::Shell::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let pid = unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() };
        let exe = to_wide("PowerToys.ShortcutGuide.exe");
        let params = to_wide(&pid.to_string());
        unsafe {
            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI;
            sei.lpFile = exe.as_ptr();
            sei.nShow = SW_SHOWNORMAL;
            sei.lpParameters = params.as_ptr();
            if ShellExecuteExW(&mut sei) != 0 { self.process_handle = Some(sei.hProcess); }
        }
    }

    fn toggle_process(&mut self) {
        if let Some(h) = self.process_handle.take() {
            unsafe {
                windows_sys::Win32::System::Threading::TerminateProcess(h, 0);
                windows_sys::Win32::Foundation::CloseHandle(h);
            }
        } else {
            self.launch_process();
        }
    }
}

impl PowerToyModule for Module {
    fn get_name(&self) -> *const u16 { MODULE_WIDE.as_ptr() }
    fn get_key(&self) -> *const u16 { MODULE_WIDE.as_ptr() }
    fn enable(&mut self) {
        self.enabled.store(true, Ordering::SeqCst);
        // Don't launch here — ShortcutGuide EXE is launched on OnHotkeyEx (long Win press)
    }
    fn disable(&mut self) {
        self.enabled.store(false, Ordering::SeqCst);
        if !self.terminate_event.is_null() {
            unsafe { windows_sys::Win32::System::Threading::SetEvent(self.terminate_event); }
        }
        if let Some(h) = self.process_handle.take() {
            unsafe {
                windows_sys::Win32::System::Threading::WaitForSingleObject(h, 1500);
                windows_sys::Win32::System::Threading::TerminateProcess(h, 0);
                windows_sys::Win32::Foundation::CloseHandle(h);
            }
        }
    }
    fn is_enabled(&self) -> bool { self.enabled.load(Ordering::SeqCst) }
    fn get_config(&self, _buf: *mut u16, sz: *mut c_int) -> bool {
        if !sz.is_null() { unsafe { *sz = 0; } }
        false
    }
    fn set_config(&mut self, _config: *const u16) {
        self.load_settings();
    }
    fn destroy(&mut self) { self.disable(); }

    fn get_hotkeys(&self, buffer: *mut Hotkey, buffer_size: usize) -> usize {
        if self.use_legacy_win_key {
            return 0; // No hotkey — using long Win press via keep_track_of_pressed_win_key
        }
        if buffer.is_null() || buffer_size == 0 {
            return 1; // Need 1 slot
        }
        let hk = Hotkey {
            win: (self.hotkey.modifiers_mask & 0x0008) != 0,
            ctrl: (self.hotkey.modifiers_mask & 0x0002) != 0,
            shift: (self.hotkey.modifiers_mask & 0x0004) != 0,
            alt: (self.hotkey.modifiers_mask & 0x0001) != 0,
            key: self.hotkey.vk_code as u8,
            id: 1,
            is_shown: true,
        };
        unsafe { *buffer = hk; }
        1
    }

    fn on_hotkey(&mut self, _hotkey_id: usize) -> bool {
        self.toggle_process();
        true
    }

    fn keep_track_of_pressed_win_key(&self) -> bool {
        self.use_legacy_win_key
    }

    fn milliseconds_win_key_must_be_pressed(&self) -> u32 {
        self.press_time_ms
    }

    fn on_hotkey_ex(&mut self) {
        if !self.enabled.load(Ordering::SeqCst) {
            return;
        }
        self.toggle_process();
    }

    fn gpo_policy_enabled_configuration(&self) -> GpoRuleConfigured {
        check_gpo("EnableShortcutGuide")
    }
}

fn to_wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

fn create_event(name: &str) -> *mut std::ffi::c_void {
    let wide = to_wide(name);
    unsafe {
        let mut sa: windows_sys::Win32::Security::SECURITY_ATTRIBUTES = std::mem::zeroed();
        sa.nLength = std::mem::size_of::<windows_sys::Win32::Security::SECURITY_ATTRIBUTES>() as u32;
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
        let r = RegQueryValueExW(hkey, val.as_ptr(), std::ptr::null(), &mut dt, &mut data as *mut u32 as *mut u8, &mut sz);
        RegCloseKey(hkey);
        if r != 0 || dt != REG_DWORD { return GpoRuleConfigured::NotConfigured; }
        match data { 1 => GpoRuleConfigured::Enabled, 0 => GpoRuleConfigured::Disabled, _ => GpoRuleConfigured::NotConfigured }
    }
}

powertoys_module_ffi::register_module!(Module::new);


#[cfg(test)]
mod tests {
    use super::*;
    use powertoys_module_ffi::*;

    #[test]
    fn test_module_creates() {
        let module = Module::new();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_name_and_key() {
        let module = Module::new();
        let name = wide_ptr_to_string(module.get_name());
        let key = wide_ptr_to_string(module.get_key());
        assert_eq!(name, "Shortcut Guide");
        assert_eq!(key, "Shortcut Guide");
    }

    #[test]
    fn test_enable_disable() {
        let mut module = Module::new();
        assert!(!module.is_enabled());
        module.enabled.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(module.is_enabled());
        module.disable();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_gpo_default() {
        let module = Module::new();
        assert_eq!(module.gpo_policy_enabled_configuration(), GpoRuleConfigured::NotConfigured);
    }

    #[test]
    fn test_register_macro_exports() {
        let table_ptr = rust_module_create();
        assert!(!table_ptr.is_null());
        unsafe {
            let table = &*table_ptr;
            let name = wide_ptr_to_string((table.get_name)(table.context));
            assert_eq!(name, "Shortcut Guide");
            (table.destroy)(table.context);
            let _ = Box::from_raw(table_ptr);
        }
    }

    fn wide_ptr_to_string(ptr: *const u16) -> String {
        if ptr.is_null() { return String::new(); }
        unsafe {
            let mut len = 0;
            while *ptr.add(len) != 0 { len += 1; }
            String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
        }
    }
}