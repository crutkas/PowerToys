//! Keyboard-based zone snapping (Win+Arrow keys).
//!
//! Mirrors C++ `WindowKeyboardSnap`:
//! - Win+Arrow → move window to next zone by index or position
//! - Win+Ctrl+Alt+Arrow → extend zone selection
//! - Delegates to `fancyzones_core::keyboard_snap` for pure logic

use crate::work_area::WorkArea;
use fancyzones_core::keyboard_snap::{self, SnapDirection};
use fancyzones_core::rect::Rect;
use fancyzones_core::zone::{ZoneIndex, ZoneIndexSet};

/// Result of a keyboard snap operation.
#[derive(Debug, Clone)]
pub struct SnapResult {
    /// The zone(s) the window should snap to.
    pub zones: ZoneIndexSet,
    /// The screen rectangle to snap the window to.
    pub rect: Rect,
    /// The work area index.
    pub work_area_idx: usize,
}

/// State for tracking extend mode (Win+Ctrl+Alt+Arrow).
#[derive(Debug, Clone, Default)]
pub struct ExtendState {
    /// The initial zone set when extend mode started.
    pub initial_zones: ZoneIndexSet,
    /// The work area index.
    pub work_area_idx: usize,
    /// Whether we are in extend mode.
    pub active: bool,
}

/// Keyboard snap handler.
#[derive(Debug, Default)]
pub struct KeyboardSnapHandler {
    extend: ExtendState,
}

impl KeyboardSnapHandler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snap by zone index (Left = decrement, Right = increment, wraps).
    /// Used when `move_windows_based_on_position` is false.
    pub fn snap_by_index(
        &self,
        current_zone: Option<ZoneIndex>,
        work_area: &WorkArea,
        direction: SnapDirection,
    ) -> Option<SnapResult> {
        let zone_count = work_area.zone_count();
        let next = keyboard_snap::snap_by_index(current_zone, zone_count, direction)?;
        let zones = vec![next];
        let rect = work_area.get_zone_rect(&zones);
        Some(SnapResult {
            zones,
            rect,
            work_area_idx: 0,
        })
    }

    /// Snap by position (find nearest zone in direction from current window position).
    /// Used when `move_windows_based_on_position` is true.
    pub fn snap_by_position(
        &self,
        window_rect: Rect,
        work_area: &WorkArea,
        direction: SnapDirection,
    ) -> Option<SnapResult> {
        let zone_rects = work_area.zone_rects_relative();
        // Convert window rect to work-area-relative coordinates
        let rel_window = Rect::new(
            window_rect.left - work_area.work_area_rect.left,
            window_rect.top - work_area.work_area_rect.top,
            window_rect.right - work_area.work_area_rect.left,
            window_rect.bottom - work_area.work_area_rect.top,
        );
        let idx = keyboard_snap::snap_by_position(
            rel_window,
            Rect::new(0, 0, work_area.work_area_rect.width(), work_area.work_area_rect.height()),
            &zone_rects,
            direction,
        )?;

        // Convert zone index (positional) to zone ID
        let zone_id = *work_area.layout().zones().keys().nth(idx)?;
        let zones = vec![zone_id];
        let rect = work_area.get_zone_rect(&zones);
        Some(SnapResult {
            zones,
            rect,
            work_area_idx: 0,
        })
    }

    /// Extend zone selection in a direction (Win+Ctrl+Alt+Arrow).
    pub fn extend(
        &mut self,
        current_zones: &ZoneIndexSet,
        work_area: &WorkArea,
        work_area_idx: usize,
        direction: SnapDirection,
    ) -> Option<SnapResult> {
        if !self.extend.active || self.extend.work_area_idx != work_area_idx {
            // Start a new extend session
            self.extend.initial_zones = current_zones.clone();
            self.extend.work_area_idx = work_area_idx;
            self.extend.active = true;
        }

        let work_area_rect = Rect::new(
            0,
            0,
            work_area.work_area_rect.width(),
            work_area.work_area_rect.height(),
        );
        let extended = keyboard_snap::extend_zone_selection(
            work_area.layout(),
            &self.extend.initial_zones,
            direction,
            work_area_rect,
        );

        if extended.is_empty() {
            return None;
        }

        // Update the initial zones for the next extend call
        self.extend.initial_zones = extended.clone();

        let rect = work_area.get_zone_rect(&extended);
        Some(SnapResult {
            zones: extended,
            rect,
            work_area_idx,
        })
    }

    /// Reset extend mode (called when modifier keys are released).
    pub fn reset_extend(&mut self) {
        self.extend = ExtendState::default();
    }

    /// Check if extend mode is active.
    pub fn is_extending(&self) -> bool {
        self.extend.active
    }
}

/// Convert a virtual key code to a SnapDirection.
pub fn direction_from_vk(vk: u32) -> Option<SnapDirection> {
    match vk {
        0x25 => Some(SnapDirection::Left),  // VK_LEFT
        0x27 => Some(SnapDirection::Right), // VK_RIGHT
        0x26 => Some(SnapDirection::Up),    // VK_UP
        0x28 => Some(SnapDirection::Down),  // VK_DOWN
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fancyzones_core::data::{LayoutData, ZoneSetLayoutType};

    fn make_work_area(zone_count: i32) -> WorkArea {
        let data = LayoutData {
            uuid: "test".into(),
            layout_type: ZoneSetLayoutType::Columns,
            show_spacing: false,
            spacing: 0,
            zone_count,
            sensitivity_radius: 20,
        };
        WorkArea::new("k".into(), Rect::new(0, 0, 1920, 1080), &data).unwrap()
    }

    #[test]
    fn snap_right_from_none() {
        let handler = KeyboardSnapHandler::new();
        let wa = make_work_area(3);
        let result = handler.snap_by_index(None, &wa, SnapDirection::Right);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.zones, vec![0]);
        assert!(r.rect.width() > 0);
    }

    #[test]
    fn snap_right_from_first() {
        let handler = KeyboardSnapHandler::new();
        let wa = make_work_area(3);
        let result = handler.snap_by_index(Some(0), &wa, SnapDirection::Right);
        assert!(result.is_some());
        assert_eq!(result.unwrap().zones, vec![1]);
    }

    #[test]
    fn snap_left_wraps() {
        let handler = KeyboardSnapHandler::new();
        let wa = make_work_area(3);
        let result = handler.snap_by_index(Some(0), &wa, SnapDirection::Left);
        assert!(result.is_some());
        assert_eq!(result.unwrap().zones, vec![2]);
    }

    #[test]
    fn snap_right_wraps() {
        let handler = KeyboardSnapHandler::new();
        let wa = make_work_area(3);
        let result = handler.snap_by_index(Some(2), &wa, SnapDirection::Right);
        assert!(result.is_some());
        assert_eq!(result.unwrap().zones, vec![0]);
    }

    #[test]
    fn snap_empty_zones() {
        let handler = KeyboardSnapHandler::new();
        let data = LayoutData {
            uuid: "test".into(),
            layout_type: ZoneSetLayoutType::Blank,
            show_spacing: false,
            spacing: 0,
            zone_count: 0,
            sensitivity_radius: 20,
        };
        let wa = WorkArea::new("k".into(), Rect::new(0, 0, 1920, 1080), &data);
        // Blank layout with 0 zones should still create (empty zone set)
        if let Some(wa) = wa {
            let result = handler.snap_by_index(None, &wa, SnapDirection::Right);
            assert!(result.is_none());
        }
    }

    #[test]
    fn snap_by_position_finds_zone() {
        let handler = KeyboardSnapHandler::new();
        let wa = make_work_area(3);
        // Window at zone 0 position, snap right
        let window_rect = Rect::new(0, 0, 640, 1080);
        let result = handler.snap_by_position(window_rect, &wa, SnapDirection::Right);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.rect.width() > 0);
    }

    #[test]
    fn extend_zones() {
        let mut handler = KeyboardSnapHandler::new();
        let wa = make_work_area(3);
        let current = vec![0i64];
        let result = handler.extend(&current, &wa, 0, SnapDirection::Right);
        assert!(result.is_some());
        let r = result.unwrap();
        // Should have at least 2 zones after extending right from zone 0
        assert!(r.zones.len() >= 2);
    }

    #[test]
    fn extend_reset() {
        let mut handler = KeyboardSnapHandler::new();
        let wa = make_work_area(3);
        handler.extend(&vec![0], &wa, 0, SnapDirection::Right);
        assert!(handler.is_extending());
        handler.reset_extend();
        assert!(!handler.is_extending());
    }

    #[test]
    fn direction_from_vk_codes() {
        assert_eq!(direction_from_vk(0x25), Some(SnapDirection::Left));
        assert_eq!(direction_from_vk(0x27), Some(SnapDirection::Right));
        assert_eq!(direction_from_vk(0x26), Some(SnapDirection::Up));
        assert_eq!(direction_from_vk(0x28), Some(SnapDirection::Down));
        assert_eq!(direction_from_vk(0x00), None);
    }

    #[test]
    fn cycle_all_zones_right() {
        let handler = KeyboardSnapHandler::new();
        let wa = make_work_area(4);
        let mut zone = handler.snap_by_index(None, &wa, SnapDirection::Right);
        let expected = [0, 1, 2, 3, 0];
        for exp in expected {
            assert_eq!(zone.as_ref().unwrap().zones, vec![exp]);
            zone = handler.snap_by_index(
                Some(zone.unwrap().zones[0]),
                &wa,
                SnapDirection::Right,
            );
        }
    }
}
