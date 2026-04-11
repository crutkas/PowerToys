// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Windows theme provider — registry-based theme read/write.

use lightswitch_core::state::ThemeProvider;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::System::Time::*;

const PERSONALIZATION_PATH: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";

const NIGHT_LIGHT_PATH: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\CloudStore\\Store\\DefaultAccount\\Current\\default$windows.data.bluelightreduction.bluelightreductionstate\\windows.data.bluelightreduction.bluelightreductionstate";

pub struct WinThemeProvider;

impl ThemeProvider for WinThemeProvider {
    fn get_current_system_theme(&self) -> bool {
        read_dword_value(PERSONALIZATION_PATH, "SystemUsesLightTheme").unwrap_or(1) == 1
    }

    fn get_current_apps_theme(&self) -> bool {
        read_dword_value(PERSONALIZATION_PATH, "AppsUseLightTheme").unwrap_or(1) == 1
    }

    fn is_night_light_enabled(&self) -> bool {
        is_night_light_on()
    }

    fn set_system_theme(&self, is_light: bool) {
        write_dword_value(PERSONALIZATION_PATH, "SystemUsesLightTheme", if is_light { 1 } else { 0 });
        if is_light {
            // Reset ColorPrevalence when switching to light mode.
            write_dword_value(PERSONALIZATION_PATH, "ColorPrevalence", 0);
        }
        broadcast_theme_change();
    }

    fn set_apps_theme(&self, is_light: bool) {
        write_dword_value(PERSONALIZATION_PATH, "AppsUseLightTheme", if is_light { 1 } else { 0 });
        broadcast_theme_change();
    }

    fn get_now_minutes(&self) -> i32 {
        unsafe {
            let mut st: SYSTEMTIME = std::mem::zeroed();
            windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st);
            st.wHour as i32 * 60 + st.wMinute as i32
        }
    }

    fn get_current_day(&self) -> i32 {
        unsafe {
            let mut st: SYSTEMTIME = std::mem::zeroed();
            windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st);
            st.wDay as i32
        }
    }

    fn get_timezone_bias_minutes(&self) -> i32 {
        unsafe {
            let mut tz: TIME_ZONE_INFORMATION = std::mem::zeroed();
            let state = GetTimeZoneInformation(&mut tz);
            let mut total_bias = tz.Bias;
            if state == 2 {
                // TIME_ZONE_ID_DAYLIGHT
                total_bias += tz.DaylightBias;
            } else if state == 1 {
                // TIME_ZONE_ID_STANDARD
                total_bias += tz.StandardBias;
            }
            total_bias
        }
    }

    fn get_current_date(&self) -> (i32, i32, i32) {
        unsafe {
            let mut st: SYSTEMTIME = std::mem::zeroed();
            windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st);
            (st.wYear as i32, st.wMonth as i32, st.wDay as i32)
        }
    }
}

fn read_dword_value(subkey: &str, value_name: &str) -> Option<u32> {
    let subkey_w = wide(subkey);
    let value_w = wide(value_name);

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey_w.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return None;
        }

        let mut data: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let result = RegQueryValueExW(
            hkey,
            value_w.as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
            &mut data as *mut u32 as *mut u8,
            &mut size,
        );
        RegCloseKey(hkey);

        if result == 0 {
            Some(data)
        } else {
            None
        }
    }
}

fn write_dword_value(subkey: &str, value_name: &str, value: u32) {
    let subkey_w = wide(subkey);
    let value_w = wide(value_name);

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey_w.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        ) != 0
        {
            return;
        }

        RegSetValueExW(
            hkey,
            value_w.as_ptr(),
            0,
            REG_DWORD,
            &value as *const u32 as *const u8,
            std::mem::size_of::<u32>() as u32,
        );
        RegCloseKey(hkey);
    }
}

fn is_night_light_on() -> bool {
    let path_w = wide(NIGHT_LIGHT_PATH);
    let value_w = wide("Data");

    unsafe {
        let mut hkey: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, path_w.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return false;
        }

        // Query size first.
        let mut size: u32 = 0;
        let mut data_type: u32 = 0;
        if RegQueryValueExW(
            hkey,
            value_w.as_ptr(),
            std::ptr::null(),
            &mut data_type,
            std::ptr::null_mut(),
            &mut size,
        ) != 0
            || size < 25
        {
            RegCloseKey(hkey);
            return false;
        }

        let mut data = vec![0u8; size as usize];
        if RegQueryValueExW(
            hkey,
            value_w.as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
            data.as_mut_ptr(),
            &mut size,
        ) != 0
        {
            RegCloseKey(hkey);
            return false;
        }

        RegCloseKey(hkey);
        data[23] == 0x10 && data[24] == 0x00
    }
}

fn broadcast_theme_change() {
    // HWND_BROADCAST = 0xFFFF
    const HWND_BROADCAST: windows_sys::Win32::Foundation::HWND = 0xFFFF as _;
    unsafe {
        let immersive = wide("ImmersiveColorSet");
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageTimeoutW(
            HWND_BROADCAST,
            windows_sys::Win32::UI::WindowsAndMessaging::WM_SETTINGCHANGE,
            0,
            immersive.as_ptr() as isize,
            windows_sys::Win32::UI::WindowsAndMessaging::SMTO_ABORTIFHUNG,
            5000,
            std::ptr::null_mut(),
        );
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageTimeoutW(
            HWND_BROADCAST,
            windows_sys::Win32::UI::WindowsAndMessaging::WM_THEMECHANGED,
            0,
            0,
            windows_sys::Win32::UI::WindowsAndMessaging::SMTO_ABORTIFHUNG,
            5000,
            std::ptr::null_mut(),
        );
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
