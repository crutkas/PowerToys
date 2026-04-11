//! UTF-16 wide-string helpers for the Crosshairs module.

use std::sync::OnceLock;

static MODULE_NAME_WIDE: OnceLock<Vec<u16>> = OnceLock::new();

/// Module display name: "MousePointerCrosshairs"
pub fn module_name_wide_ptr() -> *const u16 {
    MODULE_NAME_WIDE
        .get_or_init(|| to_wide("MousePointerCrosshairs"))
        .as_ptr()
}

pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

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
    fn name_roundtrip() {
        let s = wide_to_string(module_name_wide_ptr());
        assert_eq!(s, "MousePointerCrosshairs");
    }
}
