//! C FFI exports for runner-core — callable from C++ runner.

use crate::hotkey_conflict::HotkeyConflictManager;
use crate::settings::{GeneralSettings, parse_settings_json};
use std::ffi::c_void;

// --- Hotkey Conflict Manager ---

#[unsafe(no_mangle)]
pub extern "C" fn pt_hotkey_manager_new() -> *mut c_void {
    let mgr = Box::new(HotkeyConflictManager::new());
    Box::into_raw(mgr) as *mut c_void
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_hotkey_manager_free(mgr: *mut c_void) {
    if !mgr.is_null() {
        unsafe { drop(Box::from_raw(mgr as *mut HotkeyConflictManager)); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_hotkey_manager_has_conflict(
    mgr: *mut c_void, win: bool, ctrl: bool, shift: bool, alt: bool, key: u8,
    module: *const u16, module_len: usize, id: i32,
) -> i32 {
    if mgr.is_null() || module.is_null() { return 0; }
    let mgr = unsafe { &*(mgr as *const HotkeyConflictManager) };
    let hotkey = crate::hotkey_conflict::Hotkey { win, ctrl, shift, alt, key };
    let module_str = unsafe { String::from_utf16_lossy(std::slice::from_raw_parts(module, module_len)) };
    mgr.has_conflict(&hotkey, &module_str, id) as i32
}

// --- Settings ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_settings_parse(json: *const u16, json_len: usize) -> *mut c_void {
    if json.is_null() { return std::ptr::null_mut(); }
    let json_str = unsafe { String::from_utf16_lossy(std::slice::from_raw_parts(json, json_len)) };
    match parse_settings_json(&json_str) {
        Ok(settings) => Box::into_raw(Box::new(settings)) as *mut c_void,
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_settings_free(settings: *mut c_void) {
    if !settings.is_null() {
        unsafe { drop(Box::from_raw(settings as *mut GeneralSettings)); }
    }
}