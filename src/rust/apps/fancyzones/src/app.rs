//! FancyZonesApp: owns the engine, manages hooks, overlay lifecycle, and message loop.

use std::cell::RefCell;
use std::ptr;

use fancyzones_engine::engine::FancyZonesEngine;
use fancyzones_engine::overlay::{OverlayColors, ZoneOverlay};
use fancyzones_engine::snap::win32;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::hooks::{self, WinEventHookGuard, KeyboardHookGuard, WM_FZ_SNAP_HOTKEY};

/// Custom window messages posted by the WinEvent callback.
pub const WM_FZ_MOVESIZE_START: u32 = WM_APP + 1;
pub const WM_FZ_MOVESIZE_END: u32 = WM_APP + 2;
/// Custom message posted when display configuration changes.
pub const WM_FZ_DISPLAY_CHANGE: u32 = WM_APP + 4;

/// Timer ID for mouse-position polling during drag.
const DRAG_TIMER_ID: usize = 1;
const DRAG_TIMER_MS: u32 = 16;

thread_local! {
    pub static APP_PTR: RefCell<*mut FancyZonesApp> = const { RefCell::new(ptr::null_mut()) };
}

pub struct FancyZonesApp {
    engine: FancyZonesEngine,
    overlays: Vec<ZoneOverlay>,
    move_size_hooks: Vec<WinEventHookGuard>,
    keyboard_hook: Option<KeyboardHookGuard>,
    dragged_hwnd: HWND,
    msg_hwnd: HWND,
}

impl FancyZonesApp {
    pub fn new() -> Self {
        let mut engine = FancyZonesEngine::init();
        engine.update_work_areas();

        // Debug: log work area count
        let dbg = format!(
            "FZ Rust init: {} work areas, shift_drag={}\nMonitors: {}\n",
            engine.work_areas().len(),
            engine.settings().shift_drag,
            powertoys_win32::monitor::enum_monitors().len(),
        );
        for (i, wa) in engine.work_areas().iter().enumerate() {
            let _ = std::fs::write(
                format!(r"C:\Users\crutkas\AppData\Local\Temp\fz_rust_wa_{}.txt", i),
                format!("WorkArea {}: zones={}, rect={:?}", i, wa.zone_count(), wa.work_area_rect()),
            );
        }
        let _ = std::fs::write(r"C:\Users\crutkas\AppData\Local\Temp\fz_rust_debug.txt", &dbg);

        Self {
            engine,
            overlays: Vec::new(),
            move_size_hooks: Vec::new(),
            keyboard_hook: None,
            dragged_hwnd: ptr::null_mut(),
            msg_hwnd: ptr::null_mut(),
        }
    }

    /// Install hooks and run the message loop.
    pub fn run(&mut self) {
        self.msg_hwnd = create_message_window();
        APP_PTR.with(|p| *p.borrow_mut() = self as *mut _);

        self.move_size_hooks = hooks::install_move_size_hooks();
        self.keyboard_hook = hooks::install_keyboard_hook(
            self.engine.settings().override_snap_hotkeys
        );

        let _ = std::fs::write(r"C:\Users\crutkas\AppData\Local\Temp\fz_rust_hooks.txt",
            format!("Hooks: winevent={}, keyboard={}, override_snap={}",
                self.move_size_hooks.len(),
                self.keyboard_hook.is_some(),
                self.engine.settings().override_snap_hotkeys));

        // Standard Win32 message loop.
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
                match msg.message {
                    WM_FZ_MOVESIZE_START => self.on_move_size_start(msg.wParam as HWND),
                    WM_FZ_MOVESIZE_END => self.on_move_size_end(),
                    WM_FZ_SNAP_HOTKEY => self.on_snap_hotkey(msg.lParam as u32),
                    WM_FZ_DISPLAY_CHANGE => self.on_display_change(),
                    WM_TIMER if msg.wParam == DRAG_TIMER_ID => self.on_drag_timer(),
                    _ => {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        }
    }

    pub fn shutdown(&mut self) {
        self.hide_overlays();
        self.move_size_hooks.clear();
        self.keyboard_hook = None;
        self.engine.shutdown();
        APP_PTR.with(|p| *p.borrow_mut() = ptr::null_mut());

        if !self.msg_hwnd.is_null() {
            unsafe { DestroyWindow(self.msg_hwnd); }
            self.msg_hwnd = ptr::null_mut();
        }
    }

    // ---- Display change --------------------------------------------------

    fn on_display_change(&mut self) {
        self.hide_overlays();
        self.engine.update_work_areas();
    }

    // ---- Drag lifecycle --------------------------------------------------

    fn on_move_size_start(&mut self, hwnd: HWND) {
        if hwnd.is_null() { return; }
        self.dragged_hwnd = hwnd;
        self.engine.on_move_size_start(hwnd);

        // Always start the timer — Shift is checked continuously during drag
        if !self.msg_hwnd.is_null() {
            unsafe { SetTimer(self.msg_hwnd, DRAG_TIMER_ID, DRAG_TIMER_MS, None); }
        }
    }

    fn on_drag_timer(&mut self) {
        if !self.engine.is_dragging() {
            return;
        }
        if let Some((x, y)) = win32::get_cursor_pos() {
            let should_show = self.engine.on_mouse_move(x, y);
            if should_show {
                if self.overlays.is_empty() {
                    self.show_overlays();
                }
                self.update_overlay_highlight();
            } else if !self.overlays.is_empty() {
                self.hide_overlays();
            }
        }
    }

    fn on_snap_hotkey(&mut self, vk_code: u32) {
        // Win+Arrow intercepted by keyboard hook
        let fg = unsafe { GetForegroundWindow() };
        if fg.is_null() { return; }
        if let Some(snap_rect) = self.engine.on_snap_hotkey(fg, vk_code) {
            win32::snap_window_to_rect(fg, &snap_rect);
        }
    }

    fn on_move_size_end(&mut self) {
        // Stop the polling timer.
        if !self.msg_hwnd.is_null() {
            unsafe { KillTimer(self.msg_hwnd, DRAG_TIMER_ID); }
        }

        if let Some(snap_rect) = self.engine.on_move_size_end() {
            if !self.dragged_hwnd.is_null() {
                win32::snap_window_to_rect(self.dragged_hwnd, &snap_rect);
            }
        }

        self.hide_overlays();
        self.dragged_hwnd = ptr::null_mut();
    }

    // ---- Overlay management ----------------------------------------------

    fn show_overlays(&mut self) {
        self.hide_overlays();

        let colors = OverlayColors::from_hex(
            &self.engine.settings().zone_color,
            &self.engine.settings().zone_border_color,
            &self.engine.settings().zone_highlight_color,
            self.engine.settings().zone_highlight_opacity,
        );

        for wa in self.engine.work_areas() {
            let wa_rect = wa.work_area_rect();
            let w = wa_rect.width();
            let h = wa_rect.height();
            if w <= 0 || h <= 0 { continue; }

            if let Some(overlay) = ZoneOverlay::create(wa_rect.left, wa_rect.top, w, h) {
                overlay.render_zones(&wa.zone_rects_screen(), &wa_rect, &colors, None);
                overlay.show();
                self.overlays.push(overlay);
            }
        }
    }

    fn update_overlay_highlight(&mut self) {
        if self.overlays.is_empty() { return; }

        let colors = OverlayColors::from_hex(
            &self.engine.settings().zone_color,
            &self.engine.settings().zone_border_color,
            &self.engine.settings().zone_highlight_color,
            self.engine.settings().zone_highlight_opacity,
        );

        // Find which work area and zones are highlighted
        let active_info = self.engine.active_zone_info()
            .map(|(wi, zones)| (wi, zones.first().copied()));

        for (i, wa) in self.engine.work_areas().iter().enumerate() {
            if i >= self.overlays.len() { break; }
            let wa_rect = wa.work_area_rect();
            let highlight_zone = match active_info {
                Some((wi, zone_idx)) if wi == i => zone_idx.map(|z| z as usize),
                _ => None,
            };
            self.overlays[i].render_zones(&wa.zone_rects_screen(), &wa_rect, &colors, highlight_zone);
        }
    }

    fn hide_overlays(&mut self) {
        for overlay in &self.overlays {
            overlay.hide();
        }
        self.overlays.clear();
    }
}

// ---------------------------------------------------------------------------
// Hidden message-only window that receives WM_TIMER and our custom messages.
// ---------------------------------------------------------------------------

fn create_message_window() -> HWND {
    let class_name = powertoys_win32::string::to_wide("FancyZones_Rust_Msg");
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(msg_wnd_proc),
        lpszClassName: class_name.as_ptr(),
        hInstance: ptr::null_mut(),
        style: 0,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hIcon: ptr::null_mut(),
        hCursor: ptr::null_mut(),
        hbrBackground: ptr::null_mut(),
        lpszMenuName: ptr::null(),
        hIconSm: ptr::null_mut(),
    };
    unsafe { RegisterClassExW(&wc); }

    let class_name = powertoys_win32::string::to_wide("FancyZones_Rust_Msg");
    unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            ptr::null(),
            0,
            0, 0, 0, 0,
            HWND_MESSAGE,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    }
}

unsafe extern "system" fn msg_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match msg {
        WM_DISPLAYCHANGE => {
            unsafe {
                PostThreadMessageW(GetCurrentThreadId(), WM_FZ_DISPLAY_CHANGE, 0, 0);
            }
            0
        }
        WM_SETTINGCHANGE if wparam == SPI_SETWORKAREA as usize => {
            unsafe {
                PostThreadMessageW(GetCurrentThreadId(), WM_FZ_DISPLAY_CHANGE, 0, 0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
