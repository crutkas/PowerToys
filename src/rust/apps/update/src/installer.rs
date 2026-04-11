//! Installer execution — MSI and WiX bootstrapper support.

/// Install an MSI package using the Windows Installer API.
pub fn install_msi(msi_path: &str) -> bool {
    // MsiInstallProductW is in msi.dll
    // For simplicity, shell out to msiexec
    let status = std::process::Command::new("msiexec")
        .args(["/i", msi_path, "/passive", "/norestart"])
        .status();

    match status {
        Ok(s) => s.success(),
        Err(e) => {
            eprintln!("[Update] msiexec failed: {}", e);
            false
        }
    }
}

/// Install using a WiX bootstrapper (.exe).
pub fn install_bootstrapper(exe_path: &str) -> bool {
    let status = std::process::Command::new(exe_path)
        .args(["--passive", "-norestart"])
        .status();

    match status {
        Ok(s) => {
            // WiX bootstrapper returns 0 on success, 3010 for reboot needed
            let code = s.code().unwrap_or(-1);
            code == 0 || code == 3010
        }
        Err(e) => {
            eprintln!("[Update] Bootstrapper failed: {}", e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_msi_path_validation() {
        // Just verify the function doesn't panic with invalid paths
        // (it will fail gracefully since the file doesn't exist)
        let result = super::install_msi("nonexistent.msi");
        // msiexec will return error for nonexistent file
        assert!(!result);
    }
}
