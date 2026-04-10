//! Awake module settings — JSON serialization matching the PowerToys format.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwakeSettings {
    pub name: String,
    pub version: String,
    pub properties: AwakeProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwakeProperties {
    pub awake_keep_display_on: BoolSetting,
    pub awake_mode: IntSetting,
    pub awake_hours: IntSetting,
    pub awake_minutes: IntSetting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoolSetting {
    pub value: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntSetting {
    pub value: i32,
}

impl Default for AwakeSettings {
    fn default() -> Self {
        Self {
            name: "Awake".to_string(),
            version: "1.0".to_string(),
            properties: AwakeProperties {
                awake_keep_display_on: BoolSetting { value: true },
                awake_mode: IntSetting { value: 0 },
                awake_hours: IntSetting { value: 0 },
                awake_minutes: IntSetting { value: 0 },
            },
        }
    }
}

impl AwakeSettings {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn from_json(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = AwakeSettings::default();
        assert_eq!(settings.name, "Awake");
        assert!(settings.properties.awake_keep_display_on.value);
        assert_eq!(settings.properties.awake_mode.value, 0);
    }

    #[test]
    fn test_settings_json_roundtrip() {
        let settings = AwakeSettings::default();
        let json = settings.to_json();
        let parsed = AwakeSettings::from_json(&json).expect("Should parse");
        assert_eq!(parsed.name, settings.name);
        assert_eq!(
            parsed.properties.awake_keep_display_on.value,
            settings.properties.awake_keep_display_on.value
        );
    }

    #[test]
    fn test_settings_custom_values() {
        let mut settings = AwakeSettings::default();
        settings.properties.awake_mode.value = 2;
        settings.properties.awake_hours.value = 1;
        settings.properties.awake_minutes.value = 30;
        settings.properties.awake_keep_display_on.value = false;

        let json = settings.to_json();
        let parsed = AwakeSettings::from_json(&json).unwrap();
        assert_eq!(parsed.properties.awake_mode.value, 2);
        assert_eq!(parsed.properties.awake_hours.value, 1);
        assert_eq!(parsed.properties.awake_minutes.value, 30);
        assert!(!parsed.properties.awake_keep_display_on.value);
    }

    #[test]
    fn test_invalid_json_returns_none() {
        assert!(AwakeSettings::from_json("not json").is_none());
        assert!(AwakeSettings::from_json("").is_none());
        assert!(AwakeSettings::from_json("{}").is_none()); // missing fields
    }
}
