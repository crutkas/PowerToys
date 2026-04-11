//! Wide string (UTF-16) utilities for Win32 interop.

use std::sync::LazyLock;

/// Static wide string for "Measure Tool" — initialized once, lives forever.
pub static MEASURE_TOOL_WIDE: LazyLock<&'static [u16]> = LazyLock::new(|| {
    Box::leak(to_wide("Measure Tool").into_boxed_slice())
});

/// Get pointer to the static "Measure Tool" wide string.
pub fn measure_tool_wide_ptr() -> *const u16 {
    MEASURE_TOOL_WIDE.as_ptr()
}

/// Convert a `&str` to a null-terminated UTF-16 `Vec<u16>`.
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Convert a null-terminated wide string pointer to a Rust `String`.
///
/// # Safety
/// The pointer must be valid and null-terminated.
pub fn wide_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe {
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_wide() {
        let w = to_wide("Hello");
        assert_eq!(w, vec![72, 101, 108, 108, 111, 0]);
    }

    #[test]
    fn test_wide_to_string() {
        let w = to_wide("Measure Tool");
        let s = wide_to_string(w.as_ptr());
        assert_eq!(s, "Measure Tool");
    }

    #[test]
    fn test_wide_roundtrip() {
        let original = "MeasureTool Screen Ruler 📏";
        let wide = to_wide(original);
        let back = wide_to_string(wide.as_ptr());
        assert_eq!(back, original);
    }

    #[test]
    fn test_null_pointer() {
        let s = wide_to_string(std::ptr::null());
        assert_eq!(s, "");
    }

    #[test]
    fn test_empty_string() {
        let w = to_wide("");
        assert_eq!(w, vec![0]);
        let s = wide_to_string(w.as_ptr());
        assert_eq!(s, "");
    }
}
