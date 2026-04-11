use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a hotkey combination (mirrors C++ HotkeyObject).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HotkeyObject {
    #[serde(default)]
    pub win: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub code: u32,
    #[serde(default)]
    pub key: String,
}

impl Default for HotkeyObject {
    fn default() -> Self {
        Self {
            win: false,
            ctrl: false,
            alt: false,
            shift: false,
            code: 0,
            key: String::new(),
        }
    }
}

impl HotkeyObject {
    pub fn new(win: bool, ctrl: bool, alt: bool, shift: bool, code: u32, key: &str) -> Self {
        Self {
            win,
            ctrl,
            alt,
            shift,
            code,
            key: key.to_string(),
        }
    }

    /// Produce a human-readable string like "shift+ctrl+win+alt+A".
    pub fn to_display_string(&self) -> String {
        let mut parts = Vec::new();
        if self.shift {
            parts.push("shift");
        }
        if self.ctrl {
            parts.push("ctrl");
        }
        if self.win {
            parts.push("win");
        }
        if self.alt {
            parts.push("alt");
        }
        parts.push(&self.key);
        parts.join("+")
    }
}

/// Represents a color value (stored as a string like "#RRGGBB").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorObject {
    #[serde(default)]
    pub value: String,
}

impl ColorObject {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

/// A single settings property value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SettingsValue {
    Bool(bool),
    Int(i64),
    String(String),
    Hotkey(HotkeyObject),
    Object(serde_json::Value),
}

impl SettingsValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SettingsValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            SettingsValue::Int(v) => Some(*v),
            // JSON numbers may parse as floats; try to coerce.
            SettingsValue::Object(serde_json::Value::Number(n)) => n.as_i64(),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            SettingsValue::String(v) => Some(v.as_str()),
            _ => None,
        }
    }

    pub fn as_hotkey(&self) -> Option<&HotkeyObject> {
        match self {
            SettingsValue::Hotkey(v) => Some(v),
            _ => None,
        }
    }
}

/// Wrapper around a property entry: `{ "value": <V> }`.
/// Additional fields (display_name, editor_type, etc.) are preserved opaquely.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyEntry {
    pub value: serde_json::Value,
    /// All other fields are kept as raw JSON so nothing is lost on roundtrip.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Top-level settings document:
/// ```json
/// { "name": "...", "version": "1.0", "properties": { ... } }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsDocument {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub properties: HashMap<String, PropertyEntry>,
    /// Preserve any unknown top-level keys.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl Default for SettingsDocument {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: default_version(),
            properties: HashMap::new(),
            extra: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_default() {
        let hk = HotkeyObject::default();
        assert!(!hk.win);
        assert!(!hk.ctrl);
        assert!(!hk.alt);
        assert!(!hk.shift);
        assert_eq!(hk.code, 0);
        assert_eq!(hk.key, "");
    }

    #[test]
    fn hotkey_display_string() {
        let hk = HotkeyObject::new(true, true, false, false, 65, "A");
        assert_eq!(hk.to_display_string(), "ctrl+win+A");
    }

    #[test]
    fn hotkey_serde_roundtrip() {
        let hk = HotkeyObject::new(true, false, true, true, 0x41, "A");
        let json = serde_json::to_string(&hk).unwrap();
        let hk2: HotkeyObject = serde_json::from_str(&json).unwrap();
        assert_eq!(hk, hk2);
    }

    #[test]
    fn color_object_roundtrip() {
        let co = ColorObject::new("#FF00AA");
        let json = serde_json::to_string(&co).unwrap();
        let co2: ColorObject = serde_json::from_str(&json).unwrap();
        assert_eq!(co, co2);
    }

    #[test]
    fn settings_value_bool() {
        let v = SettingsValue::Bool(true);
        assert_eq!(v.as_bool(), Some(true));
        assert_eq!(v.as_int(), None);
    }

    #[test]
    fn settings_value_int() {
        let v = SettingsValue::Int(42);
        assert_eq!(v.as_int(), Some(42));
        assert_eq!(v.as_bool(), None);
    }

    #[test]
    fn settings_value_string() {
        let v = SettingsValue::String("hello".into());
        assert_eq!(v.as_string(), Some("hello"));
    }

    #[test]
    fn settings_document_default() {
        let doc = SettingsDocument::default();
        assert_eq!(doc.version, "1.0");
        assert!(doc.properties.is_empty());
    }
}
