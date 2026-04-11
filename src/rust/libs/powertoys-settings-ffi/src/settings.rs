use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::types::{HotkeyObject, PropertyEntry, SettingsDocument};

/// Errors that can occur during settings operations.
#[derive(Debug)]
pub enum SettingsError {
    Io(io::Error),
    Json(serde_json::Error),
    MissingField(String),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsError::Io(e) => write!(f, "IO error: {e}"),
            SettingsError::Json(e) => write!(f, "JSON error: {e}"),
            SettingsError::MissingField(name) => write!(f, "missing field: {name}"),
        }
    }
}

impl std::error::Error for SettingsError {}

impl From<io::Error> for SettingsError {
    fn from(e: io::Error) -> Self {
        SettingsError::Io(e)
    }
}

impl From<serde_json::Error> for SettingsError {
    fn from(e: serde_json::Error) -> Self {
        SettingsError::Json(e)
    }
}

/// In-memory representation of a PowerToy module's settings.
pub struct Settings {
    pub doc: SettingsDocument,
    pub module_key: String,
}

impl Settings {
    /// Create empty settings for a module.
    pub fn new(module_key: &str, module_name: &str) -> Self {
        Self {
            doc: SettingsDocument {
                name: module_name.to_string(),
                version: "1.0".to_string(),
                properties: HashMap::new(),
                extra: HashMap::new(),
            },
            module_key: module_key.to_string(),
        }
    }

    // ── Property accessors ──────────────────────────────────────────

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        let entry = self.doc.properties.get(key)?;
        entry.value.as_bool()
    }

    pub fn get_int(&self, key: &str) -> Option<i64> {
        let entry = self.doc.properties.get(key)?;
        entry.value.as_i64()
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        let entry = self.doc.properties.get(key)?;
        entry.value.as_str().map(|s| s.to_string())
    }

    pub fn get_hotkey(&self, key: &str) -> Option<HotkeyObject> {
        let entry = self.doc.properties.get(key)?;
        serde_json::from_value(entry.value.clone()).ok()
    }

    pub fn get_json_value(&self, key: &str) -> Option<&serde_json::Value> {
        let entry = self.doc.properties.get(key)?;
        Some(&entry.value)
    }

    // ── Property setters ────────────────────────────────────────────

    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.set_raw_value(key, serde_json::Value::Bool(value));
    }

    pub fn set_int(&mut self, key: &str, value: i64) {
        self.set_raw_value(
            key,
            serde_json::Value::Number(serde_json::Number::from(value)),
        );
    }

    pub fn set_string(&mut self, key: &str, value: &str) {
        self.set_raw_value(key, serde_json::Value::String(value.to_string()));
    }

    pub fn set_hotkey(&mut self, key: &str, hotkey: &HotkeyObject) {
        let value = serde_json::to_value(hotkey).unwrap_or(serde_json::Value::Null);
        self.set_raw_value(key, value);
    }

    fn set_raw_value(&mut self, key: &str, value: serde_json::Value) {
        if let Some(entry) = self.doc.properties.get_mut(key) {
            entry.value = value;
        } else {
            self.doc.properties.insert(
                key.to_string(),
                PropertyEntry {
                    value,
                    extra: HashMap::new(),
                },
            );
        }
    }

    // ── Serialization ───────────────────────────────────────────────

    /// Serialize settings to a JSON string.
    pub fn to_json(&self) -> Result<String, SettingsError> {
        // Ensure version is always set.
        let mut doc = self.doc.clone();
        doc.version = "1.0".to_string();
        Ok(serde_json::to_string_pretty(&doc)?)
    }

    /// Parse settings from a JSON string.
    pub fn from_json(module_key: &str, json: &str) -> Result<Self, SettingsError> {
        let doc: SettingsDocument = serde_json::from_str(json)?;
        Ok(Self {
            doc,
            module_key: module_key.to_string(),
        })
    }
}

// ── File I/O helpers (depend on a base path) ────────────────────────

/// Compute the settings file path for a module.
///
/// C++ layout: `{base}/Microsoft/PowerToys/{module_key}/settings.json`
/// For testing we accept a custom base; production callers use `%LOCALAPPDATA%`.
pub fn settings_file_path(base_dir: &Path, module_key: &str) -> PathBuf {
    base_dir
        .join("Microsoft")
        .join("PowerToys")
        .join(module_key)
        .join("settings.json")
}

/// Load settings from disk using a custom base directory.
pub fn load_from_settings_file(
    base_dir: &Path,
    module_key: &str,
) -> Result<Settings, SettingsError> {
    let path = settings_file_path(base_dir, module_key);
    match fs::read_to_string(&path) {
        Ok(content) => Settings::from_json(module_key, &content),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            // Mirror C++ behaviour: missing file → empty JsonObject.
            Ok(Settings::new(module_key, ""))
        }
        Err(e) => Err(SettingsError::Io(e)),
    }
}

/// Save settings to disk using a custom base directory.
pub fn save_to_settings_file(
    base_dir: &Path,
    settings: &Settings,
) -> Result<(), SettingsError> {
    let path = settings_file_path(base_dir, &settings.module_key);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = settings.to_json()?;
    fs::write(&path, json)?;
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_base() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "pt_settings_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    // ── load / from_json tests ──

    #[test]
    fn load_valid_json() {
        let json = r#"{
            "name": "TestModule",
            "version": "1.0",
            "properties": {
                "enabled": { "value": true },
                "count": { "value": 42 }
            }
        }"#;
        let s = Settings::from_json("TestModule", json).unwrap();
        assert_eq!(s.doc.name, "TestModule");
        assert_eq!(s.doc.version, "1.0");
        assert_eq!(s.get_bool("enabled"), Some(true));
        assert_eq!(s.get_int("count"), Some(42));
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let base = temp_base();
        let s = load_from_settings_file(&base, "NoSuchModule").unwrap();
        assert_eq!(s.doc.name, "");
        assert_eq!(s.doc.version, "1.0");
        assert!(s.doc.properties.is_empty());
        cleanup(&base);
    }

    #[test]
    fn load_corrupt_json_returns_error() {
        let base = temp_base();
        let dir = base.join("Microsoft").join("PowerToys").join("BadModule");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("settings.json"), "NOT JSON {{{").unwrap();
        let result = load_from_settings_file(&base, "BadModule");
        assert!(result.is_err());
        cleanup(&base);
    }

    // ── get property tests ──

    #[test]
    fn get_bool_exists() {
        let json = r#"{"name":"M","version":"1.0","properties":{"flag":{"value":false}}}"#;
        let s = Settings::from_json("M", json).unwrap();
        assert_eq!(s.get_bool("flag"), Some(false));
    }

    #[test]
    fn get_bool_missing_returns_none() {
        let s = Settings::new("M", "M");
        assert_eq!(s.get_bool("nonexistent"), None);
    }

    #[test]
    fn get_int_property() {
        let json = r#"{"name":"M","version":"1.0","properties":{"num":{"value":99}}}"#;
        let s = Settings::from_json("M", json).unwrap();
        assert_eq!(s.get_int("num"), Some(99));
    }

    #[test]
    fn get_string_property() {
        let json =
            r##"{"name":"M","version":"1.0","properties":{"color":{"value":"#FF0000"}}}"##;
        let s = Settings::from_json("M", json).unwrap();
        assert_eq!(s.get_string("color").as_deref(), Some("#FF0000"));
    }

    #[test]
    fn get_hotkey_object() {
        let json = r#"{
            "name":"M","version":"1.0",
            "properties":{
                "hk":{
                    "value":{"win":true,"ctrl":true,"alt":false,"shift":false,"code":65,"key":"A"}
                }
            }
        }"#;
        let s = Settings::from_json("M", json).unwrap();
        let hk = s.get_hotkey("hk").unwrap();
        assert!(hk.win);
        assert!(hk.ctrl);
        assert!(!hk.alt);
        assert!(!hk.shift);
        assert_eq!(hk.code, 65);
        assert_eq!(hk.key, "A");
    }

    // ── set property tests ──

    #[test]
    fn set_bool_updates_value() {
        let mut s = Settings::new("M", "M");
        s.set_bool("enabled", true);
        assert_eq!(s.get_bool("enabled"), Some(true));
        s.set_bool("enabled", false);
        assert_eq!(s.get_bool("enabled"), Some(false));
    }

    #[test]
    fn set_int_property() {
        let mut s = Settings::new("M", "M");
        s.set_int("count", 7);
        assert_eq!(s.get_int("count"), Some(7));
    }

    #[test]
    fn set_string_property() {
        let mut s = Settings::new("M", "M");
        s.set_string("theme", "dark");
        assert_eq!(s.get_string("theme").as_deref(), Some("dark"));
    }

    #[test]
    fn set_hotkey_property() {
        let mut s = Settings::new("M", "M");
        let hk = HotkeyObject::new(false, true, true, false, 0x42, "B");
        s.set_hotkey("shortcut", &hk);
        let retrieved = s.get_hotkey("shortcut").unwrap();
        assert_eq!(retrieved, hk);
    }

    // ── serialization ──

    #[test]
    fn to_json_produces_valid_output() {
        let mut s = Settings::new("TestMod", "TestMod");
        s.set_bool("enabled", true);
        s.set_int("interval", 30);
        let json_str = s.to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["name"], "TestMod");
        assert_eq!(parsed["version"], "1.0");
        assert_eq!(parsed["properties"]["enabled"]["value"], true);
        assert_eq!(parsed["properties"]["interval"]["value"], 30);
    }

    #[test]
    fn save_writes_valid_json() {
        let base = temp_base();
        let mut s = Settings::new("SaveTest", "SaveTest");
        s.set_bool("active", true);
        save_to_settings_file(&base, &s).unwrap();

        let path = settings_file_path(&base, "SaveTest");
        let content = fs::read_to_string(path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["properties"]["active"]["value"], true);
        cleanup(&base);
    }

    #[test]
    fn save_load_roundtrip() {
        let base = temp_base();
        let mut original = Settings::new("Roundtrip", "Roundtrip");
        original.set_bool("enabled", true);
        original.set_int("delay", 500);
        original.set_string("color", "#00FF00");
        let hk = HotkeyObject::new(true, true, false, false, 65, "A");
        original.set_hotkey("hotkey", &hk);

        save_to_settings_file(&base, &original).unwrap();
        let loaded = load_from_settings_file(&base, "Roundtrip").unwrap();

        assert_eq!(loaded.get_bool("enabled"), Some(true));
        assert_eq!(loaded.get_int("delay"), Some(500));
        assert_eq!(loaded.get_string("color").as_deref(), Some("#00FF00"));
        let loaded_hk = loaded.get_hotkey("hotkey").unwrap();
        assert_eq!(loaded_hk, hk);
        cleanup(&base);
    }

    #[test]
    fn module_key_to_path() {
        let base = PathBuf::from("C:\\Users\\test\\AppData\\Local");
        let path = settings_file_path(&base, "FancyZones");
        assert!(path.ends_with("Microsoft\\PowerToys\\FancyZones\\settings.json"));
    }

    #[test]
    fn settings_format_matches_cpp_structure() {
        let mut s = Settings::new("Awake", "Awake");
        s.set_bool("enabled", true);
        s.set_int("mode", 2);
        let json_str = s.to_json().unwrap();
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Must have top-level: name, version, properties
        assert!(v.get("name").is_some());
        assert!(v.get("version").is_some());
        assert!(v.get("properties").is_some());

        // Each property must have a "value" key
        let props = v["properties"].as_object().unwrap();
        for (_k, prop) in props {
            assert!(
                prop.get("value").is_some(),
                "property must have 'value' key"
            );
        }
    }

    #[test]
    fn preserves_extra_property_fields_on_roundtrip() {
        let json = r#"{
            "name":"M","version":"1.0",
            "properties":{
                "flag":{
                    "value": true,
                    "display_name": "Enable feature",
                    "editor_type": "bool_toggle",
                    "order": 1
                }
            }
        }"#;
        let s = Settings::from_json("M", json).unwrap();
        let output = s.to_json().unwrap();
        let v: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(v["properties"]["flag"]["display_name"], "Enable feature");
        assert_eq!(v["properties"]["flag"]["editor_type"], "bool_toggle");
        assert_eq!(v["properties"]["flag"]["order"], 1);
    }

    #[test]
    fn get_int_missing_returns_none() {
        let s = Settings::new("M", "M");
        assert_eq!(s.get_int("missing"), None);
    }

    #[test]
    fn get_string_missing_returns_none() {
        let s = Settings::new("M", "M");
        assert_eq!(s.get_string("missing"), None);
    }

    #[test]
    fn get_hotkey_missing_returns_none() {
        let s = Settings::new("M", "M");
        assert_eq!(s.get_hotkey("missing"), None);
    }
}
