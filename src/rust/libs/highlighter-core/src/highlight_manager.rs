// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Manages the lifecycle of mouse-click highlights.
//!
//! Tracks active highlights from mouse-down through fade-out, and an optional
//! "always" highlight that follows the cursor when no button is held.

use crate::types::{Color, HighlightPoint, MouseButton, Settings};

/// A visible highlight with pre-computed opacity for rendering.
#[derive(Debug, Clone)]
pub struct VisibleHighlight {
    pub x: i32,
    pub y: i32,
    pub color: Color,
    pub opacity: f64,
    pub button: Option<MouseButton>,
}

/// Manages creation, tracking, and cleanup of mouse highlights.
pub struct HighlightManager {
    settings: Settings,
    /// Active click-based highlights (may be fading).
    highlights: Vec<HighlightPoint>,
    /// Whether left/right buttons are currently held.
    left_pressed: bool,
    right_pressed: bool,
    /// Current cursor position for the "always" highlight.
    cursor_x: i32,
    cursor_y: i32,
}

impl HighlightManager {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            highlights: Vec::new(),
            left_pressed: false,
            right_pressed: false,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    /// Update settings (e.g., when user changes them in the UI).
    pub fn update_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Returns true if the left button's highlight is enabled.
    fn left_enabled(&self) -> bool {
        !self.settings.left_button_color.is_disabled()
            && !(self.settings.spotlight_mode && !self.settings.always_color.is_disabled())
    }

    /// Returns true if the right button's highlight is enabled.
    fn right_enabled(&self) -> bool {
        !self.settings.right_button_color.is_disabled()
            && !(self.settings.spotlight_mode && !self.settings.always_color.is_disabled())
    }

    /// Returns true if the always-cursor highlight is enabled.
    fn always_enabled(&self) -> bool {
        !self.settings.always_color.is_disabled()
    }

    /// Call when a mouse button is pressed.
    pub fn on_mouse_down(&mut self, button: MouseButton, x: i32, y: i32, timestamp: u64) {
        let (enabled, color, already_pressed) = match button {
            MouseButton::Left => (
                self.left_enabled(),
                self.settings.left_button_color,
                self.left_pressed,
            ),
            MouseButton::Right => (
                self.right_enabled(),
                self.settings.right_button_color,
                self.right_pressed,
            ),
        };

        if !enabled {
            return;
        }

        // If already pressed (stray event), start fade on the old one
        if already_pressed {
            self.start_fade_for_button(button, timestamp);
        }

        match button {
            MouseButton::Left => self.left_pressed = true,
            MouseButton::Right => self.right_pressed = true,
        }

        self.highlights.push(HighlightPoint {
            x,
            y,
            button,
            created_at: timestamp,
            fade_started_at: None,
            color,
        });
    }

    /// Call when a mouse button is released.
    pub fn on_mouse_up(&mut self, button: MouseButton, timestamp: u64) {
        match button {
            MouseButton::Left => {
                if self.left_pressed {
                    self.left_pressed = false;
                    self.start_fade_for_button(button, timestamp);
                }
            }
            MouseButton::Right => {
                if self.right_pressed {
                    self.right_pressed = false;
                    self.start_fade_for_button(button, timestamp);
                }
            }
        }
    }

    /// Start fade-out on the most recent un-faded highlight for `button`.
    fn start_fade_for_button(&mut self, button: MouseButton, timestamp: u64) {
        // Find the last highlight for this button that hasn't started fading
        if let Some(hp) = self
            .highlights
            .iter_mut()
            .rev()
            .find(|h| h.button == button && h.fade_started_at.is_none())
        {
            hp.fade_started_at = Some(timestamp);
        }
    }

    /// Call on every mouse move to update positions of held highlights.
    pub fn on_mouse_move(&mut self, x: i32, y: i32) {
        self.cursor_x = x;
        self.cursor_y = y;

        // Update position of active (held) highlights
        if self.left_pressed {
            if let Some(hp) = self
                .highlights
                .iter_mut()
                .rev()
                .find(|h| h.button == MouseButton::Left && h.fade_started_at.is_none())
            {
                hp.x = x;
                hp.y = y;
            }
        }
        if self.right_pressed {
            if let Some(hp) = self
                .highlights
                .iter_mut()
                .rev()
                .find(|h| h.button == MouseButton::Right && h.fade_started_at.is_none())
            {
                hp.x = x;
                hp.y = y;
            }
        }
    }

    /// Remove fully faded highlights.
    pub fn cleanup(&mut self, now_ms: u64) {
        let fd = self.settings.fade_delay_ms as u64;
        let dur = self.settings.fade_duration_ms as u64;
        self.highlights.retain(|h| !h.is_expired(now_ms, fd, dur));
    }

    /// Return all currently visible highlights with computed opacity.
    /// Includes the "always" cursor highlight when enabled and no buttons are held.
    pub fn get_visible_highlights(&self, now_ms: u64) -> Vec<VisibleHighlight> {
        let fd = self.settings.fade_delay_ms as u64;
        let dur = self.settings.fade_duration_ms as u64;
        let mut result = Vec::new();

        // "Always" highlight when enabled and no button is held
        if self.always_enabled() && !self.left_pressed && !self.right_pressed {
            result.push(VisibleHighlight {
                x: self.cursor_x,
                y: self.cursor_y,
                color: self.settings.always_color,
                opacity: 1.0,
                button: None,
            });
        }

        for h in &self.highlights {
            let opacity = h.opacity(now_ms, fd, dur);
            if opacity > 0.0 {
                result.push(VisibleHighlight {
                    x: h.x,
                    y: h.y,
                    color: h.color,
                    opacity,
                    button: Some(h.button),
                });
            }
        }

        result
    }

    /// Returns true if any highlight is visible or the always-highlight is active.
    pub fn has_visible_content(&self, now_ms: u64) -> bool {
        if self.always_enabled() && !self.left_pressed && !self.right_pressed {
            return true;
        }
        let fd = self.settings.fade_delay_ms as u64;
        let dur = self.settings.fade_duration_ms as u64;
        self.highlights
            .iter()
            .any(|h| h.opacity(now_ms, fd, dur) > 0.0)
    }

    /// Number of tracked highlights (for testing).
    pub fn highlight_count(&self) -> usize {
        self.highlights.len()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Color, MouseButton, Settings};

    fn default_manager() -> HighlightManager {
        HighlightManager::new(Settings::default())
    }

    fn settings_with_always() -> Settings {
        let mut s = Settings::default();
        s.always_color = Color::new(128, 255, 0, 0); // enabled
        s
    }

    // ── Basic click tests ──

    #[test]
    fn left_click_adds_yellow_highlight() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Left, 100, 200, 1000);

        let vis = mgr.get_visible_highlights(1000);
        assert_eq!(vis.len(), 1);
        assert_eq!(vis[0].x, 100);
        assert_eq!(vis[0].y, 200);
        assert_eq!(vis[0].color, Color::new(166, 255, 255, 0));
        assert_eq!(vis[0].opacity, 1.0);
        assert_eq!(vis[0].button, Some(MouseButton::Left));
    }

    #[test]
    fn right_click_adds_blue_highlight() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Right, 300, 400, 1000);

        let vis = mgr.get_visible_highlights(1000);
        assert_eq!(vis.len(), 1);
        assert_eq!(vis[0].color, Color::new(166, 0, 0, 255));
        assert_eq!(vis[0].button, Some(MouseButton::Right));
    }

    // ── Fade lifecycle tests ──

    #[test]
    fn highlight_stays_visible_for_fade_delay() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Left, 0, 0, 1000);
        mgr.on_mouse_up(MouseButton::Left, 1000);

        // At t=1400 (400ms after release), within 500ms delay → still full opacity
        let vis = mgr.get_visible_highlights(1400);
        assert_eq!(vis.len(), 1);
        assert_eq!(vis[0].opacity, 1.0);
    }

    #[test]
    fn highlight_fades_to_zero_over_duration() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Left, 0, 0, 1000);
        mgr.on_mouse_up(MouseButton::Left, 1000);

        // Midway through fade: t=1625 → 625ms elapsed, 125ms into 250ms fade → 0.5
        let vis = mgr.get_visible_highlights(1625);
        assert_eq!(vis.len(), 1);
        assert!((vis[0].opacity - 0.5).abs() < 1e-10);

        // Fully faded: t=1750 → 750ms elapsed, 250ms into 250ms fade → 0.0
        let vis = mgr.get_visible_highlights(1750);
        assert!(vis.is_empty());
    }

    // ── Alpha=0 disables button ──

    #[test]
    fn alpha_zero_disables_left_button() {
        let mut settings = Settings::default();
        settings.left_button_color = Color::new(0, 255, 255, 0);
        let mut mgr = HighlightManager::new(settings);

        mgr.on_mouse_down(MouseButton::Left, 100, 200, 1000);
        let vis = mgr.get_visible_highlights(1000);
        assert!(vis.is_empty());
    }

    #[test]
    fn alpha_zero_disables_right_button() {
        let mut settings = Settings::default();
        settings.right_button_color = Color::new(0, 0, 0, 255);
        let mut mgr = HighlightManager::new(settings);

        mgr.on_mouse_down(MouseButton::Right, 100, 200, 1000);
        let vis = mgr.get_visible_highlights(1000);
        assert!(vis.is_empty());
    }

    // ── Always-cursor highlight ──

    #[test]
    fn always_highlight_follows_cursor_when_no_click() {
        let mut mgr = HighlightManager::new(settings_with_always());
        mgr.on_mouse_move(500, 600);

        let vis = mgr.get_visible_highlights(1000);
        assert_eq!(vis.len(), 1);
        assert_eq!(vis[0].x, 500);
        assert_eq!(vis[0].y, 600);
        assert_eq!(vis[0].button, None);
        assert_eq!(vis[0].opacity, 1.0);
    }

    #[test]
    fn always_highlight_hidden_while_button_held() {
        let mut mgr = HighlightManager::new(settings_with_always());
        mgr.on_mouse_move(500, 600);
        mgr.on_mouse_down(MouseButton::Left, 500, 600, 1000);

        let vis = mgr.get_visible_highlights(1000);
        // Should only have the click highlight, not the always one
        assert_eq!(vis.len(), 1);
        assert_eq!(vis[0].button, Some(MouseButton::Left));
    }

    #[test]
    fn always_highlight_returns_after_button_release() {
        let mut mgr = HighlightManager::new(settings_with_always());
        mgr.on_mouse_down(MouseButton::Left, 100, 200, 1000);
        mgr.on_mouse_up(MouseButton::Left, 1100);
        mgr.on_mouse_move(300, 400);

        let vis = mgr.get_visible_highlights(1100);
        // Should have: always highlight + fading click highlight
        assert!(vis.len() >= 2);
        // First should be the always highlight
        assert_eq!(vis[0].button, None);
        assert_eq!(vis[0].x, 300);
        assert_eq!(vis[0].y, 400);
    }

    #[test]
    fn always_disabled_by_default() {
        let mgr = default_manager();
        let vis = mgr.get_visible_highlights(0);
        assert!(vis.is_empty());
    }

    // ── Multiple clicks ──

    #[test]
    fn multiple_clicks_create_multiple_highlights() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Left, 10, 20, 1000);
        mgr.on_mouse_up(MouseButton::Left, 1050);
        mgr.on_mouse_down(MouseButton::Left, 30, 40, 1100);
        mgr.on_mouse_up(MouseButton::Left, 1150);
        mgr.on_mouse_down(MouseButton::Right, 50, 60, 1200);
        mgr.on_mouse_up(MouseButton::Right, 1250);

        // All three should be visible shortly after creation
        let vis = mgr.get_visible_highlights(1300);
        assert_eq!(vis.len(), 3);
    }

    #[test]
    fn simultaneous_left_and_right() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Left, 10, 20, 1000);
        mgr.on_mouse_down(MouseButton::Right, 30, 40, 1050);

        let vis = mgr.get_visible_highlights(1050);
        assert_eq!(vis.len(), 2);
    }

    // ── Cleanup ──

    #[test]
    fn cleanup_removes_expired_highlights() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Left, 0, 0, 1000);
        mgr.on_mouse_up(MouseButton::Left, 1000);

        assert_eq!(mgr.highlight_count(), 1);

        // After full fade (500ms delay + 250ms duration = 750ms)
        mgr.cleanup(1751);
        assert_eq!(mgr.highlight_count(), 0);
    }

    #[test]
    fn cleanup_keeps_active_highlights() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Left, 0, 0, 1000);

        mgr.cleanup(99999);
        assert_eq!(mgr.highlight_count(), 1); // still held, never expires
    }

    // ── Position tracking ──

    #[test]
    fn held_highlight_follows_cursor() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Left, 100, 200, 1000);
        mgr.on_mouse_move(300, 400);

        let vis = mgr.get_visible_highlights(1050);
        assert_eq!(vis[0].x, 300);
        assert_eq!(vis[0].y, 400);
    }

    #[test]
    fn released_highlight_does_not_follow_cursor() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Left, 100, 200, 1000);
        mgr.on_mouse_up(MouseButton::Left, 1050);
        mgr.on_mouse_move(999, 888);

        let vis = mgr.get_visible_highlights(1100);
        assert_eq!(vis.len(), 1);
        // Position should be wherever it was when released (last move while held was to 100,200)
        assert_eq!(vis[0].x, 100);
        assert_eq!(vis[0].y, 200);
    }

    // ── Spotlight mode ──

    #[test]
    fn spotlight_mode_disables_click_highlights() {
        let mut settings = Settings::default();
        settings.spotlight_mode = true;
        settings.always_color = Color::new(128, 255, 0, 0); // enable always
        let mut mgr = HighlightManager::new(settings);

        mgr.on_mouse_down(MouseButton::Left, 100, 200, 1000);

        let vis = mgr.get_visible_highlights(1000);
        // Click highlights are disabled in spotlight mode, but the always/spotlight
        // highlight remains visible because the click was ignored (left_pressed stays false).
        assert_eq!(vis.len(), 1);
        assert_eq!(vis[0].button, None); // always highlight, not a click
    }

    #[test]
    fn spotlight_mode_without_always_color_allows_clicks() {
        let mut settings = Settings::default();
        settings.spotlight_mode = true;
        // always_color alpha=0 → spotlight not actually active
        let mut mgr = HighlightManager::new(settings);

        mgr.on_mouse_down(MouseButton::Left, 100, 200, 1000);
        let vis = mgr.get_visible_highlights(1000);
        assert_eq!(vis.len(), 1);
    }

    // ── has_visible_content ──

    #[test]
    fn has_visible_content_with_always() {
        let mgr = HighlightManager::new(settings_with_always());
        assert!(mgr.has_visible_content(0));
    }

    #[test]
    fn has_visible_content_with_click() {
        let mut mgr = default_manager();
        assert!(!mgr.has_visible_content(0));
        mgr.on_mouse_down(MouseButton::Left, 0, 0, 1000);
        assert!(mgr.has_visible_content(1000));
    }

    #[test]
    fn no_visible_content_after_full_fade() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Left, 0, 0, 1000);
        mgr.on_mouse_up(MouseButton::Left, 1000);
        // After 500ms delay + 250ms fade
        assert!(!mgr.has_visible_content(1751));
    }

    // ── Stray press handling ──

    #[test]
    fn double_press_starts_fade_on_old_highlight() {
        let mut mgr = default_manager();
        mgr.on_mouse_down(MouseButton::Left, 10, 20, 1000);
        // Simulate a stray second press (didn't get the up event)
        mgr.on_mouse_down(MouseButton::Left, 30, 40, 1500);

        assert_eq!(mgr.highlight_count(), 2);
        // The first highlight should now be fading
        let vis = mgr.get_visible_highlights(1500);
        // Both visible at the moment of second press
        assert_eq!(vis.len(), 2);

        // After first one fully fades (started at 1500, delay 500 + dur 250 = at 2250)
        let vis = mgr.get_visible_highlights(2251);
        // Only the second (still held) should remain
        assert_eq!(vis.len(), 1);
        assert_eq!(vis[0].x, 30);
    }

    // ── Settings update ──

    #[test]
    fn update_settings_changes_behavior() {
        let mut mgr = default_manager();
        assert!(!mgr.always_enabled());

        let new_settings = settings_with_always();
        mgr.update_settings(new_settings);
        assert!(mgr.has_visible_content(0));
    }
}
