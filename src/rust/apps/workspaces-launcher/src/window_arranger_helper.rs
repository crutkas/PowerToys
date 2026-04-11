//! WindowArranger helper — launches and manages the WindowArranger process.
//!
//! Ported from WindowArrangerHelper.cpp.

use powertoys_win32::process::ProcessHandle;
use powertoys_win32::string::to_wide;

/// Manages the WindowArranger process.
pub struct WindowArrangerHelper {
    process_id: u32,
}

impl WindowArrangerHelper {
    /// Launch the WindowArranger process with the given workspace ID.
    pub fn launch(workspace_id: &str) -> Option<Self> {
        use windows_sys::Win32::UI::Shell::{
            ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        };

        let exe_dir = std::env::current_exe()
            .ok()?
            .parent()?
            .to_path_buf();

        let arranger_exe = exe_dir.join("PowerToys_WorkspacesWindowArranger.exe");
        let arranger_path = arranger_exe.to_string_lossy();

        let wide_verb = to_wide("open");
        let wide_exe = to_wide(&arranger_path);
        let wide_args = to_wide(workspace_id);

        let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        sei.fMask = SEE_MASK_NOCLOSEPROCESS;
        sei.lpVerb = wide_verb.as_ptr();
        sei.lpFile = wide_exe.as_ptr();
        sei.lpParameters = wide_args.as_ptr();
        sei.nShow = 1; // SW_SHOWNORMAL

        let ok = unsafe { ShellExecuteExW(&mut sei) };
        if ok == 0 {
            eprintln!("Workspaces Launcher: failed to launch WindowArranger");
            return None;
        }

        let pid = if !sei.hProcess.is_null() {
            let p = unsafe { windows_sys::Win32::System::Threading::GetProcessId(sei.hProcess) };
            unsafe { windows_sys::Win32::Foundation::CloseHandle(sei.hProcess); }
            p
        } else {
            0
        };

        eprintln!("Workspaces Launcher: WindowArranger launched (PID {})", pid);
        Some(WindowArrangerHelper { process_id: pid })
    }
}

impl Drop for WindowArrangerHelper {
    fn drop(&mut self) {
        if self.process_id != 0 {
            if let Some(handle) = ProcessHandle::open_for_terminate(self.process_id) {
                handle.terminate(0);
                eprintln!("Workspaces Launcher: WindowArranger terminated");
            }
        }
    }
}
