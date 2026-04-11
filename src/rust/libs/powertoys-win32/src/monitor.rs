//! Monitor enumeration and DPI helpers.
//!
//! Used by FancyZones (zone layout per monitor), AlwaysOnTop (border positioning),
//! and Workspaces (window arrangement).

use std::mem;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HMONITOR, MONITORINFOEXW, MONITORINFO,
    HDC,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics;

use crate::rect::Rect;

/// Information about a display monitor.
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub handle: HMONITOR,
    pub work_area: Rect,
    pub monitor_rect: Rect,
    pub is_primary: bool,
    pub device_name: String,
}

/// Enumerate all active monitors.
pub fn enum_monitors() -> Vec<MonitorInfo> {
    let mut monitors: Vec<MonitorInfo> = Vec::new();
    let data = &mut monitors as *mut Vec<MonitorInfo>;

    unsafe {
        EnumDisplayMonitors(
            0 as HDC,
            std::ptr::null(),
            Some(monitor_enum_proc),
            data as isize,
        );
    }
    monitors
}

unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprect: *mut RECT,
    lparam: isize,
) -> i32 {
    let monitors = unsafe { &mut *(lparam as *mut Vec<MonitorInfo>) };
    let mut info: MONITORINFOEXW = unsafe { mem::zeroed() };
    info.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

    if unsafe { GetMonitorInfoW(hmonitor, &mut info as *mut _ as *mut MONITORINFO) } != 0 {
        let work = info.monitorInfo.rcWork;
        let mon = info.monitorInfo.rcMonitor;
        let name = crate::string::from_wide(&info.szDevice);

        monitors.push(MonitorInfo {
            handle: hmonitor,
            work_area: Rect::new(work.left, work.top, work.right, work.bottom),
            monitor_rect: Rect::new(mon.left, mon.top, mon.right, mon.bottom),
            is_primary: (info.monitorInfo.dwFlags & 1) != 0, // MONITORINFOF_PRIMARY
            device_name: name,
        });
    }
    1 // continue enumeration
}

/// Get virtual screen bounds (all monitors combined).
pub fn virtual_screen() -> Rect {
    unsafe {
        Rect::new(
            GetSystemMetrics(76), // SM_XVIRTUALSCREEN
            GetSystemMetrics(77), // SM_YVIRTUALSCREEN
            GetSystemMetrics(76) + GetSystemMetrics(78), // + SM_CXVIRTUALSCREEN
            GetSystemMetrics(77) + GetSystemMetrics(79), // + SM_CYVIRTUALSCREEN
        )
    }
}

/// Get the primary monitor's work area.
pub fn primary_work_area() -> Option<Rect> {
    enum_monitors().into_iter().find(|m| m.is_primary).map(|m| m.work_area)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_at_least_one_monitor() {
        let monitors = enum_monitors();
        assert!(!monitors.is_empty(), "should have at least one monitor");
    }

    #[test]
    fn has_primary_monitor() {
        let monitors = enum_monitors();
        let primary_count = monitors.iter().filter(|m| m.is_primary).count();
        assert_eq!(primary_count, 1, "exactly one primary monitor");
    }

    #[test]
    fn monitor_rects_are_valid() {
        for m in enum_monitors() {
            assert!(m.monitor_rect.width() > 0);
            assert!(m.monitor_rect.height() > 0);
            assert!(m.work_area.width() > 0);
            assert!(m.work_area.height() > 0);
        }
    }

    #[test]
    fn virtual_screen_covers_all() {
        let vs = virtual_screen();
        assert!(vs.width() > 0);
        assert!(vs.height() > 0);
        for m in enum_monitors() {
            assert!(vs.left <= m.monitor_rect.left);
            assert!(vs.top <= m.monitor_rect.top);
            assert!(vs.right >= m.monitor_rect.right);
            assert!(vs.bottom >= m.monitor_rect.bottom);
        }
    }
}
