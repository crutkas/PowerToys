use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::hotkey_conflict::Hotkey;

/// Dashboard sort order matching the C++ `DashboardSortOrder` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DashboardSortOrder {
    Alphabetical = 0,
    ByStatus = 1,
}

impl Default for DashboardSortOrder {
    fn default() -> Self {
        Self::Alphabetical
    }
}

/// Parse a `DashboardSortOrder` from a `serde_json::Value`.
/// Accepts integer (0/1) or string ("Alphabetical"/"ByStatus").
pub fn parse_dashboard_sort_order(
    value: &serde_json::Value,
    fallback: DashboardSortOrder,
) -> DashboardSortOrder {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i == DashboardSortOrder::ByStatus as i64 {
                    return DashboardSortOrder::ByStatus;
                }
                return DashboardSortOrder::Alphabetical;
            }
            fallback
        }
        serde_json::Value::String(s) => match s.as_str() {
            "ByStatus" => DashboardSortOrder::ByStatus,
            "Alphabetical" => DashboardSortOrder::Alphabetical,
            _ => fallback,
        },
        _ => fallback,
    }
}

/// Mirrors the C++ `GeneralSettings` struct (parsing-only; no Win32 apply logic).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    #[serde(default)]
    pub is_startup_enabled: bool,
    #[serde(default = "default_true")]
    pub show_system_tray_icon: bool,
    #[serde(default)]
    pub show_theme_adaptive_tray_icon: bool,
    #[serde(default)]
    pub startup_disabled_reason: String,
    #[serde(default)]
    pub is_modules_enabled_map: HashMap<String, bool>,
    #[serde(default)]
    pub is_elevated: bool,
    #[serde(default)]
    pub is_run_elevated: bool,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default = "default_true")]
    pub enable_warnings_elevated_apps: bool,
    #[serde(default = "default_true")]
    pub enable_quick_access: bool,
    #[serde(default)]
    pub quick_access_shortcut: Option<Hotkey>,
    #[serde(default = "default_true")]
    pub show_new_updates_toast_notification: bool,
    #[serde(default = "default_true")]
    pub download_updates_automatically: bool,
    #[serde(default = "default_true")]
    pub show_whats_new_after_updates: bool,
    #[serde(default = "default_true")]
    pub enable_experimentation: bool,
    #[serde(default)]
    pub dashboard_sort_order: DashboardSortOrder,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub system_theme: String,
    #[serde(default)]
    pub powertoys_version: String,
    #[serde(default)]
    pub ignored_conflict_properties: serde_json::Value,
}

fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "system".to_string()
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            is_startup_enabled: false,
            show_system_tray_icon: true,
            show_theme_adaptive_tray_icon: false,
            startup_disabled_reason: String::new(),
            is_modules_enabled_map: HashMap::new(),
            is_elevated: false,
            is_run_elevated: false,
            is_admin: false,
            enable_warnings_elevated_apps: true,
            enable_quick_access: true,
            quick_access_shortcut: None,
            show_new_updates_toast_notification: true,
            download_updates_automatically: true,
            show_whats_new_after_updates: true,
            enable_experimentation: true,
            dashboard_sort_order: DashboardSortOrder::Alphabetical,
            theme: "system".to_string(),
            system_theme: String::new(),
            powertoys_version: String::new(),
            ignored_conflict_properties: serde_json::json!({"ignored_shortcuts": []}),
        }
    }
}

/// Parse a general-settings JSON string, applying C++ validation rules.
pub fn parse_settings_json(json: &str) -> Result<GeneralSettings, serde_json::Error> {
    // Intermediate raw representation for custom parsing.
    let raw: serde_json::Value = serde_json::from_str(json)?;
    let obj = raw.as_object();

    let mut settings = GeneralSettings::default();

    if let Some(map) = obj {
        // ── booleans with defaults ──
        if let Some(v) = map.get("startup").and_then(|v| v.as_bool()) {
            settings.is_startup_enabled = v;
        }
        if let Some(v) = map.get("show_tray_icon").and_then(|v| v.as_bool()) {
            settings.show_system_tray_icon = v;
        }
        if let Some(v) = map
            .get("show_theme_adaptive_tray_icon")
            .and_then(|v| v.as_bool())
        {
            settings.show_theme_adaptive_tray_icon = v;
        }
        if let Some(v) = map.get("run_elevated").and_then(|v| v.as_bool()) {
            settings.is_run_elevated = v;
        }
        if let Some(v) = map.get("is_elevated").and_then(|v| v.as_bool()) {
            settings.is_elevated = v;
        }
        if let Some(v) = map.get("is_admin").and_then(|v| v.as_bool()) {
            settings.is_admin = v;
        }
        if let Some(v) = map
            .get("show_new_updates_toast_notification")
            .and_then(|v| v.as_bool())
        {
            settings.show_new_updates_toast_notification = v;
        }
        if let Some(v) = map
            .get("download_updates_automatically")
            .and_then(|v| v.as_bool())
        {
            settings.download_updates_automatically = v;
        }
        if let Some(v) = map
            .get("show_whats_new_after_updates")
            .and_then(|v| v.as_bool())
        {
            settings.show_whats_new_after_updates = v;
        }
        if let Some(v) = map
            .get("enable_experimentation")
            .and_then(|v| v.as_bool())
        {
            settings.enable_experimentation = v;
        }
        if let Some(v) = map
            .get("enable_warnings_elevated_apps")
            .and_then(|v| v.as_bool())
        {
            settings.enable_warnings_elevated_apps = v;
        }
        if let Some(v) = map.get("enable_quick_access").and_then(|v| v.as_bool()) {
            settings.enable_quick_access = v;
        }

        // ── strings ──
        if let Some(v) = map
            .get("startup_disabled_reason")
            .and_then(|v| v.as_str())
        {
            settings.startup_disabled_reason = v.to_string();
        }
        if let Some(v) = map.get("system_theme").and_then(|v| v.as_str()) {
            settings.system_theme = v.to_string();
        }
        if let Some(v) = map.get("powertoys_version").and_then(|v| v.as_str()) {
            settings.powertoys_version = v.to_string();
        }

        // ── theme with validation (matches C++) ──
        if let Some(v) = map.get("theme").and_then(|v| v.as_str()) {
            settings.theme = match v {
                "dark" | "light" => v.to_string(),
                _ => "system".to_string(),
            };
        }

        // ── dashboard sort order ──
        if let Some(v) = map.get("dashboard_sort_order") {
            settings.dashboard_sort_order =
                parse_dashboard_sort_order(v, DashboardSortOrder::Alphabetical);
        }

        // ── enabled map ──
        if let Some(enabled) = map.get("enabled").and_then(|v| v.as_object()) {
            for (k, v) in enabled {
                if let Some(b) = v.as_bool() {
                    settings.is_modules_enabled_map.insert(k.clone(), b);
                }
            }
        }

        // ── quick_access_shortcut (hotkey object) ──
        if let Some(obj) = map.get("quick_access_shortcut").and_then(|v| v.as_object()) {
            settings.quick_access_shortcut = Some(Hotkey {
                win: obj.get("win").and_then(|v| v.as_bool()).unwrap_or(false),
                ctrl: obj.get("ctrl").and_then(|v| v.as_bool()).unwrap_or(false),
                shift: obj.get("shift").and_then(|v| v.as_bool()).unwrap_or(false),
                alt: obj.get("alt").and_then(|v| v.as_bool()).unwrap_or(false),
                key: obj
                    .get("key")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u8,
            });
        }

        // ── ignored_conflict_properties ──
        if let Some(v) = map.get("ignored_conflict_properties") {
            if v.is_object() {
                settings.ignored_conflict_properties = v.clone();
                // Ensure shape has ignored_shortcuts array.
                if settings.ignored_conflict_properties.get("ignored_shortcuts").is_none() {
                    settings.ignored_conflict_properties["ignored_shortcuts"] =
                        serde_json::json!([]);
                }
            }
        }
    }

    Ok(settings)
}

impl GeneralSettings {
    /// Serialize to JSON matching the C++ `to_json()` format.
    pub fn to_json(&self) -> serde_json::Value {
        let mut result = serde_json::Map::new();

        result.insert("startup".into(), serde_json::json!(self.is_startup_enabled));
        if !self.startup_disabled_reason.is_empty() {
            result.insert(
                "startup_disabled_reason".into(),
                serde_json::json!(self.startup_disabled_reason),
            );
        }

        let enabled: serde_json::Map<String, serde_json::Value> = self
            .is_modules_enabled_map
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::json!(v)))
            .collect();
        result.insert("enabled".into(), serde_json::Value::Object(enabled));

        result.insert(
            "show_tray_icon".into(),
            serde_json::json!(self.show_system_tray_icon),
        );
        result.insert(
            "show_theme_adaptive_tray_icon".into(),
            serde_json::json!(self.show_theme_adaptive_tray_icon),
        );
        result.insert("is_elevated".into(), serde_json::json!(self.is_elevated));
        result.insert(
            "run_elevated".into(),
            serde_json::json!(self.is_run_elevated),
        );
        result.insert(
            "show_new_updates_toast_notification".into(),
            serde_json::json!(self.show_new_updates_toast_notification),
        );
        result.insert(
            "download_updates_automatically".into(),
            serde_json::json!(self.download_updates_automatically),
        );
        result.insert(
            "show_whats_new_after_updates".into(),
            serde_json::json!(self.show_whats_new_after_updates),
        );
        result.insert(
            "enable_experimentation".into(),
            serde_json::json!(self.enable_experimentation),
        );
        result.insert(
            "dashboard_sort_order".into(),
            serde_json::json!(self.dashboard_sort_order as i32),
        );
        result.insert("is_admin".into(), serde_json::json!(self.is_admin));
        result.insert(
            "enable_warnings_elevated_apps".into(),
            serde_json::json!(self.enable_warnings_elevated_apps),
        );
        result.insert(
            "enable_quick_access".into(),
            serde_json::json!(self.enable_quick_access),
        );
        if let Some(ref hk) = self.quick_access_shortcut {
            result.insert(
                "quick_access_shortcut".into(),
                serde_json::json!({
                    "win": hk.win,
                    "ctrl": hk.ctrl,
                    "shift": hk.shift,
                    "alt": hk.alt,
                    "key": hk.key,
                }),
            );
        }
        result.insert("theme".into(), serde_json::json!(self.theme));
        result.insert(
            "system_theme".into(),
            serde_json::json!(self.system_theme),
        );
        result.insert(
            "powertoys_version".into(),
            serde_json::json!(self.powertoys_version),
        );
        result.insert(
            "ignored_conflict_properties".into(),
            self.ignored_conflict_properties.clone(),
        );

        serde_json::Value::Object(result)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 1. Parse theme "dark".
    #[test]
    fn parse_theme_dark() {
        let s = parse_settings_json(r#"{"theme": "dark"}"#).unwrap();
        assert_eq!(s.theme, "dark");
    }

    // 2. Parse theme "light".
    #[test]
    fn parse_theme_light() {
        let s = parse_settings_json(r#"{"theme": "light"}"#).unwrap();
        assert_eq!(s.theme, "light");
    }

    // 3. Parse theme "system".
    #[test]
    fn parse_theme_system() {
        let s = parse_settings_json(r#"{"theme": "system"}"#).unwrap();
        assert_eq!(s.theme, "system");
    }

    // 4. Invalid theme falls back to "system".
    #[test]
    fn parse_theme_invalid_falls_back_to_system() {
        let s = parse_settings_json(r#"{"theme": "neon"}"#).unwrap();
        assert_eq!(s.theme, "system");
    }

    // 5. Dashboard sort order – number format.
    #[test]
    fn parse_dashboard_sort_order_number() {
        let s = parse_settings_json(r#"{"dashboard_sort_order": 1}"#).unwrap();
        assert_eq!(s.dashboard_sort_order, DashboardSortOrder::ByStatus);
    }

    // 6. Dashboard sort order – string format.
    #[test]
    fn parse_dashboard_sort_order_string() {
        let s = parse_settings_json(r#"{"dashboard_sort_order": "ByStatus"}"#).unwrap();
        assert_eq!(s.dashboard_sort_order, DashboardSortOrder::ByStatus);
    }

    // 7. Parse modules enabled map.
    #[test]
    fn parse_modules_enabled_map() {
        let s = parse_settings_json(
            r#"{"enabled": {"FancyZones": true, "Awake": false}}"#,
        )
        .unwrap();
        assert_eq!(s.is_modules_enabled_map.get("FancyZones"), Some(&true));
        assert_eq!(s.is_modules_enabled_map.get("Awake"), Some(&false));
    }

    // 8. Parse hotkey objects.
    #[test]
    fn parse_hotkey_objects() {
        let json = r#"{
            "quick_access_shortcut": {
                "win": true, "ctrl": false, "shift": false, "alt": true, "key": 65
            }
        }"#;
        let s = parse_settings_json(json).unwrap();
        let hk = s.quick_access_shortcut.unwrap();
        assert!(hk.win);
        assert!(hk.alt);
        assert!(!hk.ctrl);
        assert_eq!(hk.key, 65);
    }

    // 9. Serialization roundtrip.
    #[test]
    fn serialization_roundtrip() {
        let original = parse_settings_json(
            r#"{
                "theme": "dark",
                "startup": true,
                "show_tray_icon": false,
                "dashboard_sort_order": 1,
                "enabled": {"ModA": true}
            }"#,
        )
        .unwrap();
        let json_str = original.to_json().to_string();
        let restored = parse_settings_json(&json_str).unwrap();
        assert_eq!(restored.theme, "dark");
        assert_eq!(restored.is_startup_enabled, true);
        assert_eq!(restored.show_system_tray_icon, false);
        assert_eq!(restored.dashboard_sort_order, DashboardSortOrder::ByStatus);
        assert_eq!(restored.is_modules_enabled_map.get("ModA"), Some(&true));
    }

    // 10. Empty JSON yields defaults.
    #[test]
    fn empty_json_defaults() {
        let s = parse_settings_json("{}").unwrap();
        assert_eq!(s.theme, "system");
        assert!(s.show_system_tray_icon);
        assert!(s.enable_experimentation);
        assert_eq!(s.dashboard_sort_order, DashboardSortOrder::Alphabetical);
    }

    // 11. Partial JSON fills defaults for missing fields.
    #[test]
    fn partial_json_defaults_for_missing() {
        let s = parse_settings_json(r#"{"startup": true}"#).unwrap();
        assert!(s.is_startup_enabled);
        assert_eq!(s.theme, "system");
        assert!(s.download_updates_automatically);
    }

    // 12. Invalid JSON returns error.
    #[test]
    fn invalid_json_error() {
        assert!(parse_settings_json("not json at all").is_err());
    }

    // 13. parse_dashboard_sort_order standalone function.
    #[test]
    fn parse_dashboard_sort_order_fn_fallback() {
        let v = serde_json::json!("garbage");
        assert_eq!(
            parse_dashboard_sort_order(&v, DashboardSortOrder::ByStatus),
            DashboardSortOrder::ByStatus,
        );
    }
}
