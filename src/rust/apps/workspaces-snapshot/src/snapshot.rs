//! Snapshot utilities — enumerate visible windows and capture their layout.
//!
//! Ported from SnapshotUtils.cpp.

use powertoys_win32::monitor::{self, MonitorInfo};
use powertoys_win32::string::from_wide;
use powertoys_win32::window;
use workspaces_core::data::{Application, Position};
use workspaces_core::pwa_helper;

use windows_sys::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, GetWindowPlacement, GetWindowRect, GetWindowThreadProcessId,
    IsIconic, IsZoomed, GWL_STYLE, WINDOWPLACEMENT, WS_THICKFRAME,
};

// Token APIs
unsafe extern "system" {
    fn OpenProcessToken(
        process: windows_sys::Win32::Foundation::HANDLE,
        desired_access: u32,
        token: *mut windows_sys::Win32::Foundation::HANDLE,
    ) -> i32;
    fn GetTokenInformation(
        token: windows_sys::Win32::Foundation::HANDLE,
        info_class: u32,
        info: *mut std::ffi::c_void,
        info_len: u32,
        return_len: *mut u32,
    ) -> i32;
}
const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_ELEVATION_CLASS: u32 = 20; // TokenElevation

#[repr(C)]
struct TokenElevation {
    token_is_elevated: u32,
}

/// Get the process path for a window's owning process.
fn get_process_path(hwnd: HWND) -> Option<String> {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid); }
    if pid == 0 {
        return None;
    }

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }

    let mut buf = [0u16; 1024];
    let mut size = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };
    unsafe { CloseHandle(handle); }

    if ok != 0 && size > 0 {
        Some(from_wide(&buf[..size as usize]))
    } else {
        None
    }
}

/// Check if a process is elevated.
fn is_process_elevated(pid: u32) -> bool {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return false;
    }

    let mut token: windows_sys::Win32::Foundation::HANDLE = std::ptr::null_mut();
    let ok = unsafe { OpenProcessToken(handle, TOKEN_QUERY, &mut token) };
    if ok == 0 {
        unsafe { CloseHandle(handle); }
        return false;
    }

    let mut elevation = TokenElevation {
        token_is_elevated: 0,
    };
    let mut ret_len: u32 = 0;
    let ok = unsafe {
        GetTokenInformation(
            token,
            TOKEN_ELEVATION_CLASS,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TokenElevation>() as u32,
            &mut ret_len,
        )
    };
    unsafe {
        CloseHandle(token);
        CloseHandle(handle);
    }

    ok != 0 && elevation.token_is_elevated != 0
}

/// Determine which monitor a window is on (0-based index).
fn monitor_number_for_window(hwnd: HWND, monitors: &[MonitorInfo]) -> u32 {
    use windows_sys::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
    let hmon = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    monitors
        .iter()
        .position(|m| m.handle == hmon)
        .unwrap_or(0) as u32
}

/// Get the window rect.
fn get_window_rect_safe(hwnd: HWND) -> Option<RECT> {
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(hwnd, &mut rect) } != 0 {
        Some(rect)
    } else {
        None
    }
}

/// Has the WS_THICKFRAME (sizeable) style?
fn has_thick_frame(hwnd: HWND) -> bool {
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    (style & WS_THICKFRAME) != 0
}

/// Is the window minimized?
fn is_iconic(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd) != 0 }
}

/// Is the window maximized?
fn is_zoomed(hwnd: HWND) -> bool {
    unsafe { IsZoomed(hwnd) != 0 }
}

/// Extract the file name (without path) from a full path.
fn file_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Check if a path is excluded by default (e.g. shell, explorer desktop, etc.).
fn is_excluded_by_default(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("applicationframehost.exe")
        || lower.contains("shellexperiencehost.exe")
        || lower.contains("startmenuexperiencehost.exe")
        || lower.contains("searchhost.exe")
        || lower.contains("searchui.exe")
        || lower.contains("lockapp.exe")
        || lower.contains("cortana.exe")
        || lower.contains("textinputhost.exe")
}

/// Snapshot all visible windows and build Application structs.
pub fn get_apps(is_guid_needed: bool) -> Vec<Application> {
    let monitors = monitor::enum_monitors();
    let windows = window::enum_visible_windows();
    let mut apps = Vec::new();

    for hwnd in windows {
        // Skip windows with no title
        let title = window::get_window_text(hwnd);
        if title.is_empty() {
            continue;
        }

        // Get window rect
        let rect = match get_window_rect_safe(hwnd) {
            Some(r) => r,
            None => continue,
        };

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            if !is_iconic(hwnd) {
                continue;
            }
        }

        // Get process path
        let process_path = match get_process_path(hwnd) {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };

        // Skip excluded apps
        if is_excluded_by_default(&process_path) {
            continue;
        }

        // Skip non-sizable windows (unless they're special apps)
        let path_lower = process_path.to_lowercase();
        let is_steam = path_lower.contains("steam");
        if !has_thick_frame(hwnd) && !is_steam {
            continue;
        }

        // Get process elevation status
        let mut pid: u32 = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid); }
        let is_elevated = is_process_elevated(pid);

        // Window state
        let is_minimized = is_iconic(hwnd);
        let is_maximized = is_zoomed(hwnd);

        // Position: use restored placement for minimized/maximized windows
        let position = if is_minimized || is_maximized {
            let mut placement: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
            placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
            if unsafe { GetWindowPlacement(hwnd, &mut placement) } != 0 {
                let rc = placement.rcNormalPosition;
                Position {
                    x: rc.left,
                    y: rc.top,
                    width: rc.right - rc.left,
                    height: rc.bottom - rc.top,
                }
            } else {
                Position { x: rect.left, y: rect.top, width, height }
            }
        } else {
            Position { x: rect.left, y: rect.top, width, height }
        };

        // Monitor number
        let monitor_num = monitor_number_for_window(hwnd, &monitors);

        // Detect PWA (browser-based Progressive Web Apps)
        let browser = pwa_helper::browser_from_path(&process_path);
        let pwa_app_id = String::new(); // PWA detection requires profile parsing; stub for now

        // App name from filename
        let name = file_name_from_path(&process_path);

        let id = if is_guid_needed {
            crate::generate_guid()
        } else {
            String::new()
        };

        let _ = browser; // used for full PWA detection in future

        apps.push(Application {
            id,
            name,
            title,
            path: process_path,
            package_full_name: String::new(),
            app_user_model_id: String::new(),
            pwa_app_id,
            command_line_args: String::new(),
            is_elevated,
            can_launch_elevated: false,
            is_minimized,
            is_maximized,
            position,
            monitor: monitor_num,
        });
    }

    eprintln!("Workspaces Snapshot: found {} apps", apps.len());
    apps
}
