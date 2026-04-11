//! PowerToys Update — checks GitHub for new releases, downloads, and installs.
//!
//! Two-stage update process:
//!   Stage 1 (-update_now): Download installer, copy self to temp, launch stage 2
//!   Stage 2 (-update_now_stage_2 <path>): Run the installer, update state

#![windows_subsystem = "windows"]

mod github;
mod state;
mod installer;

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: PowerToys.Update.exe -update_now | -update_now_stage_2 <installer_path>");
        return;
    }

    match args[1].as_str() {
        "-update_now" => stage1(),
        "-update_now_stage_2" if args.len() >= 3 => stage2(&args[2]),
        _ => {
            eprintln!("Unknown argument: {}", args[1]);
        }
    }
}

/// Stage 1: Check for update, download installer, copy self to temp, launch stage 2.
fn stage1() {
    eprintln!("[Update] Stage 1: checking for updates...");

    // Check GitHub for new version
    let current_version = get_current_version();
    let release = match github::get_latest_release() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[Update] Failed to check for updates: {}", e);
            state::store_error("errorDownloading");
            return;
        }
    };

    if !release.is_newer_than(&current_version) {
        eprintln!("[Update] Already up to date ({})", current_version);
        state::store_up_to_date();
        return;
    }

    eprintln!("[Update] New version available: {}", release.version);

    // Find the right installer asset
    let asset = match release.find_installer_asset() {
        Some(a) => a,
        None => {
            eprintln!("[Update] No suitable installer found in release assets");
            state::store_error("errorDownloading");
            return;
        }
    };

    // Download installer (3 retries)
    let download_dir = get_download_dir();
    fs::create_dir_all(&download_dir).ok();
    let installer_path = download_dir.join(&asset.name);

    let mut downloaded = false;
    for attempt in 1..=3 {
        eprintln!("[Update] Downloading {} (attempt {})", asset.name, attempt);
        match github::download_file(&asset.download_url, &installer_path) {
            Ok(_) => { downloaded = true; break; }
            Err(e) => eprintln!("[Update] Download failed: {}", e),
        }
    }

    if !downloaded {
        state::store_error("errorDownloading");
        return;
    }

    eprintln!("[Update] Downloaded to {:?}", installer_path);
    state::store_ready_to_install(&installer_path);

    // Copy ourselves to temp and launch stage 2
    let self_exe = std::env::current_exe().unwrap();
    let temp_dir = std::env::temp_dir().join("PowerToysUpdate");
    fs::create_dir_all(&temp_dir).ok();
    let temp_exe = temp_dir.join("PowerToys.Update.exe");
    fs::copy(&self_exe, &temp_exe).ok();

    let installer_str = installer_path.to_string_lossy().to_string();
    launch_stage2(&temp_exe, &installer_str);
}

/// Stage 2: Run the installer (MSI or bootstrapper).
fn stage2(installer_path: &str) {
    eprintln!("[Update] Stage 2: installing from {}", installer_path);

    let path = Path::new(installer_path);
    if !path.exists() {
        eprintln!("[Update] Installer not found: {}", installer_path);
        state::store_error("errorDownloading");
        return;
    }

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let success = match ext {
        "msi" => installer::install_msi(installer_path),
        "exe" => installer::install_bootstrapper(installer_path),
        _ => {
            eprintln!("[Update] Unknown installer type: {}", ext);
            false
        }
    };

    if success {
        eprintln!("[Update] Installation successful");
        state::store_up_to_date();
        // Clean up old installers
        cleanup_old_installers();
    } else {
        eprintln!("[Update] Installation failed");
        state::store_error("errorDownloading");
    }
}

fn launch_stage2(exe: &Path, installer_path: &str) {
    let exe_wide: Vec<u16> = exe.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
    let params = format!("-update_now_stage_2 \"{}\"", installer_path);
    let params_wide: Vec<u16> = params.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut sei: windows_sys::Win32::UI::Shell::SHELLEXECUTEINFOW = std::mem::zeroed();
        sei.cbSize = std::mem::size_of::<windows_sys::Win32::UI::Shell::SHELLEXECUTEINFOW>() as u32;
        sei.fMask = windows_sys::Win32::UI::Shell::SEE_MASK_NOCLOSEPROCESS;
        sei.lpFile = exe_wide.as_ptr();
        sei.lpParameters = params_wide.as_ptr();
        sei.nShow = windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;
        windows_sys::Win32::UI::Shell::ShellExecuteExW(&mut sei);
    }
}

fn get_current_version() -> String {
    // Read installed version from registry (same as C++ get_product_version reads from version_gen.h,
    // but for the installed binary we check the Uninstall registry key)
    if let Some(v) = read_version_from_registry() {
        return v;
    }
    // Fallback: read from the runner EXE's file version info
    if let Some(v) = read_version_from_runner_exe() {
        return v;
    }
    // Last resort: unknown
    "0.0.0".to_string()
}

fn read_version_from_registry() -> Option<String> {
    use windows_sys::Win32::System::Registry::*;
    let subkey = powertoys_win32::string::to_wide(
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\PowerToys"
    );
    let value_name = powertoys_win32::string::to_wide("DisplayVersion");
    unsafe {
        let mut hkey = std::ptr::null_mut();
        // Try HKLM first (machine install), then HKCU (per-user)
        for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
            if RegOpenKeyExW(root, subkey.as_ptr(), 0, KEY_READ, &mut hkey) == 0 {
                let mut buf = [0u16; 64];
                let mut size = (buf.len() * 2) as u32;
                let mut dtype = 0u32;
                if RegQueryValueExW(hkey, value_name.as_ptr(), std::ptr::null(),
                    &mut dtype, buf.as_mut_ptr() as *mut u8, &mut size) == 0
                    && dtype == REG_SZ
                {
                    RegCloseKey(hkey);
                    let len = (size as usize / 2).saturating_sub(1); // exclude null
                    return Some(String::from_utf16_lossy(&buf[..len]));
                }
                RegCloseKey(hkey);
            }
        }
    }
    None
}

fn read_version_from_runner_exe() -> Option<String> {
    // Try to read version from PowerToys.exe in same directory
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let runner = exe_dir.join("PowerToys.exe");
    if !runner.exists() { return None; }

    use windows_sys::Win32::Storage::FileSystem::*;
    let path_wide = powertoys_win32::string::to_wide(&runner.to_string_lossy());
    unsafe {
        let size = GetFileVersionInfoSizeW(path_wide.as_ptr(), std::ptr::null_mut());
        if size == 0 { return None; }
        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(path_wide.as_ptr(), 0, size, data.as_mut_ptr() as *mut _) == 0 {
            return None;
        }
        let mut buf_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut buf_len: u32 = 0;
        let query = powertoys_win32::string::to_wide("\\");
        if VerQueryValueW(data.as_ptr() as *const _, query.as_ptr(), &mut buf_ptr, &mut buf_len) == 0 {
            return None;
        }
        #[repr(C)]
        struct VS_FIXEDFILEINFO { signature: u32, version: u32, file_ver_ms: u32, file_ver_ls: u32, prod_ver_ms: u32, prod_ver_ls: u32 }
        let info = &*(buf_ptr as *const VS_FIXEDFILEINFO);
        let major = (info.prod_ver_ms >> 16) & 0xFFFF;
        let minor = info.prod_ver_ms & 0xFFFF;
        let patch = (info.prod_ver_ls >> 16) & 0xFFFF;
        Some(format!("{}.{}.{}", major, minor, patch))
    }
}

fn get_download_dir() -> PathBuf {
    powertoys_win32::settings::module_dir("Updates")
        .unwrap_or_else(|| PathBuf::from("."))
}

fn cleanup_old_installers() {
    let dir = get_download_dir();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "msi" || e == "exe").unwrap_or(false) {
                fs::remove_file(&path).ok();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_current_version_not_placeholder() {
        // CRITICAL TEST: version must NOT be the old hardcoded "0.0.1"
        let version = get_current_version();
        assert_ne!(version, "0.0.1", "Version must not be hardcoded placeholder");
        // Should be a valid semver-like string
        let parts: Vec<&str> = version.split('.').collect();
        assert!(parts.len() >= 2, "Version should have at least major.minor: got {}", version);
    }

    #[test]
    fn read_version_from_registry_returns_option() {
        // Registry may or may not have PowerToys installed — just verify no crash
        let _ = read_version_from_registry();
    }

    #[test]
    fn read_version_from_runner_exe_returns_option() {
        // Runner may or may not exist in same dir — just verify no crash
        let _ = read_version_from_runner_exe();
    }

    #[test]
    fn version_format_valid() {
        // If we get a version, it should look like "X.Y.Z"
        let version = get_current_version();
        for part in version.split('.') {
            assert!(part.parse::<u32>().is_ok(), "Version part '{}' not a number in '{}'", part, version);
        }
    }
}