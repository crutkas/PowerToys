// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Core types for FindMyMouse — platform-independent, no Win32 dependencies.

use serde::{Deserialize, Serialize};

/// How FindMyMouse is activated. Discriminant values match the C++ enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ActivationMethod {
    DoubleLeftCtrl = 0,
    DoubleRightCtrl = 1,
    ShakeMouse = 2,
    Shortcut = 3,
}

impl ActivationMethod {
    /// Try to convert an integer to an ActivationMethod.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::DoubleLeftCtrl),
            1 => Some(Self::DoubleRightCtrl),
            2 => Some(Self::ShakeMouse),
            3 => Some(Self::Shortcut),
            _ => None,
        }
    }
}

/// State machine for double-ctrl-click detection.
/// Matches the C++ `SonarState` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SonarState {
    /// No ctrl key pressed, waiting.
    Idle,
    /// First ctrl key pressed — timestamp and cursor position recorded.
    ControlDown1,
    /// First ctrl key released — waiting for second press within timing window.
    ControlUp1,
    /// Second ctrl key pressed within window — sonar activated.
    ControlDown2,
    /// Second ctrl key released — another press will deactivate.
    ControlUp2,
}

/// ARGB color as (alpha, red, green, blue).
pub type Color = (u8, u8, u8, u8);

/// All FindMyMouse settings with defaults matching the C++ implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub activation_method: ActivationMethod,
    pub include_win_key: bool,
    pub do_not_activate_on_game_mode: bool,
    pub spotlight_radius: i32,
    pub animation_duration_ms: i32,
    pub spotlight_initial_zoom: i32,
    /// ARGB: (alpha, red, green, blue)
    pub background_color: Color,
    /// ARGB: (alpha, red, green, blue)
    pub spotlight_color: Color,
    pub shake_minimum_distance: i32,
    pub shake_interval_ms: i32,
    pub shake_factor: i32,
    pub excluded_apps: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            activation_method: ActivationMethod::DoubleLeftCtrl,
            include_win_key: false,
            do_not_activate_on_game_mode: true,
            spotlight_radius: 100,
            animation_duration_ms: 500,
            spotlight_initial_zoom: 9,
            background_color: (128, 0, 0, 0),
            spotlight_color: (128, 255, 255, 255),
            shake_minimum_distance: 1000,
            shake_interval_ms: 1000,
            shake_factor: 400,
            excluded_apps: Vec::new(),
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_method_discriminants_match_cpp() {
        assert_eq!(ActivationMethod::DoubleLeftCtrl as i32, 0);
        assert_eq!(ActivationMethod::DoubleRightCtrl as i32, 1);
        assert_eq!(ActivationMethod::ShakeMouse as i32, 2);
        assert_eq!(ActivationMethod::Shortcut as i32, 3);
    }

    #[test]
    fn activation_method_from_i32_valid() {
        assert_eq!(
            ActivationMethod::from_i32(0),
            Some(ActivationMethod::DoubleLeftCtrl)
        );
        assert_eq!(
            ActivationMethod::from_i32(1),
            Some(ActivationMethod::DoubleRightCtrl)
        );
        assert_eq!(
            ActivationMethod::from_i32(2),
            Some(ActivationMethod::ShakeMouse)
        );
        assert_eq!(
            ActivationMethod::from_i32(3),
            Some(ActivationMethod::Shortcut)
        );
    }

    #[test]
    fn activation_method_from_i32_invalid() {
        assert_eq!(ActivationMethod::from_i32(-1), None);
        assert_eq!(ActivationMethod::from_i32(4), None);
        assert_eq!(ActivationMethod::from_i32(999), None);
    }

    #[test]
    fn settings_defaults_match_cpp() {
        let s = Settings::default();
        assert_eq!(s.activation_method, ActivationMethod::DoubleLeftCtrl);
        assert!(!s.include_win_key);
        assert!(s.do_not_activate_on_game_mode);
        assert_eq!(s.spotlight_radius, 100);
        assert_eq!(s.animation_duration_ms, 500);
        assert_eq!(s.spotlight_initial_zoom, 9);
        assert_eq!(s.background_color, (128, 0, 0, 0));
        assert_eq!(s.spotlight_color, (128, 255, 255, 255));
        assert_eq!(s.shake_minimum_distance, 1000);
        assert_eq!(s.shake_interval_ms, 1000);
        assert_eq!(s.shake_factor, 400);
        assert!(s.excluded_apps.is_empty());
    }

    #[test]
    fn sonar_state_idle_is_initial() {
        let state = SonarState::Idle;
        assert_eq!(state, SonarState::Idle);
    }

    #[test]
    fn settings_serialization_roundtrip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let parsed: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.activation_method, s.activation_method);
        assert_eq!(parsed.spotlight_radius, s.spotlight_radius);
        assert_eq!(parsed.background_color, s.background_color);
        assert_eq!(parsed.spotlight_color, s.spotlight_color);
    }
}
