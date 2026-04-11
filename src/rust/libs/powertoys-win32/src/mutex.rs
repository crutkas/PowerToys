//! Singleton mutex for ensuring only one instance of a PowerToys module runs.
//!
//! Common pattern: `CreateMutexW` with a global name, check `GetLastError` for
//! `ERROR_ALREADY_EXISTS`.

use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Threading::CreateMutexW;

use crate::string::to_wide;

/// RAII wrapper around a named mutex used for singleton enforcement.
pub struct AppMutex {
    handle: HANDLE,
    already_existed: bool,
}

// SAFETY: mutex handles can be sent across threads.
unsafe impl Send for AppMutex {}

impl AppMutex {
    /// Create or open a named mutex. Returns `None` if creation fails.
    pub fn create(name: &str) -> Option<Self> {
        let wide = to_wide(name);
        let handle = unsafe { CreateMutexW(ptr::null(), 0, wide.as_ptr()) };
        if handle.is_null() {
            return None;
        }
        let already_existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        Some(Self { handle, already_existed })
    }

    /// Returns true if another instance already holds this mutex.
    pub fn already_running(&self) -> bool {
        self.already_existed
    }

    /// Raw handle.
    pub fn raw(&self) -> HANDLE {
        self.handle
    }
}

impl Drop for AppMutex {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_mutex() {
        let name = "Local\\powertoys_win32_test_mutex";
        let m1 = AppMutex::create(name).expect("create mutex");
        assert!(!m1.already_running());
    }

    #[test]
    fn detect_existing_mutex() {
        let name = "Local\\powertoys_win32_test_mutex_dup";
        let m1 = AppMutex::create(name).expect("create first");
        let m2 = AppMutex::create(name).expect("create second");
        assert!(!m1.already_running());
        assert!(m2.already_running()); // second instance detects first
    }
}
