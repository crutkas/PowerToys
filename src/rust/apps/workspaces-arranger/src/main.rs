#![windows_subsystem = "windows"]

//! WorkspacesWindowArranger — arranges windows to match a workspace layout.

use std::collections::HashSet;

use powertoys_win32::settings;
use powertoys_win32::string::from_wide;
use powertoys_win32::window;
use workspaces_core::data::{self, Application, Workspace};
use workspaces_core::launch_status::LaunchStatus;
use workspaces_core::string_utils::case_insensitive_equals;

use windows_sys::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, GetWindowRect, GetWindowThreadProcessId, IsIconic, SetWindowPlacement,
    GWL_STYLE, SW_MAXIMIZE, SW_MINIMIZE, SW_SHOWNORMAL, WINDOWPLACEMENT, WS_THICKFRAME,
};

unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(value: isize) -> i32;
}
const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

fn main() -> std::process::ExitCode {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let args: Vec<String> = std::env::args().collect();
    let workspace_id = match args.get(1) {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            eprintln!("Workspaces Arranger: missing workspace ID argument");
            return std::process::ExitCode::FAILURE;
        }
    };

    let base_dir = settings::module_dir("Workspaces")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let workspace = match load_workspace(&base_dir, &workspace_id) {
        Some(ws) => ws,
        None => {
            eprintln!("Workspaces Arranger: workspace '{workspace_id}' not found");
            return std::process::ExitCode::FAILURE;
        }
    };

    eprintln!(
        "Workspaces Arranger: arranging '{}' with {} apps",
        workspace.name,
        workspace.apps.len()
    );

    let app_ids: Vec<String> = workspace.apps.iter().map(|a| a.id.clone()).collect();
    let mut status = LaunchStatus::new(&app_ids);
    let current_windows = window::enum_visible_windows();

    // Match and arrange windows using greedy distance-based matching.
    let mut matched_windows: HashSet<HWND> = HashSet::new();

    for app in &workspace.apps {
        let mut best_hwnd: Option<HWND> = None;
        let mut best_dist = f64::MAX;

        for &hwnd in &current_windows {
            if matched_windows.contains(&hwnd) {
                continue;
            }
            if !matches_app(hwnd, app) {
                continue;
            }
            let dist = window_distance(hwnd, app);
            if dist < best_dist {
                best_dist = dist;
                best_hwnd = Some(hwnd);
            }
        }

        if let Some(hwnd) = best_hwnd {
            if move_window(hwnd, app) {
                status.mark_launched(&app.id);
                status.mark_moved(&app.id);
                matched_windows.insert(hwnd);
                eprintln!("Workspaces Arranger: arranged '{}'", app.name);
            } else {
                status.mark_launched(&app.id);
                status.mark_failed(&app.id);
            }
        }
    }

    eprintln!(
        "Workspaces Arranger: {} moved, {} failed",
        status.moved_count(),
        status.failed_count()
    );

    std::process::ExitCode::SUCCESS
}

fn load_workspace(base_dir: &str, workspace_id: &str) -> Option<Workspace> {
    let temp_path = data::temp_workspaces_file(base_dir);
    if let Ok(workspaces) = workspaces_core::json_utils::read_workspaces_file(&temp_path) {
        if let Some(ws) = workspaces.into_iter().find(|w| w.id == workspace_id) {
            return Some(ws);
        }
    }
    let main_path = data::workspaces_file(base_dir);
    if let Ok(workspaces) = workspaces_core::json_utils::read_workspaces_file(&main_path) {
        return workspaces.into_iter().find(|w| w.id == workspace_id);
    }
    None
}

fn matches_app(hwnd: HWND, app: &Application) -> bool {
    let path = match get_process_path(hwnd) {
        Some(p) => p,
        None => return false,
    };

    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    if (style & WS_THICKFRAME) == 0 {
        return false;
    }

    let path_name = file_name_from_path(&path);
    let app_name_from_path = file_name_from_path(&app.path);

    case_insensitive_equals(&path_name, &app.name)
        || case_insensitive_equals(&path, &app.path)
        || case_insensitive_equals(&path_name, &app_name_from_path)
}

fn window_distance(hwnd: HWND, app: &Application) -> f64 {
    let is_min = unsafe { IsIconic(hwnd) != 0 };
    if is_min && app.is_minimized {
        return 0.0;
    }

    let mut rect: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return f64::MAX;
    }

    let pos = &app.position;
    let delta = ((rect.left - pos.x).abs()
        + (rect.top - pos.y).abs()
        + (rect.right - (pos.x + pos.width)).abs()
        + (rect.bottom - (pos.y + pos.height)).abs()) as f64;

    if is_min != app.is_minimized {
        10000.0 + delta
    } else {
        1.0 + delta
    }
}

fn move_window(hwnd: HWND, app: &Application) -> bool {
    let pos = &app.position;
    let mut placement: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
    placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;

    placement.showCmd = if app.is_minimized {
        SW_MINIMIZE as u32
    } else {
        SW_SHOWNORMAL as u32
    };

    placement.rcNormalPosition = RECT {
        left: pos.x,
        top: pos.y,
        right: pos.x + pos.width,
        bottom: pos.y + pos.height,
    };

    if unsafe { SetWindowPlacement(hwnd, &placement) } == 0 {
        return false;
    }

    // Windows needs the normal position set first before maximizing.
    if app.is_maximized && !app.is_minimized {
        placement.showCmd = SW_MAXIMIZE as u32;
        unsafe { SetWindowPlacement(hwnd, &placement) };
    }

    true
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

fn file_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}
