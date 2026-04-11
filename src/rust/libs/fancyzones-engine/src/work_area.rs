use fancyzones_core::data::ZoneSetLayoutType;
use fancyzones_core::layout;
use fancyzones_core::rect::Rect;
use fancyzones_core::zone::Zone;

pub struct WorkArea {
    pub monitor_id: String,
    pub work_rect: Rect,
    pub zones: Vec<Zone>,
}

impl WorkArea {
    pub fn new(monitor_id: String, work_rect: Rect, layout_type: ZoneSetLayoutType, zone_count: i32, spacing: i32) -> Self {
        let zm = match layout_type {
            ZoneSetLayoutType::Columns => layout::layout_columns(work_rect, zone_count, spacing),
            ZoneSetLayoutType::Rows => layout::layout_rows(work_rect, zone_count, spacing),
            ZoneSetLayoutType::Grid => layout::layout_grid(work_rect, zone_count, spacing),
            ZoneSetLayoutType::PriorityGrid => layout::layout_priority_grid(work_rect, zone_count, spacing),
            ZoneSetLayoutType::Focus => layout::layout_focus(work_rect, zone_count),
            _ => layout::layout_columns(work_rect, zone_count, spacing),
        };
        Self { monitor_id, work_rect, zones: zm.into_values().collect() }
    }
    pub fn zone_from_point(&self, x: i32, y: i32) -> Option<usize> {
        let mut best: Option<(usize, i64)> = None;
        for (i, z) in self.zones.iter().enumerate() {
            let r = z.get_zone_rect();
            if x >= r.left && x <= r.right && y >= r.top && y <= r.bottom {
                let a = (r.right - r.left) as i64 * (r.bottom - r.top) as i64;
                if best.is_none() || a < best.unwrap().1 { best = Some((i, a)); }
            }
        }
        best.map(|(i,_)| i)
    }
    pub fn zone_rect(&self, idx: usize) -> Option<Rect> { self.zones.get(idx).map(|z| z.get_zone_rect()) }
    pub fn zone_count(&self) -> usize { self.zones.len() }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn columns() {
        let wa = WorkArea::new("m".into(), Rect{left:0,top:0,right:1920,bottom:1080}, ZoneSetLayoutType::Columns, 3, 16);
        assert_eq!(wa.zone_count(), 3);
    }
    #[test]
    fn point_inside() {
        let wa = WorkArea::new("t".into(), Rect{left:0,top:0,right:300,bottom:100}, ZoneSetLayoutType::Columns, 3, 0);
        assert!(wa.zone_from_point(50,50).is_some());
    }
    #[test]
    fn point_outside() {
        let wa = WorkArea::new("t".into(), Rect{left:0,top:0,right:100,bottom:100}, ZoneSetLayoutType::Columns, 1, 0);
        assert!(wa.zone_from_point(-10,50).is_none());
    }
}
