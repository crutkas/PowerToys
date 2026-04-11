//! Registry utilities for querying URI protocol names.
//!
//! Ported from RegistryUtils.cpp.

use powertoys_win32::string::{from_wide, to_wide};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CLASSES_ROOT,
    KEY_READ, REG_SZ,
};

/// Get URI protocol names for a packaged app from the registry.
///
/// Registry path:
/// `HKCR\Extensions\ContractId\Windows.Protocol\PackageId\{pkg}\ActivatableClassId\{class}\CustomProperties`
/// Value: `Name`
pub fn get_uri_protocol_names(package_full_name: &str) -> Vec<String> {
    let mut protocols = Vec::new();

    let key_path = format!(
        "Extensions\\ContractId\\Windows.Protocol\\PackageId\\{}\\ActivatableClassId",
        package_full_name
    );
    let wide_path = to_wide(&key_path);

    let mut hkey: HKEY = std::ptr::null_mut();
    let result = unsafe {
        RegOpenKeyExW(HKEY_CLASSES_ROOT, wide_path.as_ptr(), 0, KEY_READ, &mut hkey)
    };
    if result != 0 {
        return protocols;
    }

    // Enumerate subkeys (ActivatableClassId entries)
    let mut index: u32 = 0;
    loop {
        let mut name_buf = [0u16; 256];
        let mut name_len = name_buf.len() as u32;
        let result = unsafe {
            RegEnumKeyExW(
                hkey,
                index,
                name_buf.as_mut_ptr(),
                &mut name_len,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if result != 0 {
            break;
        }

        let subkey_name = from_wide(&name_buf[..name_len as usize]);

        // Open CustomProperties subkey
        let custom_path = format!("{}\\CustomProperties", subkey_name);
        let wide_custom = to_wide(&custom_path);

        let mut hsubkey: HKEY = std::ptr::null_mut();
        let result = unsafe {
            RegOpenKeyExW(hkey, wide_custom.as_ptr(), 0, KEY_READ, &mut hsubkey)
        };
        if result == 0 {
            // Query "Name" value
            let wide_name = to_wide("Name");
            let mut value_buf = [0u16; 256];
            let mut value_size = (value_buf.len() * 2) as u32;
            let mut value_type: u32 = 0;

            let result = unsafe {
                RegQueryValueExW(
                    hsubkey,
                    wide_name.as_ptr(),
                    std::ptr::null_mut(),
                    &mut value_type,
                    value_buf.as_mut_ptr() as *mut u8,
                    &mut value_size,
                )
            };

            if result == 0 && value_type == REG_SZ {
                let char_count = value_size as usize / 2;
                let protocol = from_wide(&value_buf[..char_count]);
                if !protocol.is_empty() {
                    protocols.push(protocol);
                }
            }

            unsafe { RegCloseKey(hsubkey); }
        }

        index += 1;
    }

    unsafe { RegCloseKey(hkey); }
    protocols
}
