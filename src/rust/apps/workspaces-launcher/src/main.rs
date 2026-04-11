#![windows_subsystem = "windows"]

//! WorkspacesLauncher — launches apps in a workspace and coordinates with window arranger.
//!
//! Ported from C++ WorkspacesLauncher.

mod app_launcher;
mod gpo;
mod ipc;
mod launcher;
mod launcher_ui_helper;
mod registry_utils;
mod window_arranger_helper;

use powertoys_win32::mutex::AppMutex;
use powertoys_win32::settings;
use workspaces_core::data;

unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(value: isize) -> i32;
    fn CoInitializeEx(reserved: *const std::ffi::c_void, coinit: u32) -> i32;
    fn CoUninitialize();
}
const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
const COINIT_MULTITHREADED: u32 = 0;

#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

unsafe extern "system" {
    fn CoCreateGuid(guid: *mut Guid) -> i32;
}

const INSTANCE_MUTEX_NAME: &str = r"Local\PowerToys_WorkspacesLauncher_InstanceMutex";

fn main() -> std::process::ExitCode {
    if gpo::is_policy_disabled() {
        eprintln!("Workspaces Launcher: disabled by GPO policy");
        return std::process::ExitCode::FAILURE;
    }

    // Set DPI awareness and initialize COM
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hr = CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED);
        if hr < 0 {
            eprintln!("Workspaces Launcher: COM init failed: 0x{:08x}", hr);
            return std::process::ExitCode::FAILURE;
        }
    }

    // Parse command line: workspace-id [invoke-point] [restarted]
    let args: Vec<String> = std::env::args().collect();
    let workspace_id = match args.get(1) {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            eprintln!("Workspaces Launcher: missing workspace ID argument");
            unsafe { CoUninitialize(); }
            return std::process::ExitCode::FAILURE;
        }
    };

    let invoke_point = args.get(2).map(|s| s.as_str()).unwrap_or("");
    let is_launch_and_edit = invoke_point == "LaunchAndEdit";
    let is_restarted = args.iter().any(|a| a == "restarted");

    // Check elevation — restart non-elevated if needed
    if !is_restarted && is_current_process_elevated() {
        eprintln!("Workspaces Launcher: elevated, restarting non-elevated");
        if restart_non_elevated(&args) {
            unsafe { CoUninitialize(); }
            return std::process::ExitCode::from(1);
        }
    }

    // Singleton mutex — only one launcher at a time
    let mutex = AppMutex::create(INSTANCE_MUTEX_NAME);
    if let Some(ref m) = mutex {
        if m.already_running() {
            eprintln!("Workspaces Launcher: another instance is running");
            unsafe { CoUninitialize(); }
            return std::process::ExitCode::SUCCESS;
        }
    }

    // Load workspace
    let base_dir = settings::module_dir("Workspaces")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let (_workspaces, workspace) = load_workspace(&base_dir, &workspace_id, is_launch_and_edit);
    let workspace = match workspace {
        Some(ws) => ws,
        None => {
            eprintln!("Workspaces Launcher: workspace '{}' not found", workspace_id);
            unsafe { CoUninitialize(); }
            return std::process::ExitCode::FAILURE;
        }
    };

    eprintln!(
        "Workspaces Launcher: launching workspace '{}' with {} apps",
        workspace.name,
        workspace.apps.len()
    );

    // Ensure all apps have IDs
    let mut workspace = workspace;
    let mut _updated = false;
    for app in &mut workspace.apps {
        if app.id.is_empty() {
            app.id = generate_guid();
            _updated = true;
        }
    }

    // Run the launcher
    launcher::run(workspace, &base_dir, is_launch_and_edit);

    // Cleanup
    unsafe { CoUninitialize(); }
    std::process::ExitCode::SUCCESS
}

/// Load workspace by ID.
fn load_workspace(
    base_dir: &str,
    workspace_id: &str,
    is_launch_and_edit: bool,
) -> (Vec<data::Workspace>, Option<data::Workspace>) {
    if is_launch_and_edit {
        let temp_path = data::temp_workspaces_file(base_dir);
        if let Ok(workspaces) = workspaces_core::json_utils::read_workspaces_file(&temp_path) {
            if let Some(ws) = workspaces.iter().find(|w| w.id == workspace_id).cloned() {
                return (workspaces, Some(ws));
            }
        }
    }

    let main_path = data::workspaces_file(base_dir);
    if let Ok(workspaces) = workspaces_core::json_utils::read_workspaces_file(&main_path) {
        let ws = workspaces.iter().find(|w| w.id == workspace_id).cloned();
        return (workspaces, ws);
    }

    (Vec::new(), None)
}

/// Check if the current process is elevated.
fn is_current_process_elevated() -> bool {
    unsafe extern "system" {
        fn GetCurrentProcess() -> windows_sys::Win32::Foundation::HANDLE;
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

    #[repr(C)]
    struct TokenElevation {
        token_is_elevated: u32,
    }

    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_ELEVATION_CLASS: u32 = 20;

    let handle = unsafe { GetCurrentProcess() };
    let mut token: windows_sys::Win32::Foundation::HANDLE = std::ptr::null_mut();
    let ok = unsafe { OpenProcessToken(handle, TOKEN_QUERY, &mut token) };
    if ok == 0 {
        return false;
    }

    let mut elevation = TokenElevation { token_is_elevated: 0 };
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
    unsafe { windows_sys::Win32::Foundation::CloseHandle(token); }

    ok != 0 && elevation.token_is_elevated != 0
}

/// Restart the process as non-elevated.
fn restart_non_elevated(args: &[String]) -> bool {
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };

    let exe_path = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().into_owned(),
        Err(_) => return false,
    };

    let mut new_args: Vec<String> = args.iter().skip(1).cloned().collect();
    new_args.push("restarted".to_string());
    let args_str = new_args.join(" ");

    let wide_verb = powertoys_win32::string::to_wide("open");
    let wide_exe = powertoys_win32::string::to_wide(&exe_path);
    let wide_args = powertoys_win32::string::to_wide(&args_str);

    let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpVerb = wide_verb.as_ptr();
    sei.lpFile = wide_exe.as_ptr();
    sei.lpParameters = wide_args.as_ptr();
    sei.nShow = 1; // SW_SHOWNORMAL

    unsafe { ShellExecuteExW(&mut sei) != 0 }
}

/// Generate a GUID string.
fn generate_guid() -> String {
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
