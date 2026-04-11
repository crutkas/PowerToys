#![windows_subsystem = "windows"]

//! WorkspacesSnapshotTool — captures current window layout as a workspace project.
//!
//! Ported from C++ WorkspacesSnapshotTool.

mod gpo;
mod snapshot;

use powertoys_win32::settings;
use workspaces_core::data;

// DPI awareness APIs not in all windows-sys feature sets — declare directly.
unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(value: isize) -> i32;
}
const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

// COM APIs
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
    if gpo::is_policy_disabled() {
        eprintln!("Workspaces: disabled by GPO policy");
        return std::process::ExitCode::FAILURE;
    }

    // Set DPI awareness and initialize COM
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hr = CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED);
        if hr < 0 {
            eprintln!("Workspaces Snapshot: COM init failed: 0x{:08x}", hr);
            return std::process::ExitCode::FAILURE;
        }
    }

    // Parse command line: determine invoke point
    let args: Vec<String> = std::env::args().collect();
    let invoke_point = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let is_launch_and_edit = invoke_point == "LaunchAndEdit";

    // Create workspace with a unique ID and timestamp
    let workspace_id = generate_guid();
    let creation_time = current_timestamp();

    let apps = snapshot::get_apps(is_launch_and_edit);

    let workspace = data::Workspace {
        id: workspace_id,
        name: String::new(),
        creation_time,
        last_launched_time: String::new(),
        is_shortcut_needed: false,
        move_existing_windows: false,
        apps,
    };

    // Write to temp-workspaces.json
    let base_dir = settings::module_dir("Workspaces")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp_path = data::temp_workspaces_file(&base_dir);

    // Ensure directory exists
    if let Some(parent) = std::path::Path::new(&temp_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let workspaces = vec![workspace];
    if let Err(e) = workspaces_core::json_utils::write_workspaces_file(&temp_path, &workspaces) {
        eprintln!("Workspaces Snapshot: failed to write {temp_path}: {e:?}");
        unsafe { CoUninitialize(); }
        return std::process::ExitCode::FAILURE;
    }

    eprintln!(
        "Workspaces Snapshot: wrote {} apps to {}",
        workspaces[0].apps.len(),
        temp_path
    );
    unsafe { CoUninitialize(); }
    std::process::ExitCode::SUCCESS
}

/// Generate a GUID string using CoCreateGuid.
pub fn generate_guid() -> String {
    unsafe {
        let mut guid = Guid {
            data1: 0,
            data2: 0,
            data3: 0,
            data4: [0; 8],
        };
        CoCreateGuid(&mut guid);
        format!(
            "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
            guid.data1, guid.data2, guid.data3,
            guid.data4[0], guid.data4[1], guid.data4[2], guid.data4[3],
            guid.data4[4], guid.data4[5], guid.data4[6], guid.data4[7],
        )
    }
}

/// Current timestamp as ISO 8601 string (simplified).
fn current_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    format!("{}", secs)
}
