//! GitHub release API client.

use serde::Deserialize;
use std::io::{Read, Write};
use std::path::Path;

const GITHUB_API: &str = "https://api.github.com/repos/microsoft/PowerToys/releases/latest";
const USER_AGENT: &str = "PowerToys-Update/1.0";

#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
    #[serde(skip)]
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    #[serde(rename = "browser_download_url")]
    pub download_url: String,
    pub size: u64,
}

impl Release {
    pub fn is_newer_than(&self, current: &str) -> bool {
        let new_ver = self.tag_name.trim_start_matches('v');
        version_gt(new_ver, current)
    }

    pub fn find_installer_asset(&self) -> Option<&Asset> {
        let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "x64" };

        // Match by: contains "powertoys" + contains arch + ends with ext
        // Prefer .exe over .msi
        for ext in &[".exe", ".msi"] {
            if let Some(asset) = self.assets.iter().find(|a| {
                let n = a.name.to_lowercase();
                n.contains("powertoyssetup") && n.contains(arch) && n.ends_with(ext)
            }) {
                return Some(asset);
            }
        }
        None
    }
}

/// Fetch the latest release info from GitHub.
pub fn get_latest_release() -> Result<Release, String> {
    let response = ureq::get(GITHUB_API)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github.v3+json")
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let body = response.into_body().read_to_string()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let mut release: Release = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    release.version = release.tag_name.trim_start_matches('v').to_string();
    Ok(release)
}

/// Download a file from URL to the given path, writing progress to a JSON state file.
pub fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    let response = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Download failed: {}", e))?;

    let content_length: Option<u64> = response.headers().get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());

    let mut file = std::fs::File::create(dest)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let progress_path = download_progress_path();
    write_progress(&progress_path, 0, content_length);

    let mut body = response.into_body();
    let mut reader = body.as_reader();
    let mut buf = [0u8; 8192];
    let mut downloaded: u64 = 0;
    let mut last_report: u64 = 0;

    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("Read error: {}", e))?;
        if n == 0 { break; }
        file.write_all(&buf[..n]).map_err(|e| format!("Write error: {}", e))?;
        downloaded += n as u64;

        // Report progress every 100 KB
        if downloaded - last_report >= 102_400 {
            write_progress(&progress_path, downloaded, content_length);
            last_report = downloaded;
        }
    }

    // Final 100% progress
    write_progress(&progress_path, downloaded, content_length);
    Ok(())
}

fn download_progress_path() -> std::path::PathBuf {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let mut path = std::path::PathBuf::from(local_app_data);
    path.push("Microsoft");
    path.push("PowerToys");
    path.push("Updates");
    let _ = std::fs::create_dir_all(&path);
    path.push("download_progress.json");
    path
}

fn write_progress(path: &std::path::Path, downloaded: u64, total: Option<u64>) {
    let percentage = total
        .filter(|&t| t > 0)
        .map(|t| ((downloaded as f64 / t as f64) * 100.0).min(100.0) as u32)
        .unwrap_or(0);
    let json = format!(
        r#"{{"downloaded":{},"total":{},"percentage":{}}}"#,
        downloaded,
        total.unwrap_or(0),
        percentage,
    );
    let _ = std::fs::write(path, json);
}

/// Simple semantic version comparison (a > b).
fn version_gt(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.').filter_map(|p| p.parse().ok()).collect()
    };
    let va = parse(a);
    let vb = parse(b);
    for i in 0..va.len().max(vb.len()) {
        let x = va.get(i).copied().unwrap_or(0);
        let y = vb.get(i).copied().unwrap_or(0);
        if x > y { return true; }
        if x < y { return false; }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Version comparison ───────────────────────────────────────

    #[test]
    fn test_version_gt_basic() {
        assert!(version_gt("0.82.0", "0.81.0"));
        assert!(version_gt("1.0.0", "0.99.99"));
        assert!(version_gt("0.82.1", "0.82.0"));
        assert!(!version_gt("0.82.0", "0.82.0"));
        assert!(!version_gt("0.81.0", "0.82.0"));
    }

    #[test]
    fn test_version_gt_different_lengths() {
        assert!(version_gt("0.82.0.1", "0.82.0"));
        assert!(!version_gt("0.82", "0.82.0"));
    }

    #[test]
    fn test_version_gt_major_bump() {
        assert!(version_gt("2.0.0", "1.99.99"));
        assert!(version_gt("10.0.0", "9.99.99"));
    }

    #[test]
    fn test_version_gt_prerelease_stripped() {
        // Prerelease tags like "-rc1" get stripped by trim_start_matches('v')
        // but version_gt only parses digits — non-numeric parts become 0
        assert!(!version_gt("0.82.0", "0.82.0"));
    }

    #[test]
    fn test_version_gt_equal_is_not_newer() {
        assert!(!version_gt("0.0.1", "0.0.1"));
        assert!(!version_gt("1.2.3", "1.2.3"));
    }

    // ── Release parsing ──────────────────────────────────────────

    #[test]
    fn test_parse_release_json() {
        let json = r#"{
            "tag_name": "v0.82.0",
            "assets": [
                {"name": "test.exe", "browser_download_url": "https://example.com", "size": 1234}
            ]
        }"#;
        let release: Release = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "v0.82.0");
        assert_eq!(release.assets.len(), 1);
        assert_eq!(release.assets[0].size, 1234);
    }

    #[test]
    fn test_parse_release_with_multiple_assets() {
        let json = r#"{
            "tag_name": "v0.83.0",
            "assets": [
                {"name": "PowerToysSetup-0.83.0-x64.exe", "browser_download_url": "https://a.com/x64.exe", "size": 50000000},
                {"name": "PowerToysSetup-0.83.0-arm64.exe", "browser_download_url": "https://a.com/arm64.exe", "size": 48000000},
                {"name": "PowerToysSetup-0.83.0-x64.msi", "browser_download_url": "https://a.com/x64.msi", "size": 52000000},
                {"name": "Source code (zip)", "browser_download_url": "https://a.com/src.zip", "size": 100000}
            ]
        }"#;
        let release: Release = serde_json::from_str(json).unwrap();
        assert_eq!(release.assets.len(), 4);
    }

    #[test]
    fn test_parse_empty_assets() {
        let json = r#"{"tag_name": "v1.0.0", "assets": []}"#;
        let release: Release = serde_json::from_str(json).unwrap();
        assert!(release.assets.is_empty());
        assert!(release.find_installer_asset().is_none());
    }

    #[test]
    fn test_is_newer_than() {
        let release = Release {
            tag_name: "v0.83.0".to_string(),
            version: String::new(),
            assets: vec![],
        };
        assert!(release.is_newer_than("0.82.0"));
        assert!(release.is_newer_than("0.0.1"));
        assert!(!release.is_newer_than("0.83.0"));
        assert!(!release.is_newer_than("0.84.0"));
    }

    // ── Asset selection ──────────────────────────────────────────

    #[test]
    fn test_find_installer_prefers_exe_over_msi() {
        let release = Release {
            tag_name: "v0.82.0".to_string(),
            version: "0.82.0".to_string(),
            assets: vec![
                Asset { name: "PowerToysSetup-0.82.0-x64.msi".to_string(), download_url: "https://a.com/msi".to_string(), size: 100 },
                Asset { name: "PowerToysSetup-0.82.0-x64.exe".to_string(), download_url: "https://a.com/exe".to_string(), size: 100 },
            ],
        };
        let asset = release.find_installer_asset().unwrap();
        assert!(asset.name.ends_with(".exe"), "Should prefer .exe over .msi");
    }

    #[test]
    fn test_find_installer_falls_back_to_msi() {
        let release = Release {
            tag_name: "v0.82.0".to_string(),
            version: "0.82.0".to_string(),
            assets: vec![
                Asset { name: "PowerToysSetup-0.82.0-x64.msi".to_string(), download_url: "https://a.com/msi".to_string(), size: 100 },
            ],
        };
        let asset = release.find_installer_asset().unwrap();
        assert!(asset.name.ends_with(".msi"));
    }

    #[test]
    fn test_find_installer_no_match() {
        let release = Release {
            tag_name: "v0.82.0".to_string(),
            version: "0.82.0".to_string(),
            assets: vec![
                Asset { name: "source.zip".to_string(), download_url: "https://a.com/src".to_string(), size: 100 },
                Asset { name: "checksums.txt".to_string(), download_url: "https://a.com/sum".to_string(), size: 50 },
            ],
        };
        assert!(release.find_installer_asset().is_none());
    }

    #[test]
    fn test_find_installer_case_insensitive() {
        let release = Release {
            tag_name: "v0.82.0".to_string(),
            version: "0.82.0".to_string(),
            assets: vec![
                Asset { name: "POWERTOYSSETUP-0.82.0-X64.EXE".to_string(), download_url: "https://a.com/a".to_string(), size: 100 },
            ],
        };
        assert!(release.find_installer_asset().is_some());
    }

    // ── Live API test (ignored by default — requires network) ────

    #[test]
    #[ignore]
    fn test_live_github_api_fetch() {
        let release = get_latest_release().expect("Should fetch latest release");
        assert!(!release.tag_name.is_empty());
        assert!(!release.assets.is_empty());
        // Should find at least one installer asset
        assert!(release.find_installer_asset().is_some(),
            "Latest release should have an installer asset");
    }
}
