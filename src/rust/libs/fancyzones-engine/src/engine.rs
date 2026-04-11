//! FancyZonesEngine: main coordinator for zone snapping.

use std::ptr;

use fancyzones_core::data::{MonitorConfigurationType, ZoneSetLayoutType};
use fancyzones_core::rect::Rect;
use fancyzones_core::settings::Settings;
use fancyzones_core::zone::ZoneIndexSet;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

use crate::app_history::AppZoneHistory;
use crate::drag_handler::DragState;
use crate::snap::{self, direction_from_vk, KeyboardSnapHandler, LoadedData};
use crate::work_area::WorkArea;

/// WinEvent hook handle (= `*mut c_void`).
type WinEventHook = *mut core::ffi::c_void;

/// Information returned after a successful snap, for app zone history recording.
pub struct SnapInfo {
    pub rect: Rect,
    pub device_key: String,
    pub layout_id: String,
    pub zones: ZoneIndexSet,
}

pub struct FancyZonesEngine {
    work_areas: Vec<WorkArea>,
    settings: Settings,
    loaded_data: LoadedData,
    active_drag: Option<DragState>,
    keyboard_handler: KeyboardSnapHandler,
    app_history: AppZoneHistory,
    event_hook: WinEventHook,
}

impl FancyZonesEngine {
    /// Load settings from disk and initialize.
    pub fn init() -> Self {
        let data = snap::load_all();
        let app_history = if let Some(dir) = snap::data_dir() {
            AppZoneHistory::load(&dir.join("app-zone-history.json"))
        } else {
            AppZoneHistory::new()
        };
        let mut engine = Self::new_with_data(data);
        engine.app_history = app_history;
        engine
    }

    /// Initialize with pre-loaded data (useful for testing).
    pub fn new_with_data(data: LoadedData) -> Self {
        Self {
            work_areas: Vec::new(),
            settings: data.settings.clone(),
            loaded_data: data,
            active_drag: None,
            keyboard_handler: KeyboardSnapHandler::new(),
            app_history: AppZoneHistory::new(),
            event_hook: ptr::null_mut(),
        }
    }

    /// Enumerate monitors and create WorkAreas.
    pub fn update_work_areas(&mut self) {
        self.work_areas.clear();
        let monitors = powertoys_win32::monitor::enum_monitors();
        for mon in &monitors {
            let device_key = format!("{}_{}", mon.handle as usize, mon.device_name);
            let wa_rect = fancyzones_core::rect::Rect::new(
                mon.work_area.left,
                mon.work_area.top,
                mon.work_area.right,
                mon.work_area.bottom,
            );

            let config = if wa_rect.width() >= wa_rect.height() {
                MonitorConfigurationType::Horizontal
            } else {
                MonitorConfigurationType::Vertical
            };
            let layout_data = snap::resolve_layout(&device_key, config, &self.loaded_data);

            let wa = if layout_data.layout_type == ZoneSetLayoutType::Custom {
                WorkArea::new_custom(
                    device_key,
                    wa_rect,
                    &layout_data,
                    &self.loaded_data.custom_layouts,
                )
            } else {
                WorkArea::new(device_key, wa_rect, &layout_data)
            };

            if let Some(wa) = wa {
                self.work_areas.push(wa);
            }
        }
    }

    /// Begin tracking a window drag. Always start — Shift check happens during move.
    pub fn on_move_size_start(&mut self, hwnd: HWND) {
        let mut drag = DragState::new(hwnd as u64);
        drag.enable_snapping();
        self.active_drag = Some(drag);
    }

    /// Update drag position. Returns whether zones should be shown.
    pub fn on_mouse_move(&mut self, x: i32, y: i32) -> bool {
        let should_snap = if self.settings.shift_drag {
            is_shift_held()
        } else {
            !is_shift_held()
        };

        if let Some(ref mut drag) = self.active_drag {
            if should_snap {
                drag.update(
                    x,
                    y,
                    &self.work_areas,
                    self.settings.overlapping_zones_algorithm,
                    is_ctrl_held(),
                );
                return true;
            }
        }
        false
    }

    /// End drag and return the snap target rect if any.
    /// Also returns (work_area_idx, zone_indices) so the caller can record history.
    pub fn on_move_size_end(&mut self) -> Option<SnapInfo> {
        let drag = self.active_drag.take()?;
        let (wa_idx, zone_idx) = drag.snap_target_info(&self.work_areas)?;
        let zones = vec![zone_idx as i64];
        let rect = self.work_areas.get(wa_idx)?.get_zone_rect(&zones);

        // Record which zone this window is in so keyboard snap knows
        if let Some(wa) = self.work_areas.get_mut(wa_idx) {
            wa.assign_window(drag.hwnd, zones.clone());
        }

        let device_key = self.work_areas.get(wa_idx)?.device_key().to_string();
        let layout_id = self.work_areas.get(wa_idx)?.layout_id().to_string();

        Some(SnapInfo { rect, device_key, layout_id, zones })
    }

    /// Handle a keyboard snap hotkey (Win+Arrow).
    /// Returns snap info including rect and metadata for history recording.
    pub fn on_snap_hotkey(&mut self, hwnd: HWND, vk_code: u32) -> Option<SnapInfo> {
        let direction = direction_from_vk(vk_code)?;
        let (wa_idx, current_zones) = self.find_window_work_area(hwnd as u64)?;

        let result = if is_shift_held() {
            self.keyboard_handler
                .extend(&current_zones, &self.work_areas, wa_idx, direction)
        } else if self.settings.move_windows_based_on_position {
            let win_rect = snap::win32::get_window_rect(hwnd)?;
            self.keyboard_handler.reset_extend();
            self.keyboard_handler
                .snap_by_position(win_rect, &self.work_areas, wa_idx, direction)
        } else {
            self.keyboard_handler.reset_extend();
            self.keyboard_handler
                .snap_by_index(&current_zones, &self.work_areas, wa_idx, direction)
        }?;

        if let Some(wa) = self.work_areas.get_mut(result.work_area_idx) {
            wa.assign_window(hwnd as u64, result.zones.clone());
        }

        let device_key = self.work_areas.get(result.work_area_idx)?.device_key().to_string();
        let layout_id = self.work_areas.get(result.work_area_idx)?.layout_id().to_string();

        Some(SnapInfo { rect: result.rect, device_key, layout_id, zones: result.zones })
    }

    /// Set work areas directly (for testing).
    pub fn set_work_areas(&mut self, work_areas: Vec<WorkArea>) {
        self.work_areas = work_areas;
    }

    pub fn work_areas(&self) -> &[WorkArea] {
        &self.work_areas
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn is_dragging(&self) -> bool {
        self.active_drag.is_some()
    }

    /// Get the active work area index and highlighted zone indices during drag.
    pub fn active_zone_info(&self) -> Option<(usize, &ZoneIndexSet)> {
        let drag = self.active_drag.as_ref()?;
        let wa_idx = drag.active_work_area?;
        Some((wa_idx, &drag.highlighted_zones))
    }

    /// Record an app's zone assignment in the history.
    pub fn record_app_history(
        &mut self,
        app_path: &str,
        device_key: &str,
        layout_id: &str,
        zones: ZoneIndexSet,
    ) {
        self.app_history.record(app_path, device_key, layout_id, zones);
    }

    /// Look up remembered zones for an app on a given device.
    pub fn lookup_app_history(&self, app_path: &str, device_key: &str) -> Option<Vec<i64>> {
        self.app_history.lookup(app_path, device_key)
    }

    /// Save app zone history to the standard data directory.
    pub fn save_app_history(&self) {
        if let Some(dir) = snap::data_dir() {
            let _ = self.app_history.save(&dir.join("app-zone-history.json"));
        }
    }

    /// Get mutable access to the app history (for direct manipulation).
    pub fn app_history_mut(&mut self) -> &mut AppZoneHistory {
        &mut self.app_history
    }

    /// Shut down and clean up. Saves app zone history to disk.
    pub fn shutdown(&mut self) {
        self.save_app_history();
        self.active_drag = None;
        self.work_areas.clear();
        if !self.event_hook.is_null() {
            unsafe {
                windows_sys::Win32::UI::Accessibility::UnhookWinEvent(self.event_hook);
            }
            self.event_hook = ptr::null_mut();
        }
    }

    fn find_window_work_area(&self, window: u64) -> Option<(usize, ZoneIndexSet)> {
        for (i, wa) in self.work_areas.iter().enumerate() {
            let zones = wa.get_window_zones(window);
            if !zones.is_empty() {
                return Some((i, zones));
            }
        }
        // Window not assigned — use first work area with empty zones
        if !self.work_areas.is_empty() {
            Some((0, vec![]))
        } else {
            None
        }
    }
}

fn is_shift_held() -> bool {
    unsafe { (GetAsyncKeyState(0x10) as u16 & 0x8000) != 0 }
}

fn is_ctrl_held() -> bool {
    unsafe { (GetAsyncKeyState(0x11) as u16 & 0x8000) != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fancyzones_core::data::{LayoutData, ZoneSetLayoutType};
    use fancyzones_core::settings::OverlappingZonesAlgorithm;

    fn make_engine_with_work_areas() -> FancyZonesEngine {
        let data = LoadedData::default();
        let mut engine = FancyZonesEngine::new_with_data(data);
        let ld = LayoutData {
            uuid: "test".into(),
            layout_type: ZoneSetLayoutType::Columns,
            show_spacing: false,
            spacing: 0,
            zone_count: 3,
            sensitivity_radius: 20,
        };
        let wa = WorkArea::new("dev1".into(), Rect::new(0, 0, 1920, 1080), &ld).unwrap();
        engine.set_work_areas(vec![wa]);
        engine
    }

    #[test]
    fn new_with_data_defaults() {
        let engine = FancyZonesEngine::new_with_data(LoadedData::default());
        assert!(engine.work_areas.is_empty());
        assert!(!engine.is_dragging());
    }

    #[test]
    fn set_work_areas_test() {
        let engine = make_engine_with_work_areas();
        assert_eq!(engine.work_areas().len(), 1);
        assert_eq!(engine.work_areas()[0].zone_count(), 3);
    }

    #[test]
    fn drag_lifecycle() {
        let mut engine = make_engine_with_work_areas();
        engine.settings.shift_drag = false;
        let mut drag = DragState::new(42);
        drag.enable_snapping();
        drag.update(
            500,
            500,
            &engine.work_areas,
            OverlappingZonesAlgorithm::Smallest,
            false,
        );
        engine.active_drag = Some(drag);
        assert!(engine.is_dragging());
        let snap_info = engine.on_move_size_end();
        assert!(snap_info.is_some());
        let info = snap_info.unwrap();
        assert!(!info.zones.is_empty());
        assert!(!engine.is_dragging());
    }

    #[test]
    fn shutdown_cleans_up() {
        let mut engine = make_engine_with_work_areas();
        engine.shutdown();
        assert!(engine.work_areas.is_empty());
        assert!(!engine.is_dragging());
    }

    #[test]
    fn find_window_unassigned() {
        let engine = make_engine_with_work_areas();
        let result = engine.find_window_work_area(999);
        assert!(result.is_some());
        let (idx, zones) = result.unwrap();
        assert_eq!(idx, 0);
        assert!(zones.is_empty());
    }

    #[test]
    fn find_window_assigned() {
        let mut engine = make_engine_with_work_areas();
        engine.work_areas[0].assign_window(123, vec![1]);
        let result = engine.find_window_work_area(123);
        assert!(result.is_some());
        let (idx, zones) = result.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(zones, vec![1]);
    }

    #[test]
    fn engine_app_history_integration() {
        let mut engine = make_engine_with_work_areas();
        engine.record_app_history("test.exe", "dev1", "layout1", vec![0, 1]);
        let zones = engine.lookup_app_history("test.exe", "dev1");
        assert_eq!(zones, Some(vec![0, 1]));
    }

    #[test]
    fn engine_app_history_miss() {
        let engine = make_engine_with_work_areas();
        assert!(engine.lookup_app_history("missing.exe", "dev1").is_none());
    }
}
