//! Virtual desktop tracking for AlwaysOnTop.
//! Queries registry for current virtual desktop GUID.

use windows_sys::Win32::System::Registry::*;

/// Get the current virtual desktop GUID from registry.
pub fn get_current_desktop_id() -> Option<[u8; 16]> {
    let subkey = powertoys_win32::string::to_wide(
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VirtualDesktops"
    );
    let value = powertoys_win32::string::to_wide("CurrentVirtualDesktop");
    unsafe {
        let mut hkey = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return None;
        }
        let mut data = [0u8; 16];
        let mut size = 16u32;
        let mut dtype = 0u32;
        let r = RegQueryValueExW(
            hkey, value.as_ptr(), std::ptr::null(), &mut dtype,
            data.as_mut_ptr(), &mut size,
        );
        RegCloseKey(hkey);
        if r != 0 || size < 16 { return None; }
        Some(data)
    }
}

/// Check if a window is on the current virtual desktop.
/// Uses IVirtualDesktopManager COM interface.
pub fn is_window_on_current_desktop(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    // For now, always return true — COM interface requires more complex setup
    // TODO: implement via IVirtualDesktopManager::IsWindowOnCurrentVirtualDesktop
    let _ = hwnd;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_current_desktop_returns_option() {
        // Should not crash — may return None if no virtual desktops configured
        let _ = get_current_desktop_id();
    }

    #[test]
    fn desktop_id_is_16_bytes_if_present() {
        if let Some(id) = get_current_desktop_id() {
            assert_eq!(id.len(), 16);
            // Should not be all zeros
            assert!(id.iter().any(|&b| b != 0), "Desktop ID should not be all zeros");
        }
    }
}