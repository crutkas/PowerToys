//! PowerToys settings path resolution and JSON loading.
//!
//! All PowerToys modules store settings under:
//!   `%LOCALAPPDATA%\Microsoft\PowerToys\<module>\settings.json`
//!
//! This module provides the path resolution and generic JSON helpers
//! that were previously duplicated in alwaysontop and update.

use std::path::PathBuf;

/// Base directory for all PowerToys settings: `%LOCALAPPDATA%\Microsoft\PowerToys`.
pub fn base_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("Microsoft").join("PowerToys"))
}

/// Settings file for a specific module: `<base>\<module_name>\settings.json`.
pub fn module_settings_path(module_name: &str) -> Option<PathBuf> {
    base_dir().map(|p| p.join(module_name).join("settings.json"))
}

/// Module-specific data directory: `<base>\<module_name>\`.
pub fn module_dir(module_name: &str) -> Option<PathBuf> {
    base_dir().map(|p| p.join(module_name))
}

/// Read and parse a JSON settings file, returning the parsed `serde_json::Value`.
pub fn read_settings_json(module_name: &str) -> Result<serde_json::Value, SettingsError> {
    let path = module_settings_path(module_name).ok_or(SettingsError::NoLocalAppData)?;
    let data = std::fs::read_to_string(&path).map_err(|e| SettingsError::Io(path.clone(), e))?;
    serde_json::from_str(&data).map_err(|e| SettingsError::Parse(path, e))
}

/// Extract a bool from PowerToys settings JSON: `root.properties.<key>.value`.
pub fn get_bool(root: &serde_json::Value, key: &str) -> Option<bool> {
    root.get("properties")?.get(key)?.get("value")?.as_bool()
}

/// Extract an i64 from PowerToys settings JSON: `root.properties.<key>.value`.
pub fn get_int(root: &serde_json::Value, key: &str) -> Option<i64> {
    root.get("properties")?.get(key)?.get("value")?.as_i64()
}

/// Extract a string from PowerToys settings JSON: `root.properties.<key>.value`.
pub fn get_str<'a>(root: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    root.get("properties")?.get(key)?.get("value")?.as_str()
}

/// Errors that can occur when loading settings.
#[derive(Debug)]
pub enum SettingsError {
    NoLocalAppData,
    Io(PathBuf, std::io::Error),
    Parse(PathBuf, serde_json::Error),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoLocalAppData => write!(f, "LOCALAPPDATA not set"),
            Self::Io(p, e) => write!(f, "reading {}: {e}", p.display()),
            Self::Parse(p, e) => write!(f, "parsing {}: {e}", p.display()),
        }
    }
}

impl std::error::Error for SettingsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_dir_exists() {
        // On any Windows machine with LOCALAPPDATA set, this should return Some
        if std::env::var_os("LOCALAPPDATA").is_some() {
            let dir = base_dir().unwrap();
            assert!(dir.to_str().unwrap().contains("Microsoft"));
            assert!(dir.to_str().unwrap().contains("PowerToys"));
        }
    }

    #[test]
    fn module_path_format() {
        if std::env::var_os("LOCALAPPDATA").is_some() {
            let path = module_settings_path("Awake").unwrap();
            let s = path.to_str().unwrap();
            assert!(s.contains("Awake"));
            assert!(s.ends_with("settings.json"));
        }
    }

    #[test]
    fn get_bool_from_json() {
        let json: serde_json::Value = serde_json::from_str(r#"{
            "properties": {
                "enabled": { "value": true },
                "disabled": { "value": false }
            }
        }"#).unwrap();
        assert_eq!(get_bool(&json, "enabled"), Some(true));
        assert_eq!(get_bool(&json, "disabled"), Some(false));
        assert_eq!(get_bool(&json, "missing"), None);
    }

    #[test]
    fn get_int_from_json() {
        let json: serde_json::Value = serde_json::from_str(r#"{
            "properties": {
                "timeout": { "value": 42 }
            }
        }"#).unwrap();
        assert_eq!(get_int(&json, "timeout"), Some(42));
    }

    #[test]
    fn get_str_from_json() {
        let json_str = r##"{"properties": {"color": {"value": "#ff0000"}}}"##;
        let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(get_str(&json, "color"), Some("#ff0000"));
    }

    #[test]
    fn get_from_empty_json() {
        let json: serde_json::Value = serde_json::from_str("{}").unwrap();
        assert_eq!(get_bool(&json, "any"), None);
        assert_eq!(get_int(&json, "any"), None);
        assert_eq!(get_str(&json, "any"), None);
    }
}
