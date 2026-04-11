//! Mouse Pointer Crosshairs Module Interface — Rust implementation
//!
//! This is the PowerToys module DLL for Mouse Pointer Crosshairs.
//! It implements the `PowerToyModule` trait from `powertoys-module-ffi`
//! and manages the overlay window, mouse hook, and crosshair rendering.
//!
//! Architecture:
//! - `enable()` starts the crosshairs overlay (hook + D2D overlay)
//! - `disable()` stops it
//! - Core crosshair geometry is in `crosshairs-core` (platform-independent)
//! - Win32/D2D rendering lives here (platform-specific)

use powertoys_module_ffi::*;
use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

mod overlay;
mod wide;

use crosshairs_core::settings::parse_settings;
use crosshairs_core::types::Settings;

pub struct CrosshairsModule {
    enabled: AtomicBool,
    settings: Settings,
    overlay: Option<overlay::OverlayHandle>,
}

impl CrosshairsModule {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            settings: Settings::default(),
            overlay: None,
        }
    }
}

impl PowerToyModule for CrosshairsModule {
    fn get_name(&self) -> *const u16 {
        wide::module_name_wide_ptr()
    }

    fn get_key(&self) -> *const u16 {
        wide::module_name_wide_ptr()
    }

    fn enable(&mut self) {
        if !self.enabled.load(Ordering::SeqCst) {
            self.enabled.store(true, Ordering::SeqCst);
            self.overlay = Some(overlay::OverlayHandle::start(self.settings.clone()));
        }
    }

    fn disable(&mut self) {
        if self.enabled.load(Ordering::SeqCst) {
            if let Some(ref mut h) = self.overlay {
                h.stop();
            }
            self.overlay = None;
            self.enabled.store(false, Ordering::SeqCst);
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    fn get_config(&self, buffer: *mut u16, buffer_size: *mut c_int) -> bool {
        let json = serde_json::to_string(&self.settings).unwrap_or_else(|_| "{}".to_string());
        let wide_str: Vec<u16> = json.encode_utf16().chain(std::iter::once(0)).collect();
        let needed = wide_str.len() as c_int;

        if buffer.is_null() || buffer_size.is_null() {
            if !buffer_size.is_null() {
                unsafe { *buffer_size = needed };
            }
            return false;
        }

        let available = unsafe { *buffer_size };
        if available < needed {
            unsafe { *buffer_size = needed };
            return false;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(wide_str.as_ptr(), buffer, wide_str.len());
            *buffer_size = needed;
        }
        true
    }

    fn set_config(&mut self, config: *const u16) {
        if config.is_null() {
            return;
        }
        let s = wide::wide_to_string(config);
        if let Ok(new_settings) = parse_settings(&s) {
            self.settings = new_settings;
        }
    }

    fn destroy(&mut self) {
        self.disable();
    }

    fn gpo_policy_enabled_configuration(&self) -> GpoRuleConfigured {
        check_gpo_for_crosshairs()
    }
}

fn check_gpo_for_crosshairs() -> GpoRuleConfigured {
    use windows_sys::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    let subkey: Vec<u16> = "SOFTWARE\\Policies\\PowerToys"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let value_name: Vec<u16> = "EnableMousePointerCrosshairs"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey = std::ptr::null_mut();
        let res = RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey);
        if res != 0 {
            return GpoRuleConfigured::NotConfigured;
        }

        let mut data: u32 = 0;
        let mut data_size = std::mem::size_of::<u32>() as u32;
        let mut data_type: u32 = 0;
        let res = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null(),
            &mut data_type,
            &mut data as *mut u32 as *mut u8,
            &mut data_size,
        );
        RegCloseKey(hkey);

        if res != 0 || data_type != REG_DWORD {
            return GpoRuleConfigured::NotConfigured;
        }

        match data {
            1 => GpoRuleConfigured::Enabled,
            0 => GpoRuleConfigured::Disabled,
            _ => GpoRuleConfigured::NotConfigured,
        }
    }
}

powertoys_module_ffi::register_module!(CrosshairsModule::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creates() {
        let module = CrosshairsModule::new();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_module_name_and_key() {
        let module = CrosshairsModule::new();
        let name = wide::wide_to_string(module.get_name());
        let key = wide::wide_to_string(module.get_key());
        assert_eq!(name, "MousePointerCrosshairs");
        assert_eq!(key, "MousePointerCrosshairs");
    }

    #[test]
    fn test_enable_disable_state() {
        let mut module = CrosshairsModule::new();
        assert!(!module.is_enabled());
        module.enabled.store(true, Ordering::SeqCst);
        assert!(module.is_enabled());
        module.enabled.store(false, Ordering::SeqCst);
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_set_null_config_no_crash() {
        let mut module = CrosshairsModule::new();
        module.set_config(std::ptr::null());
    }

    #[test]
    fn test_destroy_disables() {
        let mut module = CrosshairsModule::new();
        module.enabled.store(true, Ordering::SeqCst);
        module.destroy();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_gpo_default() {
        let module = CrosshairsModule::new();
        let gpo = module.gpo_policy_enabled_configuration();
        assert_eq!(gpo, GpoRuleConfigured::NotConfigured);
    }
}
