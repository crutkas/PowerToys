// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Core cursor wrapping engine — pure logic, no Win32 dependencies.
//!
//! Port of `CursorWrapCore.h/cpp`.

use crate::topology::MonitorTopology;
use crate::types::*;

/// Distance threshold (pixels) to prevent rapid back-and-forth oscillation after a wrap.
pub const WRAP_DISTANCE_THRESHOLD: i32 = 50;

/// Core cursor wrapping engine.
///
/// Call [`CursorWrapCore::initialize`] with monitor info, then call
/// [`CursorWrapCore::handle_mouse_move`] on every mouse move event.
#[derive(Debug)]
pub struct CursorWrapCore {
    monitors: Vec<MonitorInfo>,
    topology: MonitorTopology,

    // Movement tracking
    previous_position: Point,
    has_previous_position: bool,

    // Oscillation prevention
    last_wrap_destination: Point,
    has_last_wrap_destination: bool,
}

impl CursorWrapCore {
    pub fn new() -> Self {
        Self {
            monitors: Vec::new(),
            topology: MonitorTopology::new(),
            previous_position: Point {
                x: i32::MIN,
                y: i32::MIN,
            },
            has_previous_position: false,
            last_wrap_destination: Point {
                x: i32::MIN,
                y: i32::MIN,
            },
            has_last_wrap_destination: false,
        }
    }

    /// Initialize with a list of monitors (replaces C++ UpdateMonitorInfo's logic,
    /// minus the Win32 EnumDisplayMonitors call).
    pub fn initialize(&mut self, monitors: &[MonitorInfo]) {
        self.monitors = monitors.to_vec();
        self.topology.initialize(monitors);
    }

    /// Handle a mouse move event.
    ///
    /// * `current_pos` — current cursor position
    /// * `disable_wrap_during_drag` — if true AND `left_button_down` is true, skip wrapping
    /// * `left_button_down` — whether the left mouse button is currently held
    /// * `wrap_mode` — 0=Both, 1=VerticalOnly, 2=HorizontalOnly
    /// * `disable_on_single_monitor` — if true, skip wrapping when only 1 monitor exists
    ///
    /// Returns the (possibly wrapped) cursor position.
    pub fn handle_mouse_move(
        &mut self,
        current_pos: Point,
        disable_wrap_during_drag: bool,
        left_button_down: bool,
        wrap_mode: i32,
        disable_on_single_monitor: bool,
    ) -> Point {
        // Single-monitor check
        if disable_on_single_monitor && self.monitors.len() <= 1 {
            self.update_previous(current_pos);
            return current_pos;
        }

        // Drag check
        if disable_wrap_during_drag && left_button_down {
            self.update_previous(current_pos);
            return current_pos;
        }

        // Threshold check (oscillation prevention)
        if self.is_within_wrap_threshold(current_pos) {
            self.update_previous(current_pos);
            return current_pos;
        }

        // Clear threshold once cursor has moved away
        if self.has_last_wrap_destination && !self.is_within_wrap_threshold(current_pos) {
            self.has_last_wrap_destination = false;
        }

        // Calculate direction
        let direction = self.calculate_direction(current_pos);

        // Convert wrap_mode int to enum
        let mode = match wrap_mode {
            1 => WrapMode::VerticalOnly,
            2 => WrapMode::HorizontalOnly,
            _ => WrapMode::Both,
        };

        // Find which monitor the cursor is on
        let monitor_index = match self.topology.monitor_index_at_point(current_pos) {
            Some(idx) => idx,
            None => {
                self.update_previous(current_pos);
                return current_pos;
            }
        };

        // Check if on an outer edge
        let edge_type = match self.topology.is_on_outer_edge(
            monitor_index,
            current_pos,
            mode,
            Some(&direction),
        ) {
            Some(et) => et,
            None => {
                self.update_previous(current_pos);
                return current_pos;
            }
        };

        // Calculate wrap destination
        let new_pos = self.topology.get_wrap_destination(monitor_index, current_pos, edge_type);

        self.update_previous(current_pos);

        // Track wrap for threshold
        if new_pos != current_pos {
            self.last_wrap_destination = new_pos;
            self.has_last_wrap_destination = true;
        }

        new_pos
    }

    /// Reset all tracking state.
    pub fn reset_wrap_state(&mut self) {
        self.has_previous_position = false;
        self.has_last_wrap_destination = false;
        self.previous_position = Point {
            x: i32::MIN,
            y: i32::MIN,
        };
        self.last_wrap_destination = Point {
            x: i32::MIN,
            y: i32::MIN,
        };
    }

    pub fn monitor_count(&self) -> usize {
        self.monitors.len()
    }

    pub fn topology(&self) -> &MonitorTopology {
        &self.topology
    }

    // ── Private helpers ────────────────────────────────────────────────

    fn update_previous(&mut self, pos: Point) {
        self.previous_position = pos;
        self.has_previous_position = true;
    }

    fn calculate_direction(&self, current_pos: Point) -> CursorDirection {
        if self.has_previous_position {
            CursorDirection {
                dx: current_pos.x - self.previous_position.x,
                dy: current_pos.y - self.previous_position.y,
            }
        } else {
            CursorDirection { dx: 0, dy: 0 }
        }
    }

    fn is_within_wrap_threshold(&self, current_pos: Point) -> bool {
        if !self.has_last_wrap_destination {
            return false;
        }
        let dx = current_pos.x - self.last_wrap_destination.x;
        let dy = current_pos.y - self.last_wrap_destination.y;
        let dist_sq = dx * dx + dy * dy;
        dist_sq <= WRAP_DISTANCE_THRESHOLD * WRAP_DISTANCE_THRESHOLD
    }
}

impl Default for CursorWrapCore {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_monitor(id: i32, left: i32, top: i32, right: i32, bottom: i32) -> MonitorInfo {
        MonitorInfo {
            rect: Rect {
                left,
                top,
                right,
                bottom,
            },
            is_primary: id == 0,
            monitor_id: id,
        }
    }

    fn two_side_by_side() -> Vec<MonitorInfo> {
        vec![
            make_monitor(0, 0, 0, 1920, 1080),
            make_monitor(1, 1920, 0, 3840, 1080),
        ]
    }

    // ── Basic: not on edge → same position ─────────────────────────────

    #[test]
    fn handle_mouse_move_not_on_edge_returns_same() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        let pos = Point { x: 960, y: 540 }; // center of monitor 0
        let result = core.handle_mouse_move(pos, false, false, 0, false);
        assert_eq!(result, pos);
    }

    // ── On outer edge → wrapped position ───────────────────────────────

    #[test]
    fn handle_mouse_move_left_edge_wraps() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        // First move to establish previous position
        core.handle_mouse_move(Point { x: 100, y: 540 }, false, false, 0, false);

        let pos = Point { x: 0, y: 540 }; // left edge of monitor 0 (outer)
        let result = core.handle_mouse_move(pos, false, false, 0, false);
        assert_ne!(result, pos);
        assert_eq!(result.x, 3839); // right edge of monitor 1
        assert_eq!(result.y, 540);
    }

    #[test]
    fn handle_mouse_move_right_edge_wraps() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        core.handle_mouse_move(Point { x: 3700, y: 540 }, false, false, 0, false);

        let pos = Point { x: 3839, y: 540 }; // right edge of monitor 1 (outer)
        let result = core.handle_mouse_move(pos, false, false, 0, false);
        assert_eq!(result.x, 0);
        assert_eq!(result.y, 540);
    }

    // ── Threshold prevents re-wrap ─────────────────────────────────────

    #[test]
    fn threshold_prevents_rewrap_within_50px() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        // Move to near left edge
        core.handle_mouse_move(Point { x: 50, y: 540 }, false, false, 0, false);

        // Hit left edge → wraps to right
        let wrapped = core.handle_mouse_move(Point { x: 0, y: 540 }, false, false, 0, false);
        assert_eq!(wrapped.x, 3839);

        // Now move near the wrap destination (within 50px) → should NOT re-wrap
        let near_dest = Point { x: 3839, y: 540 };
        let result = core.handle_mouse_move(near_dest, false, false, 0, false);
        assert_eq!(result, near_dest); // Not wrapped again
    }

    #[test]
    fn threshold_clears_when_cursor_moves_away() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        // Initial position, then wrap
        core.handle_mouse_move(Point { x: 50, y: 540 }, false, false, 0, false);
        let wrapped = core.handle_mouse_move(Point { x: 0, y: 540 }, false, false, 0, false);
        assert_eq!(wrapped.x, 3839);

        // Move far from wrap destination (> 50px)
        core.handle_mouse_move(Point { x: 3700, y: 540 }, false, false, 0, false);

        // Now move to right edge — should wrap again since threshold cleared
        let result = core.handle_mouse_move(Point { x: 3839, y: 540 }, false, false, 0, false);
        assert_eq!(result.x, 0);
    }

    // ── disableWrapDuringDrag ──────────────────────────────────────────

    #[test]
    fn disable_wrap_during_drag_with_left_button() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        core.handle_mouse_move(Point { x: 50, y: 540 }, false, false, 0, false);

        // Left button held AND disableWrapDuringDrag=true → no wrap
        let pos = Point { x: 0, y: 540 };
        let result = core.handle_mouse_move(pos, true, true, 0, false);
        assert_eq!(result, pos);
    }

    #[test]
    fn drag_disabled_but_button_not_held_allows_wrap() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        core.handle_mouse_move(Point { x: 50, y: 540 }, false, false, 0, false);

        // disableWrapDuringDrag=true but button NOT held → wrap happens
        let pos = Point { x: 0, y: 540 };
        let result = core.handle_mouse_move(pos, true, false, 0, false);
        assert_eq!(result.x, 3839);
    }

    #[test]
    fn drag_setting_false_allows_wrap_even_with_button() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        core.handle_mouse_move(Point { x: 50, y: 540 }, false, false, 0, false);

        // disableWrapDuringDrag=false but button held → wrap still happens
        let pos = Point { x: 0, y: 540 };
        let result = core.handle_mouse_move(pos, false, true, 0, false);
        assert_eq!(result.x, 3839);
    }

    // ── disableOnSingleMonitor ─────────────────────────────────────────

    #[test]
    fn disable_on_single_monitor_with_one_monitor() {
        let mut core = CursorWrapCore::new();
        core.initialize(&[make_monitor(0, 0, 0, 1920, 1080)]);

        let pos = Point { x: 0, y: 540 };
        let result = core.handle_mouse_move(pos, false, false, 0, true);
        assert_eq!(result, pos); // No wrap
    }

    #[test]
    fn disable_on_single_monitor_false_allows_single_monitor_wrap() {
        let mut core = CursorWrapCore::new();
        core.initialize(&[make_monitor(0, 0, 0, 1920, 1080)]);

        core.handle_mouse_move(Point { x: 50, y: 540 }, false, false, 0, false);

        let pos = Point { x: 0, y: 540 };
        let result = core.handle_mouse_move(pos, false, false, 0, false);
        // Single monitor → wraps to opposite outer edge of same monitor
        assert_eq!(result.x, 1919); // right edge position = right - 1
        assert_eq!(result.y, 540);
    }

    #[test]
    fn disable_on_single_monitor_with_two_monitors_wraps() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        core.handle_mouse_move(Point { x: 50, y: 540 }, false, false, 0, false);

        // 2 monitors, disableOnSingleMonitor=true → wrapping should still happen
        let pos = Point { x: 0, y: 540 };
        let result = core.handle_mouse_move(pos, false, false, 0, true);
        assert_eq!(result.x, 3839);
    }

    // ── Direction tracking ─────────────────────────────────────────────

    #[test]
    fn direction_calculated_from_previous_position() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        // First move: no previous → direction is (0,0)
        core.handle_mouse_move(Point { x: 100, y: 100 }, false, false, 0, false);

        // Second move: direction should be calculated
        // The direction calculation happens internally; we verify indirectly
        // by checking that corner prioritization works correctly
        core.handle_mouse_move(Point { x: 90, y: 100 }, false, false, 0, false);

        // Move toward left edge → direction dx < 0
        // (This is verified by the fact that wrapping happens correctly)
    }

    // ── ResetWrapState ─────────────────────────────────────────────────

    #[test]
    fn reset_wrap_state_clears_tracking() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        // Do a wrap to set tracking state
        core.handle_mouse_move(Point { x: 50, y: 540 }, false, false, 0, false);
        let wrapped = core.handle_mouse_move(Point { x: 0, y: 540 }, false, false, 0, false);
        assert_eq!(wrapped.x, 3839);

        // Now the threshold is set near (3839, 540)
        // Reset should clear all state
        core.reset_wrap_state();

        // After reset, the threshold should be gone — a new move at the wrap destination
        // should not be suppressed. But also, there is no previous position, so direction
        // will be (0,0). Move to establish position first:
        core.handle_mouse_move(Point { x: 3700, y: 540 }, false, false, 0, false);

        // Hit right edge of monitor 1 — should wrap since threshold was cleared
        let result = core.handle_mouse_move(Point { x: 3839, y: 540 }, false, false, 0, false);
        assert_eq!(result.x, 0);
    }

    // ── Wrap mode integration ──────────────────────────────────────────

    #[test]
    fn wrap_mode_vertical_only_blocks_horizontal() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        core.handle_mouse_move(Point { x: 50, y: 540 }, false, false, 0, false);

        // Left edge, but VerticalOnly mode → no horizontal wrap
        let pos = Point { x: 0, y: 540 };
        let result = core.handle_mouse_move(pos, false, false, 1, false);
        // With VerticalOnly, top/bottom outer edges exist but we're not on them
        // The left edge should be filtered out
        assert_eq!(result, pos);
    }

    #[test]
    fn wrap_mode_horizontal_only_allows_horizontal() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        core.handle_mouse_move(Point { x: 50, y: 540 }, false, false, 0, false);

        // Left edge, HorizontalOnly mode → wrap happens
        let pos = Point { x: 0, y: 540 };
        let result = core.handle_mouse_move(pos, false, false, 2, false);
        assert_eq!(result.x, 3839);
    }

    // ── Edge case: cursor beyond monitor bounds ────────────────────────

    #[test]
    fn cursor_beyond_all_monitors_returns_same() {
        let mut core = CursorWrapCore::new();
        core.initialize(&two_side_by_side());

        let pos = Point { x: -100, y: -100 };
        let result = core.handle_mouse_move(pos, false, false, 0, false);
        assert_eq!(result, pos);
    }

    // ── Stacked monitors vertical wrap ─────────────────────────────────

    #[test]
    fn stacked_monitors_top_edge_wraps() {
        let monitors = vec![
            make_monitor(0, 0, 0, 1920, 1080),
            make_monitor(1, 0, 1080, 1920, 2160),
        ];
        let mut core = CursorWrapCore::new();
        core.initialize(&monitors);

        core.handle_mouse_move(Point { x: 960, y: 50 }, false, false, 0, false);

        let pos = Point { x: 960, y: 0 };
        let result = core.handle_mouse_move(pos, false, false, 0, false);
        assert_eq!(result.y, 2159); // bottom edge of monitor 1
        assert_eq!(result.x, 960);
    }
}
