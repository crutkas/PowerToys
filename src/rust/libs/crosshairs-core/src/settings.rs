use crate::types::{Color, CrosshairsOrientation, Settings};
use serde_json::Value;

/// Parse a `#RRGGBB` hex color string into a [`Color`] with alpha 255.
fn parse_color(s: &str) -> Option<Color> {
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color { a: 255, r, g, b })
}

/// Parse settings JSON in the PowerToys `{ "properties": { "<key>": { "value": ... } } }` format.
///
/// Missing keys keep their default value, matching the C++ `parse_settings` behaviour.
pub fn parse_settings(json: &str) -> Result<Settings, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut settings = Settings::default();

    let props = match v.get("properties") {
        Some(p) => p,
        None => return Ok(settings),
    };

    // Helper: extract a nested `{ "value": <number> }` as i64.
    let int_val = |key: &str| -> Option<i64> {
        props.get(key)?.get("value")?.as_i64()
    };
    let bool_val = |key: &str| -> Option<bool> {
        props.get(key)?.get("value")?.as_bool()
    };
    let str_val = |key: &str| -> Option<&str> {
        props.get(key)?.get("value")?.as_str()
    };

    if let Some(v) = int_val("crosshairs_opacity") {
        settings.opacity = v as i32;
    }
    if let Some(v) = int_val("crosshairs_radius") {
        settings.radius = v as i32;
    }
    if let Some(v) = int_val("crosshairs_thickness") {
        settings.thickness = v as i32;
    }
    if let Some(v) = int_val("crosshairs_border_size") {
        settings.border_size = v as i32;
    }
    if let Some(v) = int_val("crosshairs_fixed_length") {
        settings.fixed_length = v as i32;
    }
    if let Some(v) = bool_val("crosshairs_auto_hide") {
        settings.auto_hide = v;
    }
    if let Some(v) = bool_val("crosshairs_is_fixed_length_enabled") {
        settings.is_fixed_length_enabled = v;
    }
    if let Some(v) = int_val("crosshairs_orientation") {
        match v {
            0 => settings.orientation = CrosshairsOrientation::Both,
            1 => settings.orientation = CrosshairsOrientation::VerticalOnly,
            2 => settings.orientation = CrosshairsOrientation::HorizontalOnly,
            _ => {}
        }
    }
    if let Some(v) = str_val("crosshairs_color") {
        if let Some(c) = parse_color(v) {
            settings.color = c;
        }
    }
    if let Some(v) = str_val("crosshairs_border_color") {
        if let Some(c) = parse_color(v) {
            settings.border_color = c;
        }
    }
    if let Some(v) = bool_val("crosshairs_external_control") {
        settings.external_control = v;
    }

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_json() {
        let json = r##"{
            "properties": {
                "crosshairs_color": { "value": "#00FF00" },
                "crosshairs_border_color": { "value": "#0000FF" },
                "crosshairs_opacity": { "value": 50 },
                "crosshairs_radius": { "value": 30 },
                "crosshairs_thickness": { "value": 8 },
                "crosshairs_border_size": { "value": 2 },
                "crosshairs_auto_hide": { "value": true },
                "crosshairs_is_fixed_length_enabled": { "value": true },
                "crosshairs_fixed_length": { "value": 200 },
                "crosshairs_orientation": { "value": 1 }
            }
        }"##;
        let s = parse_settings(json).unwrap();
        assert_eq!(s.color, Color { a: 255, r: 0, g: 255, b: 0 });
        assert_eq!(s.border_color, Color { a: 255, r: 0, g: 0, b: 255 });
        assert_eq!(s.opacity, 50);
        assert_eq!(s.radius, 30);
        assert_eq!(s.thickness, 8);
        assert_eq!(s.border_size, 2);
        assert!(s.auto_hide);
        assert!(s.is_fixed_length_enabled);
        assert_eq!(s.fixed_length, 200);
        assert_eq!(s.orientation, CrosshairsOrientation::VerticalOnly);
    }

    #[test]
    fn parse_partial_opacity_only() {
        let json = r#"{ "properties": { "crosshairs_opacity": { "value": 42 } } }"#;
        let s = parse_settings(json).unwrap();
        assert_eq!(s.opacity, 42);
        assert_eq!(s.radius, 20); // default
    }

    #[test]
    fn parse_partial_radius_only() {
        let json = r#"{ "properties": { "crosshairs_radius": { "value": 50 } } }"#;
        let s = parse_settings(json).unwrap();
        assert_eq!(s.radius, 50);
        assert_eq!(s.thickness, 5); // default
    }

    #[test]
    fn parse_partial_bool() {
        let json = r#"{ "properties": { "crosshairs_auto_hide": { "value": true } } }"#;
        let s = parse_settings(json).unwrap();
        assert!(s.auto_hide);
        assert!(!s.is_fixed_length_enabled); // default
    }

    #[test]
    fn parse_invalid_json() {
        let result = parse_settings("not json {");
        assert!(result.is_err());
    }

    #[test]
    fn parse_missing_properties() {
        let s = parse_settings("{}").unwrap();
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn color_parsing_red() {
        let json = r##"{ "properties": { "crosshairs_color": { "value": "#FF0000" } } }"##;
        let s = parse_settings(json).unwrap();
        assert_eq!(s.color, Color { a: 255, r: 255, g: 0, b: 0 });
    }

    #[test]
    fn color_parsing_blue() {
        let json = r##"{ "properties": { "crosshairs_color": { "value": "#0000FF" } } }"##;
        let s = parse_settings(json).unwrap();
        assert_eq!(s.color, Color { a: 255, r: 0, g: 0, b: 255 });
    }

    #[test]
    fn parse_external_control_true() {
        let json = r#"{ "properties": { "crosshairs_external_control": { "value": true } } }"#;
        let s = parse_settings(json).unwrap();
        assert!(s.external_control);
    }

    #[test]
    fn parse_external_control_default_false() {
        let json = r#"{ "properties": {} }"#;
        let s = parse_settings(json).unwrap();
        assert!(!s.external_control);
    }
}
