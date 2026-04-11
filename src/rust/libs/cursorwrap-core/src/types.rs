// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Core types for cursor wrapping — platform-independent, no Win32 dependencies.

use serde::{Deserialize, Serialize};

/// A 2D point (matches POINT from Win32).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// A rectangle (matches RECT from Win32: left/top inclusive, right/bottom exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// Monitor information — platform-agnostic version of Win32 MONITORINFO + HMONITOR.
/// `monitor_id` is a stable index assigned during enumeration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub rect: Rect,
    pub is_primary: bool,
    pub monitor_id: i32,
}

/// Which edge of a monitor rectangle.
/// Discriminant values match the C++ `EdgeType` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum EdgeType {
    Left = 0,
    Right = 1,
    Top = 2,
    Bottom = 3,
}

/// Controls which wrap directions are active.
/// Discriminant values match the C++ `WrapMode` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum WrapMode {
    Both = 0,
    VerticalOnly = 1,
    HorizontalOnly = 2,
}

/// Cursor movement direction (delta from previous position).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorDirection {
    pub dx: i32,
    pub dy: i32,
}

impl CursorDirection {
    pub fn is_moving_left(&self) -> bool {
        self.dx < 0
    }
    pub fn is_moving_right(&self) -> bool {
        self.dx > 0
    }
    pub fn is_moving_up(&self) -> bool {
        self.dy < 0
    }
    pub fn is_moving_down(&self) -> bool {
        self.dy > 0
    }
    /// Returns true if horizontal movement magnitude >= vertical.
    pub fn is_primarily_horizontal(&self) -> bool {
        self.dx.abs() >= self.dy.abs()
    }
}

/// A single edge of a monitor, with metadata about adjacency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorEdge {
    pub monitor_index: i32,
    pub edge_type: EdgeType,
    /// For vertical edges (Left/Right): X coordinate.
    /// For horizontal edges (Top/Bottom): Y coordinate.
    pub position: i32,
    /// For vertical edges: Y start. For horizontal edges: X start.
    pub start: i32,
    /// For vertical edges: Y end. For horizontal edges: X end.
    pub end: i32,
    /// True if no adjacent monitor touches this edge.
    pub is_outer: bool,
}

/// Result of searching for an opposite edge to wrap to.
#[derive(Debug, Clone)]
pub struct OppositeEdgeResult {
    pub edge: MonitorEdge,
    pub found: bool,
    pub requires_projection: bool,
    pub projected_coordinate: i32,
}

impl OppositeEdgeResult {
    pub fn not_found() -> Self {
        Self {
            edge: MonitorEdge {
                monitor_index: -1,
                edge_type: EdgeType::Left,
                position: 0,
                start: 0,
                end: 0,
                is_outer: false,
            },
            found: false,
            requires_projection: false,
            projected_coordinate: 0,
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // CursorDirection tests

    #[test]
    fn direction_is_moving_left() {
        let dir = CursorDirection { dx: -5, dy: 0 };
        assert!(dir.is_moving_left());
        assert!(!dir.is_moving_right());
    }

    #[test]
    fn direction_is_moving_right() {
        let dir = CursorDirection { dx: 5, dy: 0 };
        assert!(dir.is_moving_right());
        assert!(!dir.is_moving_left());
    }

    #[test]
    fn direction_is_moving_up() {
        let dir = CursorDirection { dx: 0, dy: -5 };
        assert!(dir.is_moving_up());
        assert!(!dir.is_moving_down());
    }

    #[test]
    fn direction_is_moving_down() {
        let dir = CursorDirection { dx: 0, dy: 5 };
        assert!(dir.is_moving_down());
        assert!(!dir.is_moving_up());
    }

    #[test]
    fn direction_primarily_horizontal() {
        // |dx| > |dy| → horizontal
        assert!(CursorDirection { dx: 10, dy: 3 }.is_primarily_horizontal());
        // |dx| == |dy| → tie goes to horizontal (matches C++ abs(dx) >= abs(dy))
        assert!(CursorDirection { dx: 5, dy: 5 }.is_primarily_horizontal());
        // |dx| < |dy| → vertical
        assert!(!CursorDirection { dx: 2, dy: 10 }.is_primarily_horizontal());
    }

    #[test]
    fn direction_zero_is_horizontal() {
        // (0,0) → abs(0) >= abs(0) → true
        assert!(CursorDirection { dx: 0, dy: 0 }.is_primarily_horizontal());
    }

    // EdgeType enum value tests

    #[test]
    fn edge_type_discriminants_match_cpp() {
        assert_eq!(EdgeType::Left as i32, 0);
        assert_eq!(EdgeType::Right as i32, 1);
        assert_eq!(EdgeType::Top as i32, 2);
        assert_eq!(EdgeType::Bottom as i32, 3);
    }

    // WrapMode enum value tests

    #[test]
    fn wrap_mode_discriminants_match_cpp() {
        assert_eq!(WrapMode::Both as i32, 0);
        assert_eq!(WrapMode::VerticalOnly as i32, 1);
        assert_eq!(WrapMode::HorizontalOnly as i32, 2);
    }

    // Rect helper tests

    #[test]
    fn rect_dimensions() {
        let r = Rect { left: 0, top: 0, right: 1920, bottom: 1080 };
        assert_eq!(r.width(), 1920);
        assert_eq!(r.height(), 1080);
    }

    #[test]
    fn rect_negative_origin() {
        let r = Rect { left: -1920, top: 0, right: 0, bottom: 1080 };
        assert_eq!(r.width(), 1920);
        assert_eq!(r.height(), 1080);
    }

    // Point equality

    #[test]
    fn point_equality() {
        assert_eq!(Point { x: 10, y: 20 }, Point { x: 10, y: 20 });
        assert_ne!(Point { x: 10, y: 20 }, Point { x: 11, y: 20 });
    }

    // OppositeEdgeResult::not_found

    #[test]
    fn opposite_edge_not_found() {
        let r = OppositeEdgeResult::not_found();
        assert!(!r.found);
        assert_eq!(r.edge.monitor_index, -1);
    }
}
