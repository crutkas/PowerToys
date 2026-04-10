//! Core AlwaysOnTop logic: event hooks, pin/unpin, border management.

use crate::border::WindowBorder;
use crate::settings::Settings;
use std::collections::HashMap;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::Accessibility::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const WM_PRIV_SETTINGS_CHANGED: u32 = WM_APP + 1;

// Named events from CommonSharedConstants
const PIN_EVENT: &str =
    "Local\\AlwaysOnTopPinEvent-892e0aa2-cfa8-4cc4-b196-ddeb32314ce8";
const TERMINATE_EVENT: &str =
    "Local\\AlwaysOnTopTerminateEvent-cfdf1eae-791f-4953-8021-2f18f3837eae";
const INCREASE_OPACITY_EVENT: &str =
    "Local\\AlwaysOnTopIncreaseOpacityEvent-a1b2c3d4-e5f6-7890-abcd-ef1234567890";
const DECREASE_OPACITY_EVENT: &str =
    "Local\\AlwaysOnTopDecreaseOpacityEvent-b2c3d4e5-f6a7-8901-bcde-f12345678901";

const WINDOW_CLASS: &str = "AlwaysOnTopWindow";
const PINNED_PROP: &str = "AlwaysOnTop_Pinned";

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub struct AlwaysOnTop {
    main_window: HWND,
    hinstance: HINSTANCE,
    settings: Settings,
    pinned_windows: HashMap<isize, WindowBorder>, // HWND → border
    event_handles: [*mut std::ffi::c_void; 4],     // pin, terminate, inc, dec
    win_event_hooks: Vec<*mut std::ffi::c_void>,
}

unsafe impl Send for AlwaysOnTop {}

// Global pointer for window proc callback
static mut AOT_INSTANCE: *mut AlwaysOnTop = std::ptr::null_mut();

impl AlwaysOnTop {
    pub fn new(settings: Settings) -> Option<Self> {
        let hinstance = unsafe {
            windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null())
        } as HINSTANCE;

        // Create named events
        let events = [
            create_event(PIN_EVENT),
            create_event(TERMINATE_EVENT),
            create_event(INCREASE_OPACITY_EVENT),
            create_event(DECREASE_OPACITY_EVENT),
        ];

        let mut aot = Self {
            main_window: std::ptr::null_mut(),
            hinstance,
            settings,
            pinned_windows: HashMap::new(),
            event_handles: events,
            win_event_hooks: Vec::new(),
        };

        if !aot.init_main_window() {
            return None;
        }

        // Subscribe to window events
        aot.subscribe_to_events();

        // Track already-topmost windows
        aot.start_tracking_topmost_windows();

        // Start event listener thread
        let pin_event = events[0] as usize;
        let terminate_event = events[1] as usize;
        let inc_event = events[2] as usize;
        let dec_event = events[3] as usize;
        let main_hwnd = aot.main_window as usize;

        std::thread::spawn(move || {
            event_listener_thread(
                pin_event as *mut std::ffi::c_void,
                terminate_event as *mut std::ffi::c_void,
                inc_event as *mut std::ffi::c_void,
                dec_event as *mut std::ffi::c_void,
                main_hwnd as HWND,
            );
        });

        unsafe { AOT_INSTANCE = &mut aot as *mut _ };

        Some(aot)
    }

    fn init_main_window(&mut self) -> bool {
        let class_name = to_wide(WINDOW_CLASS);

        unsafe {
            let mut wc: WNDCLASSEXW = std::mem::zeroed();
            wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
            wc.lpfnWndProc = Some(wnd_proc);
            wc.hInstance = self.hinstance;
            wc.lpszClassName = class_name.as_ptr();
            RegisterClassExW(&wc);

            self.main_window = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                std::ptr::null(),
                WS_POPUP,
                0, 0, 0, 0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                self.hinstance,
                std::ptr::null(),
            );
        }

        !self.main_window.is_null()
    }

    fn subscribe_to_events(&mut self) {
        let events_to_hook = [
            (EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE),
            (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZESTART),
            (EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZEEND),
            (EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZEEND),
            (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            (EVENT_OBJECT_DESTROY, EVENT_OBJECT_DESTROY),
            (EVENT_OBJECT_FOCUS, EVENT_OBJECT_FOCUS),
        ];

        for (min, max) in events_to_hook {
            let hook = unsafe {
                SetWinEventHook(
                    min,
                    max,
                    std::ptr::null_mut(),
                    Some(win_event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                )
            };
            if !hook.is_null() {
                self.win_event_hooks.push(hook);
            }
        }
    }

    fn start_tracking_topmost_windows(&mut self) {
        unsafe {
            EnumWindows(Some(enum_windows_proc), &mut self.pinned_windows as *mut _ as LPARAM);
        }
    }

    pub fn process_pin_command(&mut self) {
        let fg = unsafe { GetForegroundWindow() };
        if fg.is_null() {
            return;
        }

        if self.is_pinned(fg) {
            self.unpin_window(fg);
        } else {
            self.pin_window(fg);
        }
    }

    pub fn process_opacity_change(&mut self, delta: i32) {
        let fg = unsafe { GetForegroundWindow() };
        if fg.is_null() || !self.is_pinned(fg) {
            return;
        }
        self.step_window_transparency(fg, delta);
    }

    fn is_pinned(&self, hwnd: HWND) -> bool {
        self.pinned_windows.contains_key(&(hwnd as isize))
    }

    fn pin_window(&mut self, hwnd: HWND) {
        let prop_name = to_wide(PINNED_PROP);
        unsafe {
            SetPropW(hwnd, prop_name.as_ptr(), 1 as HANDLE);
            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }

        if self.settings.frame_enabled {
            if let Some(border) = WindowBorder::create(hwnd, self.hinstance, &self.settings) {
                self.pinned_windows.insert(hwnd as isize, border);
            }
        } else {
            self.pinned_windows.insert(hwnd as isize, WindowBorder::empty());
        }

        if self.settings.sound_enabled {
            play_sound();
        }
    }

    fn unpin_window(&mut self, hwnd: HWND) {
        let prop_name = to_wide(PINNED_PROP);
        unsafe {
            RemovePropW(hwnd, prop_name.as_ptr());
            SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }

        if let Some(border) = self.pinned_windows.remove(&(hwnd as isize)) {
            border.destroy();
        }
        self.restore_window_alpha(hwnd);

        if self.settings.sound_enabled {
            play_sound();
        }
    }

    pub fn unpin_all(&mut self) {
        let hwnds: Vec<isize> = self.pinned_windows.keys().cloned().collect();
        for hwnd in hwnds {
            self.unpin_window(hwnd as HWND);
        }
    }

    fn step_window_transparency(&self, hwnd: HWND, delta: i32) {
        // Get current alpha or default to 100%
        let current = self.get_window_alpha(hwnd).unwrap_or(100);
        let new_pct = (current + delta * 10).clamp(20, 100);
        self.apply_window_alpha(hwnd, new_pct);
    }

    fn get_window_alpha(&self, hwnd: HWND) -> Option<i32> {
        let mut alpha: u8 = 255;
        let mut flags: u32 = 0;
        unsafe {
            if GetLayeredWindowAttributes(hwnd, std::ptr::null_mut(), &mut alpha, &mut flags) != 0 {
                Some((alpha as i32 * 100) / 255)
            } else {
                None
            }
        }
    }

    fn apply_window_alpha(&self, hwnd: HWND, percentage: i32) {
        let alpha = ((percentage * 255) / 100) as u8;
        unsafe {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            if ex_style & (WS_EX_LAYERED as i32) == 0 {
                SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED as i32);
            }
            SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);
        }
    }

    fn restore_window_alpha(&self, hwnd: HWND) {
        unsafe {
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style & !(WS_EX_LAYERED as i32));
        }
    }

    pub fn handle_win_event(&mut self, event: u32, hwnd: HWND) {
        if hwnd.is_null() {
            return;
        }

        match event {
            EVENT_OBJECT_LOCATIONCHANGE | EVENT_SYSTEM_MOVESIZEEND => {
                // Update border position for tracked windows
                if let Some(border) = self.pinned_windows.get(&(hwnd as isize)) {
                    border.update_position(hwnd);
                }
            }
            EVENT_SYSTEM_MINIMIZESTART => {
                if let Some(border) = self.pinned_windows.get(&(hwnd as isize)) {
                    border.hide();
                }
            }
            EVENT_SYSTEM_MINIMIZEEND => {
                if let Some(border) = self.pinned_windows.get(&(hwnd as isize)) {
                    border.show();
                    border.update_position(hwnd);
                }
            }
            EVENT_OBJECT_DESTROY => {
                // Window was destroyed — remove tracking
                if self.pinned_windows.contains_key(&(hwnd as isize)) {
                    self.pinned_windows.remove(&(hwnd as isize));
                }
            }
            EVENT_SYSTEM_FOREGROUND | EVENT_OBJECT_FOCUS => {
                // Re-apply topmost to ensure it sticks
                if self.is_pinned(hwnd) {
                    unsafe {
                        SetWindowPos(
                            hwnd, HWND_TOPMOST,
                            0, 0, 0, 0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }
                // Refresh all borders (virtual desktop changes)
                self.refresh_borders();
            }
            _ => {}
        }
    }

    fn refresh_borders(&self) {
        for (&hwnd_key, border) in &self.pinned_windows {
            let hwnd = hwnd_key as HWND;
            unsafe {
                if IsWindow(hwnd) != 0 && IsWindowVisible(hwnd) != 0 {
                    border.update_position(hwnd);
                    border.show();
                } else {
                    border.hide();
                }
            }
        }
    }

    pub fn reload_settings(&mut self) {
        self.settings = Settings::load();
        // Update all borders with new settings
        for (&hwnd_key, border) in &self.pinned_windows {
            border.update_properties(hwnd_key as HWND, &self.settings);
        }
    }

    pub fn cleanup(&mut self) {
        self.unpin_all();
        for hook in &self.win_event_hooks {
            unsafe { UnhookWinEvent(*hook) };
        }
        self.win_event_hooks.clear();
    }
}

// ── Event listener thread ──────────────────────────────────────────────────

fn event_listener_thread(
    pin: *mut std::ffi::c_void,
    terminate: *mut std::ffi::c_void,
    increase: *mut std::ffi::c_void,
    decrease: *mut std::ffi::c_void,
    main_hwnd: HWND,
) {
    // Custom messages sent to the main window
    const WM_PIN: u32 = WM_APP + 100;
    const WM_INC_OPACITY: u32 = WM_APP + 101;
    const WM_DEC_OPACITY: u32 = WM_APP + 102;

    let handles = [pin, terminate, increase, decrease];
    loop {
        let result = unsafe {
            MsgWaitForMultipleObjects(4, handles.as_ptr(), 0, u32::MAX, QS_ALLINPUT)
        };

        match result {
            0 => {
                // Pin event
                unsafe {
                    ResetEvent(pin);
                    PostMessageW(main_hwnd, WM_PIN, 0, 0);
                }
            }
            1 => {
                // Terminate event
                let tid = *crate::MAIN_THREAD_ID.lock().unwrap();
                unsafe { PostThreadMessageW(tid, WM_QUIT, 0, 0) };
                return;
            }
            2 => {
                // Increase opacity
                unsafe {
                    ResetEvent(increase);
                    PostMessageW(main_hwnd, WM_INC_OPACITY, 0, 0);
                }
            }
            3 => {
                // Decrease opacity
                unsafe {
                    ResetEvent(decrease);
                    PostMessageW(main_hwnd, WM_DEC_OPACITY, 0, 0);
                }
            }
            4 => {
                // Windows message — pump it
                unsafe {
                    let mut msg: MSG = std::mem::zeroed();
                    while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
            _ => break,
        }
    }
}

// ── Window procedures ──────────────────────────────────────────────────────

const WM_PIN: u32 = WM_APP + 100;
const WM_INC_OPACITY: u32 = WM_APP + 101;
const WM_DEC_OPACITY: u32 = WM_APP + 102;

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let aot = unsafe { AOT_INSTANCE.as_mut() };
    match msg {
        WM_PIN => {
            if let Some(a) = aot { a.process_pin_command(); }
            0
        }
        WM_INC_OPACITY => {
            if let Some(a) = aot { a.process_opacity_change(1); }
            0
        }
        WM_DEC_OPACITY => {
            if let Some(a) = aot { a.process_opacity_change(-1); }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    // Only process window-level events (not child objects)
    if id_object != 0 {
        return;
    }
    if let Some(aot) = unsafe { AOT_INSTANCE.as_mut() } {
        aot.handle_win_event(event, hwnd);
    }
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let prop_name = to_wide(PINNED_PROP);
    let prop = unsafe { GetPropW(hwnd, prop_name.as_ptr()) };
    if !prop.is_null() {
        // Window is already marked as pinned — track it
        let map = unsafe { &mut *(lparam as *mut HashMap<isize, WindowBorder>) };
        map.insert(hwnd as isize, WindowBorder::empty());
    }
    1 // TRUE — continue enumeration
}

fn create_event(name: &str) -> *mut std::ffi::c_void {
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
    let wide = to_wide(name);
    unsafe {
        let mut sa: SECURITY_ATTRIBUTES = std::mem::zeroed();
        sa.nLength = std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32;
        CreateEventW(&sa, 0, 0, wide.as_ptr())
    }
}

fn play_sound() {
    // Play the default Windows notification sound
    // MessageBeep requires Win32_Media feature; just skip sound for now
}
