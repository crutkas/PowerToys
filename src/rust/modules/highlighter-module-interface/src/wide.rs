//! Wide string (UTF-16) utilities for Win32 interop.

use std::sync::LazyLock;

/// Static wide string for "MouseHighlighter" — initialized once, lives forever.
pub static MODULE_NAME_WIDE: LazyLock<&'static [u16]> = LazyLock::new(|| {
    Box::leak(to_wide("MouseHighlighter").into_boxed_slice())
});

/// Get pointer to the static "MouseHighlighter" wide string.
pub fn module_name_wide_ptr() -> *const u16 {
    MODULE_NAME_WIDE.as_ptr()
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
    fn test_module_name() {
        let s = wide_to_string(module_name_wide_ptr());
        assert_eq!(s, "MouseHighlighter");
    }

    #[test]
    fn test_roundtrip() {
        let original = "Mouse Highlighter 🖱️";
        let wide = to_wide(original);
        let back = wide_to_string(wide.as_ptr());
        assert_eq!(back, original);
    }

    #[test]
    fn test_null_pointer() {
        assert_eq!(wide_to_string(std::ptr::null()), "");
    }
}
