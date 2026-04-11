#![windows_subsystem = "windows"]

//! WorkspacesLauncher — launches apps in a workspace.

use powertoys_win32::mutex::AppMutex;
use powertoys_win32::settings;
use powertoys_win32::string::to_wide;
use workspaces_core::app_utils::{build_launch_args, AppData};
use workspaces_core::data::{self, Workspace};
use workspaces_core::launch_status::LaunchStatus;

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW,
};
use windows_sys::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SEE_MASK_NO_CONSOLE, SHELLEXECUTEINFOW,
};

unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(value: isize) -> i32;
    fn CoInitializeEx(reserved: *const std::ffi::c_void, coinit: u32) -> i32;
    fn CoUninitialize();
    fn CoCreateGuid(guid: *mut Guid) -> i32;
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

const INSTANCE_MUTEX_NAME: &str = r"Local\PowerToys_WorkspacesLauncher_InstanceMutex";

fn main() -> std::process::ExitCode {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hr = CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED);
        if hr < 0 {
            eprintln!("Workspaces Launcher: COM init failed: 0x{hr:08x}");
            return std::process::ExitCode::FAILURE;
        }
    }

    // Singleton mutex — only one launcher at a time.
    let mutex = AppMutex::create(INSTANCE_MUTEX_NAME);
    if let Some(ref m) = mutex {
        if m.already_running() {
            eprintln!("Workspaces Launcher: another instance is running");
            unsafe { CoUninitialize() };
            return std::process::ExitCode::SUCCESS;
        }
    }

    // Parse command line: workspace-id [invoke-point]
    let args: Vec<String> = std::env::args().collect();
    let workspace_id = match args.get(1) {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            eprintln!("Workspaces Launcher: missing workspace ID argument");
            unsafe { CoUninitialize() };
            return std::process::ExitCode::FAILURE;
        }
    };

    let invoke_point = args.get(2).map(|s| s.as_str()).unwrap_or("");
    let is_launch_and_edit = invoke_point == "LaunchAndEdit";

    let base_dir = settings::module_dir("Workspaces")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut workspace = match load_workspace(&base_dir, &workspace_id, is_launch_and_edit) {
        Some(ws) => ws,
        None => {
            eprintln!("Workspaces Launcher: workspace '{workspace_id}' not found");
            unsafe { CoUninitialize() };
            return std::process::ExitCode::FAILURE;
        }
    };

    eprintln!(
        "Workspaces Launcher: launching '{}' with {} apps",
        workspace.name,
        workspace.apps.len()
    );

    // Ensure all apps have IDs.
    for app in &mut workspace.apps {
        if app.id.is_empty() {
            app.id = generate_guid();
        }
    }

    let app_ids: Vec<String> = workspace.apps.iter().map(|a| a.id.clone()).collect();
    let mut status = LaunchStatus::new(&app_ids);

    // Launch each app.
    for app in &workspace.apps {
        let app_data = AppData {
            name: app.name.clone(),
            install_path: app.path.clone(),
            package_full_name: app.package_full_name.clone(),
            app_user_model_id: app.app_user_model_id.clone(),
            pwa_app_id: app.pwa_app_id.clone(),
            ..Default::default()
        };

        let launch_cmd = build_launch_args(&app_data, &app.command_line_args);
        eprintln!(
            "Workspaces Launcher: launching '{}' -> {}",
            app.name, launch_cmd.executable
        );

        let ok = if launch_cmd.use_shell_execute {
            shell_execute(&launch_cmd.executable, &launch_cmd.args, app.is_elevated)
        } else {
            create_process(&launch_cmd.executable, &launch_cmd.args)
        };

        if ok {
            status.mark_launched(&app.id);
        } else {
            status.mark_failed(&app.id);
            eprintln!("Workspaces Launcher: failed to launch '{}'", app.name);
        }

        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    eprintln!(
        "Workspaces Launcher: {} launched, {} failed",
        status.launched_count(),
        status.failed_count()
    );

    unsafe { CoUninitialize() };
    drop(mutex);
    std::process::ExitCode::SUCCESS
}

fn load_workspace(
    base_dir: &str,
    workspace_id: &str,
    is_launch_and_edit: bool,
) -> Option<Workspace> {
    if is_launch_and_edit {
        let temp_path = data::temp_workspaces_file(base_dir);
        if let Ok(workspaces) = workspaces_core::json_utils::read_workspaces_file(&temp_path) {
            if let Some(ws) = workspaces.into_iter().find(|w| w.id == workspace_id) {
                return Some(ws);
            }
        }
    }
    let main_path = data::workspaces_file(base_dir);
    if let Ok(workspaces) = workspaces_core::json_utils::read_workspaces_file(&main_path) {
        return workspaces.into_iter().find(|w| w.id == workspace_id);
    }
    None
}

const ERROR_CANCELLED: u32 = 1223;

fn shell_execute(path: &str, args: &str, elevated: bool) -> bool {
    let parent_dir = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    // First attempt with the requested verb
    if try_shell_execute(path, args, &parent_dir, elevated) {
        return true;
    }

    // If non-elevated launch failed, retry with elevation ("runas")
    if !elevated {
        let err = unsafe { GetLastError() };
        eprintln!("ShellExecuteEx failed (error {err}), retrying with elevation");
        if try_shell_execute(path, args, &parent_dir, true) {
            return true;
        }
    }

    // Report final failure
    let err = unsafe { GetLastError() };
    if err == ERROR_CANCELLED {
        eprintln!("User declined UAC prompt for '{path}'");
    } else {
        eprintln!("ShellExecuteEx failed: error {err}");
    }
    false
}

fn try_shell_execute(path: &str, args: &str, parent_dir: &str, elevated: bool) -> bool {
    let verb = if elevated { "runas" } else { "open" };
    let wide_verb = to_wide(verb);
    let wide_file = to_wide(path);
    let wide_args = to_wide(args);
    let wide_dir = to_wide(parent_dir);

    let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    sei.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NO_CONSOLE;
    sei.lpVerb = wide_verb.as_ptr();
    sei.lpFile = wide_file.as_ptr();
    sei.lpParameters = wide_args.as_ptr();
    sei.lpDirectory = wide_dir.as_ptr();
    sei.nShow = 1; // SW_SHOWNORMAL

    let ok = unsafe { ShellExecuteExW(&mut sei) };
    if ok != 0 {
        if !sei.hProcess.is_null() {
            unsafe { CloseHandle(sei.hProcess) };
        }
        true
    } else {
        false
    }
}

fn create_process(exe: &str, args: &str) -> bool {
    let cmd_line = format!("\"{}\" {}", exe, args);
    let mut wide_cmd: Vec<u16> = cmd_line.encode_utf16().chain(std::iter::once(0)).collect();

    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let ok = unsafe {
        CreateProcessW(
            std::ptr::null(),
            wide_cmd.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            0,
            std::ptr::null(),
            std::ptr::null(),
            &si,
            &mut pi,
        )
    };

    if ok != 0 {
        unsafe {
            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
        }
        true
    } else {
        let err = unsafe { GetLastError() };
        eprintln!("CreateProcess failed: error {err}");
        false
    }
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
