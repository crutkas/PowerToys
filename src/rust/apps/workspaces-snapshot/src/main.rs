#![windows_subsystem = "windows"]

//! WorkspacesSnapshotTool — captures current window layout as a workspace project.

use powertoys_win32::monitor::{self, MonitorInfo};
use powertoys_win32::settings;
use powertoys_win32::string::from_wide;
use powertoys_win32::window;
use workspaces_core::data::{Application, Position, Workspace};

use windows_sys::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, GetWindowPlacement, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    IsZoomed, GWL_STYLE, WINDOWPLACEMENT, WS_THICKFRAME,
};

unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(value: isize) -> i32;
}
const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

unsafe extern "system" {
    fn CoInitializeEx(reserved: *const std::ffi::c_void, coinit: u32) -> i32;
    fn CoUninitialize();
    fn CoCreateGuid(guid: *mut Guid) -> i32;
}
const COINIT_MULTITHREADED: u32 = 0;

#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

fn main() -> std::process::ExitCode {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hr = CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED);
        if hr < 0 {
            eprintln!("Workspaces Snapshot: COM init failed: 0x{hr:08x}");
            return std::process::ExitCode::FAILURE;
        }
    }

    let args: Vec<String> = std::env::args().collect();
    let invoke_point = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let is_launch_and_edit = invoke_point == "LaunchAndEdit";

    let workspace_id = generate_guid();
    let creation_time = current_timestamp();
    let apps = snapshot_apps(is_launch_and_edit);

    let workspace = Workspace {
        id: workspace_id,
        name: String::new(),
        creation_time,
        last_launched_time: String::new(),
        is_shortcut_needed: false,
        move_existing_windows: false,
        apps,
    };

    let base_dir = settings::module_dir("Workspaces")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp_path = workspaces_core::data::temp_workspaces_file(&base_dir);

    if let Some(parent) = std::path::Path::new(&temp_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let workspaces = vec![workspace];
    if let Err(e) = workspaces_core::json_utils::write_workspaces_file(&temp_path, &workspaces) {
        eprintln!("Workspaces Snapshot: failed to write {temp_path}: {e:?}");
        unsafe { CoUninitialize() };
        return std::process::ExitCode::FAILURE;
    }

    eprintln!(
        "Workspaces Snapshot: wrote {} apps to {}",
        workspaces[0].apps.len(),
        temp_path
    );
    unsafe { CoUninitialize() };
    std::process::ExitCode::SUCCESS
}

/// Snapshot all visible windows and build Application structs.
fn snapshot_apps(is_guid_needed: bool) -> Vec<Application> {
    let monitors = monitor::enum_monitors();
    let windows = window::enum_visible_windows();
    let mut apps = Vec::new();

    for hwnd in windows {
        let title = window::get_window_text(hwnd);
        if title.is_empty() {
            continue;
        }

        let rect = match get_window_rect_safe(hwnd) {
            Some(r) => r,
            None => continue,
        };

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let is_minimized = unsafe { IsIconic(hwnd) != 0 };
        if (width <= 0 || height <= 0) && !is_minimized {
            continue;
        }

        let process_path = match get_process_path(hwnd) {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };

        if is_excluded(&process_path) {
            continue;
        }

        let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
        if (style & WS_THICKFRAME) == 0 {
            continue;
        }

        let is_maximized = unsafe { IsZoomed(hwnd) != 0 };

        // Use restored placement for minimized/maximized windows.
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

        let monitor_num = monitor_for_window(hwnd, &monitors);
        let name = file_name_from_path(&process_path);
        let id = if is_guid_needed { generate_guid() } else { String::new() };

        apps.push(Application {
            id,
            name,
            title,
            path: process_path,
            package_full_name: String::new(),
            app_user_model_id: String::new(),
            pwa_app_id: String::new(),
            command_line_args: String::new(),
            is_elevated: false,
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

fn get_process_path(hwnd: HWND) -> Option<String> {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
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
    unsafe { CloseHandle(handle) };
    if ok != 0 && size > 0 {
        Some(from_wide(&buf[..size as usize]))
    } else {
        None
    }
}

fn get_window_rect_safe(hwnd: HWND) -> Option<RECT> {
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(hwnd, &mut rect) } != 0 {
        Some(rect)
    } else {
        None
    }
}

fn monitor_for_window(hwnd: HWND, monitors: &[MonitorInfo]) -> u32 {
    use windows_sys::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
    let hmon = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    monitors.iter().position(|m| m.handle == hmon).unwrap_or(0) as u32
}

fn is_excluded(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("applicationframehost.exe")
        || lower.contains("shellexperiencehost.exe")
        || lower.contains("startmenuexperiencehost.exe")
        || lower.contains("searchhost.exe")
        || lower.contains("lockapp.exe")
        || lower.contains("textinputhost.exe")
}

fn file_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn generate_guid() -> String {
    unsafe {
        let mut guid = Guid { data1: 0, data2: 0, data3: 0, data4: [0; 8] };
        CoCreateGuid(&mut guid);
        format!(
            "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
            guid.data1, guid.data2, guid.data3,
            guid.data4[0], guid.data4[1], guid.data4[2], guid.data4[3],
            guid.data4[4], guid.data4[5], guid.data4[6], guid.data4[7],
        )
    }
}

fn current_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format!("{}", duration.as_secs())
}
