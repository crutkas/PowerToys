//! Mouse Highlighter Module Interface — Rust implementation
//!
//! This is the PowerToys module DLL for Mouse Highlighter. It implements the
//! `PowerToyModule` trait from `powertoys-module-ffi` and manages the
//! overlay window, mouse hook, and highlight rendering.
//!
//! Architecture:
//! - `enable()` starts the highlighter (hook + overlay)
//! - `disable()` stops it
//! - Settings are loaded from JSON matching the C++ format
//! - Core highlight logic is in `highlighter-core` (platform-independent)
//! - Win32/D2D rendering lives here (platform-specific)

use powertoys_module_ffi::*;
use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

mod wide;

use highlighter_core::settings::load_settings_from_json;
use highlighter_core::types::Settings;

pub struct HighlighterModule {
    enabled: AtomicBool,
    settings: Settings,
}

impl HighlighterModule {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            settings: Settings::default(),
        }
    }
}

impl PowerToyModule for HighlighterModule {
    fn get_name(&self) -> *const u16 {
        wide::module_name_wide_ptr()
    }

    fn get_key(&self) -> *const u16 {
        wide::module_name_wide_ptr()
    }

    fn enable(&mut self) {
        if !self.enabled.load(Ordering::SeqCst) {
            self.enabled.store(true, Ordering::SeqCst);
            // TODO: Start overlay window + mouse hook on a dedicated thread
        }
    }

    fn disable(&mut self) {
        if self.enabled.load(Ordering::SeqCst) {
            self.enabled.store(false, Ordering::SeqCst);
            // TODO: Stop overlay window + mouse hook
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
        self.settings = load_settings_from_json(&s);
    }

    fn destroy(&mut self) {
        self.disable();
    }

    fn gpo_policy_enabled_configuration(&self) -> GpoRuleConfigured {
        check_gpo_for_highlighter()
    }
}

fn check_gpo_for_highlighter() -> GpoRuleConfigured {
    use windows_sys::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    let subkey: Vec<u16> = "SOFTWARE\\Policies\\PowerToys"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let value_name: Vec<u16> = "EnableMouseHighlighter"
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

// Register this module — generates rust_module_create / rust_module_destroy exports
powertoys_module_ffi::register_module!(HighlighterModule::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creates() {
        let module = HighlighterModule::new();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_module_name_and_key() {
        let module = HighlighterModule::new();
        let name = wide::wide_to_string(module.get_name());
        let key = wide::wide_to_string(module.get_key());
        assert_eq!(name, "MouseHighlighter");
        assert_eq!(key, "MouseHighlighter");
    }

    #[test]
    fn test_enable_disable_state() {
        let mut module = HighlighterModule::new();
        assert!(!module.is_enabled());

        module.enable();
        assert!(module.is_enabled());

        module.disable();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_config_roundtrip() {
        let mut module = HighlighterModule::new();

        // Get needed buffer size
        let mut size: c_int = 0;
        module.get_config(std::ptr::null_mut(), &mut size);
        assert!(size > 0);

        // Fill buffer
        let mut buffer = vec![0u16; size as usize];
        let mut buf_size = size;
        let ok = module.get_config(buffer.as_mut_ptr(), &mut buf_size);
        assert!(ok);

        // Set it back (should not crash)
        module.set_config(buffer.as_ptr());
    }

    #[test]
    fn test_set_null_config_no_crash() {
        let mut module = HighlighterModule::new();
        module.set_config(std::ptr::null());
    }

    #[test]
    fn test_destroy_disables() {
        let mut module = HighlighterModule::new();
        module.enable();
        assert!(module.is_enabled());
        module.destroy();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_gpo_default() {
        let module = HighlighterModule::new();
        let gpo = module.gpo_policy_enabled_configuration();
        assert_eq!(gpo, GpoRuleConfigured::NotConfigured);
    }

    #[test]
    fn test_register_macro_exports() {
        let table_ptr = rust_module_create();
        assert!(!table_ptr.is_null());

        unsafe {
            let table = &*table_ptr;
            assert!(!table.context.is_null());

            let name = (table.get_name)(table.context);
            let name_str = wide::wide_to_string(name);
            assert_eq!(name_str, "MouseHighlighter");

            (table.destroy)(table.context);
            let _ = Box::from_raw(table_ptr);
        }
    }
}
