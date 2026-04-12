use serde::{Deserialize, Serialize};

/// Overlapping zones resolution algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlappingZonesAlgorithm {
    Smallest = 0,
    Largest = 1,
    Positional = 2,
    ClosestCenter = 3,
}

impl Default for OverlappingZonesAlgorithm {
    fn default() -> Self {
        Self::Smallest
    }
}

/// FancyZones settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub shift_drag: bool,
    pub mouse_switch: bool,
    pub mouse_middle_click_spanning_multiple_zones: bool,
    pub display_or_work_area_change_move_windows: bool,
    pub zone_set_change_flash_zones: bool,
    pub zone_set_change_move_windows: bool,
    pub override_snap_hotkeys: bool,
    pub move_window_across_monitors: bool,
    pub move_windows_based_on_position: bool,
    pub app_last_zone_move_windows: bool,
    pub open_window_on_active_monitor: bool,
    pub restore_size: bool,
    pub use_cursorpos_editor_startupscreen: bool,
    pub show_zones_on_all_monitors: bool,
    pub span_zones_across_monitors: bool,
    pub make_dragged_window_transparent: bool,
    pub window_switching: bool,
    pub quick_layout_switch: bool,
    pub flash_zones_on_quick_switch: bool,
    pub system_theme: bool,
    pub show_zone_number: bool,
    pub allow_snap_child_windows: bool,
    pub disable_round_corners: bool,
    pub zone_color: String,
    pub zone_border_color: String,
    pub zone_highlight_color: String,
    pub zone_number_color: String,
    pub zone_highlight_opacity: i32,
    pub overlapping_zones_algorithm: OverlappingZonesAlgorithm,
    pub excluded_apps: String,
    pub excluded_apps_array: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shift_drag: true,
            mouse_switch: false,
            mouse_middle_click_spanning_multiple_zones: false,
            display_or_work_area_change_move_windows: true,
            zone_set_change_flash_zones: false,
            zone_set_change_move_windows: false,
            override_snap_hotkeys: false,
            move_window_across_monitors: false,
            move_windows_based_on_position: false,
            app_last_zone_move_windows: false,
            open_window_on_active_monitor: false,
            restore_size: false,
            use_cursorpos_editor_startupscreen: true,
            show_zones_on_all_monitors: false,
            span_zones_across_monitors: false,
            make_dragged_window_transparent: true,
            window_switching: true,
            quick_layout_switch: true,
            flash_zones_on_quick_switch: true,
            system_theme: true,
            show_zone_number: true,
            allow_snap_child_windows: false,
            disable_round_corners: false,
            zone_color: "#AACDFF".to_string(),
            zone_border_color: "#FFFFFF".to_string(),
            zone_highlight_color: "#008CFF".to_string(),
            zone_number_color: "#000000".to_string(),
            zone_highlight_opacity: 50,
            overlapping_zones_algorithm: OverlappingZonesAlgorithm::Smallest,
            excluded_apps: String::new(),
            excluded_apps_array: Vec::new(),
        }
    }
}

impl Settings {
    /// Parse settings from JSON.
    pub fn from_json(json: &serde_json::Value) -> Self {
        let defaults = Self::default();

        let get_bool = |key: &str, default: bool| -> bool {
            json.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
        };
        let get_str = |key: &str, default: &str| -> String {
            json.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| default.to_string())
        };
        let get_i32 = |key: &str, default: i32| -> i32 {
            json.get(key).and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(default)
        };

        let excluded_apps = get_str("fancyzones_excluded_apps", &defaults.excluded_apps);
        let excluded_apps_array: Vec<String> = excluded_apps
            .lines()
            .map(|line| line.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            shift_drag: get_bool("fancyzones_shiftDrag", defaults.shift_drag),
            mouse_switch: get_bool("fancyzones_mouseSwitch", defaults.mouse_switch),
            mouse_middle_click_spanning_multiple_zones: get_bool("fancyzones_mouseMiddleClickSpanningMultipleZones", defaults.mouse_middle_click_spanning_multiple_zones),
            display_or_work_area_change_move_windows: get_bool("fancyzones_displayOrWorkAreaChange_moveWindows", defaults.display_or_work_area_change_move_windows),
            zone_set_change_flash_zones: get_bool("fancyzones_zoneSetChange_flashZones", defaults.zone_set_change_flash_zones),
            zone_set_change_move_windows: get_bool("fancyzones_zoneSetChange_moveWindows", defaults.zone_set_change_move_windows),
            override_snap_hotkeys: get_bool("fancyzones_overrideSnapHotkeys", defaults.override_snap_hotkeys),
            move_window_across_monitors: get_bool("fancyzones_moveWindowAcrossMonitors", defaults.move_window_across_monitors),
            move_windows_based_on_position: get_bool("fancyzones_moveWindowsBasedOnPosition", defaults.move_windows_based_on_position),
            app_last_zone_move_windows: get_bool("fancyzones_appLastZone_moveWindows", defaults.app_last_zone_move_windows),
            open_window_on_active_monitor: get_bool("fancyzones_openWindowOnActiveMonitor", defaults.open_window_on_active_monitor),
            restore_size: get_bool("fancyzones_restoreSize", defaults.restore_size),
            use_cursorpos_editor_startupscreen: get_bool("use_cursorpos_editor_startupscreen", defaults.use_cursorpos_editor_startupscreen),
            show_zones_on_all_monitors: get_bool("fancyzones_show_on_all_monitors", defaults.show_zones_on_all_monitors),
            span_zones_across_monitors: get_bool("fancyzones_span_zones_across_monitors", defaults.span_zones_across_monitors),
            make_dragged_window_transparent: get_bool("fancyzones_makeDraggedWindowTransparent", defaults.make_dragged_window_transparent),
            window_switching: get_bool("fancyzones_windowSwitching", defaults.window_switching),
            quick_layout_switch: get_bool("fancyzones_quickLayoutSwitch", defaults.quick_layout_switch),
            flash_zones_on_quick_switch: get_bool("fancyzones_flashZonesOnQuickSwitch", defaults.flash_zones_on_quick_switch),
            system_theme: get_bool("fancyzones_systemTheme", defaults.system_theme),
            show_zone_number: get_bool("fancyzones_showZoneNumber", defaults.show_zone_number),
            allow_snap_child_windows: get_bool("fancyzones_allowChildWindowSnap", defaults.allow_snap_child_windows),
            disable_round_corners: get_bool("fancyzones_disableRoundCornersOnSnap", defaults.disable_round_corners),
            zone_color: get_str("fancyzones_zoneColor", &defaults.zone_color),
            zone_border_color: get_str("fancyzones_zoneBorderColor", &defaults.zone_border_color),
            zone_highlight_color: get_str("fancyzones_zoneHighlightColor", &defaults.zone_highlight_color),
            zone_number_color: get_str("fancyzones_zoneNumberColor", &defaults.zone_number_color),
            zone_highlight_opacity: get_i32("fancyzones_highlight_opacity", defaults.zone_highlight_opacity),
            overlapping_zones_algorithm: defaults.overlapping_zones_algorithm,
            excluded_apps: excluded_apps.clone(),
            excluded_apps_array,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Settings tests (from FancyZonesSettings.Spec.cpp) ----

    fn compare_settings(expected: &Settings, actual: &Settings) {
        assert_eq!(expected.shift_drag, actual.shift_drag);
        assert_eq!(expected.mouse_switch, actual.mouse_switch);
        assert_eq!(expected.mouse_middle_click_spanning_multiple_zones, actual.mouse_middle_click_spanning_multiple_zones);
        assert_eq!(expected.display_or_work_area_change_move_windows, actual.display_or_work_area_change_move_windows);
        assert_eq!(expected.zone_set_change_flash_zones, actual.zone_set_change_flash_zones);
        assert_eq!(expected.zone_set_change_move_windows, actual.zone_set_change_move_windows);
        assert_eq!(expected.override_snap_hotkeys, actual.override_snap_hotkeys);
        assert_eq!(expected.move_window_across_monitors, actual.move_window_across_monitors);
        assert_eq!(expected.move_windows_based_on_position, actual.move_windows_based_on_position);
        assert_eq!(expected.app_last_zone_move_windows, actual.app_last_zone_move_windows);
        assert_eq!(expected.open_window_on_active_monitor, actual.open_window_on_active_monitor);
        assert_eq!(expected.restore_size, actual.restore_size);
        assert_eq!(expected.use_cursorpos_editor_startupscreen, actual.use_cursorpos_editor_startupscreen);
        assert_eq!(expected.show_zones_on_all_monitors, actual.show_zones_on_all_monitors);
        assert_eq!(expected.span_zones_across_monitors, actual.span_zones_across_monitors);
        assert_eq!(expected.make_dragged_window_transparent, actual.make_dragged_window_transparent);
        assert_eq!(expected.window_switching, actual.window_switching);
        assert_eq!(expected.quick_layout_switch, actual.quick_layout_switch);
        assert_eq!(expected.flash_zones_on_quick_switch, actual.flash_zones_on_quick_switch);
        assert_eq!(expected.system_theme, actual.system_theme);
        assert_eq!(expected.show_zone_number, actual.show_zone_number);
        assert_eq!(expected.allow_snap_child_windows, actual.allow_snap_child_windows);
        assert_eq!(expected.disable_round_corners, actual.disable_round_corners);
        assert_eq!(expected.zone_color, actual.zone_color);
        assert_eq!(expected.zone_border_color, actual.zone_border_color);
        assert_eq!(expected.zone_highlight_color, actual.zone_highlight_color);
        assert_eq!(expected.zone_number_color, actual.zone_number_color);
        assert_eq!(expected.zone_highlight_opacity, actual.zone_highlight_opacity);
        assert_eq!(expected.excluded_apps, actual.excluded_apps);
        assert_eq!(expected.excluded_apps_array, actual.excluded_apps_array);
    }

    #[test]
    fn parse() {
        let expected = Settings {
            excluded_apps: "app\r\napp1\r\napp2\r\nanother app".to_string(),
            excluded_apps_array: vec!["APP".to_string(), "APP1".to_string(), "APP2".to_string(), "ANOTHER APP".to_string()],
            ..Settings::default()
        };

        let json = serde_json::json!({
            "fancyzones_shiftDrag": expected.shift_drag,
            "fancyzones_mouseSwitch": expected.mouse_switch,
            "fancyzones_mouseMiddleClickSpanningMultipleZones": expected.mouse_middle_click_spanning_multiple_zones,
            "fancyzones_displayOrWorkAreaChange_moveWindows": expected.display_or_work_area_change_move_windows,
            "fancyzones_zoneSetChange_flashZones": expected.zone_set_change_flash_zones,
            "fancyzones_zoneSetChange_moveWindows": expected.zone_set_change_move_windows,
            "fancyzones_overrideSnapHotkeys": expected.override_snap_hotkeys,
            "fancyzones_moveWindowAcrossMonitors": expected.move_window_across_monitors,
            "fancyzones_moveWindowsBasedOnPosition": expected.move_windows_based_on_position,
            "fancyzones_appLastZone_moveWindows": expected.app_last_zone_move_windows,
            "fancyzones_openWindowOnActiveMonitor": expected.open_window_on_active_monitor,
            "fancyzones_restoreSize": expected.restore_size,
            "use_cursorpos_editor_startupscreen": expected.use_cursorpos_editor_startupscreen,
            "fancyzones_show_on_all_monitors": expected.show_zones_on_all_monitors,
            "fancyzones_span_zones_across_monitors": expected.span_zones_across_monitors,
            "fancyzones_makeDraggedWindowTransparent": expected.make_dragged_window_transparent,
            "fancyzones_windowSwitching": expected.window_switching,
            "fancyzones_quickLayoutSwitch": expected.quick_layout_switch,
            "fancyzones_flashZonesOnQuickSwitch": expected.flash_zones_on_quick_switch,
            "fancyzones_systemTheme": expected.system_theme,
            "fancyzones_showZoneNumber": expected.show_zone_number,
            "fancyzones_allowChildWindowSnap": expected.allow_snap_child_windows,
            "fancyzones_disableRoundCornersOnSnap": expected.disable_round_corners,
            "fancyzones_zoneColor": expected.zone_color,
            "fancyzones_zoneBorderColor": expected.zone_border_color,
            "fancyzones_zoneHighlightColor": expected.zone_highlight_color,
            "fancyzones_zoneNumberColor": expected.zone_number_color,
            "fancyzones_highlight_opacity": expected.zone_highlight_opacity,
            "fancyzones_excluded_apps": expected.excluded_apps,
        });

        let actual = Settings::from_json(&json);
        compare_settings(&expected, &actual);
    }

    #[test]
    fn parse_invalid() {
        let json = serde_json::json!({
            "non_fancyzones_value": false,
        });
        let actual = Settings::from_json(&json);
        compare_settings(&Settings::default(), &actual);
    }

    #[test]
    fn parse_empty() {
        let json = serde_json::json!({});
        let actual = Settings::from_json(&json);
        compare_settings(&Settings::default(), &actual);
    }
}
