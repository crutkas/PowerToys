use crate::work_area::WorkArea;
use fancyzones_core::rect::Rect;

pub struct DragState {
    pub hwnd: isize, pub current_x: i32, pub current_y: i32,
    pub active_zone: Option<usize>, pub active_monitor: Option<usize>,
}
impl DragState {
    pub fn new(hwnd: isize, x: i32, y: i32) -> Self {
        Self { hwnd, current_x: x, current_y: y, active_zone: None, active_monitor: None }
    }
    pub fn update(&mut self, x: i32, y: i32, was: &[WorkArea]) {
        self.current_x = x; self.current_y = y;
        self.active_zone = None; self.active_monitor = None;
        for (mi, wa) in was.iter().enumerate() {
            if let Some(zi) = wa.zone_from_point(x, y) {
                self.active_zone = Some(zi); self.active_monitor = Some(mi); break;
            }
        }
    }
    pub fn snap_target(&self, was: &[WorkArea]) -> Option<Rect> {
        was.get(self.active_monitor?)?.zone_rect(self.active_zone?)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use fancyzones_core::data::ZoneSetLayoutType;
    #[test]
    fn finds_zone() {
        let w = vec![WorkArea::new("m".into(), Rect{left:0,top:0,right:1920,bottom:1080}, ZoneSetLayoutType::Columns, 3, 0)];
        let mut d = DragState::new(1,100,100); d.update(100,500,&w);
        assert!(d.active_zone.is_some());
    }
    #[test]
    fn no_zone_outside() {
        let w = vec![WorkArea::new("m".into(), Rect{left:0,top:0,right:1920,bottom:1080}, ZoneSetLayoutType::Columns, 3, 0)];
        let mut d = DragState::new(1,0,0); d.update(-100,-100,&w);
        assert!(d.active_zone.is_none());
    }
}
