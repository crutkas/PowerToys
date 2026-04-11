//! Window filtering for FancyZones — determines if a window should be snapped.
//! Ports FancyZonesWindowProcessing::DefineWindowType from C++.

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Why a window was rejected for snapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    Minimized,
    NotVisible,
    ToolWindow,
    NonRootWindow,
    NonProcessablePopup,
    ChildWindow,
    Excluded,
}

/// Check if a window is suitable for zone snapping.
/// Returns Ok(()) if processable, Err(reason) if not.
pub fn is_window_processable(hwnd: HWND, excluded_apps: &[String], allow_child_windows: bool) -> Result<(), RejectReason> {
    unsafe {
        // Minimized
        if IsIconic(hwnd) != 0 {
            return Err(RejectReason::Minimized);
        }

        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

        // Not visible
        if (style & WS_VISIBLE) == 0 {
            return Err(RejectReason::NotVisible);
        }

        // Tool window (no taskbar button)
        if (ex_style & WS_EX_TOOLWINDOW) != 0 {
            return Err(RejectReason::ToolWindow);
        }

        // Non-root window (child controls)
        if GetAncestor(hwnd, GA_ROOT) != hwnd {
            return Err(RejectReason::NonRootWindow);
        }

        // Non-processable popup: popup WITHOUT thick frame AND without caption/min/max
        let is_popup = (style & WS_POPUP) != 0;
        let has_thick_frame = (style & WS_THICKFRAME) != 0;
        let has_caption = (style & WS_CAPTION) != 0;
        let has_minmax = (style & WS_MINIMIZEBOX) != 0 || (style & WS_MAXIMIZEBOX) != 0;
        if is_popup && !(has_thick_frame && (has_caption || has_minmax)) {
            return Err(RejectReason::NonProcessablePopup);
        }

        // Child window with visible owner (unless allow_child_windows)
        if !allow_child_windows {
            let owner = GetWindow(hwnd, GW_OWNER);
            if !owner.is_null() && IsWindowVisible(owner) != 0 {
                return Err(RejectReason::ChildWindow);
            }
        }

        // Excluded apps
        if !excluded_apps.is_empty() {
            if let Some(exe_name) = get_exe_name(hwnd) {
                let exe_lower = exe_name.to_lowercase();
                for app in excluded_apps {
                    if exe_lower.contains(&app.to_lowercase()) {
                        return Err(RejectReason::Excluded);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Get the executable name for a window's process.
fn get_exe_name(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 { return None; }

        let handle = windows_sys::Win32::System::Threading::OpenProcess(
            windows_sys::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION, 0, pid
        );
        if handle.is_null() { return None; }

        let mut buf = [0u16; 260];
        let mut size = buf.len() as u32;
        let ok = windows_sys::Win32::System::Threading::QueryFullProcessImageNameW(
            handle, 0, buf.as_mut_ptr(), &mut size
        );
        windows_sys::Win32::Foundation::CloseHandle(handle);

        if ok == 0 { return None; }
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        path.rsplit('\\').next().map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_reason_values() {
        // Ensure all variants exist and are distinct
        let reasons = [
            RejectReason::Minimized,
            RejectReason::NotVisible,
            RejectReason::ToolWindow,
            RejectReason::NonRootWindow,
            RejectReason::NonProcessablePopup,
            RejectReason::ChildWindow,
            RejectReason::Excluded,
        ];
        for (i, a) in reasons.iter().enumerate() {
            for (j, b) in reasons.iter().enumerate() {
                if i != j { assert_ne!(a, b); }
            }
        }
    }

    #[test]
    fn null_hwnd_is_not_processable() {
        // Null window should fail one of the checks
        let result = is_window_processable(std::ptr::null_mut(), &[], false);
        assert!(result.is_err());
    }

    #[test]
    fn excluded_apps_matching() {
        // Test the string matching logic directly
        let excluded = vec!["notepad.exe".to_string(), "calc.exe".to_string()];
        let exe = "notepad.exe";
        let exe_lower = exe.to_lowercase();
        let matched = excluded.iter().any(|app| exe_lower.contains(&app.to_lowercase()));
        assert!(matched);
    }

    #[test]
    fn excluded_apps_no_match() {
        let excluded = vec!["notepad.exe".to_string()];
        let exe = "explorer.exe";
        let exe_lower = exe.to_lowercase();
        let matched = excluded.iter().any(|app| exe_lower.contains(&app.to_lowercase()));
        assert!(!matched);
    }

    #[test]
    fn excluded_apps_case_insensitive() {
        let excluded = vec!["Notepad.EXE".to_string()];
        let exe = "NOTEPAD.exe";
        let exe_lower = exe.to_lowercase();
        let matched = excluded.iter().any(|app| exe_lower.contains(&app.to_lowercase()));
        assert!(matched);
    }
}