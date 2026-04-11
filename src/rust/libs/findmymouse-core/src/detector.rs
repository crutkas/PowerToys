// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Double-ctrl-click detection state machine.
//!
//! Mirrors the C++ `OnSonarKeyboardInput` logic. All timing and cursor
//! position are injected — no Win32 calls — making the detector fully testable.

use crate::types::{ActivationMethod, SonarState};

/// Minimum time between first press and second press (ms).
/// Protects against keyboards sending rapid duplicate events.
pub const MIN_DOUBLE_CLICK_TIME: u64 = 100;

/// Which physical ctrl key was pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtrlSide {
    Left,
    Right,
}

/// A keyboard event fed into the detector.
#[derive(Debug, Clone)]
pub struct KeyEvent {
    /// True = key pressed, false = key released.
    pub pressed: bool,
    /// Is this a VK_CONTROL event?
    pub is_ctrl: bool,
    /// Which ctrl side (only meaningful when `is_ctrl` is true).
    pub side: CtrlSide,
    /// Timestamp in milliseconds (e.g. GetTickCount64).
    pub timestamp_ms: u64,
    /// Cursor position at the time of the event.
    pub cursor_x: i32,
    pub cursor_y: i32,
}

/// Result from feeding a key event into the detector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectorResult {
    /// No state change of interest.
    Nothing,
    /// Sonar should be activated.
    Activate,
    /// Sonar should be deactivated.
    Deactivate,
}

/// Double-ctrl-click detector state machine.
///
/// Feed it `KeyEvent`s and it will tell you when to activate/deactivate.
pub struct CtrlDetector {
    state: SonarState,
    activation_method: ActivationMethod,
    include_win_key: bool,
    /// Maximum time between first press and second press (ms).
    max_double_click_time: u64,
    /// Recorded timestamp of the first press.
    last_key_time: u64,
    /// Recorded cursor position of the first press.
    last_key_x: i32,
    last_key_y: i32,
    /// Whether the sonar is currently active.
    sonar_active: bool,
    /// External query: is the Windows key currently held?
    /// This is set by the caller before feeding events.
    win_key_held: bool,
}

impl CtrlDetector {
    pub fn new(activation_method: ActivationMethod, include_win_key: bool) -> Self {
        Self {
            state: SonarState::Idle,
            activation_method,
            include_win_key,
            max_double_click_time: 500,
            last_key_time: 0,
            last_key_x: 0,
            last_key_y: 0,
            sonar_active: false,
            win_key_held: false,
        }
    }

    /// Set the maximum double-click interval (normally from GetDoubleClickTime).
    pub fn set_max_double_click_time(&mut self, ms: u64) {
        self.max_double_click_time = ms;
    }

    /// Inform the detector whether the Windows key is currently held.
    pub fn set_win_key_held(&mut self, held: bool) {
        self.win_key_held = held;
    }

    /// Whether sonar is currently active.
    pub fn is_sonar_active(&self) -> bool {
        self.sonar_active
    }

    /// Current state of the state machine.
    pub fn state(&self) -> SonarState {
        self.state
    }

    /// Feed a key event and get the result.
    pub fn feed(&mut self, event: &KeyEvent) -> DetectorResult {
        // Shortcut mode: ignore key releases (don't stop sonar on release).
        if self.activation_method == ActivationMethod::Shortcut && !event.pressed {
            return DetectorResult::Nothing;
        }

        // If not in double-ctrl mode, or the key isn't VK_CONTROL, stop sonar.
        if (self.activation_method != ActivationMethod::DoubleLeftCtrl
            && self.activation_method != ActivationMethod::DoubleRightCtrl)
            || !event.is_ctrl
        {
            return self.stop_sonar();
        }

        // Check that the correct ctrl side is pressed.
        let expected_side = match self.activation_method {
            ActivationMethod::DoubleLeftCtrl => CtrlSide::Left,
            ActivationMethod::DoubleRightCtrl => CtrlSide::Right,
            _ => return self.stop_sonar(),
        };
        if event.side != expected_side {
            return self.stop_sonar();
        }

        match self.state {
            SonarState::Idle => {
                if event.pressed {
                    self.state = SonarState::ControlDown1;
                    self.last_key_time = event.timestamp_ms;
                    self.last_key_x = event.cursor_x;
                    self.last_key_y = event.cursor_y;
                }
                DetectorResult::Nothing
            }
            SonarState::ControlDown1 => {
                if !event.pressed {
                    self.state = SonarState::ControlUp1;
                }
                DetectorResult::Nothing
            }
            SonarState::ControlUp1 => {
                if event.pressed && self.keyboard_input_can_activate() {
                    let interval = event.timestamp_ms.saturating_sub(self.last_key_time);
                    let min_time =
                        MIN_DOUBLE_CLICK_TIME.min(self.max_double_click_time / 5);
                    let cursor_same = event.cursor_x == self.last_key_x
                        && event.cursor_y == self.last_key_y;

                    if interval >= min_time
                        && interval <= self.max_double_click_time
                        && cursor_same
                    {
                        self.state = SonarState::ControlDown2;
                        self.last_key_time = event.timestamp_ms;
                        self.last_key_x = event.cursor_x;
                        self.last_key_y = event.cursor_y;
                        self.sonar_active = true;
                        return DetectorResult::Activate;
                    }
                    // Timing or position mismatch — restart sequence.
                    self.state = SonarState::ControlDown1;
                    self.last_key_time = event.timestamp_ms;
                    self.last_key_x = event.cursor_x;
                    self.last_key_y = event.cursor_y;
                }
                DetectorResult::Nothing
            }
            SonarState::ControlDown2 => {
                if !event.pressed {
                    self.state = SonarState::ControlUp2;
                }
                DetectorResult::Nothing
            }
            SonarState::ControlUp2 => {
                if event.pressed {
                    return self.stop_sonar();
                }
                DetectorResult::Nothing
            }
        }
    }

    fn stop_sonar(&mut self) -> DetectorResult {
        let was_active = self.sonar_active;
        self.sonar_active = false;
        self.state = SonarState::Idle;
        if was_active {
            DetectorResult::Deactivate
        } else {
            DetectorResult::Nothing
        }
    }

    fn keyboard_input_can_activate(&self) -> bool {
        !self.include_win_key || self.win_key_held
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctrl_event(
        pressed: bool,
        side: CtrlSide,
        timestamp_ms: u64,
        cursor_x: i32,
        cursor_y: i32,
    ) -> KeyEvent {
        KeyEvent {
            pressed,
            is_ctrl: true,
            side,
            timestamp_ms,
            cursor_x,
            cursor_y,
        }
    }

    fn make_non_ctrl_event(pressed: bool, timestamp_ms: u64) -> KeyEvent {
        KeyEvent {
            pressed,
            is_ctrl: false,
            side: CtrlSide::Left,
            timestamp_ms,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    // ── Double-left-ctrl triggers activation ──

    #[test]
    fn double_left_ctrl_triggers_activation() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleLeftCtrl, false);
        det.set_max_double_click_time(500);

        // First press
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 100, 200));
        assert_eq!(r, DetectorResult::Nothing);
        assert_eq!(det.state(), SonarState::ControlDown1);

        // First release
        let r = det.feed(&make_ctrl_event(false, CtrlSide::Left, 1050, 100, 200));
        assert_eq!(r, DetectorResult::Nothing);
        assert_eq!(det.state(), SonarState::ControlUp1);

        // Second press within time window, cursor unchanged
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 1200, 100, 200));
        assert_eq!(r, DetectorResult::Activate);
        assert_eq!(det.state(), SonarState::ControlDown2);
        assert!(det.is_sonar_active());
    }

    // ── Double-right-ctrl triggers activation (when configured) ──

    #[test]
    fn double_right_ctrl_triggers_activation() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleRightCtrl, false);
        det.set_max_double_click_time(500);

        let r = det.feed(&make_ctrl_event(true, CtrlSide::Right, 1000, 0, 0));
        assert_eq!(r, DetectorResult::Nothing);

        let r = det.feed(&make_ctrl_event(false, CtrlSide::Right, 1050, 0, 0));
        assert_eq!(r, DetectorResult::Nothing);

        let r = det.feed(&make_ctrl_event(true, CtrlSide::Right, 1200, 0, 0));
        assert_eq!(r, DetectorResult::Activate);
        assert!(det.is_sonar_active());
    }

    // ── Too-slow double-click doesn't trigger ──

    #[test]
    fn too_slow_double_click_does_not_trigger() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleLeftCtrl, false);
        det.set_max_double_click_time(500);

        det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 0, 0));
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 1050, 0, 0));

        // Second press 600ms later — exceeds 500ms max
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 1600, 0, 0));
        assert_eq!(r, DetectorResult::Nothing);
        assert!(!det.is_sonar_active());
        // State restarted to ControlDown1
        assert_eq!(det.state(), SonarState::ControlDown1);
    }

    // ── Too-fast double-click doesn't trigger ──

    #[test]
    fn too_fast_double_click_does_not_trigger() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleLeftCtrl, false);
        det.set_max_double_click_time(500);

        det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 0, 0));
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 1010, 0, 0));

        // Second press only 50ms later — under MIN_DOUBLE_CLICK_TIME (100ms)
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 1050, 0, 0));
        assert_eq!(r, DetectorResult::Nothing);
        assert!(!det.is_sonar_active());
        assert_eq!(det.state(), SonarState::ControlDown1);
    }

    // ── Cursor movement between clicks doesn't trigger ──

    #[test]
    fn cursor_movement_between_clicks_does_not_trigger() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleLeftCtrl, false);
        det.set_max_double_click_time(500);

        det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 100, 200));
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 1050, 100, 200));

        // Second press — cursor moved!
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 1200, 150, 200));
        assert_eq!(r, DetectorResult::Nothing);
        assert!(!det.is_sonar_active());
        assert_eq!(det.state(), SonarState::ControlDown1);
    }

    // ── Wrong ctrl side doesn't trigger ──

    #[test]
    fn wrong_ctrl_side_does_not_trigger() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleLeftCtrl, false);
        det.set_max_double_click_time(500);

        det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 0, 0));
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 1050, 0, 0));

        // Second press is right ctrl — wrong side
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Right, 1200, 0, 0));
        assert_eq!(r, DetectorResult::Nothing);
        assert!(!det.is_sonar_active());
        assert_eq!(det.state(), SonarState::Idle);
    }

    #[test]
    fn right_ctrl_configured_but_left_pressed_does_not_trigger() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleRightCtrl, false);
        det.set_max_double_click_time(500);

        // Pressing left ctrl when right is configured
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 0, 0));
        assert_eq!(r, DetectorResult::Nothing);
        assert_eq!(det.state(), SonarState::Idle);
    }

    // ── Win key required but not held → doesn't trigger ──

    #[test]
    fn win_key_required_but_not_held_does_not_trigger() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleLeftCtrl, true);
        det.set_max_double_click_time(500);
        det.set_win_key_held(false);

        det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 0, 0));
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 1050, 0, 0));

        // Second press — win key not held
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 1200, 0, 0));
        assert_eq!(r, DetectorResult::Nothing);
        assert!(!det.is_sonar_active());
    }

    // ── Win key required and held → triggers ──

    #[test]
    fn win_key_required_and_held_triggers() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleLeftCtrl, true);
        det.set_max_double_click_time(500);
        det.set_win_key_held(true);

        det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 0, 0));
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 1050, 0, 0));

        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 1200, 0, 0));
        assert_eq!(r, DetectorResult::Activate);
        assert!(det.is_sonar_active());
    }

    // ── Third press deactivates ──

    #[test]
    fn third_ctrl_press_deactivates() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleLeftCtrl, false);
        det.set_max_double_click_time(500);

        // Activate
        det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 0, 0));
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 1050, 0, 0));
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 1200, 0, 0));
        assert_eq!(r, DetectorResult::Activate);

        // Release second press
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 1250, 0, 0));
        assert_eq!(det.state(), SonarState::ControlUp2);

        // Third press — deactivates
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 1400, 0, 0));
        assert_eq!(r, DetectorResult::Deactivate);
        assert!(!det.is_sonar_active());
        assert_eq!(det.state(), SonarState::Idle);
    }

    // ── Non-ctrl key stops sonar ──

    #[test]
    fn non_ctrl_key_stops_sonar() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleLeftCtrl, false);
        det.set_max_double_click_time(500);

        // Activate
        det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 0, 0));
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 1050, 0, 0));
        det.feed(&make_ctrl_event(true, CtrlSide::Left, 1200, 0, 0));

        // Non-ctrl key pressed
        let r = det.feed(&make_non_ctrl_event(true, 1300));
        assert_eq!(r, DetectorResult::Deactivate);
        assert!(!det.is_sonar_active());
    }

    // ── Shortcut mode ignores key releases ──

    #[test]
    fn shortcut_mode_ignores_key_releases() {
        let mut det = CtrlDetector::new(ActivationMethod::Shortcut, false);
        // Any key release should be ignored
        let r = det.feed(&make_non_ctrl_event(false, 1000));
        assert_eq!(r, DetectorResult::Nothing);
    }

    // ── Low max_double_click_time also lowers min check ──

    #[test]
    fn low_double_click_time_lowers_min_threshold() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleLeftCtrl, false);
        // Set a very low double-click time (e.g., 50ms)
        det.set_max_double_click_time(50);

        det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 0, 0));
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 1005, 0, 0));

        // 15ms interval. min(100, 50/5=10) = 10. 15 >= 10 and 15 <= 50 → should activate
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 1015, 0, 0));
        assert_eq!(r, DetectorResult::Activate);
    }

    // ── State is reset on failed second press ──

    #[test]
    fn failed_second_press_restarts_sequence() {
        let mut det = CtrlDetector::new(ActivationMethod::DoubleLeftCtrl, false);
        det.set_max_double_click_time(500);

        // First sequence: press, release
        det.feed(&make_ctrl_event(true, CtrlSide::Left, 1000, 0, 0));
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 1050, 0, 0));

        // Second press too late — restarts as new ControlDown1
        det.feed(&make_ctrl_event(true, CtrlSide::Left, 2000, 0, 0));
        assert_eq!(det.state(), SonarState::ControlDown1);

        // Now complete a valid double-click from the restarted state
        det.feed(&make_ctrl_event(false, CtrlSide::Left, 2050, 0, 0));
        let r = det.feed(&make_ctrl_event(true, CtrlSide::Left, 2200, 0, 0));
        assert_eq!(r, DetectorResult::Activate);
    }
}
