//! Case-insensitive string comparison.
//! Ported from WorkspacesLib/StringUtils.h

/// Case-insensitive equality check (ASCII).
pub fn case_insensitive_equals(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_strings_returns_true() {
        assert!(case_insensitive_equals("test", "test"));
    }

    #[test]
    fn different_case_returns_true() {
        assert!(case_insensitive_equals("Test", "TEST"));
    }

    #[test]
    fn mixed_case_returns_true() {
        assert!(case_insensitive_equals("TeSt StRiNg", "test STRING"));
    }

    #[test]
    fn different_strings_returns_false() {
        assert!(!case_insensitive_equals("test", "different"));
    }

    #[test]
    fn different_lengths_returns_false() {
        assert!(!case_insensitive_equals("test", "testing"));
    }

    #[test]
    fn empty_strings_returns_true() {
        assert!(case_insensitive_equals("", ""));
    }

    #[test]
    fn one_empty_returns_false() {
        assert!(!case_insensitive_equals("test", ""));
    }

    #[test]
    fn special_characters_returns_true() {
        assert!(case_insensitive_equals("Test-123_Special!", "test-123_special!"));
    }
}
