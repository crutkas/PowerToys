//! GitHub release API client.

use serde::Deserialize;
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

/// Download a file from URL to the given path.
pub fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    let response = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| format!("Download failed: {}", e))?;

    let mut file = std::fs::File::create(dest)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    std::io::copy(&mut response.into_body().as_reader(), &mut file)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
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

    #[test]
    fn test_version_gt() {
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
    fn test_find_installer_asset() {
        let release = Release {
            tag_name: "v0.82.0".to_string(),
            version: "0.82.0".to_string(),
            assets: vec![
                Asset { name: "PowerToysSetup-0.82.0-x64.exe".to_string(), download_url: "https://example.com/a".to_string(), size: 100 },
                Asset { name: "PowerToysSetup-0.82.0-arm64.exe".to_string(), download_url: "https://example.com/b".to_string(), size: 100 },
            ],
        };

        let asset = release.find_installer_asset();
        assert!(asset.is_some(), "Should find an installer asset");
        let a = asset.unwrap();
        assert!(a.name.ends_with(".exe"));
    }

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
}
