//! C FFI exports for consumption by C++ (via header) and C# (via P/Invoke).
//!
//! All strings cross the boundary as null-terminated UTF-16 (`*const u16` /
//! `*mut u16`) matching the `wchar_t*` convention used throughout PowerToys.

use crate::settings::{self, Settings};
use std::path::PathBuf;

// ── Helpers ─────────────────────────────────────────────────────────

/// Convert a null-terminated UTF-16 pointer to a Rust `String`.
///
/// # Safety
/// The pointer must be non-null and point to a valid null-terminated UTF-16
/// string.
unsafe fn utf16_ptr_to_string(ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let mut len = 0usize;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf16(slice).ok()
}

/// Allocate a null-terminated UTF-16 string on the heap and return a pointer.
/// The caller must free it with `pt_string_free`.
fn string_to_utf16_ptr(s: &str) -> *mut u16 {
    let mut buf: Vec<u16> = s.encode_utf16().collect();
    buf.push(0); // null terminator
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Get the base directory for settings files (`%LOCALAPPDATA%`).
fn settings_base_dir() -> PathBuf {
    if let Ok(val) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(val)
    } else {
        // Fallback that should not happen on Windows.
        PathBuf::from(".")
    }
}

// ── FFI Exports ─────────────────────────────────────────────────────

/// Load settings for a module from disk.
///
/// Returns a heap-allocated `Settings` pointer, or null on failure.
/// The caller must free the returned pointer with `pt_settings_free`.
///
/// # Safety
/// `module_key` must be a valid null-terminated UTF-16 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_settings_load(module_key: *const u16) -> *mut Settings {
    let key = match unsafe { utf16_ptr_to_string(module_key) } {
        Some(k) => k,
        None => return std::ptr::null_mut(),
    };
    let base = settings_base_dir();
    match settings::load_from_settings_file(&base, &key) {
        Ok(s) => Box::into_raw(Box::new(s)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a `Settings` pointer previously returned by `pt_settings_load`.
///
/// # Safety
/// `settings` must be a pointer returned by `pt_settings_load` (or null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_settings_free(settings: *mut Settings) {
    if !settings.is_null() {
        drop(unsafe { Box::from_raw(settings) });
    }
}

/// Get a boolean property.
///
/// Returns 1 for true, 0 for false, -1 if the property does not exist.
///
/// # Safety
/// `s` must be a valid `Settings` pointer. `key` must be a valid
/// null-terminated UTF-16 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_settings_get_bool(s: *mut Settings, key: *const u16) -> i32 {
    let settings = match unsafe { s.as_ref() } {
        Some(r) => r,
        None => return -1,
    };
    let key_str = match unsafe { utf16_ptr_to_string(key) } {
        Some(k) => k,
        None => return -1,
    };
    match settings.get_bool(&key_str) {
        Some(true) => 1,
        Some(false) => 0,
        None => -1,
    }
}

/// Get an integer property.
///
/// Returns `i64::MIN` if the property does not exist.
///
/// # Safety
/// `s` must be a valid `Settings` pointer. `key` must be a valid
/// null-terminated UTF-16 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_settings_get_int(s: *mut Settings, key: *const u16) -> i64 {
    let settings = match unsafe { s.as_ref() } {
        Some(r) => r,
        None => return i64::MIN,
    };
    let key_str = match unsafe { utf16_ptr_to_string(key) } {
        Some(k) => k,
        None => return i64::MIN,
    };
    settings.get_int(&key_str).unwrap_or(i64::MIN)
}

/// Get a string property.
///
/// Returns a heap-allocated null-terminated UTF-16 string, or null if the
/// property does not exist. The caller must free it with `pt_string_free`.
///
/// # Safety
/// `s` must be a valid `Settings` pointer. `key` must be a valid
/// null-terminated UTF-16 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_settings_get_string(s: *mut Settings, key: *const u16) -> *mut u16 {
    let settings = match unsafe { s.as_ref() } {
        Some(r) => r,
        None => return std::ptr::null_mut(),
    };
    let key_str = match unsafe { utf16_ptr_to_string(key) } {
        Some(k) => k,
        None => return std::ptr::null_mut(),
    };
    match settings.get_string(&key_str) {
        Some(val) => string_to_utf16_ptr(&val),
        None => std::ptr::null_mut(),
    }
}

/// Set a boolean property.
///
/// # Safety
/// `s` must be a valid `Settings` pointer. `key` must be a valid
/// null-terminated UTF-16 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_settings_set_bool(s: *mut Settings, key: *const u16, val: i32) {
    let settings = match unsafe { s.as_mut() } {
        Some(r) => r,
        None => return,
    };
    let key_str = match unsafe { utf16_ptr_to_string(key) } {
        Some(k) => k,
        None => return,
    };
    settings.set_bool(&key_str, val != 0);
}

/// Save settings to disk.
///
/// Returns 0 on success, -1 on failure.
///
/// # Safety
/// `s` must be a valid `Settings` pointer. `module_key` must be a valid
/// null-terminated UTF-16 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_settings_save(s: *mut Settings, module_key: *const u16) -> i32 {
    let settings = match unsafe { s.as_ref() } {
        Some(r) => r,
        None => return -1,
    };
    let key = match unsafe { utf16_ptr_to_string(module_key) } {
        Some(k) => k,
        None => return -1,
    };
    // Create a temporary copy with the correct module_key for saving.
    let save_settings = Settings {
        doc: settings.doc.clone(),
        module_key: key,
    };
    let base = settings_base_dir();
    match settings::save_to_settings_file(&base, &save_settings) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Serialize settings to a JSON string.
///
/// Returns a heap-allocated null-terminated UTF-16 string. The caller must
/// free it with `pt_string_free`.
///
/// # Safety
/// `s` must be a valid `Settings` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_settings_to_json(s: *mut Settings) -> *mut u16 {
    let settings = match unsafe { s.as_ref() } {
        Some(r) => r,
        None => return std::ptr::null_mut(),
    };
    match settings.to_json() {
        Ok(json) => string_to_utf16_ptr(&json),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a UTF-16 string previously returned by FFI functions.
///
/// # Safety
/// `s` must be a pointer returned by `pt_settings_get_string` or
/// `pt_settings_to_json` (or null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_string_free(s: *mut u16) {
    if s.is_null() {
        return;
    }
    // Reconstruct the Vec to drop it properly.
    let mut len = 0usize;
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }
    // +1 for the null terminator that was part of the original Vec.
    let _ = unsafe { Vec::from_raw_parts(s, len + 1, len + 1) };
}

// ── FFI Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn make_utf16(s: &str) -> Vec<u16> {
        let mut v: Vec<u16> = s.encode_utf16().collect();
        v.push(0);
        v
    }

    #[test]
    fn utf16_ptr_roundtrip() {
        let original = "Hello, PowerToys!";
        let ptr = string_to_utf16_ptr(original);
        let recovered = unsafe { utf16_ptr_to_string(ptr) }.unwrap();
        assert_eq!(recovered, original);
        unsafe { pt_string_free(ptr) };
    }

    #[test]
    fn utf16_null_ptr_returns_none() {
        let result = unsafe { utf16_ptr_to_string(std::ptr::null()) };
        assert!(result.is_none());
    }

    #[test]
    fn ffi_load_get_set_save_free_cycle() {
        // Build a Settings in memory via the Rust API, then interact via FFI.
        let mut s = Settings::new("FFITest", "FFITest");
        s.set_bool("active", true);
        s.set_int("count", 10);
        s.set_string("label", "hello");

        let ptr = Box::into_raw(Box::new(s));

        let key_active = make_utf16("active");
        let key_count = make_utf16("count");
        let key_label = make_utf16("label");

        // get_bool
        assert_eq!(
            unsafe { pt_settings_get_bool(ptr, key_active.as_ptr()) },
            1
        );

        // get_int
        assert_eq!(
            unsafe { pt_settings_get_int(ptr, key_count.as_ptr()) },
            10
        );

        // get_string
        let str_ptr = unsafe { pt_settings_get_string(ptr, key_label.as_ptr()) };
        assert!(!str_ptr.is_null());
        let label = unsafe { utf16_ptr_to_string(str_ptr) }.unwrap();
        assert_eq!(label, "hello");
        unsafe { pt_string_free(str_ptr) };

        // set_bool
        unsafe { pt_settings_set_bool(ptr, key_active.as_ptr(), 0) };
        assert_eq!(
            unsafe { pt_settings_get_bool(ptr, key_active.as_ptr()) },
            0
        );

        // to_json
        let json_ptr = unsafe { pt_settings_to_json(ptr) };
        assert!(!json_ptr.is_null());
        let json_str = unsafe { utf16_ptr_to_string(json_ptr) }.unwrap();
        assert!(json_str.contains("\"FFITest\""));
        unsafe { pt_string_free(json_ptr) };

        // free
        unsafe { pt_settings_free(ptr) };
    }

    #[test]
    fn ffi_null_settings_returns_sentinel() {
        let key = make_utf16("anything");
        assert_eq!(
            unsafe { pt_settings_get_bool(std::ptr::null_mut(), key.as_ptr()) },
            -1
        );
        assert_eq!(
            unsafe { pt_settings_get_int(std::ptr::null_mut(), key.as_ptr()) },
            i64::MIN
        );
        assert!(unsafe { pt_settings_get_string(std::ptr::null_mut(), key.as_ptr()) }.is_null());
    }

    #[test]
    fn ffi_null_key_returns_sentinel() {
        let mut s = Settings::new("M", "M");
        let ptr = &mut s as *mut Settings;
        assert_eq!(
            unsafe { pt_settings_get_bool(ptr, std::ptr::null()) },
            -1
        );
        assert_eq!(
            unsafe { pt_settings_get_int(ptr, std::ptr::null()) },
            i64::MIN
        );
        assert!(unsafe { pt_settings_get_string(ptr, std::ptr::null()) }.is_null());
    }

    #[test]
    fn pt_string_free_null_is_safe() {
        unsafe { pt_string_free(std::ptr::null_mut()) };
    }

    #[test]
    fn pt_settings_free_null_is_safe() {
        unsafe { pt_settings_free(std::ptr::null_mut()) };
    }
}
