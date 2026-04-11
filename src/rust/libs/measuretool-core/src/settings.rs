//! Settings for MeasureTool — JSON (de)serialization matching the C++ format.
//!
//! The C++ code loads settings via `PTSettingsHelper::load_module_settings(L"Measure Tool")`
//! which reads a JSON file with properties nested as `{ "properties": { "Key": { "value": ... } } }`.

use crate::types::{Color, MeasureUnit};
use serde::{Deserialize, Serialize};

/// MeasureTool settings — matches C++ `Settings` struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasureToolSettings {
    /// Pixel tolerance for edge detection (0–255). Default: 30.
    #[serde(default = "default_pixel_tolerance")]
    pub pixel_tolerance: u8,

    /// Whether to continuously capture screenshots. Default: false.
    #[serde(default)]
    pub continuous_capture: bool,

    /// Whether to draw perpendicular "feet" at the ends of cross lines. Default: true.
    #[serde(default = "default_true")]
    pub draw_feet_on_cross: bool,

    /// Use per-channel edge detection (each channel individually) vs total diff. Default: false.
    #[serde(default)]
    pub per_color_channel_edge_detection: bool,

    /// Line color as RGB. Default: OrangeRed (255, 69, 0).
    #[serde(default)]
    pub line_color: Color,

    /// Units of measurement. Default: Pixel.
    #[serde(default = "default_units")]
    pub units: MeasureUnit,
}

fn default_pixel_tolerance() -> u8 {
    30
}
fn default_true() -> bool {
    true
}
fn default_units() -> MeasureUnit {
    MeasureUnit::Pixel
}

impl Default for MeasureToolSettings {
    fn default() -> Self {
        Self {
            pixel_tolerance: 30,
            continuous_capture: false,
            draw_feet_on_cross: true,
            per_color_channel_edge_detection: false,
            line_color: Color::default(),
            units: MeasureUnit::Pixel,
        }
    }
}

/// The PowerToys JSON settings format: `{ "properties": { "Key": { "value": V } } }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SettingsFile {
    #[serde(default)]
    properties: SettingsProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SettingsProperties {
    #[serde(rename = "ContinuousCapture", default)]
    continuous_capture: Option<ValueWrapper<bool>>,

    #[serde(rename = "DrawFeetOnCross", default)]
    draw_feet_on_cross: Option<ValueWrapper<bool>>,

    #[serde(rename = "PixelTolerance", default)]
    pixel_tolerance: Option<ValueWrapper<u8>>,

    #[serde(rename = "MeasureCrossColor", default)]
    measure_cross_color: Option<ValueWrapper<String>>,

    #[serde(rename = "PerColorChannelEdgeDetection", default)]
    per_color_channel_edge_detection: Option<ValueWrapper<bool>>,

    #[serde(rename = "UnitsOfMeasure", default)]
    units_of_measure: Option<ValueWrapper<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValueWrapper<T> {
    value: T,
}

impl MeasureToolSettings {
    /// Parse settings from the PowerToys JSON format.
    ///
    /// Each setting is individually guarded — missing or invalid keys use defaults,
    /// matching the C++ try/catch-per-field pattern.
    pub fn from_json(json: &str) -> Self {
        let file: SettingsFile = match serde_json::from_str(json) {
            Ok(f) => f,
            Err(_) => return Self::default(),
        };

        let props = file.properties;
        let mut settings = Self::default();

        if let Some(v) = props.continuous_capture {
            settings.continuous_capture = v.value;
        }
        if let Some(v) = props.draw_feet_on_cross {
            settings.draw_feet_on_cross = v.value;
        }
        if let Some(v) = props.pixel_tolerance {
            settings.pixel_tolerance = v.value;
        }
        if let Some(v) = props.measure_cross_color {
            if let Some(color) = parse_rgb_color(&v.value) {
                settings.line_color = color;
            }
        }
        if let Some(v) = props.per_color_channel_edge_detection {
            settings.per_color_channel_edge_detection = v.value;
        }
        if let Some(v) = props.units_of_measure {
            settings.units = MeasureUnit::from_index(v.value);
        }

        settings
    }

    /// Serialize settings to the PowerToys JSON format.
    pub fn to_json(&self) -> String {
        let color_str = format!("#{:02X}{:02X}{:02X}", self.line_color.r, self.line_color.g, self.line_color.b);
        let units_index = match self.units {
            MeasureUnit::Pixel => 0,
            MeasureUnit::Inch => 1,
            MeasureUnit::Centimetre => 2,
            MeasureUnit::Millimetre => 3,
        };

        let file = serde_json::json!({
            "properties": {
                "ContinuousCapture": { "value": self.continuous_capture },
                "DrawFeetOnCross": { "value": self.draw_feet_on_cross },
                "PixelTolerance": { "value": self.pixel_tolerance },
                "MeasureCrossColor": { "value": color_str },
                "PerColorChannelEdgeDetection": { "value": self.per_color_channel_edge_detection },
                "UnitsOfMeasure": { "value": units_index }
            }
        });

        serde_json::to_string(&file).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Parse an RGB color string like "#FF4500" into a Color.
fn parse_rgb_color(s: &str) -> Option<Color> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color { r, g, b })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings_match_cpp() {
        let s = MeasureToolSettings::default();
        assert_eq!(s.pixel_tolerance, 30);
        assert!(!s.continuous_capture);
        assert!(s.draw_feet_on_cross);
        assert!(!s.per_color_channel_edge_detection);
        assert_eq!(s.line_color, Color { r: 255, g: 69, b: 0 });
        assert_eq!(s.units, MeasureUnit::Pixel);
    }

    #[test]
    fn test_parse_full_settings_json() {
        let json = r##"{
            "properties": {
                "ContinuousCapture": { "value": true },
                "DrawFeetOnCross": { "value": false },
                "PixelTolerance": { "value": 50 },
                "MeasureCrossColor": { "value": "#00FF00" },
                "PerColorChannelEdgeDetection": { "value": true },
                "UnitsOfMeasure": { "value": 2 }
            }
        }"##;
        let s = MeasureToolSettings::from_json(json);
        assert!(s.continuous_capture);
        assert!(!s.draw_feet_on_cross);
        assert_eq!(s.pixel_tolerance, 50);
        assert_eq!(s.line_color, Color { r: 0, g: 255, b: 0 });
        assert!(s.per_color_channel_edge_detection);
        assert_eq!(s.units, MeasureUnit::Centimetre);
    }

    #[test]
    fn test_parse_partial_settings_uses_defaults() {
        let json = r#"{
            "properties": {
                "PixelTolerance": { "value": 100 }
            }
        }"#;
        let s = MeasureToolSettings::from_json(json);
        assert_eq!(s.pixel_tolerance, 100);
        // All others should be defaults
        assert!(!s.continuous_capture);
        assert!(s.draw_feet_on_cross);
        assert_eq!(s.line_color, Color::default());
    }

    #[test]
    fn test_parse_empty_json_returns_defaults() {
        let s = MeasureToolSettings::from_json("{}");
        assert_eq!(s.pixel_tolerance, 30);
        assert_eq!(s.units, MeasureUnit::Pixel);
    }

    #[test]
    fn test_parse_invalid_json_returns_defaults() {
        let s = MeasureToolSettings::from_json("not json at all");
        assert_eq!(s.pixel_tolerance, 30);
    }

    #[test]
    fn test_parse_invalid_color_uses_default() {
        let json = r#"{
            "properties": {
                "MeasureCrossColor": { "value": "not-a-color" }
            }
        }"#;
        let s = MeasureToolSettings::from_json(json);
        assert_eq!(s.line_color, Color::default()); // OrangeRed
    }

    #[test]
    fn test_settings_json_roundtrip() {
        let original = MeasureToolSettings {
            pixel_tolerance: 42,
            continuous_capture: true,
            draw_feet_on_cross: false,
            per_color_channel_edge_detection: true,
            line_color: Color {
                r: 0,
                g: 128,
                b: 255,
            },
            units: MeasureUnit::Millimetre,
        };
        let json = original.to_json();
        let parsed = MeasureToolSettings::from_json(&json);
        assert_eq!(parsed.pixel_tolerance, 42);
        assert!(parsed.continuous_capture);
        assert!(!parsed.draw_feet_on_cross);
        assert!(parsed.per_color_channel_edge_detection);
        assert_eq!(parsed.line_color, Color { r: 0, g: 128, b: 255 });
        assert_eq!(parsed.units, MeasureUnit::Millimetre);
    }

    #[test]
    fn test_to_json_produces_valid_json() {
        let s = MeasureToolSettings::default();
        let json = s.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should be valid JSON");
        assert!(parsed["properties"]["PixelTolerance"]["value"].is_number());
        assert!(parsed["properties"]["ContinuousCapture"]["value"].is_boolean());
    }

    #[test]
    fn test_parse_rgb_color() {
        assert_eq!(
            parse_rgb_color("#FF4500"),
            Some(Color { r: 255, g: 69, b: 0 })
        );
        assert_eq!(
            parse_rgb_color("#000000"),
            Some(Color { r: 0, g: 0, b: 0 })
        );
        assert_eq!(
            parse_rgb_color("#ffffff"),
            Some(Color {
                r: 255,
                g: 255,
                b: 255
            })
        );
        assert_eq!(parse_rgb_color("FF4500"), Some(Color { r: 255, g: 69, b: 0 }));
        assert_eq!(parse_rgb_color("#GG0000"), None);
        assert_eq!(parse_rgb_color("#FFF"), None);
        assert_eq!(parse_rgb_color(""), None);
    }

    #[test]
    fn test_units_index_mapping() {
        let json_pixel = r#"{ "properties": { "UnitsOfMeasure": { "value": 0 } } }"#;
        let json_inch = r#"{ "properties": { "UnitsOfMeasure": { "value": 1 } } }"#;
        let json_cm = r#"{ "properties": { "UnitsOfMeasure": { "value": 2 } } }"#;
        let json_mm = r#"{ "properties": { "UnitsOfMeasure": { "value": 3 } } }"#;

        assert_eq!(MeasureToolSettings::from_json(json_pixel).units, MeasureUnit::Pixel);
        assert_eq!(MeasureToolSettings::from_json(json_inch).units, MeasureUnit::Inch);
        assert_eq!(MeasureToolSettings::from_json(json_cm).units, MeasureUnit::Centimetre);
        assert_eq!(MeasureToolSettings::from_json(json_mm).units, MeasureUnit::Millimetre);
    }

    #[test]
    fn test_settings_to_json_color_format() {
        let s = MeasureToolSettings {
            line_color: Color { r: 10, g: 20, b: 30 },
            ..Default::default()
        };
        let json = s.to_json();
        assert!(json.contains("#0A141E"), "Color should be uppercase hex: {}", json);
    }
}
