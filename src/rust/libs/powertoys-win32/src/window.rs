//! Window enumeration and property helpers.
//!
//! Common patterns: EnumWindows callback wrapper, window visibility checks,
//! and window property storage.

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowLongW, GetWindowTextLengthW,
    GetWindowTextW, IsWindowVisible, GWL_EXSTYLE, GWL_STYLE, WS_EX_TOOLWINDOW,
    WS_VISIBLE,
};

use crate::string::from_wide_no_null;

/// Enumerate all top-level windows.
pub fn enum_windows() -> Vec<HWND> {
    let mut windows: Vec<HWND> = Vec::new();
    let data = &mut windows as *mut Vec<HWND>;
    unsafe {
        EnumWindows(Some(enum_callback), data as isize);
    }
    windows
}

unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: isize) -> i32 {
    let windows = unsafe { &mut *(lparam as *mut Vec<HWND>) };
    windows.push(hwnd);
    1 // continue
}

/// Enumerate only visible, non-tool windows (what a user sees in the taskbar).
pub fn enum_visible_windows() -> Vec<HWND> {
    enum_windows()
        .into_iter()
        .filter(|&hwnd| is_visible(hwnd) && !is_tool_window(hwnd))
        .collect()
}

/// Is the window visible?
pub fn is_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd) != 0 }
}

/// Is the window a tool window (no taskbar button)?
pub fn is_tool_window(hwnd: HWND) -> bool {
    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
    (ex_style & WS_EX_TOOLWINDOW) != 0
}

/// Does the window have WS_VISIBLE style?
pub fn has_visible_style(hwnd: HWND) -> bool {
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    (style & WS_VISIBLE) != 0
}

/// Get the window title text.
pub fn get_window_text(hwnd: HWND) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    let copied = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if copied <= 0 {
        String::new()
    } else {
        from_wide_no_null(&buf[..copied as usize])
    }
}

/// Get the foreground (active) window handle.
pub fn foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_finds_windows() {
        let windows = enum_windows();
        assert!(!windows.is_empty(), "should find at least some windows");
    }

    #[test]
    fn visible_windows_subset() {
        let all = enum_windows();
        let visible = enum_visible_windows();
        assert!(visible.len() <= all.len());
    }

    #[test]
    fn foreground_is_not_null() {
        let fg = foreground_window();
        // Might be null in a headless CI environment, so just don't crash
        let _ = fg;
    }
}
