//! Update state persistence — thread-safe JSON state file.

use serde::{Deserialize, Serialize};
use std::path::Path;

const STATE_MUTEX: &str = "Local\\PowerToysRunnerUpdateStateMutex";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateState {
    pub state: String,
    #[serde(default)]
    pub release_page_url: String,
    #[serde(default)]
    pub installer_path: String,
    #[serde(default)]
    pub new_version: String,
}

fn state_file_path() -> std::path::PathBuf {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    std::path::PathBuf::from(local)
        .join("Microsoft")
        .join("PowerToys")
        .join("UpdateState.json")
}

fn write_state(state: &UpdateState) {
    // Use a named mutex for thread safety with the runner
    let mutex_name: Vec<u16> = STATE_MUTEX.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mutex = windows_sys::Win32::System::Threading::CreateMutexW(
            std::ptr::null(), 0, mutex_name.as_ptr(),
        );
        if !mutex.is_null() {
            windows_sys::Win32::System::Threading::WaitForSingleObject(mutex, 5000);
        }

        let json = serde_json::to_string_pretty(state).unwrap_or_default();
        std::fs::write(state_file_path(), json).ok();

        if !mutex.is_null() {
            windows_sys::Win32::System::Threading::ReleaseMutex(mutex);
            windows_sys::Win32::Foundation::CloseHandle(mutex);
        }
    }
}

pub fn store_up_to_date() {
    write_state(&UpdateState {
        state: "upToDate".to_string(),
        release_page_url: String::new(),
        installer_path: String::new(),
        new_version: String::new(),
    });
}

pub fn store_ready_to_install(installer_path: &Path) {
    write_state(&UpdateState {
        state: "readyToInstall".to_string(),
        release_page_url: String::new(),
        installer_path: installer_path.to_string_lossy().to_string(),
        new_version: String::new(),
    });
}

pub fn store_error(error_state: &str) {
    write_state(&UpdateState {
        state: error_state.to_string(),
        release_page_url: String::new(),
        installer_path: String::new(),
        new_version: String::new(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_serialization() {
        let state = UpdateState {
            state: "readyToInstall".to_string(),
            release_page_url: "https://github.com/test".to_string(),
            installer_path: "C:\\temp\\installer.exe".to_string(),
            new_version: "0.82.0".to_string(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let parsed: UpdateState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.state, "readyToInstall");
        assert_eq!(parsed.installer_path, "C:\\temp\\installer.exe");
    }

    #[test]
    fn test_state_file_path() {
        let path = state_file_path();
        assert!(path.to_string_lossy().contains("PowerToys"));
        assert!(path.to_string_lossy().ends_with("UpdateState.json"));
    }
}
