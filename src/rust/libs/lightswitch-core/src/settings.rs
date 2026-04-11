// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! LightSwitch settings — JSON parsing matching the PowerToys format.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Schedule mode for automatic theme switching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleMode {
    Off,
    FixedHours,
    SunsetToSunrise,
    FollowNightLight,
}

impl ScheduleMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "FixedHours" => Self::FixedHours,
            "SunsetToSunrise" => Self::SunsetToSunrise,
            "FollowNightLight" => Self::FollowNightLight,
            _ => Self::Off,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::FixedHours => "FixedHours",
            Self::SunsetToSunrise => "SunsetToSunrise",
            Self::FollowNightLight => "FollowNightLight",
        }
    }
}

/// LightSwitch configuration loaded from settings.json.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightSwitchConfig {
    pub schedule_mode: ScheduleMode,
    pub latitude: String,
    pub longitude: String,
    /// Minutes since midnight for light mode start (default 480 = 08:00).
    pub light_time: i32,
    /// Minutes since midnight for dark mode start (default 1200 = 20:00).
    pub dark_time: i32,
    pub sunrise_offset: i32,
    pub sunset_offset: i32,
    pub change_system: bool,
    pub change_apps: bool,
    pub enable_dark_mode_profile: bool,
    pub enable_light_mode_profile: bool,
    pub dark_mode_profile: String,
    pub light_mode_profile: String,
}

impl Default for LightSwitchConfig {
    fn default() -> Self {
        Self {
            schedule_mode: ScheduleMode::FixedHours,
            latitude: "0.0".to_string(),
            longitude: "0.0".to_string(),
            light_time: 8 * 60,  // 08:00
            dark_time: 20 * 60,  // 20:00
            sunrise_offset: 0,
            sunset_offset: 0,
            change_system: false,
            change_apps: false,
            enable_dark_mode_profile: false,
            enable_light_mode_profile: false,
            dark_mode_profile: String::new(),
            light_mode_profile: String::new(),
        }
    }
}

/// Parse LightSwitch settings from a PowerToys JSON config string.
pub fn parse_settings_json(json_str: &str) -> LightSwitchConfig {
    let mut config = LightSwitchConfig::default();

    let root: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return config,
    };

    let props = match root.get("properties") {
        Some(Value::Object(p)) => p,
        _ => {
            // Try flat format (direct keys).
            parse_flat(&root, &mut config);
            return config;
        }
    };

    if let Some(v) = get_str(props, "scheduleMode") {
        config.schedule_mode = ScheduleMode::from_str(&v);
    }
    if let Some(v) = get_str(props, "latitude") {
        config.latitude = v;
    }
    if let Some(v) = get_str(props, "longitude") {
        config.longitude = v;
    }
    if let Some(v) = get_int(props, "lightTime") {
        config.light_time = v;
    }
    if let Some(v) = get_int(props, "darkTime") {
        config.dark_time = v;
    }
    if let Some(v) = get_int(props, "sunrise_offset") {
        config.sunrise_offset = v;
    }
    if let Some(v) = get_int(props, "sunset_offset") {
        config.sunset_offset = v;
    }
    if let Some(v) = get_bool(props, "changeSystem") {
        config.change_system = v;
    }
    if let Some(v) = get_bool(props, "changeApps") {
        config.change_apps = v;
    }
    if let Some(v) = get_bool(props, "enableDarkModeProfile") {
        config.enable_dark_mode_profile = v;
    }
    if let Some(v) = get_bool(props, "enableLightModeProfile") {
        config.enable_light_mode_profile = v;
    }
    if let Some(v) = get_str(props, "darkModeProfile") {
        config.dark_mode_profile = v;
    }
    if let Some(v) = get_str(props, "lightModeProfile") {
        config.light_mode_profile = v;
    }

    config
}

/// Parse a flat JSON format (keys at top level, no "properties" wrapper).
fn parse_flat(root: &Value, config: &mut LightSwitchConfig) {
    if let Some(v) = root.get("scheduleMode").and_then(|v| v.as_str()) {
        config.schedule_mode = ScheduleMode::from_str(v);
    }
    if let Some(v) = root.get("latitude").and_then(|v| v.as_str()) {
        config.latitude = v.to_string();
    }
    if let Some(v) = root.get("longitude").and_then(|v| v.as_str()) {
        config.longitude = v.to_string();
    }
    if let Some(v) = root.get("lightTime").and_then(|v| v.as_f64()) {
        config.light_time = v as i32;
    }
    if let Some(v) = root.get("darkTime").and_then(|v| v.as_f64()) {
        config.dark_time = v as i32;
    }
    if let Some(v) = root.get("sunrise_offset").and_then(|v| v.as_f64()) {
        config.sunrise_offset = v as i32;
    }
    if let Some(v) = root.get("sunset_offset").and_then(|v| v.as_f64()) {
        config.sunset_offset = v as i32;
    }
    if let Some(v) = root.get("changeSystem").and_then(|v| v.as_bool()) {
        config.change_system = v;
    }
    if let Some(v) = root.get("changeApps").and_then(|v| v.as_bool()) {
        config.change_apps = v;
    }
    if let Some(v) = root.get("enableDarkModeProfile").and_then(|v| v.as_bool()) {
        config.enable_dark_mode_profile = v;
    }
    if let Some(v) = root.get("enableLightModeProfile").and_then(|v| v.as_bool()) {
        config.enable_light_mode_profile = v;
    }
    if let Some(v) = root.get("darkModeProfile").and_then(|v| v.as_str()) {
        config.dark_mode_profile = v.to_string();
    }
    if let Some(v) = root.get("lightModeProfile").and_then(|v| v.as_str()) {
        config.light_mode_profile = v.to_string();
    }
}

// JSON helpers for PowerToys {"key": {"value": ...}} format.
fn get_str(props: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    props.get(key)?.get("value")?.as_str().map(|s| s.to_string())
}

fn get_int(props: &serde_json::Map<String, Value>, key: &str) -> Option<i32> {
    props.get(key)?.get("value")?.as_f64().map(|v| v as i32)
}

fn get_bool(props: &serde_json::Map<String, Value>, key: &str) -> Option<bool> {
    props.get(key)?.get("value")?.as_bool()
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_mode_from_str() {
        assert_eq!(ScheduleMode::from_str("FixedHours"), ScheduleMode::FixedHours);
        assert_eq!(ScheduleMode::from_str("SunsetToSunrise"), ScheduleMode::SunsetToSunrise);
        assert_eq!(ScheduleMode::from_str("FollowNightLight"), ScheduleMode::FollowNightLight);
        assert_eq!(ScheduleMode::from_str("Off"), ScheduleMode::Off);
        assert_eq!(ScheduleMode::from_str("invalid"), ScheduleMode::Off);
        assert_eq!(ScheduleMode::from_str(""), ScheduleMode::Off);
    }

    #[test]
    fn schedule_mode_roundtrip() {
        for mode in [ScheduleMode::Off, ScheduleMode::FixedHours, ScheduleMode::SunsetToSunrise, ScheduleMode::FollowNightLight] {
            assert_eq!(ScheduleMode::from_str(mode.as_str()), mode);
        }
    }

    #[test]
    fn default_config() {
        let c = LightSwitchConfig::default();
        assert_eq!(c.schedule_mode, ScheduleMode::FixedHours);
        assert_eq!(c.light_time, 480);
        assert_eq!(c.dark_time, 1200);
        assert_eq!(c.latitude, "0.0");
        assert_eq!(c.longitude, "0.0");
        assert!(!c.change_system);
        assert!(!c.change_apps);
    }

    #[test]
    fn parse_empty_json_returns_defaults() {
        let c = parse_settings_json("{}");
        assert_eq!(c, LightSwitchConfig::default());
    }

    #[test]
    fn parse_invalid_json_returns_defaults() {
        let c = parse_settings_json("not json");
        assert_eq!(c, LightSwitchConfig::default());
    }

    #[test]
    fn parse_powertoys_format() {
        let json = r#"{
            "properties": {
                "scheduleMode": { "value": "SunsetToSunrise" },
                "latitude": { "value": "40.7128" },
                "longitude": { "value": "-74.0060" },
                "lightTime": { "value": 420 },
                "darkTime": { "value": 1140 },
                "sunrise_offset": { "value": 15 },
                "sunset_offset": { "value": -30 },
                "changeSystem": { "value": true },
                "changeApps": { "value": true },
                "enableDarkModeProfile": { "value": true },
                "darkModeProfile": { "value": "Night" }
            }
        }"#;
        let c = parse_settings_json(json);
        assert_eq!(c.schedule_mode, ScheduleMode::SunsetToSunrise);
        assert_eq!(c.latitude, "40.7128");
        assert_eq!(c.longitude, "-74.0060");
        assert_eq!(c.light_time, 420);
        assert_eq!(c.dark_time, 1140);
        assert_eq!(c.sunrise_offset, 15);
        assert_eq!(c.sunset_offset, -30);
        assert!(c.change_system);
        assert!(c.change_apps);
        assert!(c.enable_dark_mode_profile);
        assert_eq!(c.dark_mode_profile, "Night");
    }

    #[test]
    fn parse_flat_format() {
        let json = r#"{
            "scheduleMode": "FollowNightLight",
            "changeSystem": true,
            "changeApps": false,
            "lightTime": 360,
            "darkTime": 1080
        }"#;
        let c = parse_settings_json(json);
        assert_eq!(c.schedule_mode, ScheduleMode::FollowNightLight);
        assert!(c.change_system);
        assert!(!c.change_apps);
        assert_eq!(c.light_time, 360);
        assert_eq!(c.dark_time, 1080);
    }

    #[test]
    fn parse_partial_settings_uses_defaults() {
        let json = r#"{
            "properties": {
                "scheduleMode": { "value": "FixedHours" }
            }
        }"#;
        let c = parse_settings_json(json);
        assert_eq!(c.schedule_mode, ScheduleMode::FixedHours);
        assert_eq!(c.light_time, 480);
        assert_eq!(c.dark_time, 1200);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let c = LightSwitchConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        let parsed: LightSwitchConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, c);
    }
}
