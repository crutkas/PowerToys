//! LauncherUI helper — launches and manages the LauncherUI process.
//!
//! Ported from LauncherUIHelper.cpp.

use powertoys_win32::process::ProcessHandle;
use powertoys_win32::string::to_wide;

use crate::ipc;

/// Manages the LauncherUI process.
#[derive(Clone)]
pub struct LauncherUiHelper {
    process_id: u32,
}

impl LauncherUiHelper {
    /// Launch the LauncherUI process (if it exists).
    pub fn launch() -> Option<Self> {
        use windows_sys::Win32::UI::Shell::{
            ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        };

        let exe_dir = std::env::current_exe()
            .ok()?
            .parent()?
            .to_path_buf();

        let ui_exe = exe_dir.join("PowerToys.WorkspacesLauncherUI.exe");
        if !ui_exe.exists() {
            eprintln!("Workspaces Launcher: UI executable not found at {:?}", ui_exe);
            return None;
        }

        let ui_path = ui_exe.to_string_lossy();
        let wide_verb = to_wide("open");
        let wide_exe = to_wide(&ui_path);

        let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        sei.fMask = SEE_MASK_NOCLOSEPROCESS;
        sei.lpVerb = wide_verb.as_ptr();
        sei.lpFile = wide_exe.as_ptr();
        sei.nShow = 1; // SW_SHOWNORMAL

        let ok = unsafe { ShellExecuteExW(&mut sei) };
        if ok == 0 {
            eprintln!("Workspaces Launcher: failed to launch UI");
            return None;
        }

        let pid = if !sei.hProcess.is_null() {
            let p = unsafe { windows_sys::Win32::System::Threading::GetProcessId(sei.hProcess) };
            unsafe { windows_sys::Win32::Foundation::CloseHandle(sei.hProcess); }
            p
        } else {
            0
        };

        eprintln!("Workspaces Launcher: UI launched (PID {})", pid);
        Some(LauncherUiHelper { process_id: pid })
    }

    /// Send a status update to the UI via IPC.
    pub fn send_status(&self, message: &str) {
        ipc::send_message(ipc::UI_PIPE, message);
    }
}

impl Drop for LauncherUiHelper {
    fn drop(&mut self) {
        if self.process_id != 0 {
            // Give UI a moment to show final status
            std::thread::sleep(std::time::Duration::from_secs(1));
            if let Some(handle) = ProcessHandle::open_for_terminate(self.process_id) {
                handle.terminate(0);
                eprintln!("Workspaces Launcher: UI terminated");
            }
        }
    }
}
