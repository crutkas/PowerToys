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
    // Read from the installed PowerToys version
    // For now, use a placeholder that matches the build
    "0.0.1".to_string()
}

fn get_download_dir() -> PathBuf {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    PathBuf::from(local).join("Microsoft").join("PowerToys").join("Updates")
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
