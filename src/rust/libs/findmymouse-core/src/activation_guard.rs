// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Activation guard — pre-activation checks for game mode and excluded apps.
//!
//! Mirrors the C++ `StartSonar` guard logic:
//! - If `do_not_activate_on_game_mode` is set and game mode is active, reject.
//! - If the foreground app is in the excluded apps list, reject.
//!
//! All external state (game mode, foreground app name) is passed in as
//! parameters so the logic is fully testable without Win32 calls.

use crate::types::Settings;

/// Result of an activation check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationCheck {
    /// Activation is allowed.
    Allow,
    /// Blocked because game mode is active and the setting forbids it.
    BlockedByGameMode,
    /// Blocked because the foreground app is in the excluded list.
    BlockedByExcludedApp,
}

/// Check whether FindMyMouse activation should proceed.
///
/// # Arguments
/// * `settings` — current FindMyMouse settings (carries the exclusion lists).
/// * `is_game_mode` — whether the system is currently in game/presentation mode
///   (mirrors `SHQueryUserNotificationState` / `detect_game_mode()` in C++).
/// * `foreground_app` — the name (or full path) of the foreground application,
///   already uppercased by the caller (matches C++ `CharUpperBuffW` convention).
pub fn can_activate(settings: &Settings, is_game_mode: bool, foreground_app: &str) -> ActivationCheck {
    // Game mode check — mirrors C++ `m_doNotActivateOnGameMode && detect_game_mode()`.
    if settings.do_not_activate_on_game_mode && is_game_mode {
        return ActivationCheck::BlockedByGameMode;
    }

    // Excluded apps check — mirrors C++ `IsForegroundAppExcluded`.
    if !settings.excluded_apps.is_empty() && !foreground_app.is_empty() {
        let app_upper = foreground_app.to_uppercase();
        for excluded in &settings.excluded_apps {
            if app_upper.contains(excluded.as_str()) {
                return ActivationCheck::BlockedByExcludedApp;
            }
        }
    }

    ActivationCheck::Allow
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Settings;

    fn default_settings() -> Settings {
        Settings::default()
    }

    // ── Game mode checks ──

    #[test]
    fn game_mode_off_allows_activation() {
        let settings = default_settings(); // do_not_activate_on_game_mode = true by default
        assert_eq!(can_activate(&settings, false, ""), ActivationCheck::Allow);
    }

    #[test]
    fn game_mode_on_with_setting_enabled_blocks() {
        let mut settings = default_settings();
        settings.do_not_activate_on_game_mode = true;
        assert_eq!(
            can_activate(&settings, true, ""),
            ActivationCheck::BlockedByGameMode
        );
    }

    #[test]
    fn game_mode_on_with_setting_disabled_allows() {
        let mut settings = default_settings();
        settings.do_not_activate_on_game_mode = false;
        assert_eq!(can_activate(&settings, true, ""), ActivationCheck::Allow);
    }

    // ── Excluded apps checks ──

    #[test]
    fn no_excluded_apps_allows_any_foreground() {
        let settings = default_settings();
        assert_eq!(
            can_activate(&settings, false, "NOTEPAD.EXE"),
            ActivationCheck::Allow
        );
    }

    #[test]
    fn excluded_app_blocks_activation() {
        let mut settings = default_settings();
        settings.excluded_apps = vec!["NOTEPAD.EXE".to_string()];
        assert_eq!(
            can_activate(&settings, false, "NOTEPAD.EXE"),
            ActivationCheck::BlockedByExcludedApp
        );
    }

    #[test]
    fn excluded_app_case_insensitive_match() {
        let mut settings = default_settings();
        settings.excluded_apps = vec!["NOTEPAD.EXE".to_string()];
        // Foreground app in lowercase — uppercased internally.
        assert_eq!(
            can_activate(&settings, false, "notepad.exe"),
            ActivationCheck::BlockedByExcludedApp
        );
    }

    #[test]
    fn excluded_app_partial_path_match() {
        let mut settings = default_settings();
        settings.excluded_apps = vec!["NOTEPAD.EXE".to_string()];
        // Full path contains the excluded app name.
        assert_eq!(
            can_activate(&settings, false, "C:\\WINDOWS\\SYSTEM32\\NOTEPAD.EXE"),
            ActivationCheck::BlockedByExcludedApp
        );
    }

    #[test]
    fn non_excluded_app_allows_activation() {
        let mut settings = default_settings();
        settings.excluded_apps = vec!["NOTEPAD.EXE".to_string()];
        assert_eq!(
            can_activate(&settings, false, "CALC.EXE"),
            ActivationCheck::Allow
        );
    }

    #[test]
    fn multiple_excluded_apps_second_matches() {
        let mut settings = default_settings();
        settings.excluded_apps = vec!["NOTEPAD.EXE".to_string(), "CALC.EXE".to_string()];
        assert_eq!(
            can_activate(&settings, false, "CALC.EXE"),
            ActivationCheck::BlockedByExcludedApp
        );
    }

    #[test]
    fn empty_foreground_app_not_blocked() {
        let mut settings = default_settings();
        settings.excluded_apps = vec!["NOTEPAD.EXE".to_string()];
        assert_eq!(can_activate(&settings, false, ""), ActivationCheck::Allow);
    }

    // ── Combined checks: game mode takes priority ──

    #[test]
    fn game_mode_checked_before_excluded_apps() {
        let mut settings = default_settings();
        settings.do_not_activate_on_game_mode = true;
        settings.excluded_apps = vec!["NOTEPAD.EXE".to_string()];
        // Both conditions true — game mode check comes first.
        assert_eq!(
            can_activate(&settings, true, "NOTEPAD.EXE"),
            ActivationCheck::BlockedByGameMode
        );
    }

    #[test]
    fn all_clear_allows_activation() {
        let mut settings = default_settings();
        settings.do_not_activate_on_game_mode = true;
        settings.excluded_apps = vec!["NOTEPAD.EXE".to_string()];
        assert_eq!(
            can_activate(&settings, false, "CALC.EXE"),
            ActivationCheck::Allow
        );
    }
}
