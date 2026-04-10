//! AlwaysOnTop Module Interface — Rust implementation
//!
//! Manages the AlwaysOnTop process lifecycle, hotkey registration,
//! and named event signaling for pin/unpin/opacity operations.

use powertoys_module_ffi::*;
use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

mod wide;

use wide::{to_wide, wide_to_string};

// Static wide strings (leaked once)
static MODULE_NAME: std::sync::LazyLock<&'static [u16]> = std::sync::LazyLock::new(|| {
    Box::leak(to_wide("AlwaysOnTop").into_boxed_slice())
});
static MODULE_KEY: std::sync::LazyLock<&'static [u16]> = std::sync::LazyLock::new(|| {
    Box::leak(to_wide("AlwaysOnTop").into_boxed_slice())
});

// Named events — must match CommonSharedConstants in the C++ codebase
const PIN_EVENT: &str =
    "Local\\AlwaysOnTopPinEvent-892e0aa2-cfa8-4cc4-b196-ddeb32314ce8";
const TERMINATE_EVENT: &str =
    "Local\\AlwaysOnTopTerminateEvent-cfdf1eae-791f-4953-8021-2f18f3837eae";
const INCREASE_OPACITY_EVENT: &str =
    "Local\\AlwaysOnTopIncreaseOpacityEvent-a1b2c3d4-e5f6-7890-abcd-ef1234567890";
const DECREASE_OPACITY_EVENT: &str =
    "Local\\AlwaysOnTopDecreaseOpacityEvent-b2c3d4e5-f6a7-8901-bcde-f12345678901";

// Window property name for checking if a window is pinned
const PINNED_WINDOW_PROP: &str = "AlwaysOnTop_Pinned";

// Virtual key codes
const VK_OEM_PLUS: u8 = 0xBB;
const VK_OEM_MINUS: u8 = 0xBD;

pub struct AlwaysOnTopModule {
    enabled: AtomicBool,
    process_handle: Option<*mut std::ffi::c_void>,

    // Named event handles (created in constructor, signaled on hotkey)
    pin_event: *mut std::ffi::c_void,
    terminate_event: *mut std::ffi::c_void,
    increase_opacity_event: *mut std::ffi::c_void,
    decrease_opacity_event: *mut std::ffi::c_void,

    // Hotkeys (3 total: pin, increase opacity, decrease opacity)
    pin_hotkey: Hotkey,
    increase_opacity_hotkey: Hotkey,
    decrease_opacity_hotkey: Hotkey,
}

// SAFETY: Event handles are thread-safe Win32 kernel objects
unsafe impl Send for AlwaysOnTopModule {}

impl AlwaysOnTopModule {
    pub fn new() -> Self {
        let pin_event = create_default_event(PIN_EVENT);
        let terminate_event = create_default_event(TERMINATE_EVENT);
        let inc_event = create_default_event(INCREASE_OPACITY_EVENT);
        let dec_event = create_default_event(DECREASE_OPACITY_EVENT);

        let mut module = Self {
            enabled: AtomicBool::new(false),
            process_handle: None,
            pin_event,
            terminate_event,
            increase_opacity_event: inc_event,
            decrease_opacity_event: dec_event,
            // Default hotkeys: Win+Ctrl+T, Win+Ctrl++, Win+Ctrl+-
            pin_hotkey: Hotkey {
                win: true, ctrl: true, shift: false, alt: false,
                key: b'T', id: 0, is_shown: true,
            },
            increase_opacity_hotkey: Hotkey {
                win: true, ctrl: true, shift: false, alt: false,
                key: VK_OEM_PLUS, id: 1, is_shown: true,
            },
            decrease_opacity_hotkey: Hotkey {
                win: true, ctrl: true, shift: false, alt: false,
                key: VK_OEM_MINUS, id: 2, is_shown: true,
            },
        };

        module.load_settings();
        module
    }

    fn launch_process(&mut self) {
        use windows_sys::Win32::UI::Shell::ShellExecuteExW;
        use windows_sys::Win32::UI::Shell::SHELLEXECUTEINFOW;
        use windows_sys::Win32::UI::Shell::SEE_MASK_NOCLOSEPROCESS;
        use windows_sys::Win32::UI::Shell::SEE_MASK_FLAG_NO_UI;
        use windows_sys::Win32::System::Threading::GetCurrentProcessId;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        use windows_sys::Win32::System::Threading::ResetEvent;

        let pid = unsafe { GetCurrentProcessId() };
        let params = to_wide(&pid.to_string());
        let exe = to_wide("PowerToys.AlwaysOnTop.exe");

        unsafe {
            ResetEvent(self.pin_event);

            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI;
            sei.lpFile = exe.as_ptr();
            sei.nShow = SW_SHOWNORMAL;
            sei.lpParameters = params.as_ptr();

            if ShellExecuteExW(&mut sei) != 0 {
                self.process_handle = Some(sei.hProcess);
                eprintln!("[AlwaysOnTop] Launched PowerToys.AlwaysOnTop.exe (handle: {:?})", sei.hProcess);
            } else {
                eprintln!("[AlwaysOnTop] Failed to launch PowerToys.AlwaysOnTop.exe");
            }
        }
    }

    fn is_process_running(&self) -> bool {
        use windows_sys::Win32::System::Threading::WaitForSingleObject;
        match self.process_handle {
            Some(h) => unsafe { WaitForSingleObject(h, 0) == 0x00000102 }, // WAIT_TIMEOUT
            None => false,
        }
    }

    fn terminate_process(&mut self) {
        use windows_sys::Win32::System::Threading::{
            SetEvent, ResetEvent, WaitForSingleObject, TerminateProcess,
        };
        use windows_sys::Win32::Foundation::CloseHandle;

        unsafe {
            ResetEvent(self.pin_event);
            SetEvent(self.terminate_event);

            if let Some(h) = self.process_handle {
                // Wait 1.5 seconds for clean exit
                if WaitForSingleObject(h, 1500) != 0 {
                    // Still running — force terminate
                    TerminateProcess(h, 0);
                }
                CloseHandle(h);
            }
        }
        self.process_handle = None;
    }

    fn load_settings(&mut self) {
        // Load hotkeys from settings file
        let settings_path = get_settings_path("AlwaysOnTop");
        let json = match std::fs::read_to_string(&settings_path) {
            Ok(s) => s,
            Err(_) => return,
        };

        let parsed: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(_) => return,
        };

        if let Some(props) = parsed.get("properties") {
            self.parse_hotkey_from_json(props, "hotkey", &mut self.pin_hotkey.clone(), 0);
            self.parse_hotkey_from_json(props, "increase-opacity-hotkey", &mut self.increase_opacity_hotkey.clone(), 1);
            self.parse_hotkey_from_json(props, "decrease-opacity-hotkey", &mut self.decrease_opacity_hotkey.clone(), 2);
        }
    }

    fn parse_hotkey_from_json(&mut self, props: &serde_json::Value, key: &str, target: &mut Hotkey, id: i32) {
        if let Some(obj) = props.get(key).and_then(|v| v.get("value")) {
            let mut hk = Hotkey {
                win: obj.get("win").and_then(|v| v.as_bool()).unwrap_or(false),
                ctrl: obj.get("ctrl").and_then(|v| v.as_bool()).unwrap_or(false),
                shift: obj.get("shift").and_then(|v| v.as_bool()).unwrap_or(false),
                alt: obj.get("alt").and_then(|v| v.as_bool()).unwrap_or(false),
                key: obj.get("code").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                id,
                is_shown: true,
            };
            hk.is_shown = hk.key != 0;

            match key {
                "hotkey" => self.pin_hotkey = hk,
                "increase-opacity-hotkey" => self.increase_opacity_hotkey = hk,
                "decrease-opacity-hotkey" => self.decrease_opacity_hotkey = hk,
                _ => {}
            }
        }
    }
}

impl PowerToyModule for AlwaysOnTopModule {
    fn get_name(&self) -> *const u16 {
        MODULE_NAME.as_ptr()
    }

    fn get_key(&self) -> *const u16 {
        MODULE_KEY.as_ptr()
    }

    fn enable(&mut self) {
        if !self.enabled.load(Ordering::SeqCst) {
            self.enabled.store(true, Ordering::SeqCst);
            self.launch_process();
        }
    }

    fn disable(&mut self) {
        if self.enabled.load(Ordering::SeqCst) {
            self.enabled.store(false, Ordering::SeqCst);
            self.terminate_process();
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    fn get_config(&self, buffer: *mut u16, buffer_size: *mut c_int) -> bool {
        // Minimal config — settings are managed by the Settings UI
        let json = r#"{"name":"AlwaysOnTop","version":"1.0"}"#;
        let wide: Vec<u16> = json.encode_utf16().chain(std::iter::once(0)).collect();
        let needed = wide.len() as c_int;

        if buffer.is_null() || buffer_size.is_null() {
            if !buffer_size.is_null() {
                unsafe { *buffer_size = needed };
            }
            return false;
        }

        let available = unsafe { *buffer_size };
        if available < needed {
            unsafe { *buffer_size = needed };
            return false;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), buffer, wide.len());
            *buffer_size = needed;
        }
        true
    }

    fn set_config(&mut self, config: *const u16) {
        if config.is_null() {
            return;
        }
        let s = wide_to_string(config);
        // Parse hotkeys from the config JSON if provided
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&s) {
            if let Some(props) = parsed.get("properties") {
                let mut pin = self.pin_hotkey;
                let mut inc = self.increase_opacity_hotkey;
                let mut dec = self.decrease_opacity_hotkey;
                self.parse_hotkey_from_json(props, "hotkey", &mut pin, 0);
                self.parse_hotkey_from_json(props, "increase-opacity-hotkey", &mut inc, 1);
                self.parse_hotkey_from_json(props, "decrease-opacity-hotkey", &mut dec, 2);
            }
        }
    }

    fn destroy(&mut self) {
        // Disable without telemetry, matching C++ Disable(false)
        self.enabled.store(false, Ordering::SeqCst);
        self.terminate_process();
    }

    fn get_hotkeys(&self, buffer: *mut Hotkey, buffer_size: usize) -> usize {
        let hotkeys = [
            Hotkey { id: 0, is_shown: self.pin_hotkey.key != 0, ..self.pin_hotkey },
            Hotkey { id: 1, is_shown: self.increase_opacity_hotkey.key != 0, ..self.increase_opacity_hotkey },
            Hotkey { id: 2, is_shown: self.decrease_opacity_hotkey.key != 0, ..self.decrease_opacity_hotkey },
        ];

        if !buffer.is_null() {
            let count = buffer_size.min(3);
            unsafe {
                std::ptr::copy_nonoverlapping(hotkeys.as_ptr(), buffer, count);
            }
        }

        3
    }

    fn on_hotkey(&mut self, hotkey_id: usize) -> bool {
        // Debug: write to a file since stderr might be swallowed
        let _ = std::fs::write(
            format!("{}\\aot_hotkey_debug.txt", std::env::var("TEMP").unwrap_or_default()),
            format!("on_hotkey called! id={}, enabled={}\n", hotkey_id, self.enabled.load(Ordering::SeqCst))
        );

        if !self.enabled.load(Ordering::SeqCst) {
            return false;
        }

        match hotkey_id {
            0 => {
                let running = self.is_process_running();
                eprintln!("[AlwaysOnTop DLL] Pin hotkey! process_running={}", running);
                if !running {
                    self.launch_process();
                }
                let result = unsafe { windows_sys::Win32::System::Threading::SetEvent(self.pin_event) };
                eprintln!("[AlwaysOnTop DLL] SetEvent(pin) result={}", result);
                true
            }
            1 | 2 => {
                // Opacity: only if foreground window is pinned
                let fg = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
                if fg.is_null() {
                    return false;
                }
                let prop_name = to_wide(PINNED_WINDOW_PROP);
                let prop = unsafe {
                    windows_sys::Win32::UI::WindowsAndMessaging::GetPropW(fg, prop_name.as_ptr())
                };
                if prop.is_null() {
                    return false;
                }

                if !self.is_process_running() {
                    self.launch_process();
                }
                let event = if hotkey_id == 1 {
                    self.increase_opacity_event
                } else {
                    self.decrease_opacity_event
                };
                unsafe { windows_sys::Win32::System::Threading::SetEvent(event) };
                true
            }
            _ => false,
        }
    }

    fn gpo_policy_enabled_configuration(&self) -> GpoRuleConfigured {
        check_gpo("EnableAlwaysOnTop")
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn create_default_event(name: &str) -> *mut std::ffi::c_void {
    use windows_sys::Win32::System::Threading::CreateEventW;
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;

    let wide_name = to_wide(name);
    unsafe {
        let mut sa: SECURITY_ATTRIBUTES = std::mem::zeroed();
        sa.nLength = std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32;
        sa.bInheritHandle = 0;
        CreateEventW(&sa, 0, 0, wide_name.as_ptr())
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
        let mut data_size = std::mem::size_of::<u32>() as u32;
        let mut data_type: u32 = 0;
        let res = RegQueryValueExW(hkey, val.as_ptr(), std::ptr::null(), &mut data_type, &mut data as *mut u32 as *mut u8, &mut data_size);
        RegCloseKey(hkey);
        if res != 0 || data_type != REG_DWORD { return GpoRuleConfigured::NotConfigured; }
        match data { 1 => GpoRuleConfigured::Enabled, 0 => GpoRuleConfigured::Disabled, _ => GpoRuleConfigured::NotConfigured }
    }
}

fn get_settings_path(module_key: &str) -> String {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
    format!("{}\\Microsoft\\PowerToys\\{}\\settings.json", local_app_data, module_key)
}

powertoys_module_ffi::register_module!(AlwaysOnTopModule::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creates() {
        let module = AlwaysOnTopModule::new();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_name_and_key() {
        let module = AlwaysOnTopModule::new();
        assert_eq!(wide_to_string(module.get_name()), "AlwaysOnTop");
        assert_eq!(wide_to_string(module.get_key()), "AlwaysOnTop");
    }

    #[test]
    fn test_default_hotkeys() {
        let module = AlwaysOnTopModule::new();
        let mut hotkeys = [Hotkey::default(); 3];
        let count = module.get_hotkeys(hotkeys.as_mut_ptr(), 3);
        assert_eq!(count, 3);
        // Pin: Win+Ctrl+T
        assert!(hotkeys[0].win);
        assert!(hotkeys[0].ctrl);
        assert_eq!(hotkeys[0].key, b'T');
        assert_eq!(hotkeys[0].id, 0);
        // Increase opacity: Win+Ctrl++
        assert_eq!(hotkeys[1].key, VK_OEM_PLUS);
        assert_eq!(hotkeys[1].id, 1);
        // Decrease opacity: Win+Ctrl+-
        assert_eq!(hotkeys[2].key, VK_OEM_MINUS);
        assert_eq!(hotkeys[2].id, 2);
    }

    #[test]
    fn test_on_hotkey_disabled() {
        let mut module = AlwaysOnTopModule::new();
        // Should return false when disabled
        assert!(!module.on_hotkey(0));
        assert!(!module.on_hotkey(1));
        assert!(!module.on_hotkey(2));
    }

    #[test]
    fn test_gpo_default() {
        let module = AlwaysOnTopModule::new();
        assert_eq!(module.gpo_policy_enabled_configuration(), GpoRuleConfigured::NotConfigured);
    }

    #[test]
    fn test_config_roundtrip() {
        let module = AlwaysOnTopModule::new();
        let mut size: c_int = 0;
        module.get_config(std::ptr::null_mut(), &mut size);
        assert!(size > 0);

        let mut buf = vec![0u16; size as usize];
        let mut bs = size;
        assert!(module.get_config(buf.as_mut_ptr(), &mut bs));
        let json = wide_to_string(buf.as_ptr());
        assert!(json.contains("AlwaysOnTop"));
    }

    #[test]
    fn test_enable_disable_state() {
        let mut module = AlwaysOnTopModule::new();
        assert!(!module.is_enabled());
        // Direct state toggle (launch will fail without exe)
        module.enabled.store(true, Ordering::SeqCst);
        assert!(module.is_enabled());
        module.disable();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_register_macro_exports() {
        let table_ptr = rust_module_create();
        assert!(!table_ptr.is_null());
        unsafe {
            let table = &*table_ptr;
            let name = (table.get_name)(table.context);
            assert_eq!(wide_to_string(name), "AlwaysOnTop");
            (table.destroy)(table.context);
            let _ = Box::from_raw(table_ptr);
        }
    }
}
