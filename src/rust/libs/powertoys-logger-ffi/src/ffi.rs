//! C FFI exports for the PowerToys logger.
//!
//! All string parameters are null-terminated UTF-16 (`*const u16`) for
//! seamless interop with `wchar_t*` on Windows.

use crate::logger;

// ---------------------------------------------------------------------------
// Wide-string helper
// ---------------------------------------------------------------------------

/// Convert a null-terminated UTF-16 pointer to a Rust `String`.
///
/// Returns an empty string when `ptr` is null.
///
/// # Safety
///
/// `ptr` must be null **or** point to a valid, null-terminated UTF-16 sequence.
unsafe fn wstr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(ptr, len) })
}

// ---------------------------------------------------------------------------
// Exported functions
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn pt_log_init(module: *const u16, path: *const u16, name: *const u16) {
    let module = unsafe { wstr_to_string(module) };
    let path = unsafe { wstr_to_string(path) };
    let name = unsafe { wstr_to_string(name) };
    let _ = logger::init_logger(&module, &path, &name);
}

#[unsafe(no_mangle)]
pub extern "C" fn pt_log_trace(msg: *const u16) {
    let msg = unsafe { wstr_to_string(msg) };
    logger::log_trace(&msg);
}

#[unsafe(no_mangle)]
pub extern "C" fn pt_log_debug(msg: *const u16) {
    let msg = unsafe { wstr_to_string(msg) };
    logger::log_debug(&msg);
}

#[unsafe(no_mangle)]
pub extern "C" fn pt_log_info(msg: *const u16) {
    let msg = unsafe { wstr_to_string(msg) };
    logger::log_info(&msg);
}

#[unsafe(no_mangle)]
pub extern "C" fn pt_log_warn(msg: *const u16) {
    let msg = unsafe { wstr_to_string(msg) };
    logger::log_warn(&msg);
}

#[unsafe(no_mangle)]
pub extern "C" fn pt_log_error(msg: *const u16) {
    let msg = unsafe { wstr_to_string(msg) };
    logger::log_error(&msg);
}

#[unsafe(no_mangle)]
pub extern "C" fn pt_log_flush() {
    logger::flush();
}
