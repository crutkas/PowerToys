// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Core types for mouse highlighting — platform-independent, no Win32 dependencies.

use serde::{Deserialize, Serialize};

/// Which mouse button triggered the highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
}

/// ARGB color with alpha, red, green, blue components.
/// Alpha = 0 means the highlight for that button is DISABLED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { a, r, g, b }
    }

    /// Returns true if this color is effectively disabled (fully transparent).
    pub fn is_disabled(&self) -> bool {
        self.a == 0
    }
}

/// All configurable settings for the mouse highlighter.
/// Default values match the C++ implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Color for left-button click highlights (ARGB).
    pub left_button_color: Color,
    /// Color for right-button click highlights (ARGB).
    pub right_button_color: Color,
    /// Color for the always-on cursor highlight (ARGB). Alpha=0 disables.
    pub always_color: Color,
    /// Highlight circle radius in pixels.
    pub radius: i32,
    /// Milliseconds to wait at full opacity before starting fade.
    pub fade_delay_ms: i32,
    /// Milliseconds for the fade-to-transparent animation.
    pub fade_duration_ms: i32,
    /// Whether to auto-activate when the module is enabled.
    pub auto_activate: bool,
    /// When true, shows a radial spotlight instead of always-circle.
    pub spotlight_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            left_button_color: Color::new(166, 255, 255, 0),   // semi-transparent yellow
            right_button_color: Color::new(166, 0, 0, 255),    // semi-transparent blue
            always_color: Color::new(0, 255, 0, 0),            // disabled (alpha=0)
            radius: 20,
            fade_delay_ms: 500,
            fade_duration_ms: 250,
            auto_activate: false,
            spotlight_mode: false,
        }
    }
}

/// A single active highlight circle on screen.
#[derive(Debug, Clone)]
pub struct HighlightPoint {
    /// Screen X coordinate.
    pub x: i32,
    /// Screen Y coordinate.
    pub y: i32,
    /// Which mouse button created this highlight.
    pub button: MouseButton,
    /// Timestamp (ms) when the highlight was created (mouse-down).
    pub created_at: u64,
    /// Timestamp (ms) when fade-out was triggered (mouse-up), or `None` if still held.
    pub fade_started_at: Option<u64>,
    /// The base color for this highlight (from settings at creation time).
    pub color: Color,
}

impl HighlightPoint {
    /// Compute the current opacity (0.0–1.0) of this highlight at `now_ms`.
    ///
    /// - While the button is held (`fade_started_at` is None), opacity is 1.0.
    /// - For `fade_delay_ms` after release, opacity stays at 1.0.
    /// - Over the next `fade_duration_ms`, opacity linearly drops to 0.0.
    pub fn opacity(&self, now_ms: u64, fade_delay_ms: u64, fade_duration_ms: u64) -> f64 {
        let start = match self.fade_started_at {
            Some(t) => t,
            None => return 1.0, // still held
        };

        if now_ms < start {
            return 1.0;
        }

        let elapsed = now_ms - start;

        if elapsed < fade_delay_ms {
            return 1.0;
        }

        let fade_elapsed = elapsed - fade_delay_ms;

        if fade_duration_ms == 0 {
            return 0.0;
        }

        let progress = fade_elapsed as f64 / fade_duration_ms as f64;
        (1.0 - progress).clamp(0.0, 1.0)
    }

    /// Returns true if this highlight has fully faded out and can be removed.
    pub fn is_expired(&self, now_ms: u64, fade_delay_ms: u64, fade_duration_ms: u64) -> bool {
        self.opacity(now_ms, fade_delay_ms, fade_duration_ms) <= 0.0
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_disabled_when_alpha_zero() {
        assert!(Color::new(0, 255, 0, 0).is_disabled());
    }

    #[test]
    fn color_enabled_when_alpha_nonzero() {
        assert!(!Color::new(166, 255, 255, 0).is_disabled());
        assert!(!Color::new(1, 0, 0, 0).is_disabled());
    }

    #[test]
    fn default_settings_match_cpp() {
        let s = Settings::default();
        assert_eq!(s.left_button_color, Color::new(166, 255, 255, 0));
        assert_eq!(s.right_button_color, Color::new(166, 0, 0, 255));
        assert_eq!(s.always_color, Color::new(0, 255, 0, 0));
        assert_eq!(s.radius, 20);
        assert_eq!(s.fade_delay_ms, 500);
        assert_eq!(s.fade_duration_ms, 250);
        assert!(!s.auto_activate);
        assert!(!s.spotlight_mode);
    }

    #[test]
    fn default_always_color_is_disabled() {
        let s = Settings::default();
        assert!(s.always_color.is_disabled());
    }

    #[test]
    fn opacity_while_held_is_full() {
        let pt = HighlightPoint {
            x: 100,
            y: 200,
            button: MouseButton::Left,
            created_at: 1000,
            fade_started_at: None,
            color: Color::new(166, 255, 255, 0),
        };
        assert_eq!(pt.opacity(5000, 500, 250), 1.0);
    }

    #[test]
    fn opacity_during_fade_delay_is_full() {
        let pt = HighlightPoint {
            x: 100,
            y: 200,
            button: MouseButton::Left,
            created_at: 1000,
            fade_started_at: Some(2000),
            color: Color::new(166, 255, 255, 0),
        };
        // At t=2400, only 400ms elapsed since fade start; delay is 500ms → still full
        assert_eq!(pt.opacity(2400, 500, 250), 1.0);
    }

    #[test]
    fn opacity_during_fade_animation() {
        let pt = HighlightPoint {
            x: 100,
            y: 200,
            button: MouseButton::Left,
            created_at: 1000,
            fade_started_at: Some(2000),
            color: Color::new(166, 255, 255, 0),
        };
        // At t=2625, elapsed=625ms, past delay(500ms), fade_elapsed=125ms
        // progress = 125/250 = 0.5 → opacity = 0.5
        let o = pt.opacity(2625, 500, 250);
        assert!((o - 0.5).abs() < 1e-10);
    }

    #[test]
    fn opacity_after_fully_faded() {
        let pt = HighlightPoint {
            x: 100,
            y: 200,
            button: MouseButton::Left,
            created_at: 1000,
            fade_started_at: Some(2000),
            color: Color::new(166, 255, 255, 0),
        };
        // At t=2750, elapsed=750ms, past delay(500ms), fade_elapsed=250ms
        // progress = 250/250 = 1.0 → opacity = 0.0
        assert_eq!(pt.opacity(2750, 500, 250), 0.0);
    }

    #[test]
    fn is_expired_after_full_fade() {
        let pt = HighlightPoint {
            x: 0,
            y: 0,
            button: MouseButton::Right,
            created_at: 0,
            fade_started_at: Some(100),
            color: Color::new(166, 0, 0, 255),
        };
        // At t=850: elapsed=750, fade_elapsed=250, progress=1.0 → expired
        assert!(pt.is_expired(850, 500, 250));
    }

    #[test]
    fn is_not_expired_while_held() {
        let pt = HighlightPoint {
            x: 0,
            y: 0,
            button: MouseButton::Left,
            created_at: 0,
            fade_started_at: None,
            color: Color::new(166, 255, 255, 0),
        };
        assert!(!pt.is_expired(99999, 500, 250));
    }

    #[test]
    fn opacity_zero_fade_duration_instant() {
        let pt = HighlightPoint {
            x: 0,
            y: 0,
            button: MouseButton::Left,
            created_at: 0,
            fade_started_at: Some(100),
            color: Color::new(166, 255, 255, 0),
        };
        // fade_delay=0, fade_duration=0 → instantly expired once fade starts
        assert_eq!(pt.opacity(100, 0, 0), 0.0);
    }

    #[test]
    fn mouse_button_equality() {
        assert_eq!(MouseButton::Left, MouseButton::Left);
        assert_ne!(MouseButton::Left, MouseButton::Right);
    }
}
