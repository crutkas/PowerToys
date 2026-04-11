//! WorkArea: a zone layout bound to a monitor work area.

use fancyzones_core::data::{
    CustomLayoutInfo, CustomLayoutsStore, LayoutAssignedWindows, WindowHandle, ZoneSetLayoutType,
};
use fancyzones_core::layout::{self, Layout, LayoutData as LayoutInitData};
use fancyzones_core::rect::{Point, Rect};
use fancyzones_core::settings::OverlappingZonesAlgorithm;
use fancyzones_core::zone::ZoneIndexSet;

/// A work area owns a layout and tracks assigned windows.
/// Zones are stored in relative coordinates (0-based).
/// Screen coordinate conversion happens at the WorkArea boundary.
pub struct WorkArea {
    device_key: String,
    work_area_rect: Rect,
    layout: Layout,
    assigned_windows: LayoutAssignedWindows,
}

impl WorkArea {
    /// Create a WorkArea from a standard (non-custom) layout.
    pub fn new(
        device_key: String,
        work_area_rect: Rect,
        layout_data: &fancyzones_core::data::LayoutData,
    ) -> Option<Self> {
        let init_data = LayoutInitData {
            uuid: layout_data.uuid.clone(),
            layout_type: layout_data.layout_type,
            show_spacing: layout_data.show_spacing,
            spacing: layout_data.spacing,
            zone_count: layout_data.zone_count,
            sensitivity_radius: layout_data.sensitivity_radius,
        };
        let mut layout = Layout::new(init_data);
        let relative_rect = Rect::new(0, 0, work_area_rect.width(), work_area_rect.height());
        if !layout.init(relative_rect) {
            return None;
        }
        Some(Self {
            device_key,
            work_area_rect,
            layout,
            assigned_windows: LayoutAssignedWindows::new(),
        })
    }

    /// Create a WorkArea from a custom layout stored in the CustomLayoutsStore.
    pub fn new_custom(
        device_key: String,
        work_area_rect: Rect,
        layout_data: &fancyzones_core::data::LayoutData,
        custom_layouts: &CustomLayoutsStore,
    ) -> Option<Self> {
        let custom = custom_layouts.get_layout(&layout_data.uuid)?;
        let zones: Vec<Rect> = match &custom.info {
            CustomLayoutInfo::Canvas(canvas) => canvas
                .zones
                .iter()
                .map(|z| Rect::new(z.x, z.y, z.x + z.width, z.y + z.height))
                .collect(),
            CustomLayoutInfo::Grid(grid) => {
                let spacing = if grid.show_spacing { grid.spacing } else { 0 };
                let relative_rect =
                    Rect::new(0, 0, work_area_rect.width(), work_area_rect.height());
                let zones_map = layout::calculate_grid_zones(relative_rect, grid, spacing);
                zones_map.values().map(|z| z.get_zone_rect()).collect()
            }
        };

        let init_data = LayoutInitData {
            uuid: layout_data.uuid.clone(),
            layout_type: ZoneSetLayoutType::Custom,
            show_spacing: layout_data.show_spacing,
            spacing: layout_data.spacing,
            zone_count: zones.len() as i32,
            sensitivity_radius: layout_data.sensitivity_radius,
        };
        let mut layout = Layout::new(init_data);
        if !layout.init_custom(zones) {
            return None;
        }

        Some(Self {
            device_key,
            work_area_rect,
            layout,
            assigned_windows: LayoutAssignedWindows::new(),
        })
    }

    /// Find zones under a screen-coordinate point.
    pub fn zones_from_point(
        &self,
        screen_pt: Point,
        algorithm: OverlappingZonesAlgorithm,
    ) -> ZoneIndexSet {
        let relative_pt = Point::new(
            screen_pt.x - self.work_area_rect.left,
            screen_pt.y - self.work_area_rect.top,
        );
        self.layout.zones_from_point(relative_pt, algorithm)
    }

    /// Get the combined bounding rect for a set of zones, in screen coordinates.
    pub fn get_zone_rect(&self, zones: &ZoneIndexSet) -> Rect {
        let r = self.layout.get_combined_zones_rect(zones);
        Rect::new(
            r.left + self.work_area_rect.left,
            r.top + self.work_area_rect.top,
            r.right + self.work_area_rect.left,
            r.bottom + self.work_area_rect.top,
        )
    }

    /// All zone rects in screen coordinates.
    pub fn zone_rects_screen(&self) -> Vec<Rect> {
        let ox = self.work_area_rect.left;
        let oy = self.work_area_rect.top;
        self.layout
            .zones()
            .values()
            .map(|z| {
                let r = z.get_zone_rect();
                Rect::new(r.left + ox, r.top + oy, r.right + ox, r.bottom + oy)
            })
            .collect()
    }

    /// All zone rects in relative (0-based) coordinates.
    pub fn zone_rects_relative(&self) -> Vec<Rect> {
        self.layout
            .zones()
            .values()
            .map(|z| z.get_zone_rect())
            .collect()
    }

    pub fn assign_window(&mut self, window: WindowHandle, zones: ZoneIndexSet) {
        self.assigned_windows.assign(window, zones);
    }

    pub fn unassign_window(&mut self, window: WindowHandle) {
        self.assigned_windows.dismiss(window);
    }

    pub fn get_window_zones(&self, window: WindowHandle) -> ZoneIndexSet {
        self.assigned_windows.get_zone_index_set_from_window(window)
    }

    pub fn zone_count(&self) -> usize {
        self.layout.zones().len()
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn device_key(&self) -> &str {
        &self.device_key
    }

    pub fn work_area_rect(&self) -> Rect {
        self.work_area_rect
    }

    pub fn layout_id(&self) -> &str {
        self.layout.id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fancyzones_core::data::LayoutData;

    fn make_layout_data(lt: ZoneSetLayoutType, zone_count: i32) -> LayoutData {
        LayoutData {
            uuid: "test-uuid".to_string(),
            layout_type: lt,
            show_spacing: false,
            spacing: 0,
            zone_count,
            sensitivity_radius: 20,
        }
    }

    #[test]
    fn new_columns_3() {
        let ld = make_layout_data(ZoneSetLayoutType::Columns, 3);
        let wa = WorkArea::new("dev1".into(), Rect::new(0, 0, 1920, 1080), &ld);
        assert!(wa.is_some());
        assert_eq!(wa.unwrap().zone_count(), 3);
    }

    #[test]
    fn new_rows_2() {
        let ld = make_layout_data(ZoneSetLayoutType::Rows, 2);
        let wa = WorkArea::new("dev1".into(), Rect::new(0, 0, 1920, 1080), &ld);
        assert!(wa.is_some());
        assert_eq!(wa.unwrap().zone_count(), 2);
    }

    #[test]
    fn new_grid_4() {
        let ld = make_layout_data(ZoneSetLayoutType::Grid, 4);
        let wa = WorkArea::new("dev1".into(), Rect::new(0, 0, 1920, 1080), &ld);
        assert!(wa.is_some());
        assert_eq!(wa.unwrap().zone_count(), 4);
    }

    #[test]
    fn new_priority_grid() {
        let ld = make_layout_data(ZoneSetLayoutType::PriorityGrid, 3);
        let wa = WorkArea::new("dev1".into(), Rect::new(0, 0, 1920, 1080), &ld);
        assert!(wa.is_some());
        assert_eq!(wa.unwrap().zone_count(), 3);
    }

    #[test]
    fn new_focus() {
        let ld = make_layout_data(ZoneSetLayoutType::Focus, 3);
        let wa = WorkArea::new("dev1".into(), Rect::new(0, 0, 1920, 1080), &ld);
        assert!(wa.is_some());
        assert_eq!(wa.unwrap().zone_count(), 3);
    }

    #[test]
    fn new_blank() {
        let ld = make_layout_data(ZoneSetLayoutType::Blank, 0);
        let wa = WorkArea::new("dev1".into(), Rect::new(0, 0, 1920, 1080), &ld);
        assert!(wa.is_some());
        assert_eq!(wa.unwrap().zone_count(), 0);
    }

    #[test]
    fn new_custom_returns_none() {
        let ld = make_layout_data(ZoneSetLayoutType::Custom, 3);
        let wa = WorkArea::new("dev1".into(), Rect::new(0, 0, 1920, 1080), &ld);
        assert!(wa.is_none());
    }

    #[test]
    fn zones_from_point_finds_zone() {
        let ld = make_layout_data(ZoneSetLayoutType::Columns, 3);
        let wa = WorkArea::new("dev1".into(), Rect::new(100, 200, 1000, 800), &ld).unwrap();
        let zones = wa.zones_from_point(
            Point::new(150, 400),
            OverlappingZonesAlgorithm::Smallest,
        );
        assert!(!zones.is_empty());
    }

    #[test]
    fn screen_coords_offset() {
        let ld = make_layout_data(ZoneSetLayoutType::Columns, 1);
        let wa = WorkArea::new("dev1".into(), Rect::new(100, 200, 500, 600), &ld).unwrap();
        let screen_rects = wa.zone_rects_screen();
        assert_eq!(screen_rects.len(), 1);
        assert_eq!(screen_rects[0].left, 100);
        assert_eq!(screen_rects[0].top, 200);
        assert_eq!(screen_rects[0].right, 500);
        assert_eq!(screen_rects[0].bottom, 600);
    }

    #[test]
    fn relative_coords() {
        let ld = make_layout_data(ZoneSetLayoutType::Columns, 1);
        let wa = WorkArea::new("dev1".into(), Rect::new(100, 200, 500, 600), &ld).unwrap();
        let rel_rects = wa.zone_rects_relative();
        assert_eq!(rel_rects.len(), 1);
        assert_eq!(rel_rects[0].left, 0);
        assert_eq!(rel_rects[0].top, 0);
        assert_eq!(rel_rects[0].right, 400);
        assert_eq!(rel_rects[0].bottom, 400);
    }

    #[test]
    fn assign_unassign_window() {
        let ld = make_layout_data(ZoneSetLayoutType::Columns, 3);
        let mut wa = WorkArea::new("dev1".into(), Rect::new(0, 0, 1920, 1080), &ld).unwrap();
        wa.assign_window(42, vec![0]);
        assert_eq!(wa.get_window_zones(42), vec![0]);
        wa.unassign_window(42);
        assert!(wa.get_window_zones(42).is_empty());
    }

    #[test]
    fn get_zone_rect_screen() {
        let ld = make_layout_data(ZoneSetLayoutType::Columns, 2);
        let wa = WorkArea::new("dev1".into(), Rect::new(100, 200, 500, 600), &ld).unwrap();
        let rect = wa.get_zone_rect(&vec![0, 1]);
        assert_eq!(rect.left, 100);
        assert_eq!(rect.top, 200);
        assert_eq!(rect.right, 500);
        assert_eq!(rect.bottom, 600);
    }
}
