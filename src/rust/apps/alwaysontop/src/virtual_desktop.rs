//! Virtual desktop tracking for AlwaysOnTop.
//! Uses IVirtualDesktopManager COM interface to check desktop membership,
//! with registry fallback for reading the current desktop GUID.

use std::cell::OnceCell;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::UI::Shell::{IVirtualDesktopManager, VirtualDesktopManager};
use windows_sys::Win32::System::Registry::*;

thread_local! {
    static VDM: OnceCell<Option<IVirtualDesktopManager>> = const { OnceCell::new() };
}

/// Check if a window is on the current virtual desktop.
/// Falls back to `true` if the COM interface is unavailable or the call fails.
pub fn is_window_on_current_desktop(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    VDM.with(|cell| {
        let vdm = cell.get_or_init(|| {
            unsafe { CoCreateInstance(&VirtualDesktopManager, None, CLSCTX_ALL).ok() }
        });
        match vdm {
            Some(vdm) => unsafe {
                vdm.IsWindowOnCurrentVirtualDesktop(windows::Win32::Foundation::HWND(hwnd))
                    .map(|b| b.as_bool())
                    .unwrap_or(true)
            },
            None => true,
        }
    })
}

/// Get the current virtual desktop GUID from registry.
pub fn get_current_desktop_id() -> Option<[u8; 16]> {
    let subkey = powertoys_win32::string::to_wide(
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VirtualDesktops",
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
            hkey,
            value.as_ptr(),
            std::ptr::null(),
            &mut dtype,
            data.as_mut_ptr(),
            &mut size,
        );
        RegCloseKey(hkey);
        if r != 0 || size < 16 {
            return None;
        }
        Some(data)
    }
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