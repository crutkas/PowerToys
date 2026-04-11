// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Settings loading for the Mouse Highlighter module.
//!
//! Parses the PowerToys JSON settings format with the nested
//! `{ "properties": { "key": { "value": ... } } }` structure.

use crate::types::{Color, Settings};

/// Parse an "#AARRGGBB" hex color string into a Color.
/// Returns None if the string is malformed.
pub fn parse_argb_color(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#')?;
    if s.len() != 8 {
        return None;
    }
    let a = u8::from_str_radix(&s[0..2], 16).ok()?;
    let r = u8::from_str_radix(&s[2..4], 16).ok()?;
    let g = u8::from_str_radix(&s[4..6], 16).ok()?;
    let b = u8::from_str_radix(&s[6..8], 16).ok()?;
    Some(Color::new(a, r, g, b))
}

/// Load settings from a JSON string in the PowerToys settings format.
/// Missing or invalid fields fall back to defaults.
pub fn load_settings_from_json(json: &str) -> Settings {
    let mut settings = Settings::default();

    let parsed: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return settings,
    };

    let props = match parsed.get("properties") {
        Some(p) => p,
        None => return settings,
    };

    // Helper: extract a nested "value" from a property key
    fn get_value<'a>(props: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
        props.get(key)?.get("value")
    }

    // Left button color
    if let Some(serde_json::Value::String(s)) = get_value(props, "left_button_click_color") {
        if let Some(c) = parse_argb_color(s) {
            settings.left_button_color = c;
        }
    }

    // Right button color
    if let Some(serde_json::Value::String(s)) = get_value(props, "right_button_click_color") {
        if let Some(c) = parse_argb_color(s) {
            settings.right_button_color = c;
        }
    }

    // Always color
    if let Some(serde_json::Value::String(s)) = get_value(props, "always_color") {
        if let Some(c) = parse_argb_color(s) {
            settings.always_color = c;
        }
    }

    // Radius
    if let Some(serde_json::Value::Number(n)) = get_value(props, "highlight_radius") {
        if let Some(v) = n.as_i64() {
            if v >= 0 {
                settings.radius = v as i32;
            }
        }
    }

    // Fade delay
    if let Some(serde_json::Value::Number(n)) = get_value(props, "highlight_fade_delay_ms") {
        if let Some(v) = n.as_i64() {
            if v >= 0 {
                settings.fade_delay_ms = v as i32;
            }
        }
    }

    // Fade duration
    if let Some(serde_json::Value::Number(n)) = get_value(props, "highlight_fade_duration_ms") {
        if let Some(v) = n.as_i64() {
            if v >= 0 {
                settings.fade_duration_ms = v as i32;
            }
        }
    }

    // Auto activate
    if let Some(serde_json::Value::Bool(b)) = get_value(props, "auto_activate") {
        settings.auto_activate = *b;
    }

    // Spotlight mode
    if let Some(serde_json::Value::Bool(b)) = get_value(props, "spotlight_mode") {
        settings.spotlight_mode = *b;
    }

    settings
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Color;

    #[test]
    fn parse_argb_valid() {
        assert_eq!(
            parse_argb_color("#A6FFFF00"),
            Some(Color::new(0xA6, 0xFF, 0xFF, 0x00))
        );
    }

    #[test]
    fn parse_argb_blue() {
        assert_eq!(
            parse_argb_color("#A60000FF"),
            Some(Color::new(0xA6, 0x00, 0x00, 0xFF))
        );
    }

    #[test]
    fn parse_argb_invalid_no_hash() {
        assert_eq!(parse_argb_color("A6FFFF00"), None);
    }

    #[test]
    fn parse_argb_invalid_short() {
        assert_eq!(parse_argb_color("#FFFF00"), None);
    }

    #[test]
    fn parse_argb_invalid_hex() {
        assert_eq!(parse_argb_color("#GGHHIIJJ"), None);
    }

    #[test]
    fn load_empty_json_returns_defaults() {
        let s = load_settings_from_json("{}");
        let d = Settings::default();
        assert_eq!(s.left_button_color, d.left_button_color);
        assert_eq!(s.right_button_color, d.right_button_color);
        assert_eq!(s.always_color, d.always_color);
        assert_eq!(s.radius, d.radius);
        assert_eq!(s.fade_delay_ms, d.fade_delay_ms);
        assert_eq!(s.fade_duration_ms, d.fade_duration_ms);
        assert_eq!(s.auto_activate, d.auto_activate);
        assert_eq!(s.spotlight_mode, d.spotlight_mode);
    }

    #[test]
    fn load_invalid_json_returns_defaults() {
        let s = load_settings_from_json("not valid json");
        assert_eq!(s.radius, Settings::default().radius);
    }

    #[test]
    fn load_full_settings() {
        let json = r##"{
            "properties": {
                "left_button_click_color": { "value": "#FF00FF00" },
                "right_button_click_color": { "value": "#80FF0000" },
                "always_color": { "value": "#400000FF" },
                "highlight_radius": { "value": 30 },
                "highlight_fade_delay_ms": { "value": 1000 },
                "highlight_fade_duration_ms": { "value": 500 },
                "auto_activate": { "value": true },
                "spotlight_mode": { "value": true }
            }
        }"##;
        let s = load_settings_from_json(json);
        assert_eq!(s.left_button_color, Color::new(0xFF, 0x00, 0xFF, 0x00));
        assert_eq!(s.right_button_color, Color::new(0x80, 0xFF, 0x00, 0x00));
        assert_eq!(s.always_color, Color::new(0x40, 0x00, 0x00, 0xFF));
        assert_eq!(s.radius, 30);
        assert_eq!(s.fade_delay_ms, 1000);
        assert_eq!(s.fade_duration_ms, 500);
        assert!(s.auto_activate);
        assert!(s.spotlight_mode);
    }

    #[test]
    fn load_partial_settings_keeps_defaults() {
        let json = r#"{
            "properties": {
                "highlight_radius": { "value": 50 }
            }
        }"#;
        let s = load_settings_from_json(json);
        assert_eq!(s.radius, 50);
        // Others remain default
        assert_eq!(s.left_button_color, Settings::default().left_button_color);
        assert_eq!(s.fade_delay_ms, 500);
        assert!(!s.auto_activate);
    }

    #[test]
    fn load_negative_radius_ignored() {
        let json = r#"{
            "properties": {
                "highlight_radius": { "value": -5 }
            }
        }"#;
        let s = load_settings_from_json(json);
        assert_eq!(s.radius, Settings::default().radius);
    }

    #[test]
    fn load_invalid_color_ignored() {
        let json = r#"{
            "properties": {
                "left_button_click_color": { "value": "not-a-color" }
            }
        }"#;
        let s = load_settings_from_json(json);
        assert_eq!(s.left_button_color, Settings::default().left_button_color);
    }
}
