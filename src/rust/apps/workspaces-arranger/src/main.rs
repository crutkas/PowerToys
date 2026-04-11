#![windows_subsystem = "windows"]

//! WorkspacesWindowArranger — arranges windows to match a workspace layout.
//!
//! Ported from C++ WorkspacesWindowArranger.

mod gpo;
mod ipc;
mod window_arranger;

use powertoys_win32::settings;
use workspaces_core::data;

unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(value: isize) -> i32;
}
const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

fn main() -> std::process::ExitCode {
    if gpo::is_policy_disabled() {
        eprintln!("Workspaces Arranger: disabled by GPO policy");
        return std::process::ExitCode::FAILURE;
    }

    // Set DPI awareness
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // Parse command line: workspace-id [invoke-point]
    let args: Vec<String> = std::env::args().collect();
    let workspace_id = match args.get(1) {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            eprintln!("Workspaces Arranger: missing workspace ID argument");
            return std::process::ExitCode::FAILURE;
        }
    };

    // Load workspace
    let base_dir = settings::module_dir("Workspaces")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let workspace = load_workspace(&base_dir, &workspace_id);
    let workspace = match workspace {
        Some(ws) => ws,
        None => {
            eprintln!("Workspaces Arranger: workspace '{}' not found", workspace_id);
            return std::process::ExitCode::FAILURE;
        }
    };

    eprintln!(
        "Workspaces Arranger: arranging workspace '{}' with {} apps",
        workspace.name,
        workspace.apps.len()
    );

    // Run the window arranger
    window_arranger::run(workspace);

    std::process::ExitCode::SUCCESS
}

/// Load workspace by ID — try temp file first, then main file.
fn load_workspace(base_dir: &str, workspace_id: &str) -> Option<data::Workspace> {
    // Try temp file first (for newly created/edited workspaces)
    let temp_path = data::temp_workspaces_file(base_dir);
    if let Ok(workspaces) = workspaces_core::json_utils::read_workspaces_file(&temp_path) {
        if let Some(ws) = workspaces.into_iter().find(|w| w.id == workspace_id) {
            return Some(ws);
        }
    }

    // Fall back to main workspaces.json
    let main_path = data::workspaces_file(base_dir);
    if let Ok(workspaces) = workspaces_core::json_utils::read_workspaces_file(&main_path) {
        if let Some(ws) = workspaces.into_iter().find(|w| w.id == workspace_id) {
            return Some(ws);
        }
    }

    None
}
