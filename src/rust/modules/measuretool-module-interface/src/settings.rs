//! MeasureTool module settings — JSON serialization matching the PowerToys format.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSettings {
    pub name: String,
    pub version: String,
    pub properties: ModuleProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleProperties {
    #[serde(rename = "ContinuousCapture")]
    pub continuous_capture: BoolSetting,
    #[serde(rename = "DrawFeetOnCross")]
    pub draw_feet_on_cross: BoolSetting,
    #[serde(rename = "PixelTolerance")]
    pub pixel_tolerance: IntSetting,
    #[serde(rename = "PerColorChannelEdgeDetection")]
    pub per_color_channel_edge_detection: BoolSetting,
    #[serde(rename = "UnitsOfMeasure")]
    pub units_of_measure: IntSetting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoolSetting {
    pub value: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntSetting {
    pub value: i32,
}

impl Default for ModuleSettings {
    fn default() -> Self {
        Self {
            name: "Measure Tool".to_string(),
            version: "1.0".to_string(),
            properties: ModuleProperties {
                continuous_capture: BoolSetting { value: false },
                draw_feet_on_cross: BoolSetting { value: true },
                pixel_tolerance: IntSetting { value: 30 },
                per_color_channel_edge_detection: BoolSetting { value: false },
                units_of_measure: IntSetting { value: 0 },
            },
        }
    }
}

impl ModuleSettings {
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
        let settings = ModuleSettings::default();
        assert_eq!(settings.name, "Measure Tool");
        assert_eq!(settings.properties.pixel_tolerance.value, 30);
        assert!(settings.properties.draw_feet_on_cross.value);
        assert!(!settings.properties.continuous_capture.value);
    }

    #[test]
    fn test_settings_json_roundtrip() {
        let settings = ModuleSettings::default();
        let json = settings.to_json();
        let parsed = ModuleSettings::from_json(&json).expect("Should parse");
        assert_eq!(parsed.name, settings.name);
        assert_eq!(
            parsed.properties.pixel_tolerance.value,
            settings.properties.pixel_tolerance.value
        );
    }

    #[test]
    fn test_invalid_json_returns_none() {
        assert!(ModuleSettings::from_json("not json").is_none());
        assert!(ModuleSettings::from_json("").is_none());
        assert!(ModuleSettings::from_json("{}").is_none());
    }
}
