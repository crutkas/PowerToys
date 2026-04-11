//! Consolidated snapping utilities: settings loading, keyboard snap, app zone history, Win32 helpers.

use fancyzones_core::data::{
    AppliedLayouts, CustomLayoutsStore, DefaultLayouts, LayoutData, LayoutHotkeys,
    LayoutTemplatesStore, MonitorConfigurationType,
};
use fancyzones_core::keyboard_snap::{self, SnapDirection};
use fancyzones_core::rect::Rect;
use fancyzones_core::settings::Settings;
use fancyzones_core::zone::ZoneIndexSet;

use crate::work_area::WorkArea;

// ---- Settings Loading ----

/// All loaded configuration data.
pub struct LoadedData {
    pub settings: Settings,
    pub applied_layouts: AppliedLayouts,
    pub custom_layouts: CustomLayoutsStore,
    pub default_layouts: DefaultLayouts,
    pub layout_hotkeys: LayoutHotkeys,
    pub layout_templates: LayoutTemplatesStore,
}

impl Default for LoadedData {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            applied_layouts: AppliedLayouts::new(),
            custom_layouts: CustomLayoutsStore::new(),
            default_layouts: DefaultLayouts::new(),
            layout_hotkeys: LayoutHotkeys::new(),
            layout_templates: LayoutTemplatesStore::new(),
        }
    }
}

/// Path to the FancyZones data directory.
pub fn data_dir() -> Option<std::path::PathBuf> {
    powertoys_win32::settings::module_dir("FancyZones")
}

/// Parse PowerToys JSON format (`properties.key.value` → flat map).
fn flatten_properties(json: &serde_json::Value) -> serde_json::Value {
    if let Some(props) = json.get("properties").and_then(|v| v.as_object()) {
        let mut flat = serde_json::Map::new();
        for (key, val) in props {
            if let Some(inner) = val.get("value") {
                flat.insert(key.clone(), inner.clone());
            }
        }
        serde_json::Value::Object(flat)
    } else {
        json.clone()
    }
}

/// Load all FancyZones configuration from disk.
pub fn load_all() -> LoadedData {
    let mut data = LoadedData::default();

    if let Ok(json) = powertoys_win32::settings::read_settings_json("FancyZones") {
        let flat = flatten_properties(&json);
        data.settings = Settings::from_json(&flat);
    }

    if let Some(dir) = data_dir() {
        let load = |name: &str| -> Option<serde_json::Value> {
            let content = std::fs::read_to_string(dir.join(name)).ok()?;
            serde_json::from_str(&content).ok()
        };
        if let Some(json) = load("applied-layouts.json") {
            data.applied_layouts.load_from_json(&json);
        }
        if let Some(json) = load("custom-layouts.json") {
            data.custom_layouts.load_from_json(&json);
        }
        if let Some(json) = load("default-layouts.json") {
            data.default_layouts.load_from_json(&json);
        }
        if let Some(json) = load("layout-hotkeys.json") {
            data.layout_hotkeys.load_from_json(&json);
        }
        if let Some(json) = load("layout-templates.json") {
            data.layout_templates.load_from_json(&json);
        }
    }

    data
}

/// Resolve which layout to use for a given device key and monitor configuration.
pub fn resolve_layout(
    device_key: &str,
    config: MonitorConfigurationType,
    data: &LoadedData,
) -> LayoutData {
    if let Some(layout) = data.applied_layouts.get_device_layout(device_key) {
        layout.clone()
    } else {
        data.default_layouts.get_default_layout(config)
    }
}

// ---- Keyboard Snap ----

/// Result of a keyboard snap operation.
pub struct SnapResult {
    pub zones: ZoneIndexSet,
    pub rect: Rect,
    pub work_area_idx: usize,
}

/// Handler for keyboard-based zone snapping.
pub struct KeyboardSnapHandler {
    extend_zones: Option<ZoneIndexSet>,
}

impl KeyboardSnapHandler {
    pub fn new() -> Self {
        Self { extend_zones: None }
    }

    /// Snap by cycling through zone indices.
    pub fn snap_by_index(
        &self,
        current_zones: &ZoneIndexSet,
        work_areas: &[WorkArea],
        wa_idx: usize,
        direction: SnapDirection,
    ) -> Option<SnapResult> {
        let wa = work_areas.get(wa_idx)?;
        let current = current_zones.first().copied();
        let new_index = keyboard_snap::snap_by_index(current, wa.zone_count(), direction)?;
        let zones = vec![new_index];
        let rect = wa.get_zone_rect(&zones);
        Some(SnapResult {
            zones,
            rect,
            work_area_idx: wa_idx,
        })
    }

    /// Snap by spatial position.
    pub fn snap_by_position(
        &self,
        window_rect: Rect,
        work_areas: &[WorkArea],
        wa_idx: usize,
        direction: SnapDirection,
    ) -> Option<SnapResult> {
        let wa = work_areas.get(wa_idx)?;
        let zone_rects = wa.zone_rects_relative();
        let wa_rect = Rect::new(0, 0, wa.work_area_rect().width(), wa.work_area_rect().height());
        let rel_window = Rect::new(
            window_rect.left - wa.work_area_rect().left,
            window_rect.top - wa.work_area_rect().top,
            window_rect.right - wa.work_area_rect().left,
            window_rect.bottom - wa.work_area_rect().top,
        );
        let idx = keyboard_snap::snap_by_position(rel_window, wa_rect, &zone_rects, direction)?;
        let zone_id = *wa.layout().zones().keys().nth(idx)?;
        let zones = vec![zone_id];
        let rect = wa.get_zone_rect(&zones);
        Some(SnapResult {
            zones,
            rect,
            work_area_idx: wa_idx,
        })
    }

    /// Extend current zone selection in a direction.
    pub fn extend(
        &mut self,
        current_zones: &ZoneIndexSet,
        work_areas: &[WorkArea],
        wa_idx: usize,
        direction: SnapDirection,
    ) -> Option<SnapResult> {
        let wa = work_areas.get(wa_idx)?;
        let initial = self.extend_zones.as_ref().unwrap_or(current_zones);
        let wa_rect = Rect::new(0, 0, wa.work_area_rect().width(), wa.work_area_rect().height());
        let new_zones =
            keyboard_snap::extend_zone_selection(wa.layout(), initial, direction, wa_rect);
        self.extend_zones = Some(new_zones.clone());
        let rect = wa.get_zone_rect(&new_zones);
        Some(SnapResult {
            zones: new_zones,
            rect,
            work_area_idx: wa_idx,
        })
    }

    pub fn reset_extend(&mut self) {
        self.extend_zones = None;
    }
}

/// Map a virtual-key code to a SnapDirection.
pub fn direction_from_vk(vk: u32) -> Option<SnapDirection> {
    match vk {
        0x25 => Some(SnapDirection::Left),
        0x27 => Some(SnapDirection::Right),
        0x26 => Some(SnapDirection::Up),
        0x28 => Some(SnapDirection::Down),
        _ => None,
    }
}

// ---- App Zone History (re-exported from app_history module) ----

pub use crate::app_history::{AppZoneHistory, HistoryEntry as AppZoneEntry};

// ---- Win32 Window Utilities ----

#[cfg(windows)]
pub mod win32 {
    use fancyzones_core::rect::Rect;
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    pub fn snap_window_to_rect(hwnd: HWND, rect: &Rect) -> bool {
        unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            ) != 0
        }
    }

    pub fn restore_if_minimized(hwnd: HWND) {
        let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
        if style & WS_MINIMIZE != 0 {
            unsafe {
                ShowWindow(hwnd, SW_RESTORE);
            }
        }
    }

    pub fn activate_window(hwnd: HWND) {
        unsafe {
            SetForegroundWindow(hwnd);
        }
    }

    pub fn get_window_rect(hwnd: HWND) -> Option<Rect> {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
        if ok != 0 {
            Some(Rect::new(rect.left, rect.top, rect.right, rect.bottom))
        } else {
            None
        }
    }

    pub fn is_snap_eligible(hwnd: HWND) -> bool {
        let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
        let visible = (style & WS_VISIBLE) != 0;
        let tool = (ex_style & WS_EX_TOOLWINDOW) != 0;
        visible && !tool
    }

    pub fn set_window_transparency(hwnd: HWND, alpha: u8) {
        unsafe {
            let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex | WS_EX_LAYERED as i32);
            // LWA_ALPHA = 0x02
            SetLayeredWindowAttributes(hwnd, 0, alpha, 0x02);
        }
    }

    pub fn clear_window_transparency(hwnd: HWND) {
        unsafe {
            let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex & !(WS_EX_LAYERED as i32));
        }
    }

    pub fn is_shift_held() -> bool {
        unsafe { (GetKeyState(0x10) as u16 & 0x8000) != 0 }
    }

    pub fn is_ctrl_held() -> bool {
        unsafe { (GetKeyState(0x11) as u16 & 0x8000) != 0 }
    }

    pub fn is_alt_held() -> bool {
        unsafe { (GetKeyState(0x12) as u16 & 0x8000) != 0 }
    }

    pub fn get_cursor_pos() -> Option<(i32, i32)> {
        let mut pt = POINT { x: 0, y: 0 };
        let ok = unsafe { GetCursorPos(&mut pt) };
        if ok != 0 {
            Some((pt.x, pt.y))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fancyzones_core::data::ZoneSetLayoutType;

    // ---- Settings loading tests ----

    #[test]
    fn flatten_properties_extracts_values() {
        let json = serde_json::json!({
            "properties": {
                "fancyzones_shiftDrag": { "value": true },
                "fancyzones_zoneColor": { "value": "#FF0000" },
            }
        });
        let flat = flatten_properties(&json);
        assert_eq!(
            flat.get("fancyzones_shiftDrag").unwrap(),
            &serde_json::json!(true)
        );
        assert_eq!(flat.get("fancyzones_zoneColor").unwrap(), "#FF0000");
    }

    #[test]
    fn flatten_properties_passthrough() {
        let json = serde_json::json!({ "fancyzones_shiftDrag": true });
        let flat = flatten_properties(&json);
        assert_eq!(
            flat.get("fancyzones_shiftDrag").unwrap(),
            &serde_json::json!(true)
        );
    }

    #[test]
    fn loaded_data_defaults() {
        let data = LoadedData::default();
        assert!(data.settings.shift_drag);
        assert_eq!(data.layout_hotkeys.get_hotkeys_count(), 0);
    }

    #[test]
    fn resolve_layout_uses_applied() {
        let mut data = LoadedData::default();
        let ld = LayoutData {
            uuid: "applied-uuid".into(),
            layout_type: ZoneSetLayoutType::Grid,
            show_spacing: true,
            spacing: 8,
            zone_count: 4,
            sensitivity_radius: 20,
        };
        data.applied_layouts.apply_layout("dev1".into(), ld);
        let resolved = resolve_layout("dev1", MonitorConfigurationType::Horizontal, &data);
        assert_eq!(resolved.uuid, "applied-uuid");
        assert_eq!(resolved.zone_count, 4);
    }

    #[test]
    fn resolve_layout_falls_back_to_default() {
        let data = LoadedData::default();
        let resolved =
            resolve_layout("unknown-dev", MonitorConfigurationType::Horizontal, &data);
        assert_eq!(resolved.layout_type, ZoneSetLayoutType::PriorityGrid);
    }

    // ---- Direction mapping tests ----

    #[test]
    fn direction_from_vk_left() {
        assert_eq!(direction_from_vk(0x25), Some(SnapDirection::Left));
    }

    #[test]
    fn direction_from_vk_right() {
        assert_eq!(direction_from_vk(0x27), Some(SnapDirection::Right));
    }

    #[test]
    fn direction_from_vk_up() {
        assert_eq!(direction_from_vk(0x26), Some(SnapDirection::Up));
    }

    #[test]
    fn direction_from_vk_down() {
        assert_eq!(direction_from_vk(0x28), Some(SnapDirection::Down));
    }

    #[test]
    fn direction_from_vk_invalid() {
        assert_eq!(direction_from_vk(0x00), None);
    }

    // ---- Keyboard snap handler tests ----

    #[test]
    fn keyboard_snap_by_index_right() {
        let handler = KeyboardSnapHandler::new();
        let ld = LayoutData {
            uuid: "test".into(),
            layout_type: ZoneSetLayoutType::Columns,
            show_spacing: false,
            spacing: 0,
            zone_count: 3,
            sensitivity_radius: 20,
        };
        let wa = WorkArea::new("dev".into(), Rect::new(0, 0, 1920, 1080), &ld).unwrap();
        let result = handler.snap_by_index(&vec![0], &[wa], 0, SnapDirection::Right);
        assert!(result.is_some());
        assert_eq!(result.unwrap().zones, vec![1]);
    }

    #[test]
    fn keyboard_snap_by_index_wraps() {
        let handler = KeyboardSnapHandler::new();
        let ld = LayoutData {
            uuid: "test".into(),
            layout_type: ZoneSetLayoutType::Columns,
            show_spacing: false,
            spacing: 0,
            zone_count: 3,
            sensitivity_radius: 20,
        };
        let wa = WorkArea::new("dev".into(), Rect::new(0, 0, 1920, 1080), &ld).unwrap();
        let result = handler.snap_by_index(&vec![2], &[wa], 0, SnapDirection::Right);
        assert!(result.is_some());
        assert_eq!(result.unwrap().zones, vec![0]);
    }

    #[test]
    fn keyboard_snap_extend() {
        let ld = LayoutData {
            uuid: "test".into(),
            layout_type: ZoneSetLayoutType::Grid,
            show_spacing: false,
            spacing: 0,
            zone_count: 4,
            sensitivity_radius: 20,
        };
        let wa = WorkArea::new("dev".into(), Rect::new(0, 0, 960, 1080), &ld).unwrap();
        let mut handler = KeyboardSnapHandler::new();
        let result = handler.extend(&vec![0], &[wa], 0, SnapDirection::Right);
        assert!(result.is_some());
        assert!(result.unwrap().zones.contains(&0));
    }

    #[test]
    fn keyboard_snap_reset_extend() {
        let mut handler = KeyboardSnapHandler::new();
        handler.extend_zones = Some(vec![0, 1]);
        handler.reset_extend();
        assert!(handler.extend_zones.is_none());
    }

    // ---- App Zone History re-export tests ----

    #[test]
    fn app_history_record_and_lookup() {
        let mut history = AppZoneHistory::new();
        history.record("app.exe", "dev1", "layout1", vec![0, 1]);
        let zones = history.lookup("app.exe", "dev1");
        assert!(zones.is_some());
        assert_eq!(zones.unwrap(), vec![0, 1]);
    }

    #[test]
    fn app_history_missing_app() {
        let history = AppZoneHistory::new();
        assert!(history.lookup("missing.exe", "dev1").is_none());
    }

    #[test]
    fn app_history_to_json_roundtrip() {
        let mut history = AppZoneHistory::new();
        history.record("app.exe", "dk", "lid", vec![2]);
        let json = history.to_json();
        let arr = json
            .get("app-zone-history")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(arr.len(), 1);
    }

    #[test]
    fn app_history_load_from_json() {
        let json = serde_json::json!({
            "app-zone-history": [
                {
                    "app-path": "test.exe",
                    "history": [
                        {
                            "layout-id": "L1",
                            "device-id": "D1",
                            "zone-index-set": [0, 1, 2],
                        }
                    ]
                }
            ]
        });
        let history = AppZoneHistory::load_from_json(&json);
        let zones = history.lookup("test.exe", "D1");
        assert!(zones.is_some());
        assert_eq!(zones.unwrap(), vec![0, 1, 2]);
    }
}
