//! Core window arrangement logic.
//!
//! Ported from WindowArranger.cpp.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use powertoys_win32::monitor::{self, MonitorInfo};
use powertoys_win32::string::from_wide;
use powertoys_win32::window;
use workspaces_core::data::{Application, Workspace};
use workspaces_core::launch_status::{AppLaunchState, LaunchStatus};
use workspaces_core::string_utils::case_insensitive_equals;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, GetWindowRect, GetWindowThreadProcessId,
    IsIconic, SetWindowPlacement,
    GWL_STYLE, SW_MAXIMIZE, SW_MINIMIZE, SW_SHOWNORMAL,
    WINDOWPLACEMENT, WS_THICKFRAME,
};

use crate::ipc::{self, IpcHelper};

/// Run the window arranger for a given workspace.
pub fn run(workspace: Workspace) {
    let monitors = monitor::enum_monitors();
    let windows_before: Vec<HWND> = window::enum_visible_windows();

    // Initialize launch status tracking
    let app_ids: Vec<String> = workspace.apps.iter().map(|a| a.id.clone()).collect();
    let launch_status = Arc::new(Mutex::new(LaunchStatus::new(&app_ids)));

    // Set up IPC — listen for messages from Launcher
    let status_clone = Arc::clone(&launch_status);
    let ipc = IpcHelper::new(
        ipc::WINDOW_ARRANGER_PIPE,
        ipc::LAUNCHER_ARRANGER_PIPE,
        move |msg| {
            handle_launcher_message(&msg, &status_clone);
        },
    );

    // Send "ready" to launcher
    ipc.send("ready");

    // Phase 1: Move existing windows if configured
    if workspace.move_existing_windows {
        move_existing_windows(&workspace, &monitors);
    }

    // Phase 2: Wait for launched apps and position them (max 10 seconds)
    let mut timeout_counter = 0;
    let max_wait_ms = 10_000;
    let poll_interval_ms = 300;

    while timeout_counter < max_wait_ms {
        let current_windows = window::enum_visible_windows();
        let new_windows: Vec<HWND> = current_windows
            .into_iter()
            .filter(|w| !windows_before.contains(w))
            .collect();

        let mut moved_any = false;
        for hwnd in &new_windows {
            if process_window(*hwnd, &workspace, &launch_status, &monitors, &ipc) {
                moved_any = true;
            }
        }

        if moved_any {
            timeout_counter = 0; // reset timeout on successful moves
        } else {
            timeout_counter += poll_interval_ms;
        }

        // Check if all apps are done
        {
            let status = launch_status.lock().unwrap();
            if status.all_done() {
                break;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(poll_interval_ms as u64));
    }

    // Phase 3: Final reposition pass (max 3 seconds)
    let mut reposition_counter = 0;
    let max_reposition_ms = 3_000;
    while reposition_counter < max_reposition_ms {
        let current_windows = window::enum_visible_windows();
        let mut moved_any = false;
        for hwnd in &current_windows {
            if process_window(*hwnd, &workspace, &launch_status, &monitors, &ipc) {
                moved_any = true;
            }
        }
        if !moved_any {
            reposition_counter += poll_interval_ms;
        } else {
            reposition_counter = 0;
        }

        {
            let status = launch_status.lock().unwrap();
            if status.all_done() {
                break;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(poll_interval_ms as u64));
    }

    eprintln!("Workspaces Arranger: finished");
}

/// Handle an IPC message from the Launcher.
fn handle_launcher_message(msg: &str, status: &Arc<Mutex<LaunchStatus>>) {
    // Messages are JSON: {"id": "app-id", "state": "launched"}
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(msg) {
        let app_id = val.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let state = val.get("state").and_then(|v| v.as_str()).unwrap_or("");

        let mut status = status.lock().unwrap();
        match state {
            "launched" => { status.mark_launched(app_id); }
            "failed" => { status.mark_failed(app_id); }
            _ => {}
        }
    }
}

/// Move existing windows to match workspace positions (greedy distance-based matching).
fn move_existing_windows(workspace: &Workspace, monitors: &[MonitorInfo]) {
    let current_windows = window::enum_visible_windows();
    let mut moved_windows: HashSet<HWND> = HashSet::new();
    let mut matched_apps: HashSet<usize> = HashSet::new();

    // Greedy: repeatedly find the closest (app, window) pair and move it
    loop {
        let mut best_distance = f64::MAX;
        let mut best_app_idx: Option<usize> = None;
        let mut best_hwnd: Option<HWND> = None;

        for (app_idx, app) in workspace.apps.iter().enumerate() {
            if matched_apps.contains(&app_idx) {
                continue;
            }

            for &hwnd in &current_windows {
                if moved_windows.contains(&hwnd) {
                    continue;
                }

                if !matches_app(hwnd, app) {
                    continue;
                }

                let dist = calculate_distance(hwnd, app);
                if dist < best_distance {
                    best_distance = dist;
                    best_app_idx = Some(app_idx);
                    best_hwnd = Some(hwnd);
                }
            }
        }

        match (best_app_idx, best_hwnd) {
            (Some(app_idx), Some(hwnd)) => {
                let app = &workspace.apps[app_idx];
                if move_window(hwnd, app, monitors) {
                    eprintln!("Workspaces Arranger: moved existing window for '{}'", app.name);
                }
                moved_windows.insert(hwnd);
                matched_apps.insert(app_idx);
            }
            _ => break,
        }
    }

    if !moved_windows.is_empty() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Check if a window matches an app (by name or path).
fn matches_app(hwnd: HWND, app: &Application) -> bool {
    let path = match get_process_path(hwnd) {
        Some(p) => p,
        None => return false,
    };

    // Skip non-sizable windows
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    if (style & WS_THICKFRAME) == 0 {
        return false;
    }

    let path_name = file_name_from_path(&path);
    let app_name_from_path = file_name_from_path(&app.path);

    (case_insensitive_equals(&path_name, &app.name)
        || case_insensitive_equals(&path, &app.path)
        || case_insensitive_equals(&path_name, &app_name_from_path))
        && case_insensitive_equals(&String::new(), &app.pwa_app_id)
}

/// Calculate distance between a window's current position and the app's saved position.
fn calculate_distance(hwnd: HWND, app: &Application) -> f64 {
    let is_min = unsafe { IsIconic(hwnd) != 0 };

    // Both minimized = perfect match
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

/// Move a window to match the saved app position.
fn move_window(hwnd: HWND, app: &Application, _monitors: &[MonitorInfo]) -> bool {
    let pos = &app.position;

    let mut placement: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
    placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;

    // First pass: set normal position
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

    let ok = unsafe { SetWindowPlacement(hwnd, &placement) };
    if ok == 0 {
        return false;
    }

    // Second pass: handle maximize (Windows needs the normal position set first)
    if app.is_maximized && !app.is_minimized {
        placement.showCmd = SW_MAXIMIZE as u32;
        unsafe { SetWindowPlacement(hwnd, &placement); }
    }

    true
}

/// Process a single window — try to match it to a launched app and move it.
fn process_window(
    hwnd: HWND,
    workspace: &Workspace,
    launch_status: &Arc<Mutex<LaunchStatus>>,
    monitors: &[MonitorInfo],
    ipc: &IpcHelper,
) -> bool {
    let path = match get_process_path(hwnd) {
        Some(p) => p,
        None => return false,
    };

    // Skip non-sizable windows
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    if (style & WS_THICKFRAME) == 0 {
        return false;
    }

    // Skip windows with invalid rect
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return false;
    }
    if rect.right - rect.left <= 0 || rect.bottom - rect.top <= 0 {
        return false;
    }

    let path_name = file_name_from_path(&path);

    // Find matching app in workspace that is in Launched state
    let mut status = launch_status.lock().unwrap();
    let matched_app = workspace.apps.iter().find(|app| {
        let app_state = status.get_state(&app.id);
        if app_state != Some(AppLaunchState::Launched) {
            return false;
        }
        let app_name_from_path = file_name_from_path(&app.path);
        case_insensitive_equals(&path_name, &app.name)
            || case_insensitive_equals(&path, &app.path)
            || case_insensitive_equals(&path_name, &app_name_from_path)
    });

    if let Some(app) = matched_app {
        let app_id = app.id.clone();
        let app_name = app.name.clone();
        let moved = move_window(hwnd, app, monitors);
        if moved {
            status.mark_moved(&app_id);
            drop(status);
            // Notify launcher
            let msg = serde_json::json!({
                "id": app_id,
                "state": "moved"
            });
            ipc.send(&msg.to_string());
            eprintln!("Workspaces Arranger: moved window for '{}'", app_name);
            true
        } else {
            status.mark_failed(&app_id);
            drop(status);
            let msg = serde_json::json!({
                "id": app_id,
                "state": "failed"
            });
            ipc.send(&msg.to_string());
            false
        }
    } else {
        false
    }
}

/// Get the process path for a window's owning process.
fn get_process_path(hwnd: HWND) -> Option<String> {
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };

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
    unsafe { windows_sys::Win32::Foundation::CloseHandle(handle); }

    if ok != 0 && size > 0 {
        Some(from_wide(&buf[..size as usize]))
    } else {
        None
    }
}

/// Extract the file name (without extension) from a full path.
fn file_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}
