// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! FindMyMouse settings — JSON parsing matching the PowerToys format.
//!
//! Supports both the new ARGB color schema and legacy RGB + overlay_opacity
//! migration, matching the C++ `parse_settings` logic.

use crate::types::{ActivationMethod, Color, Settings};
use serde_json::Value;

/// Parse an ARGB color string like `"#80FF00FF"` (alpha, red, green, blue).
/// Returns `Some((a, r, g, b))` on success.
pub fn parse_argb(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#')?;
    if s.len() != 8 {
        return None;
    }
    let a = u8::from_str_radix(&s[0..2], 16).ok()?;
    let r = u8::from_str_radix(&s[2..4], 16).ok()?;
    let g = u8::from_str_radix(&s[4..6], 16).ok()?;
    let b = u8::from_str_radix(&s[6..8], 16).ok()?;
    Some((a, r, g, b))
}

/// Parse an RGB color string like `"#FF00FF"`.
/// Returns `Some((r, g, b))` on success, alpha is NOT included.
pub fn parse_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Convert a legacy overlay opacity percentage (0–100) to an alpha byte (0–255).
/// Matches C++ `LegacyOpacityToAlpha`.
pub fn legacy_opacity_to_alpha(percent: i32) -> u8 {
    if percent < 0 {
        return 255;
    }
    let clamped = percent.min(100);
    ((clamped * 255 + 50) / 100) as u8
}

/// Parse a color value from the settings JSON. Handles both ARGB and legacy RGB+opacity.
fn parse_color(value_str: &str, legacy_opacity: Option<i32>) -> Option<Color> {
    // Try ARGB first (new schema).
    if let Some(color) = parse_argb(value_str) {
        return Some(color);
    }
    // Fall back to RGB (legacy schema) + opacity.
    if let Some((r, g, b)) = parse_rgb(value_str) {
        let alpha = legacy_opacity.map_or(255, legacy_opacity_to_alpha);
        return Some((alpha, r, g, b));
    }
    None
}

/// Parse FindMyMouse settings from a PowerToys JSON config string.
///
/// Expected format:
/// ```json
/// {
///   "name": "Find My Mouse",
///   "version": "2.0",
///   "properties": {
///     "activation_method": { "value": 0 },
///     "background_color": { "value": "#80000000" },
///     ...
///   }
/// }
/// ```
pub fn parse_settings_json(json_str: &str) -> Settings {
    let mut settings = Settings::default();

    let root: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return settings,
    };

    let properties = match root.get("properties") {
        Some(Value::Object(p)) => p,
        _ => return settings,
    };

    // Parse activation method.
    if let Some(val) = get_int_value(properties, "activation_method") {
        // Handle legacy v1.0 migration: value 1 meant ShakeMouse in v1.0.
        let version = root
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let method = if version == "1.0" && val == 1 {
            Some(ActivationMethod::ShakeMouse)
        } else {
            ActivationMethod::from_i32(val)
        };
        if let Some(m) = method {
            settings.activation_method = m;
        }
    }

    if let Some(val) = get_bool_value(properties, "include_win_key") {
        settings.include_win_key = val;
    }

    if let Some(val) = get_bool_value(properties, "do_not_activate_on_game_mode") {
        settings.do_not_activate_on_game_mode = val;
    }

    // Legacy overlay opacity (read first, used for color migration).
    let legacy_opacity = get_int_value(properties, "overlay_opacity").filter(|&v| v >= 0 && v <= 100);

    if let Some(color_str) = get_string_value(properties, "background_color") {
        if let Some(color) = parse_color(&color_str, legacy_opacity) {
            settings.background_color = color;
        }
    }

    if let Some(color_str) = get_string_value(properties, "spotlight_color") {
        if let Some(color) = parse_color(&color_str, legacy_opacity) {
            settings.spotlight_color = color;
        }
    }

    if let Some(val) = get_int_value(properties, "spotlight_radius") {
        if val >= 0 {
            settings.spotlight_radius = val;
        }
    }

    if let Some(val) = get_int_value(properties, "animation_duration_ms") {
        if val >= 0 {
            settings.animation_duration_ms = val;
        }
    }

    if let Some(val) = get_int_value(properties, "spotlight_initial_zoom") {
        if val >= 0 {
            settings.spotlight_initial_zoom = val;
        }
    }

    if let Some(val) = get_int_value(properties, "shaking_minimum_distance") {
        if val >= 0 {
            settings.shake_minimum_distance = val;
        }
    }

    if let Some(val) = get_int_value(properties, "shaking_interval_ms") {
        if val >= 0 {
            settings.shake_interval_ms = val;
        }
    }

    if let Some(val) = get_int_value(properties, "shaking_factor") {
        if val >= 0 {
            settings.shake_factor = val;
        }
    }

    // Excluded apps: newline-separated string, uppercased.
    if let Some(apps_str) = get_string_value(properties, "excluded_apps") {
        let apps: Vec<String> = apps_str
            .to_uppercase()
            .split(|c: char| c == '\r' || c == '\n')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        settings.excluded_apps = apps;
    }

    settings
}

// ── JSON helpers ──

fn get_int_value(props: &serde_json::Map<String, Value>, key: &str) -> Option<i32> {
    props
        .get(key)?
        .get("value")?
        .as_f64()
        .map(|v| v as i32)
}

fn get_bool_value(props: &serde_json::Map<String, Value>, key: &str) -> Option<bool> {
    props.get(key)?.get("value")?.as_bool()
}

fn get_string_value(props: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    props
        .get(key)?
        .get("value")?
        .as_str()
        .map(|s| s.to_string())
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Color parsing ──

    #[test]
    fn parse_argb_valid() {
        assert_eq!(parse_argb("#80000000"), Some((128, 0, 0, 0)));
        assert_eq!(parse_argb("#80FFFFFF"), Some((128, 255, 255, 255)));
        assert_eq!(parse_argb("#FF112233"), Some((255, 17, 34, 51)));
    }

    #[test]
    fn parse_argb_invalid() {
        assert_eq!(parse_argb("#000000"), None); // too short (RGB)
        assert_eq!(parse_argb("80000000"), None); // no #
        assert_eq!(parse_argb("#ZZZZZZZZ"), None); // invalid hex
        assert_eq!(parse_argb(""), None);
    }

    #[test]
    fn parse_rgb_valid() {
        assert_eq!(parse_rgb("#000000"), Some((0, 0, 0)));
        assert_eq!(parse_rgb("#FFFFFF"), Some((255, 255, 255)));
        assert_eq!(parse_rgb("#1A2B3C"), Some((26, 43, 60)));
    }

    #[test]
    fn parse_rgb_invalid() {
        assert_eq!(parse_rgb("#80000000"), None); // too long (ARGB)
        assert_eq!(parse_rgb("000000"), None); // no #
        assert_eq!(parse_rgb("#GGGGGG"), None);
    }

    // ── Legacy opacity conversion ──

    #[test]
    fn legacy_opacity_to_alpha_values() {
        assert_eq!(legacy_opacity_to_alpha(0), 0);
        assert_eq!(legacy_opacity_to_alpha(50), 128);
        assert_eq!(legacy_opacity_to_alpha(100), 255);
        assert_eq!(legacy_opacity_to_alpha(-1), 255); // negative → fully opaque
        assert_eq!(legacy_opacity_to_alpha(200), 255); // clamped to 100
    }

    // ── Full settings parsing ──

    #[test]
    fn parse_default_settings_from_empty_json() {
        let s = parse_settings_json("{}");
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn parse_invalid_json_returns_defaults() {
        let s = parse_settings_json("not json at all");
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn parse_complete_settings() {
        let json = r##"{
            "name": "Find My Mouse",
            "version": "2.0",
            "properties": {
                "activation_method": { "value": 2 },
                "include_win_key": { "value": true },
                "do_not_activate_on_game_mode": { "value": false },
                "background_color": { "value": "#FF112233" },
                "spotlight_color": { "value": "#80AABBCC" },
                "spotlight_radius": { "value": 200 },
                "animation_duration_ms": { "value": 300 },
                "spotlight_initial_zoom": { "value": 5 },
                "shaking_minimum_distance": { "value": 2000 },
                "shaking_interval_ms": { "value": 500 },
                "shaking_factor": { "value": 300 },
                "excluded_apps": { "value": "notepad.exe\r\ncalc.exe" }
            }
        }"##;

        let s = parse_settings_json(json);
        assert_eq!(s.activation_method, ActivationMethod::ShakeMouse);
        assert!(s.include_win_key);
        assert!(!s.do_not_activate_on_game_mode);
        assert_eq!(s.background_color, (255, 17, 34, 51));
        assert_eq!(s.spotlight_color, (128, 170, 187, 204));
        assert_eq!(s.spotlight_radius, 200);
        assert_eq!(s.animation_duration_ms, 300);
        assert_eq!(s.spotlight_initial_zoom, 5);
        assert_eq!(s.shake_minimum_distance, 2000);
        assert_eq!(s.shake_interval_ms, 500);
        assert_eq!(s.shake_factor, 300);
        assert_eq!(s.excluded_apps, vec!["NOTEPAD.EXE", "CALC.EXE"]);
    }

    // ── Legacy v1.0 activation method migration ──

    #[test]
    fn legacy_v1_activation_method_1_maps_to_shake() {
        let json = r##"{
            "version": "1.0",
            "properties": {
                "activation_method": { "value": 1 }
            }
        }"##;

        let s = parse_settings_json(json);
        assert_eq!(s.activation_method, ActivationMethod::ShakeMouse);
    }

    #[test]
    fn v2_activation_method_1_maps_to_double_right_ctrl() {
        let json = r##"{
            "version": "2.0",
            "properties": {
                "activation_method": { "value": 1 }
            }
        }"##;

        let s = parse_settings_json(json);
        assert_eq!(s.activation_method, ActivationMethod::DoubleRightCtrl);
    }

    // ── Legacy color migration with overlay_opacity ──

    #[test]
    fn legacy_rgb_color_with_opacity_migration() {
        let json = r##"{
            "properties": {
                "overlay_opacity": { "value": 50 },
                "background_color": { "value": "#000000" },
                "spotlight_color": { "value": "#FFFFFF" }
            }
        }"##;

        let s = parse_settings_json(json);
        // 50% opacity → alpha = (50*255+50)/100 = 128
        assert_eq!(s.background_color, (128, 0, 0, 0));
        assert_eq!(s.spotlight_color, (128, 255, 255, 255));
    }

    #[test]
    fn argb_color_ignores_legacy_opacity() {
        let json = r##"{
            "properties": {
                "overlay_opacity": { "value": 50 },
                "background_color": { "value": "#FF112233" }
            }
        }"##;

        let s = parse_settings_json(json);
        // ARGB parsed → overlay_opacity is ignored.
        assert_eq!(s.background_color, (255, 17, 34, 51));
    }

    // ── Excluded apps parsing ──

    #[test]
    fn excluded_apps_trimmed_and_uppercased() {
        let json = r##"{
            "properties": {
                "excluded_apps": { "value": "  notepad.exe  \n  calc.EXE  \r\n  " }
            }
        }"##;

        let s = parse_settings_json(json);
        assert_eq!(s.excluded_apps, vec!["NOTEPAD.EXE", "CALC.EXE"]);
    }

    #[test]
    fn excluded_apps_empty_string() {
        let json = r##"{
            "properties": {
                "excluded_apps": { "value": "" }
            }
        }"##;

        let s = parse_settings_json(json);
        assert!(s.excluded_apps.is_empty());
    }

    // ── Partial settings: missing fields use defaults ──

    #[test]
    fn partial_settings_use_defaults() {
        let json = r##"{
            "properties": {
                "spotlight_radius": { "value": 250 }
            }
        }"##;

        let s = parse_settings_json(json);
        assert_eq!(s.spotlight_radius, 250);
        // Everything else should be default.
        assert_eq!(s.activation_method, ActivationMethod::DoubleLeftCtrl);
        assert_eq!(s.animation_duration_ms, 500);
    }

    // ── Invalid property values don't crash, use defaults ──

    #[test]
    fn invalid_activation_method_uses_default() {
        let json = r##"{
            "properties": {
                "activation_method": { "value": 99 }
            }
        }"##;

        let s = parse_settings_json(json);
        assert_eq!(s.activation_method, ActivationMethod::DoubleLeftCtrl);
    }

    #[test]
    fn negative_radius_uses_default() {
        let json = r##"{
            "properties": {
                "spotlight_radius": { "value": -5 }
            }
        }"##;

        let s = parse_settings_json(json);
        assert_eq!(s.spotlight_radius, 100); // default
    }
}

// Implement PartialEq for Settings to support assert_eq! in tests.
impl PartialEq for Settings {
    fn eq(&self, other: &Self) -> bool {
        self.activation_method == other.activation_method
            && self.include_win_key == other.include_win_key
            && self.do_not_activate_on_game_mode == other.do_not_activate_on_game_mode
            && self.spotlight_radius == other.spotlight_radius
            && self.animation_duration_ms == other.animation_duration_ms
            && self.spotlight_initial_zoom == other.spotlight_initial_zoom
            && self.background_color == other.background_color
            && self.spotlight_color == other.spotlight_color
            && self.shake_minimum_distance == other.shake_minimum_distance
            && self.shake_interval_ms == other.shake_interval_ms
            && self.shake_factor == other.shake_factor
            && self.excluded_apps == other.excluded_apps
    }
}
