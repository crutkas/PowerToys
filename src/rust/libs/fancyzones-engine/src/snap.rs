use fancyzones_core::rect::Rect;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub fn snap_window_to_rect(hwnd: HWND, rect: &Rect) -> bool {
    unsafe { SetWindowPos(hwnd, std::ptr::null_mut(), rect.left, rect.top,
        rect.right - rect.left, rect.bottom - rect.top,
        SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW) != 0 }
}
pub fn restore_if_minimized(hwnd: HWND) {
    let s = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    if s & WS_MINIMIZE != 0 { unsafe { ShowWindow(hwnd, SW_RESTORE); } }
}
