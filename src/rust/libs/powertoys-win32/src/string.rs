//! UTF-8 ↔ UTF-16 wide string conversions.
//!
//! Every Rust module that calls Win32 APIs needs these. Previously duplicated
//! in alwaysontop, actionrunner, update, and every module interface crate.

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

/// Encode a UTF-8 `&str` as a null-terminated UTF-16 vector suitable for Win32 `*PCWSTR` parameters.
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Decode a null-terminated UTF-16 slice into a Rust `String`.
/// Stops at the first null or at the end of the slice.
pub fn from_wide(v: &[u16]) -> String {
    let len = v.iter().position(|&c| c == 0).unwrap_or(v.len());
    OsString::from_wide(&v[..len])
        .to_string_lossy()
        .into_owned()
}

/// Decode a UTF-16 slice (no null terminator expected) into a Rust `String`.
pub fn from_wide_no_null(v: &[u16]) -> String {
    OsString::from_wide(v).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_ascii() {
        let s = "Hello, PowerToys!";
        let wide = to_wide(s);
        assert_eq!(wide.last(), Some(&0)); // null terminated
        assert_eq!(from_wide(&wide), s);
    }

    #[test]
    fn round_trip_unicode() {
        let s = "日本語テスト 🦀";
        let wide = to_wide(s);
        assert_eq!(from_wide(&wide), s);
    }

    #[test]
    fn empty_string() {
        let wide = to_wide("");
        assert_eq!(wide, vec![0u16]);
        assert_eq!(from_wide(&wide), "");
    }

    #[test]
    fn from_wide_stops_at_null() {
        let data: Vec<u16> = vec![b'A' as u16, b'B' as u16, 0, b'C' as u16];
        assert_eq!(from_wide(&data), "AB");
    }

    #[test]
    fn from_wide_no_null_includes_all() {
        let data: Vec<u16> = vec![b'A' as u16, b'B' as u16, b'C' as u16];
        assert_eq!(from_wide_no_null(&data), "ABC");
    }

    #[test]
    fn to_wide_special_chars() {
        let s = "path\\to\\file with spaces & (parens)";
        let wide = to_wide(s);
        assert_eq!(from_wide(&wide), s);
    }
}
