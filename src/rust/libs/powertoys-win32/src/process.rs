//! Process management: open, wait, terminate, query.
//!
//! Common patterns used by AlwaysOnTop (parent PID monitoring),
//! ActionRunner (non-elevated launch), and module DLLs (child EXE lifecycle).

use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE, WAIT_OBJECT_0,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcessId, GetExitCodeProcess, OpenProcess, TerminateProcess,
    WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    PROCESS_TERMINATE,
};

/// RAII wrapper around a process handle.
pub struct ProcessHandle {
    handle: HANDLE,
    pid: u32,
}

// SAFETY: process handles can be sent across threads.
unsafe impl Send for ProcessHandle {}

impl ProcessHandle {
    /// Open a process by PID with the specified access rights.
    fn open_with(pid: u32, access: u32) -> Option<Self> {
        let handle = unsafe { OpenProcess(access, 0, pid) };
        if handle.is_null() { None } else { Some(Self { handle, pid }) }
    }

    /// Open a process for synchronization (wait for exit).
    pub fn open_for_wait(pid: u32) -> Option<Self> {
        Self::open_with(pid, PROCESS_SYNCHRONIZE)
    }

    /// Open a process for termination.
    pub fn open_for_terminate(pid: u32) -> Option<Self> {
        Self::open_with(pid, PROCESS_TERMINATE | PROCESS_SYNCHRONIZE)
    }

    /// Open a process for querying information.
    pub fn open_for_query(pid: u32) -> Option<Self> {
        Self::open_with(pid, PROCESS_QUERY_LIMITED_INFORMATION)
    }

    /// Wait for the process to exit. Returns true if it exited, false on timeout.
    pub fn wait(&self, timeout_ms: u32) -> bool {
        unsafe { WaitForSingleObject(self.handle, timeout_ms) == WAIT_OBJECT_0 }
    }

    /// Wait indefinitely for the process to exit.
    pub fn wait_infinite(&self) -> bool {
        self.wait(0xFFFFFFFF)
    }

    /// Check if the process is still running.
    pub fn is_running(&self) -> bool {
        let mut exit_code: u32 = 0;
        let ok = unsafe { GetExitCodeProcess(self.handle, &mut exit_code) };
        ok != 0 && exit_code == 259 // STATUS_PENDING / STILL_ACTIVE
    }

    /// Terminate the process with the given exit code.
    pub fn terminate(&self, exit_code: u32) -> bool {
        unsafe { TerminateProcess(self.handle, exit_code) != 0 }
    }

    /// The PID this handle was opened for.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Raw handle for use with Win32 APIs.
    pub fn raw(&self) -> HANDLE {
        self.handle
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle); }
    }
}

/// Get the current process ID.
pub fn current_pid() -> u32 {
    unsafe { GetCurrentProcessId() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_current_process() {
        let pid = current_pid();
        let proc = ProcessHandle::open_for_query(pid).expect("open self");
        assert!(proc.is_running());
        assert_eq!(proc.pid(), pid);
    }

    #[test]
    fn wait_timeout_on_running() {
        let pid = current_pid();
        let proc = ProcessHandle::open_for_wait(pid).expect("open self");
        assert!(!proc.wait(0)); // should timeout immediately — we're still running
    }

    #[test]
    fn open_nonexistent_pid() {
        // PID 0 is System Idle — opening it should fail or succeed but it's not a normal process
        // PID 99999999 almost certainly doesn't exist
        assert!(ProcessHandle::open_for_query(99999999).is_none());
    }
}
