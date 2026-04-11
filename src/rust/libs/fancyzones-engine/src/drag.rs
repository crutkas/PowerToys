//! Mouse drag detection and zone hit-testing.
//!
//! Mirrors C++ `WindowMouseSnap` and `DraggingState`:
//! - Created when EVENT_SYSTEM_MOVESIZESTART fires
//! - Updated on each EVENT_OBJECT_LOCATIONCHANGE during drag
//! - Produces a snap target on EVENT_SYSTEM_MOVESIZEEND

use crate::work_area::WorkArea;
use fancyzones_core::rect::{Point, Rect};
use fancyzones_core::settings::OverlappingZonesAlgorithm;
use fancyzones_core::zone::ZoneIndexSet;

/// State of an active mouse drag operation.
#[derive(Debug)]
pub struct DragState {
    /// The window handle being dragged.
    pub hwnd: u64,
    /// Current mouse screen position.
    pub cursor: Point,
    /// The work area index the cursor is currently over.
    pub active_work_area: Option<usize>,
    /// The zones currently highlighted (under cursor).
    pub highlighted_zones: ZoneIndexSet,
    /// Whether snapping mode is active (overlay visible).
    pub snapping_mode: bool,
    /// Initial highlighted zone (for multi-zone spanning with Shift).
    initial_zones: Option<ZoneIndexSet>,
}

impl DragState {
    pub fn new(hwnd: u64) -> Self {
        Self {
            hwnd,
            cursor: Point::new(0, 0),
            active_work_area: None,
            highlighted_zones: Vec::new(),
            snapping_mode: false,
            initial_zones: None,
        }
    }

    /// Enable snapping mode (show overlays).
    pub fn enable_snapping(&mut self) {
        self.snapping_mode = true;
    }

    /// Update cursor position and find the zone under it.
    pub fn update(
        &mut self,
        x: i32,
        y: i32,
        work_areas: &[WorkArea],
        algorithm: OverlappingZonesAlgorithm,
        select_many: bool,
    ) {
        self.cursor = Point::new(x, y);
        self.highlighted_zones.clear();
        self.active_work_area = None;

        if !self.snapping_mode {
            return;
        }

        let pt = Point::new(x, y);

        for (idx, wa) in work_areas.iter().enumerate() {
            let zones = wa.zones_from_point(pt, algorithm);
            if !zones.is_empty() {
                self.active_work_area = Some(idx);

                if select_many {
                    // Multi-zone spanning: combine initial zone with current
                    if self.initial_zones.is_none() {
                        self.initial_zones = Some(zones.clone());
                    }
                    if let Some(initial) = &self.initial_zones {
                        self.highlighted_zones =
                            wa.layout().get_combined_zone_range(initial, &zones);
                    }
                } else {
                    self.initial_zones = None;
                    self.highlighted_zones = zones;
                }
                return;
            }
        }
    }

    /// Get the snap target rectangle if zones are highlighted. Returns None if no zone is active.
    pub fn snap_target(&self, work_areas: &[WorkArea]) -> Option<(u64, Rect)> {
        if !self.snapping_mode || self.highlighted_zones.is_empty() {
            return None;
        }
        let wa_idx = self.active_work_area?;
        let wa = work_areas.get(wa_idx)?;
        let rect = wa.get_zone_rect(&self.highlighted_zones);
        if rect.width() <= 0 || rect.height() <= 0 {
            return None;
        }
        Some((self.hwnd, rect))
    }

    /// Get the highlighted zones and work area index for overlay rendering.
    pub fn overlay_info(&self) -> Option<(usize, &ZoneIndexSet)> {
        if !self.snapping_mode || self.highlighted_zones.is_empty() {
            return None;
        }
        Some((self.active_work_area?, &self.highlighted_zones))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fancyzones_core::data::{LayoutData, ZoneSetLayoutType};

    fn make_work_areas() -> Vec<WorkArea> {
        let data = LayoutData {
            uuid: "test".into(),
            layout_type: ZoneSetLayoutType::Columns,
            show_spacing: false,
            spacing: 0,
            zone_count: 3,
            sensitivity_radius: 20,
        };
        vec![
            WorkArea::new(
                "mon0".into(),
                Rect::new(0, 0, 1920, 1080),
                &data,
            )
            .unwrap(),
        ]
    }

    #[test]
    fn new_drag_not_snapping() {
        let drag = DragState::new(42);
        assert!(!drag.snapping_mode);
        assert!(drag.highlighted_zones.is_empty());
    }

    #[test]
    fn drag_finds_zone_when_snapping() {
        let was = make_work_areas();
        let mut drag = DragState::new(42);
        drag.enable_snapping();
        drag.update(100, 540, &was, OverlappingZonesAlgorithm::Smallest, false);
        assert_eq!(drag.active_work_area, Some(0));
        assert!(!drag.highlighted_zones.is_empty());
    }

    #[test]
    fn drag_no_zone_when_not_snapping() {
        let was = make_work_areas();
        let mut drag = DragState::new(42);
        // Don't enable snapping
        drag.update(100, 540, &was, OverlappingZonesAlgorithm::Smallest, false);
        assert!(drag.highlighted_zones.is_empty());
        assert!(drag.active_work_area.is_none());
    }

    #[test]
    fn drag_no_zone_outside() {
        let was = make_work_areas();
        let mut drag = DragState::new(42);
        drag.enable_snapping();
        drag.update(-100, -100, &was, OverlappingZonesAlgorithm::Smallest, false);
        assert!(drag.highlighted_zones.is_empty());
    }

    #[test]
    fn snap_target_returns_rect() {
        let was = make_work_areas();
        let mut drag = DragState::new(42);
        drag.enable_snapping();
        drag.update(100, 540, &was, OverlappingZonesAlgorithm::Smallest, false);
        let target = drag.snap_target(&was);
        assert!(target.is_some());
        let (hwnd, rect) = target.unwrap();
        assert_eq!(hwnd, 42);
        assert!(rect.width() > 0);
        assert!(rect.height() > 0);
    }

    #[test]
    fn snap_target_none_when_outside() {
        let was = make_work_areas();
        let mut drag = DragState::new(42);
        drag.enable_snapping();
        drag.update(-100, -100, &was, OverlappingZonesAlgorithm::Smallest, false);
        assert!(drag.snap_target(&was).is_none());
    }

    #[test]
    fn snap_target_none_when_not_snapping() {
        let was = make_work_areas();
        let drag = DragState::new(42);
        assert!(drag.snap_target(&was).is_none());
    }

    #[test]
    fn multi_zone_spanning() {
        let data = LayoutData {
            uuid: "test".into(),
            layout_type: ZoneSetLayoutType::Columns,
            show_spacing: false,
            spacing: 0,
            zone_count: 3,
            sensitivity_radius: 20,
        };
        let was = vec![
            WorkArea::new("m0".into(), Rect::new(0, 0, 900, 600), &data).unwrap(),
        ];
        let mut drag = DragState::new(1);
        drag.enable_snapping();

        // First update with select_many: sets initial zone
        drag.update(100, 300, &was, OverlappingZonesAlgorithm::Smallest, true);
        assert!(!drag.highlighted_zones.is_empty());

        // Second update to a different zone with select_many: should span
        drag.update(600, 300, &was, OverlappingZonesAlgorithm::Smallest, true);
        // Should have multiple zones highlighted (spanning from initial to current)
        assert!(drag.highlighted_zones.len() >= 2);
    }

    #[test]
    fn overlay_info_matches_state() {
        let was = make_work_areas();
        let mut drag = DragState::new(42);
        drag.enable_snapping();
        drag.update(100, 540, &was, OverlappingZonesAlgorithm::Smallest, false);
        let info = drag.overlay_info();
        assert!(info.is_some());
        let (wa_idx, zones) = info.unwrap();
        assert_eq!(wa_idx, 0);
        assert!(!zones.is_empty());
    }
}
