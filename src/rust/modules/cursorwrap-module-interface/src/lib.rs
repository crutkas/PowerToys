//! CursorWrap Module Interface — Rust implementation
//!
//! Implements PowertoyModuleIface for CursorWrap. Installs a WH_MOUSE_LL hook
//! to intercept mouse moves and wraps the cursor across monitor edges using
//! the pure-logic engine in `cursorwrap-core`.

use powertoys_module_ffi::*;
use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

use cursorwrap_core::types::{MonitorInfo, Point, Rect};
use cursorwrap_core::wrap_core::CursorWrapCore;

mod wide;
use wide::{to_wide, wide_to_string};

// Static wide strings (leaked once)
static MODULE_NAME: std::sync::LazyLock<&'static [u16]> =
    std::sync::LazyLock::new(|| Box::leak(to_wide("CursorWrap").into_boxed_slice()));
static MODULE_KEY: std::sync::LazyLock<&'static [u16]> =
    std::sync::LazyLock::new(|| Box::leak(to_wide("CursorWrap").into_boxed_slice()));

// Named event for CmdPal/automation integration
const TRIGGER_EVENT: &str =
    "Local\\PowerToysCursorWrapTriggerEvent-9b5c1f5e-3c7d-4a8b-b6e2-f1a2d3c4e5f6";

/// CursorWrap PowerToy module.
pub struct CursorWrapModule {
    enabled: AtomicBool,

    // Settings
    auto_activate: bool,
    disable_wrap_during_drag: bool,
    disable_on_single_monitor: bool,
    wrap_mode: i32,         // 0=Both, 1=VerticalOnly, 2=HorizontalOnly
    activation_mode: i32,   // 0=Always, 1=HoldingCtrl, 2=HoldingShift
    activation_hotkey: Hotkey,

    // Mouse hook
    mouse_hook: Option<*mut std::ffi::c_void>, // HHOOK
    hook_active: AtomicBool,

    // Core wrapping engine
    core: CursorWrapCore,

    // Event-driven trigger support
    trigger_event: *mut std::ffi::c_void,
    terminate_event: *mut std::ffi::c_void,
    event_thread: Option<std::thread::JoinHandle<()>>,
    listening: std::sync::Arc<AtomicBool>,
}

// SAFETY: Event handles and HHOOK are thread-safe Win32 kernel objects
unsafe impl Send for CursorWrapModule {}

// Global instance for mouse hook callback
static mut G_INSTANCE: *mut CursorWrapModule = std::ptr::null_mut();

impl CursorWrapModule {
    pub fn new() -> Self {
        let trigger_event = create_event(TRIGGER_EVENT);
        let terminate_event = create_event_unnamed();

        let mut module = Self {
            enabled: AtomicBool::new(false),
            auto_activate: true,
            disable_wrap_during_drag: true,
            disable_on_single_monitor: false,
            wrap_mode: 0,
            activation_mode: 0,
            activation_hotkey: Hotkey {
                win: true,
                ctrl: false,
                shift: false,
                alt: true,
                key: b'U',
                id: 0,
                is_shown: true,
            },
            mouse_hook: None,
            hook_active: AtomicBool::new(false),
            core: CursorWrapCore::new(),
            trigger_event,
            terminate_event,
            event_thread: None,
            listening: std::sync::Arc::new(AtomicBool::new(false)),
        };

        module.load_settings();
        module.update_monitor_info();
        module
    }

    fn update_monitor_info(&mut self) {
        let monitors = enumerate_monitors();
        self.core.initialize(&monitors);
    }

    fn load_settings(&mut self) {
        let settings_path = get_settings_path("Cursor Wrap");
        let json = match std::fs::read_to_string(&settings_path) {
            Ok(s) => s,
            Err(_) => return,
        };

        let parsed: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(_) => return,
        };

        if let Some(props) = parsed.get("properties") {
            if let Some(v) = props
                .get("auto_activate")
                .and_then(|o| o.get("value"))
                .and_then(|v| v.as_bool())
            {
                self.auto_activate = v;
            }
            if let Some(v) = props
                .get("disable_wrap_during_drag")
                .and_then(|o| o.get("value"))
                .and_then(|v| v.as_bool())
            {
                self.disable_wrap_during_drag = v;
            }
            if let Some(v) = props
                .get("wrap_mode")
                .and_then(|o| o.get("value"))
                .and_then(|v| v.as_i64())
            {
                self.wrap_mode = v as i32;
            }
            if let Some(v) = props
                .get("activation_mode")
                .and_then(|o| o.get("value"))
                .and_then(|v| v.as_i64())
            {
                self.activation_mode = v as i32;
            }
            if let Some(v) = props
                .get("disable_cursor_wrap_on_single_monitor")
                .and_then(|o| o.get("value"))
                .and_then(|v| v.as_bool())
            {
                self.disable_on_single_monitor = v;
            }

            // Parse hotkey
            if let Some(hk) = props
                .get("activation_shortcut")
                .and_then(|o| o.get("value"))
            {
                self.activation_hotkey = Hotkey {
                    win: hk.get("win").and_then(|v| v.as_bool()).unwrap_or(false),
                    ctrl: hk.get("ctrl").and_then(|v| v.as_bool()).unwrap_or(false),
                    shift: hk.get("shift").and_then(|v| v.as_bool()).unwrap_or(false),
                    alt: hk.get("alt").and_then(|v| v.as_bool()).unwrap_or(false),
                    key: hk.get("code").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                    id: 0,
                    is_shown: true,
                };
            }
        }

        // Set default hotkey if not configured
        if self.activation_hotkey.key == 0 {
            self.activation_hotkey = Hotkey {
                win: true,
                alt: true,
                ctrl: false,
                shift: false,
                key: b'U',
                id: 0,
                is_shown: true,
            };
        }
    }

    fn start_mouse_hook(&mut self) {
        if self.mouse_hook.is_some() || self.hook_active.load(Ordering::SeqCst) {
            return;
        }

        self.update_monitor_info();
        self.core.reset_wrap_state();

        unsafe {
            let hook = windows_sys::Win32::UI::WindowsAndMessaging::SetWindowsHookExW(
                windows_sys::Win32::UI::WindowsAndMessaging::WH_MOUSE_LL,
                Some(mouse_hook_proc),
                windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null()),
                0,
            );
            if !hook.is_null() {
                self.mouse_hook = Some(hook);
                self.hook_active.store(true, Ordering::SeqCst);
            }
        }
    }

    fn stop_mouse_hook(&mut self) {
        if let Some(hook) = self.mouse_hook.take() {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx(hook);
            }
            self.hook_active.store(false, Ordering::SeqCst);
        }
    }

    fn toggle_mouse_hook(&mut self) {
        if self.hook_active.load(Ordering::SeqCst) {
            self.stop_mouse_hook();
        } else {
            self.start_mouse_hook();
        }
    }
}

impl PowerToyModule for CursorWrapModule {
    fn get_name(&self) -> *const u16 {
        MODULE_NAME.as_ptr()
    }

    fn get_key(&self) -> *const u16 {
        MODULE_KEY.as_ptr()
    }

    fn enable(&mut self) {
        if self.enabled.load(Ordering::SeqCst) {
            return;
        }
        self.enabled.store(true, Ordering::SeqCst);

        unsafe {
            G_INSTANCE = self as *mut CursorWrapModule;
        }

        if self.auto_activate {
            self.start_mouse_hook();
        }
    }

    fn disable(&mut self) {
        self.enabled.store(false, Ordering::SeqCst);
        self.stop_mouse_hook();

        unsafe {
            G_INSTANCE = std::ptr::null_mut();
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    fn get_config(&self, buffer: *mut u16, buffer_size: *mut c_int) -> bool {
        let json = r#"{"name":"CursorWrap","version":"1.0"}"#;
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
        let _s = wide_to_string(config);
        // Reload settings from disk on config change
        self.load_settings();
        self.update_monitor_info();
    }

    fn destroy(&mut self) {
        self.disable();
    }

    fn get_hotkeys(&self, buffer: *mut Hotkey, buffer_size: usize) -> usize {
        if !buffer.is_null() && buffer_size >= 1 {
            unsafe {
                std::ptr::write(buffer, self.activation_hotkey);
            }
        }
        1
    }

    fn on_hotkey(&mut self, hotkey_id: usize) -> bool {
        if !self.enabled.load(Ordering::SeqCst) || hotkey_id != 0 {
            return false;
        }
        self.toggle_mouse_hook();
        true
    }

    fn is_enabled_by_default(&self) -> bool {
        false
    }

    fn gpo_policy_enabled_configuration(&self) -> GpoRuleConfigured {
        check_gpo("EnableCursorWrap")
    }
}

// ── Mouse hook callback ────────────────────────────────────────────────────

unsafe extern "system" fn mouse_hook_proc(
    n_code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, WM_MOUSEMOVE,
    };

    unsafe {
        if n_code >= 0 && w_param == WM_MOUSEMOVE as usize {
            if !G_INSTANCE.is_null() {
                let instance = &mut *G_INSTANCE;
                if instance.hook_active.load(Ordering::SeqCst) {
                    let ms = &*(l_param as *const windows_sys::Win32::UI::WindowsAndMessaging::MSLLHOOKSTRUCT);
                    let current_pos = Point {
                        x: ms.pt.x,
                        y: ms.pt.y,
                    };

                    // Check activation mode
                    let should_wrap = match instance.activation_mode {
                        1 => {
                            // HoldingCtrl
                            (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
                                windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL as i32,
                            ) & 0x8000u16 as i16)
                                != 0
                        }
                        2 => {
                            // HoldingShift
                            (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
                                windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_SHIFT as i32,
                            ) & 0x8000u16 as i16)
                                != 0
                        }
                        _ => true, // Always
                    };

                    if should_wrap {
                        let left_button_down = (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(
                            windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_LBUTTON as i32,
                        ) & 0x8000u16 as i16)
                            != 0;

                        let new_pos = instance.core.handle_mouse_move(
                            current_pos,
                            instance.disable_wrap_during_drag,
                            left_button_down,
                            instance.wrap_mode,
                            instance.disable_on_single_monitor,
                        );

                        if new_pos != current_pos {
                            windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos(
                                new_pos.x, new_pos.y,
                            );
                        }
                    }
                }
            }
        }

        CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn enumerate_monitors() -> Vec<MonitorInfo> {
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, MONITORINFO,
    };

    // MONITORINFOF_PRIMARY = 1
    const MONITORINFOF_PRIMARY: u32 = 1;

    let mut monitors: Vec<MonitorInfo> = Vec::new();

    unsafe extern "system" fn callback(
        _hmonitor: windows_sys::Win32::Graphics::Gdi::HMONITOR,
        _hdc: windows_sys::Win32::Graphics::Gdi::HDC,
        _lprect: *mut windows_sys::Win32::Foundation::RECT,
        lparam: isize,
    ) -> windows_sys::Win32::Foundation::BOOL {
        unsafe {
            let monitors = &mut *(lparam as *mut Vec<MonitorInfo>);
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(_hmonitor, &mut mi) != 0 {
                monitors.push(MonitorInfo {
                    rect: Rect {
                        left: mi.rcMonitor.left,
                        top: mi.rcMonitor.top,
                        right: mi.rcMonitor.right,
                        bottom: mi.rcMonitor.bottom,
                    },
                    is_primary: (mi.dwFlags & MONITORINFOF_PRIMARY) != 0,
                    monitor_id: monitors.len() as i32,
                });
            }
            1 // TRUE
        }
    }

    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(callback),
            &mut monitors as *mut Vec<MonitorInfo> as isize,
        );
    }

    monitors
}

fn create_event(name: &str) -> *mut std::ffi::c_void {
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
    use windows_sys::Win32::System::Threading::CreateEventW;

    let wide_name = to_wide(name);
    unsafe {
        let mut sa: SECURITY_ATTRIBUTES = std::mem::zeroed();
        sa.nLength = std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32;
        sa.bInheritHandle = 0;
        CreateEventW(&sa, 0, 0, wide_name.as_ptr())
    }
}

fn create_event_unnamed() -> *mut std::ffi::c_void {
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
    use windows_sys::Win32::System::Threading::CreateEventW;

    unsafe {
        let mut sa: SECURITY_ATTRIBUTES = std::mem::zeroed();
        sa.nLength = std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32;
        sa.bInheritHandle = 0;
        CreateEventW(&sa, 0, 0, std::ptr::null())
    }
}

fn check_gpo(value_name: &str) -> GpoRuleConfigured {
    use windows_sys::Win32::System::Registry::*;
    let subkey = to_wide("SOFTWARE\\Policies\\PowerToys");
    let val = to_wide(value_name);
    unsafe {
        let mut hkey = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        ) != 0
        {
            return GpoRuleConfigured::NotConfigured;
        }
        let mut data: u32 = 0;
        let mut data_size = std::mem::size_of::<u32>() as u32;
        let mut data_type: u32 = 0;
        let res = RegQueryValueExW(
            hkey,
            val.as_ptr(),
            std::ptr::null(),
            &mut data_type,
            &mut data as *mut u32 as *mut u8,
            &mut data_size,
        );
        RegCloseKey(hkey);
        if res != 0 || data_type != REG_DWORD {
            return GpoRuleConfigured::NotConfigured;
        }
        match data {
            1 => GpoRuleConfigured::Enabled,
            0 => GpoRuleConfigured::Disabled,
            _ => GpoRuleConfigured::NotConfigured,
        }
    }
}

fn get_settings_path(module_key: &str) -> String {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
    format!(
        "{}\\Microsoft\\PowerToys\\{}\\settings.json",
        local_app_data, module_key
    )
}

powertoys_module_ffi::register_module!(CursorWrapModule::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creates() {
        let module = CursorWrapModule::new();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_name_and_key() {
        let module = CursorWrapModule::new();
        assert_eq!(wide_to_string(module.get_name()), "CursorWrap");
        assert_eq!(wide_to_string(module.get_key()), "CursorWrap");
    }

    #[test]
    fn test_default_hotkey() {
        let module = CursorWrapModule::new();
        let mut hotkeys = [Hotkey::default(); 1];
        let count = module.get_hotkeys(hotkeys.as_mut_ptr(), 1);
        assert_eq!(count, 1);
        assert!(hotkeys[0].win);
        assert!(hotkeys[0].alt);
        assert_eq!(hotkeys[0].key, b'U');
    }

    #[test]
    fn test_on_hotkey_disabled() {
        let mut module = CursorWrapModule::new();
        assert!(!module.on_hotkey(0));
    }

    #[test]
    fn test_config_roundtrip() {
        let module = CursorWrapModule::new();
        let mut size: c_int = 0;
        module.get_config(std::ptr::null_mut(), &mut size);
        assert!(size > 0);

        let mut buf = vec![0u16; size as usize];
        let mut bs = size;
        assert!(module.get_config(buf.as_mut_ptr(), &mut bs));
        let json = wide_to_string(buf.as_ptr());
        assert!(json.contains("CursorWrap"));
    }

    #[test]
    fn test_is_enabled_by_default_false() {
        let module = CursorWrapModule::new();
        assert!(!module.is_enabled_by_default());
    }

    #[test]
    fn test_register_macro_exports() {
        let table_ptr = rust_module_create();
        assert!(!table_ptr.is_null());
        unsafe {
            let table = &*table_ptr;
            let name = (table.get_name)(table.context);
            assert_eq!(wide_to_string(name), "CursorWrap");
            (table.destroy)(table.context);
            let _ = Box::from_raw(table_ptr);
        }
    }
}
