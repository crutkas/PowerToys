use crate::drag_handler::DragState;
use crate::work_area::WorkArea;
use fancyzones_core::data::ZoneSetLayoutType;
use fancyzones_core::rect::Rect;
use fancyzones_core::settings::Settings;
use fancyzones_core::util::hex_to_rgb;

pub struct FancyZonesEngine {
    pub work_areas: Vec<WorkArea>,
    pub settings: Settings,
    pub active_drag: Option<DragState>,
    pub zone_color: (u8,u8,u8),
    pub highlight_color: (u8,u8,u8),
    pub opacity: u8,
}
impl FancyZonesEngine {
    pub fn new(s: Settings) -> Self {
        let zc = hex_to_rgb(&s.zone_color);
        let hc = hex_to_rgb(&s.zone_highlight_color);
        let op = ((s.zone_highlight_opacity.clamp(0,100) as u32 * 255)/100) as u8;
        Self { work_areas: Vec::new(), settings: s, active_drag: None, zone_color: zc, highlight_color: hc, opacity: op }
    }
    pub fn init_work_areas(&mut self, monitors: &[(String, Rect)], lt: ZoneSetLayoutType, zc: i32, sp: i32) {
        self.work_areas.clear();
        for (id, r) in monitors { self.work_areas.push(WorkArea::new(id.clone(), *r, lt.clone(), zc, sp)); }
    }
    pub fn on_drag_start(&mut self, hwnd: isize, x: i32, y: i32) {
        if !self.settings.shift_drag || shift_held() {
            let mut d = DragState::new(hwnd, x, y); d.update(x, y, &self.work_areas);
            self.active_drag = Some(d);
        }
    }
    pub fn on_drag_move(&mut self, x: i32, y: i32) {
        if let Some(d) = &mut self.active_drag { d.update(x, y, &self.work_areas); }
    }
    pub fn on_drag_end(&mut self) -> Option<(isize, Rect)> {
        let d = self.active_drag.take()?; let t = d.snap_target(&self.work_areas)?; Some((d.hwnd, t))
    }
    pub fn cancel_drag(&mut self) { self.active_drag = None; }
    pub fn is_dragging(&self) -> bool { self.active_drag.is_some() }
    pub fn active_zone_info(&self) -> Option<(usize, usize)> {
        let d = self.active_drag.as_ref()?; Some((d.active_monitor?, d.active_zone?))
    }
}
fn shift_held() -> bool {
    unsafe { (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x10) as u16 & 0x8000) != 0 }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn mk() -> FancyZonesEngine {
        let mut e = FancyZonesEngine::new(Settings::default());
        e.init_work_areas(&[("m".into(), Rect{left:0,top:0,right:1920,bottom:1080})], ZoneSetLayoutType::Columns, 3, 16);
        e
    }
    #[test] fn init() { assert_eq!(mk().work_areas[0].zone_count(), 3); }
    #[test] fn drag() {
        let mut e = mk(); e.settings.shift_drag = false;
        e.on_drag_start(42,500,500); assert!(e.is_dragging());
        e.on_drag_move(500,500); assert!(e.on_drag_end().is_some());
    }
    #[test] fn cancel() {
        let mut e = mk(); e.settings.shift_drag = false;
        e.on_drag_start(1,100,100); e.cancel_drag(); assert!(!e.is_dragging());
    }
}
