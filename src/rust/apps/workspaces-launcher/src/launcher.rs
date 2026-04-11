//! Orchestrates the workspace launch.
//!
//! Ported from Launcher.cpp.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use workspaces_core::data::{Application, Workspace};
use workspaces_core::launch_status::{AppLaunchState, LaunchStatus};

use crate::app_launcher;
use crate::ipc::{self, IpcHelper};
use crate::launcher_ui_helper::LauncherUiHelper;
use crate::window_arranger_helper::WindowArrangerHelper;

/// Run the launcher for a workspace.
pub fn run(workspace: Workspace, _base_dir: &str, _is_launch_and_edit: bool) {
    let app_ids: Vec<String> = workspace.apps.iter().map(|a| a.id.clone()).collect();
    let launch_status = Arc::new(Mutex::new(LaunchStatus::new(&app_ids)));
    let start = Instant::now();
    let launched_successfully = Arc::new(Mutex::new(true));

    // Set up IPC to receive messages from Window Arranger
    let status_for_arranger = Arc::clone(&launch_status);
    let arranger_ipc = Arc::new(IpcHelper::new(
        ipc::LAUNCHER_ARRANGER_PIPE,
        ipc::WINDOW_ARRANGER_PIPE,
        move |msg| {
            handle_arranger_message(&msg, &status_for_arranger);
        },
    ));

    // Launch the LauncherUI process
    let ui_helper = LauncherUiHelper::launch();

    // Launch the WindowArranger process
    let arranger = WindowArrangerHelper::launch(&workspace.id);

    // Send initial status to UI
    update_ui_status(&ui_helper, &launch_status);

    // Wait for arranger "ready" (the C++ code uses IPC, but since we start
    // launch here directly, just give arranger time to initialize)
    std::thread::sleep(Duration::from_millis(500));

    // Launch apps sequentially
    let apps = workspace.apps.clone();
    let status_for_launch = Arc::clone(&launch_status);
    let arranger_for_launch = Arc::clone(&arranger_ipc);
    let launched_ok = Arc::clone(&launched_successfully);
    let ui_for_launch = ui_helper.clone();

    let launch_thread = std::thread::spawn(move || {
        launch_apps(
            &apps,
            &status_for_launch,
            &arranger_for_launch,
            &launched_ok,
            &ui_for_launch,
        );
    });

    // Wait for completion: all apps launched and moved, or timeout
    let timeout = Duration::from_secs(30);
    let five_secs = Duration::from_secs(5);
    let poll = Duration::from_millis(100);

    loop {
        std::thread::sleep(poll);

        let status = launch_status.lock().unwrap();
        if status.all_done() {
            break;
        }
        if status.all_launched() && start.elapsed() > five_secs {
            break;
        }
        drop(status);

        if start.elapsed() > timeout {
            break;
        }
    }

    // Wait for launch thread to finish
    let _ = launch_thread.join();

    // Update last launched time
    let elapsed = start.elapsed();
    eprintln!(
        "Workspaces Launcher: completed in {:.1}s",
        elapsed.as_secs_f64()
    );

    // Terminate UI and arranger
    drop(ui_helper);
    drop(arranger);

    let status = launch_status.lock().unwrap();
    eprintln!(
        "Workspaces Launcher: {} moved, {} failed, {} pending",
        status.moved_count(),
        status.failed_count(),
        status.pending_count()
    );
}

/// Launch all apps in sequence.
fn launch_apps(
    apps: &[Application],
    status: &Arc<Mutex<LaunchStatus>>,
    arranger_ipc: &Arc<IpcHelper>,
    launched_ok: &Arc<Mutex<bool>>,
    ui_helper: &Option<LauncherUiHelper>,
) {
    for app in apps {
        // Wait for previous instances of same app to be moved (max 3 seconds)
        let wait_start = Instant::now();
        let max_wait = Duration::from_millis(3000);
        while wait_start.elapsed() < max_wait {
            let s = status.lock().unwrap();
            // Check if there are any Launched (but not yet Moved) apps with same name
            let has_pending_same_app = apps
                .iter()
                .any(|a| {
                    a.id != app.id
                        && a.name == app.name
                        && s.get_state(&a.id) == Some(AppLaunchState::Launched)
                });
            drop(s);
            if !has_pending_same_app {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        // Extra wait if we timed out (Outlook workaround)
        if wait_start.elapsed() >= max_wait {
            std::thread::sleep(Duration::from_millis(1000));
        }

        eprintln!("Workspaces Launcher: launching '{}'", app.name);
        let result = app_launcher::launch(app);

        let mut s = status.lock().unwrap();
        if result.success {
            s.mark_launched(&app.id);
            drop(s);

            // Notify arranger
            let msg = serde_json::json!({
                "id": app.id,
                "state": "launched"
            });
            arranger_ipc.send(&msg.to_string());
        } else {
            s.mark_failed(&app.id);
            drop(s);

            *launched_ok.lock().unwrap() = false;
            if let Some(err) = &result.error {
                eprintln!(
                    "Workspaces Launcher: failed to launch '{}': {}",
                    app.name, err
                );
            }
        }

        update_ui_status(ui_helper, status);
    }
}

/// Handle IPC message from Window Arranger.
fn handle_arranger_message(msg: &str, status: &Arc<Mutex<LaunchStatus>>) {
    if msg == "ready" {
        eprintln!("Workspaces Launcher: arranger is ready");
        return;
    }

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(msg) {
        let app_id = val.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let state = val.get("state").and_then(|v| v.as_str()).unwrap_or("");

        let mut status = status.lock().unwrap();
        match state {
            "moved" => { status.mark_moved(app_id); }
            "failed" => { status.mark_failed(app_id); }
            _ => {}
        }
    }
}

/// Send status update to UI.
fn update_ui_status(ui_helper: &Option<LauncherUiHelper>, status: &Arc<Mutex<LaunchStatus>>) {
    if let Some(ui) = ui_helper {
        let s = status.lock().unwrap();
        let msg = serde_json::json!({
            "launched": s.launched_count(),
            "moved": s.moved_count(),
            "failed": s.failed_count(),
            "pending": s.pending_count(),
            "total": s.total(),
        });
        ui.send_status(&msg.to_string());
    }
}
