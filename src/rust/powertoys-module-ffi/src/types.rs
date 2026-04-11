//! C-compatible types matching the PowerToys C++ definitions.
//!
//! These must match the memory layout of the corresponding C++ types
//! in `src/common/interop/PowertoyModuleIface.h`.

use std::ffi::c_int;

/// GPO rule configuration status.
/// Must match `powertoys_gpo::gpo_rule_configured_t` from C++.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpoRuleConfigured {
    /// GPO policy set to an unrecognized value.
    WrongValue = -3,
    /// Couldn't access registry.
    Unavailable = -2,
    /// Policy is not configured.
    NotConfigured = -1,
    /// Policy is disabled.
    Disabled = 0,
    /// Policy is enabled.
    Enabled = 1,
}

impl GpoRuleConfigured {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => GpoRuleConfigured::Enabled,
            0 => GpoRuleConfigured::Disabled,
            -1 => GpoRuleConfigured::NotConfigured,
            -2 => GpoRuleConfigured::Unavailable,
            _ => GpoRuleConfigured::WrongValue,
        }
    }
}

/// A keyboard hotkey definition.
/// Must match `PowertoyModuleIface::Hotkey` from the C++ header.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Hotkey {
    pub win: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: u8,      // unsigned char in C++
    pub id: i32,
    pub is_shown: bool,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            win: false,
            ctrl: false,
            shift: false,
            alt: false,
            key: 0,
            id: 0,
            is_shown: true, // matches C++ default
        }
    }
}

/// Extended hotkey with modifier mask.
/// Must match `PowertoyModuleIface::HotkeyEx` from the C++ header.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct HotkeyEx {
    pub modifiers_mask: u16, // WORD
    pub vk_code: u16,        // WORD
    pub id: i32,
}

/// The C function table that the C++ adapter delegates to.
/// Each function pointer corresponds to a PowertoyModuleIface virtual method.
#[repr(C)]
pub struct ModuleFunctionTable {
    pub context: *mut std::ffi::c_void,
    pub get_name: unsafe extern "C" fn(*mut std::ffi::c_void) -> *const u16,
    pub get_key: unsafe extern "C" fn(*mut std::ffi::c_void) -> *const u16,
    pub enable: unsafe extern "C" fn(*mut std::ffi::c_void),
    pub disable: unsafe extern "C" fn(*mut std::ffi::c_void),
    pub is_enabled: unsafe extern "C" fn(*mut std::ffi::c_void) -> bool,
    pub get_config: unsafe extern "C" fn(*mut std::ffi::c_void, *mut u16, *mut c_int) -> bool,
    pub set_config: unsafe extern "C" fn(*mut std::ffi::c_void, *const u16),
    pub call_custom_action: unsafe extern "C" fn(*mut std::ffi::c_void, *const u16),
    pub destroy: unsafe extern "C" fn(*mut std::ffi::c_void),
    pub get_hotkeys:
        unsafe extern "C" fn(*mut std::ffi::c_void, *mut Hotkey, usize) -> usize,
    pub on_hotkey: unsafe extern "C" fn(*mut std::ffi::c_void, usize) -> bool,
    pub is_enabled_by_default: unsafe extern "C" fn(*mut std::ffi::c_void) -> bool,
    pub keep_track_of_pressed_win_key: unsafe extern "C" fn(*mut std::ffi::c_void) -> bool,
    pub milliseconds_win_key_must_be_pressed: unsafe extern "C" fn(*mut std::ffi::c_void) -> u32,
    pub gpo_policy_enabled_configuration:
        unsafe extern "C" fn(*mut std::ffi::c_void) -> GpoRuleConfigured,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_gpo_enum_values() {
        assert_eq!(GpoRuleConfigured::WrongValue as i32, -3);
        assert_eq!(GpoRuleConfigured::Unavailable as i32, -2);
        assert_eq!(GpoRuleConfigured::NotConfigured as i32, -1);
        assert_eq!(GpoRuleConfigured::Disabled as i32, 0);
        assert_eq!(GpoRuleConfigured::Enabled as i32, 1);
    }

    #[test]
    fn test_gpo_roundtrip() {
        for val in [-3, -2, -1, 0, 1] {
            let gpo = GpoRuleConfigured::from_i32(val);
            assert_eq!(gpo as i32, val);
        }
    }

    #[test]
    fn test_hotkey_is_repr_c() {
        let size = mem::size_of::<Hotkey>();
        assert!(size > 0, "Hotkey should have non-zero size");
        // C++ layout: bool win(1) + bool ctrl(1) + bool shift(1) + bool alt(1) 
        //           + unsigned char key(1) + pad(3) + int id(4) + bool isShown(1) + pad(3) = 16
        eprintln!("Hotkey size: {} bytes", size);
        assert_eq!(size, 16, "Hotkey should be 16 bytes matching C++ layout");
    }

    #[test]
    fn test_hotkey_default() {
        let hk = Hotkey::default();
        assert!(!hk.win);
        assert!(!hk.ctrl);
        assert!(!hk.shift);
        assert!(!hk.alt);
        assert_eq!(hk.key, 0);
        assert_eq!(hk.id, 0);
        assert!(hk.is_shown);
    }

    #[test]
    fn test_module_function_table_is_repr_c() {
        let size = mem::size_of::<ModuleFunctionTable>();
        assert!(size > 0, "ModuleFunctionTable should have non-zero size");
    }
}
