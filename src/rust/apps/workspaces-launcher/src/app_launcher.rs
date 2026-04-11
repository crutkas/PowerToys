//! Per-app launch logic.
//!
//! Ported from AppLauncher.cpp.

use powertoys_win32::string::to_wide;
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SEE_MASK_NO_CONSOLE, SHELLEXECUTEINFOW,
};

use workspaces_core::data::Application;

use crate::registry_utils;

/// Result of launching an app.
pub struct LaunchResult {
    pub success: bool,
    pub process_id: u32,
    pub error: Option<String>,
}

/// Launch an application using the appropriate method.
pub fn launch(app: &Application) -> LaunchResult {
    // Try protocol launch for packaged apps
    if !app.package_full_name.is_empty() {
        // Get URI protocol names from registry
        let protocols = registry_utils::get_uri_protocol_names(&app.package_full_name);
        if let Some(protocol) = protocols.first() {
            let uri = if app.command_line_args.is_empty() {
                format!("{}:", protocol)
            } else {
                format!("{}:{}", protocol, app.command_line_args)
            };
            if let Some(result) = shell_execute(&uri, "", false) {
                if result.success {
                    return result;
                }
            }
        }

        // Try shell:AppsFolder launch
        if !app.app_user_model_id.is_empty() {
            let apps_folder = format!("shell:AppsFolder\\{}", app.app_user_model_id);
            if let Some(result) = shell_execute(&apps_folder, &app.command_line_args, false) {
                if result.success {
                    return result;
                }
            }
        }
    }

    // Steam protocol launch
    if app.app_user_model_id.contains("steam:") {
        let steam_uri = &app.app_user_model_id;
        if let Some(result) = shell_execute(steam_uri, &app.command_line_args, false) {
            if result.success {
                return result;
            }
        }
    }

    // PWA app launch
    if !app.pwa_app_id.is_empty() {
        // Try shell:AppsFolder first
        if !app.app_user_model_id.is_empty() {
            let apps_folder = format!("shell:AppsFolder\\{}", app.app_user_model_id);
            if let Some(result) = shell_execute(&apps_folder, "", false) {
                if result.success {
                    return result;
                }
            }
        }

        // Try direct browser launch with PWA args
        let path_lower = app.path.to_lowercase();
        let (proxy_exe, profile_dir) = if path_lower.contains("msedge.exe") {
            // Edge PWA: use msedge_proxy.exe
            let proxy = app.path.replace("msedge.exe", "msedge_proxy.exe");
            (proxy, "Default")
        } else if path_lower.contains("chrome.exe") {
            // Chrome PWA: use chrome_proxy.exe
            let proxy = app.path.replace("chrome.exe", "chrome_proxy.exe");
            (proxy, "Default")
        } else {
            (app.path.clone(), "Default")
        };

        let pwa_args = format!(
            "--profile-directory={} --app-id={}",
            profile_dir, app.pwa_app_id
        );
        if let Some(result) = shell_execute(&proxy_exe, &pwa_args, false) {
            if result.success {
                return result;
            }
        }
    }

    // Direct file launch
    if app.path.is_empty() {
        return LaunchResult {
            success: false,
            process_id: 0,
            error: Some("Empty application path".to_string()),
        };
    }

    if !std::path::Path::new(&app.path).exists() {
        return LaunchResult {
            success: false,
            process_id: 0,
            error: Some(format!("File not found: {}", app.path)),
        };
    }

    shell_execute(&app.path, &app.command_line_args, app.is_elevated).unwrap_or(LaunchResult {
        success: false,
        process_id: 0,
        error: Some("ShellExecuteEx failed".to_string()),
    })
}

/// Launch via ShellExecuteExW.
fn shell_execute(path: &str, args: &str, elevated: bool) -> Option<LaunchResult> {
    let parent_dir = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let verb = if elevated { "runas" } else { "open" };

    let wide_verb = to_wide(verb);
    let wide_file = to_wide(path);
    let wide_args = to_wide(args);
    let wide_dir = to_wide(&parent_dir);

    let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    sei.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NO_CONSOLE;
    sei.lpVerb = wide_verb.as_ptr();
    sei.lpFile = wide_file.as_ptr();
    sei.lpParameters = wide_args.as_ptr();
    sei.lpDirectory = wide_dir.as_ptr();
    sei.nShow = 1; // SW_SHOWNORMAL

    let ok = unsafe { ShellExecuteExW(&mut sei) };
    if ok == 0 {
        let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
        return Some(LaunchResult {
            success: false,
            process_id: 0,
            error: Some(format!("ShellExecuteEx failed: error {}", err)),
        });
    }

    let process_id = if !sei.hProcess.is_null() {
        let pid = unsafe { windows_sys::Win32::System::Threading::GetProcessId(sei.hProcess) };
        unsafe { CloseHandle(sei.hProcess); }
        pid
    } else {
        0
    };

    Some(LaunchResult {
        success: true,
        process_id,
        error: None,
    })
}
