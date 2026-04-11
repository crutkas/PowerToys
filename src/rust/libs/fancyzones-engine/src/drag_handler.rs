//! DragState: tracks mouse drag for zone snapping.

use crate::work_area::WorkArea;
use fancyzones_core::rect::{Point, Rect};
use fancyzones_core::settings::OverlappingZonesAlgorithm;
use fancyzones_core::zone::ZoneIndexSet;

/// Tracks the state of a window drag for zone snapping.
pub struct DragState {
    pub hwnd: u64,
    pub cursor: Point,
    pub active_work_area: Option<usize>,
    pub highlighted_zones: ZoneIndexSet,
    pub snapping_mode: bool,
    pub initial_zones: Option<ZoneIndexSet>,
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

    pub fn enable_snapping(&mut self) {
        self.snapping_mode = true;
    }

    /// Update drag position and find zones under cursor.
    /// If `select_many` is true, zones are spanned from the initial selection.
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

        let screen_pt = Point::new(x, y);

        for (i, wa) in work_areas.iter().enumerate() {
            let zones = wa.zones_from_point(screen_pt, algorithm);
            if !zones.is_empty() {
                self.active_work_area = Some(i);
                if select_many {
                    if let Some(ref initial) = self.initial_zones {
                        self.highlighted_zones =
                            wa.layout().get_combined_zone_range(initial, &zones);
                    } else {
                        self.initial_zones = Some(zones.clone());
                        self.highlighted_zones = zones;
                    }
                } else {
                    self.highlighted_zones = zones;
                    self.initial_zones = None;
                }
                return;
            }
        }
    }

    /// Returns `(hwnd, snap_rect)` if zones are highlighted.
    pub fn snap_target(&self, work_areas: &[WorkArea]) -> Option<(u64, Rect)> {
        let wa_idx = self.active_work_area?;
        let wa = work_areas.get(wa_idx)?;
        if self.highlighted_zones.is_empty() {
            return None;
        }
        let rect = wa.get_zone_rect(&self.highlighted_zones);
        Some((self.hwnd, rect))
    }

    pub fn snap_target_info(&self, work_areas: &[WorkArea]) -> Option<(usize, usize)> {
        let wa_idx = self.active_work_area?;
        let _wa = work_areas.get(wa_idx)?;
        let first_zone = *self.highlighted_zones.first()? as usize;
        Some((wa_idx, first_zone))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fancyzones_core::data::{LayoutData, ZoneSetLayoutType};

    fn make_work_area() -> WorkArea {
        let ld = LayoutData {
            uuid: "test".into(),
            layout_type: ZoneSetLayoutType::Columns,
            show_spacing: false,
            spacing: 0,
            zone_count: 3,
            sensitivity_radius: 20,
        };
        WorkArea::new("dev".into(), Rect::new(0, 0, 1920, 1080), &ld).unwrap()
    }

    #[test]
    fn finds_zone_on_update() {
        let was = vec![make_work_area()];
        let mut ds = DragState::new(42);
        ds.enable_snapping();
        ds.update(100, 500, &was, OverlappingZonesAlgorithm::Smallest, false);
        assert!(ds.active_work_area.is_some());
        assert!(!ds.highlighted_zones.is_empty());
    }

    #[test]
    fn no_zone_outside() {
        let was = vec![make_work_area()];
        let mut ds = DragState::new(42);
        ds.enable_snapping();
        ds.update(-100, -100, &was, OverlappingZonesAlgorithm::Smallest, false);
        assert!(ds.active_work_area.is_none());
        assert!(ds.highlighted_zones.is_empty());
    }

    #[test]
    fn snap_target_returns_rect() {
        let was = vec![make_work_area()];
        let mut ds = DragState::new(42);
        ds.enable_snapping();
        ds.update(100, 500, &was, OverlappingZonesAlgorithm::Smallest, false);
        let target = ds.snap_target(&was);
        assert!(target.is_some());
        let (hwnd, rect) = target.unwrap();
        assert_eq!(hwnd, 42);
        assert!(rect.width() > 0);
        assert!(rect.height() > 0);
    }

    #[test]
    fn snap_target_none_when_no_update() {
        let was = vec![make_work_area()];
        let ds = DragState::new(42);
        assert!(ds.snap_target(&was).is_none());
    }

    #[test]
    fn multi_zone_spanning() {
        let was = vec![make_work_area()];
        let mut ds = DragState::new(42);
        ds.enable_snapping();
        ds.update(100, 500, &was, OverlappingZonesAlgorithm::Smallest, true);
        assert!(!ds.highlighted_zones.is_empty());
        let initial_count = ds.highlighted_zones.len();
        ds.update(1800, 500, &was, OverlappingZonesAlgorithm::Smallest, true);
        assert!(ds.highlighted_zones.len() >= initial_count);
    }

    #[test]
    fn new_default_state() {
        let ds = DragState::new(99);
        assert_eq!(ds.hwnd, 99);
        assert!(!ds.snapping_mode);
        assert!(ds.highlighted_zones.is_empty());
        assert!(ds.active_work_area.is_none());
        assert!(ds.initial_zones.is_none());
    }
}
