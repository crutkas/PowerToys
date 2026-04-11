// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! LightSwitch state manager — runtime state machine.
//!
//! All external dependencies (theme reading, time, theme application) are
//! injected via the `ThemeProvider` trait, making the state machine fully
//! testable without registry or system calls.

use crate::schedule::{coordinates_are_valid, crossed_boundary, should_be_light};
use crate::settings::{LightSwitchConfig, ScheduleMode};
use crate::sun::calculate_sunrise_sunset;

/// Trait for abstracting system-level theme operations.
pub trait ThemeProvider {
    fn get_current_system_theme(&self) -> bool; // true = light
    fn get_current_apps_theme(&self) -> bool;
    fn is_night_light_enabled(&self) -> bool;
    fn set_system_theme(&self, is_light: bool);
    fn set_apps_theme(&self, is_light: bool);
    /// Get current time as minutes since midnight (0–1439).
    fn get_now_minutes(&self) -> i32;
    /// Get current day of month (1–31).
    fn get_current_day(&self) -> i32;
    /// Get timezone bias in minutes for sunrise/sunset calculation.
    fn get_timezone_bias_minutes(&self) -> i32;
    /// Get current date as (year, month, day).
    fn get_current_date(&self) -> (i32, i32, i32);
}

/// Runtime-only state (not persisted in settings).
#[derive(Debug, Clone, PartialEq)]
pub struct LightSwitchState {
    pub last_applied_mode: ScheduleMode,
    pub is_manual_override: bool,
    pub is_system_light_active: bool,
    pub is_apps_light_active: bool,
    pub is_night_light_active: bool,
    pub last_evaluated_day: i32,
    pub last_tick_minutes: i32,
    pub effective_light_minutes: i32,
    pub effective_dark_minutes: i32,
}

impl Default for LightSwitchState {
    fn default() -> Self {
        Self {
            last_applied_mode: ScheduleMode::Off,
            is_manual_override: false,
            is_system_light_active: false,
            is_apps_light_active: false,
            is_night_light_active: false,
            last_evaluated_day: -1,
            last_tick_minutes: -1,
            effective_light_minutes: 0,
            effective_dark_minutes: 0,
        }
    }
}

/// The state manager that drives theme switching decisions.
pub struct StateManager {
    state: LightSwitchState,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            state: LightSwitchState::default(),
        }
    }

    pub fn state(&self) -> &LightSwitchState {
        &self.state
    }

    /// Called at startup to align internal state with system theme.
    pub fn sync_initial_theme_state(
        &mut self,
        config: &LightSwitchConfig,
        provider: &dyn ThemeProvider,
    ) {
        self.state.is_system_light_active = provider.get_current_system_theme();
        self.state.is_apps_light_active = provider.get_current_apps_theme();
        self.state.is_night_light_active = provider.is_night_light_enabled();
        self.evaluate_and_apply(config, provider);
    }

    /// Called when settings.json changes.
    pub fn on_settings_changed(
        &mut self,
        config: &LightSwitchConfig,
        provider: &dyn ThemeProvider,
    ) {
        if self.state.is_manual_override {
            self.state.is_manual_override = false;
        }
        self.evaluate_and_apply(config, provider);
    }

    /// Called every minute.
    pub fn on_tick(&mut self, config: &LightSwitchConfig, provider: &dyn ThemeProvider) {
        if self.state.last_applied_mode != ScheduleMode::FollowNightLight {
            self.evaluate_and_apply(config, provider);
        }
    }

    /// Called when manual override is triggered.
    pub fn on_manual_override(
        &mut self,
        config: &LightSwitchConfig,
        provider: &dyn ThemeProvider,
    ) {
        self.state.is_manual_override = !self.state.is_manual_override;

        if self.state.is_manual_override {
            self.state.is_system_light_active = provider.get_current_system_theme();
            self.state.is_apps_light_active = provider.get_current_apps_theme();
        }

        self.evaluate_and_apply(config, provider);
    }

    /// Called when Night Light registry changes.
    pub fn on_night_light_change(
        &mut self,
        config: &LightSwitchConfig,
        provider: &dyn ThemeProvider,
    ) {
        let new_state = provider.is_night_light_enabled();

        if self.state.last_applied_mode == ScheduleMode::FollowNightLight
            && self.state.is_manual_override
        {
            self.state.is_manual_override = false;
        }

        self.state.is_night_light_active = new_state;
        self.evaluate_and_apply(config, provider);
    }

    /// Core evaluation: determine and apply the correct theme.
    fn evaluate_and_apply(
        &mut self,
        config: &LightSwitchConfig,
        provider: &dyn ThemeProvider,
    ) {
        let now = provider.get_now_minutes();

        if config.schedule_mode == ScheduleMode::Off {
            self.state.last_tick_minutes = now;
            return;
        }

        let coords_valid = coordinates_are_valid(&config.latitude, &config.longitude);

        // Handle SunsetToSunrise recalculation.
        if config.schedule_mode == ScheduleMode::SunsetToSunrise && coords_valid {
            let current_day = provider.get_current_day();
            let new_day = self.state.last_evaluated_day != current_day;
            let mode_changed = self.state.last_applied_mode != ScheduleMode::SunsetToSunrise;

            if new_day || mode_changed {
                let (year, month, day) = provider.get_current_date();
                let bias = provider.get_timezone_bias_minutes();
                let lat: f64 = config.latitude.parse().unwrap_or(0.0);
                let lon: f64 = config.longitude.parse().unwrap_or(0.0);

                if let Some(times) = calculate_sunrise_sunset(lat, lon, year, month, day, bias) {
                    self.state.effective_light_minutes =
                        times.sunrise_minutes() + config.sunrise_offset;
                    self.state.effective_dark_minutes =
                        times.sunset_minutes() + config.sunset_offset;
                }
                self.state.last_evaluated_day = current_day;
            } else {
                self.state.effective_light_minutes = config.light_time + config.sunrise_offset;
                self.state.effective_dark_minutes = config.dark_time + config.sunset_offset;
            }
        } else if config.schedule_mode == ScheduleMode::FixedHours {
            self.state.effective_light_minutes = config.light_time;
            self.state.effective_dark_minutes = config.dark_time;
        }

        // Handle manual override logic.
        if self.state.is_manual_override {
            if self.state.last_tick_minutes != -1 {
                let crossed = crossed_boundary(
                    self.state.last_tick_minutes,
                    now,
                    self.state.effective_light_minutes,
                    self.state.effective_dark_minutes,
                );
                if crossed {
                    self.state.is_manual_override = false;
                } else {
                    self.state.last_tick_minutes = now;
                    return;
                }
            } else {
                self.state.last_tick_minutes = now;
                return;
            }
        }

        self.state.last_applied_mode = config.schedule_mode;

        let should_light = if config.schedule_mode == ScheduleMode::FollowNightLight {
            !self.state.is_night_light_active
        } else {
            should_be_light(
                now,
                self.state.effective_light_minutes,
                self.state.effective_dark_minutes,
            )
        };

        let apps_needs_change =
            config.change_apps && (self.state.is_apps_light_active != should_light);
        let system_needs_change =
            config.change_system && (self.state.is_system_light_active != should_light);

        if !self.state.is_manual_override && (apps_needs_change || system_needs_change) {
            if config.change_system && system_needs_change {
                provider.set_system_theme(should_light);
            }
            if config.change_apps && apps_needs_change {
                provider.set_apps_theme(should_light);
            }
            self.state.is_system_light_active = provider.get_current_system_theme();
            self.state.is_apps_light_active = provider.get_current_apps_theme();
        }

        self.state.last_tick_minutes = now;
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Mock theme provider for testing.
    struct MockProvider {
        system_light: Cell<bool>,
        apps_light: Cell<bool>,
        night_light: bool,
        now_minutes: i32,
        current_day: i32,
        timezone_bias: i32,
        date: (i32, i32, i32),
    }

    impl MockProvider {
        fn new(now: i32) -> Self {
            Self {
                system_light: Cell::new(true),
                apps_light: Cell::new(true),
                night_light: false,
                now_minutes: now,
                current_day: 1,
                timezone_bias: 300, // EST (Windows convention: positive = minutes UTC ahead of local)
                date: (2024, 6, 21),
            }
        }
    }

    impl ThemeProvider for MockProvider {
        fn get_current_system_theme(&self) -> bool {
            self.system_light.get()
        }
        fn get_current_apps_theme(&self) -> bool {
            self.apps_light.get()
        }
        fn is_night_light_enabled(&self) -> bool {
            self.night_light
        }
        fn set_system_theme(&self, is_light: bool) {
            self.system_light.set(is_light);
        }
        fn set_apps_theme(&self, is_light: bool) {
            self.apps_light.set(is_light);
        }
        fn get_now_minutes(&self) -> i32 {
            self.now_minutes
        }
        fn get_current_day(&self) -> i32 {
            self.current_day
        }
        fn get_timezone_bias_minutes(&self) -> i32 {
            self.timezone_bias
        }
        fn get_current_date(&self) -> (i32, i32, i32) {
            self.date
        }
    }

    fn fixed_hours_config(change_system: bool, change_apps: bool) -> LightSwitchConfig {
        LightSwitchConfig {
            schedule_mode: ScheduleMode::FixedHours,
            light_time: 480,  // 08:00
            dark_time: 1200,  // 20:00
            change_system,
            change_apps,
            ..LightSwitchConfig::default()
        }
    }

    #[test]
    fn initial_state_is_default() {
        let sm = StateManager::new();
        assert_eq!(sm.state().last_applied_mode, ScheduleMode::Off);
        assert!(!sm.state().is_manual_override);
    }

    #[test]
    fn off_mode_does_nothing() {
        let mut sm = StateManager::new();
        let config = LightSwitchConfig {
            schedule_mode: ScheduleMode::Off,
            ..LightSwitchConfig::default()
        };
        let provider = MockProvider::new(600);
        provider.system_light.set(false);

        sm.on_tick(&config, &provider);
        // Theme should not change.
        assert!(!provider.get_current_system_theme());
    }

    #[test]
    fn fixed_hours_applies_light_during_day() {
        let mut sm = StateManager::new();
        let config = fixed_hours_config(true, false);
        let provider = MockProvider::new(600); // 10:00 → should be light
        provider.system_light.set(false); // Currently dark

        sm.on_tick(&config, &provider);
        assert!(provider.get_current_system_theme()); // Changed to light
    }

    #[test]
    fn fixed_hours_applies_dark_at_night() {
        let mut sm = StateManager::new();
        let config = fixed_hours_config(true, false);
        let provider = MockProvider::new(1300); // 21:40 → should be dark
        provider.system_light.set(true); // Currently light

        sm.sync_initial_theme_state(&config, &provider);
        // sync_initial already applies, but let's verify:
        assert!(!provider.get_current_system_theme()); // Changed to dark
    }

    #[test]
    fn follow_night_light_mode() {
        let mut sm = StateManager::new();
        let config = LightSwitchConfig {
            schedule_mode: ScheduleMode::FollowNightLight,
            change_system: true,
            ..LightSwitchConfig::default()
        };
        let provider = MockProvider {
            night_light: true, // Night Light ON → dark mode
            ..MockProvider::new(600)
        };
        provider.system_light.set(true); // Currently light

        sm.sync_initial_theme_state(&config, &provider);
        assert!(!provider.get_current_system_theme()); // Changed to dark
    }

    #[test]
    fn manual_override_blocks_changes() {
        let mut sm = StateManager::new();
        let config = fixed_hours_config(true, false);

        // Set up: it's daytime (10:00), system is dark.
        let provider = MockProvider::new(600);
        provider.system_light.set(false);

        // First tick applies light theme.
        sm.on_tick(&config, &provider);
        assert!(provider.get_current_system_theme());

        // User triggers manual override (toggles to dark).
        provider.system_light.set(false);
        sm.on_manual_override(&config, &provider);
        assert!(sm.state().is_manual_override);

        // Next tick should NOT change theme back.
        sm.on_tick(&config, &provider);
        assert!(!provider.get_current_system_theme()); // Still dark
    }

    #[test]
    fn settings_changed_clears_override() {
        let mut sm = StateManager::new();
        let config = fixed_hours_config(true, false);
        let provider = MockProvider::new(600);
        provider.system_light.set(true);

        sm.on_tick(&config, &provider);
        sm.on_manual_override(&config, &provider);
        assert!(sm.state().is_manual_override);

        sm.on_settings_changed(&config, &provider);
        assert!(!sm.state().is_manual_override);
    }

    #[test]
    fn change_apps_only() {
        let mut sm = StateManager::new();
        let config = fixed_hours_config(false, true); // apps only
        let provider = MockProvider::new(600); // daytime
        provider.system_light.set(false);
        provider.apps_light.set(false);

        sm.on_tick(&config, &provider);
        // System should NOT change (change_system = false).
        assert!(!provider.system_light.get());
        // Apps should change to light.
        assert!(provider.apps_light.get());
    }

    #[test]
    fn no_change_when_already_correct() {
        let mut sm = StateManager::new();
        let config = fixed_hours_config(true, true);
        let provider = MockProvider::new(600); // daytime → light
        provider.system_light.set(true);
        provider.apps_light.set(true);

        sm.on_tick(&config, &provider);
        // Already light, should stay light (no unnecessary writes).
        assert!(provider.system_light.get());
        assert!(provider.apps_light.get());
    }

    #[test]
    fn night_light_change_clears_override_in_follow_mode() {
        let mut sm = StateManager::new();
        let config = LightSwitchConfig {
            schedule_mode: ScheduleMode::FollowNightLight,
            change_system: true,
            ..LightSwitchConfig::default()
        };
        let mut provider = MockProvider::new(600);
        provider.night_light = false;
        provider.system_light.set(true);

        sm.sync_initial_theme_state(&config, &provider);
        sm.on_manual_override(&config, &provider);
        assert!(sm.state().is_manual_override);

        // Night light toggles.
        provider.night_light = true;
        sm.on_night_light_change(&config, &provider);
        assert!(!sm.state().is_manual_override);
    }
}
