/// Normalise a Windows path for comparison.
///
/// - Converts to lowercase (case-insensitive file system)
/// - Replaces forward slashes with backslashes
/// - Collapses consecutive separators
/// - Removes a trailing separator (unless the path is a root like `C:\`)
pub fn normalize_path(path: &str) -> String {
    let lower = path.to_lowercase();
    let mut result = String::with_capacity(lower.len());
    let mut prev_was_sep = false;

    for ch in lower.chars() {
        let is_sep = ch == '\\' || ch == '/';
        if is_sep {
            if !prev_was_sep {
                result.push('\\');
            }
            prev_was_sep = true;
        } else {
            result.push(ch);
            prev_was_sep = false;
        }
    }

    // Remove trailing backslash unless it's a root path (e.g. `c:\` or `\`)
    if result.len() > 1 && result.ends_with('\\') {
        // Keep if this is a drive root like `c:\`
        let bytes = result.as_bytes();
        let is_drive_root = bytes.len() == 3 && bytes[1] == b':';
        let is_bare_root = bytes.len() == 1;
        if !is_drive_root && !is_bare_root {
            result.pop();
        }
    }

    result
}

/// Case-insensitive path comparison after normalisation.
pub fn paths_match(a: &str, b: &str) -> bool {
    normalize_path(a) == normalize_path(b)
}

/// Returns `true` if `child` is a file/directory directly inside or nested
/// under `parent` (after normalisation).
pub fn is_subpath(parent: &str, child: &str) -> bool {
    let np = normalize_path(parent);
    let nc = normalize_path(child);

    if nc.len() <= np.len() {
        return false;
    }

    // The child must start with parent + backslash
    let prefix = if np.ends_with('\\') {
        np.clone()
    } else {
        format!("{np}\\")
    };

    nc.starts_with(&prefix)
}

/// A matcher that can check whether a handle's kernel path matches any of the
/// user-supplied target paths (files or directories).
pub struct PathMatcher {
    /// Normalised file paths to match exactly.
    file_targets: Vec<(String, String)>, // (normalised, original)
    /// Normalised directory paths to match as prefixes.
    dir_targets: Vec<(String, String)>, // (normalised, original)
}

impl PathMatcher {
    /// Create a new matcher.
    ///
    /// * `files` – exact file paths to match.
    /// * `dirs` – directory paths; any handle under a directory matches.
    ///
    /// All paths should already be converted to kernel-name form before being
    /// passed in, or both sides should use the same normalisation scheme.
    pub fn new(files: Vec<String>, dirs: Vec<String>) -> Self {
        let file_targets = files
            .into_iter()
            .map(|p| {
                let n = normalize_path(&p);
                (n, p)
            })
            .collect();
        let dir_targets = dirs
            .into_iter()
            .map(|p| {
                let n = normalize_path(&p);
                (n, p)
            })
            .collect();
        Self {
            file_targets,
            dir_targets,
        }
    }

    /// Check whether `kernel_path` matches any target.
    ///
    /// Returns the *original* (non-normalised) user path on match.
    pub fn matches(&self, kernel_path: &str) -> Option<String> {
        let normalised = normalize_path(kernel_path);

        // Exact file match
        for (target, original) in &self.file_targets {
            if normalised == *target {
                return Some(original.clone());
            }
        }

        // Directory prefix match
        for (target, original) in &self.dir_targets {
            let prefix = if target.ends_with('\\') {
                target.clone()
            } else {
                format!("{target}\\")
            };
            if normalised.starts_with(&prefix) || normalised == *target {
                return Some(original.clone());
            }
        }

        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lowercase() {
        assert_eq!(
            normalize_path(r"C:\Users\Test\File.TXT"),
            r"c:\users\test\file.txt"
        );
    }

    #[test]
    fn normalize_forward_slashes() {
        assert_eq!(normalize_path("C:/Users/Test"), r"c:\users\test");
    }

    #[test]
    fn normalize_collapses_double_separators() {
        assert_eq!(normalize_path(r"C:\\Users\\Test"), r"c:\users\test");
    }

    #[test]
    fn normalize_trailing_separator_removed() {
        assert_eq!(normalize_path(r"C:\Users\Test\"), r"c:\users\test");
    }

    #[test]
    fn normalize_drive_root_keeps_backslash() {
        assert_eq!(normalize_path(r"C:\"), r"c:\");
    }

    #[test]
    fn paths_match_case_insensitive() {
        assert!(paths_match(
            r"C:\Users\TEST\file.txt",
            r"c:\users\test\FILE.TXT"
        ));
    }

    #[test]
    fn paths_match_mixed_separators() {
        assert!(paths_match(r"C:\Users\Test", "C:/Users/Test"));
    }

    #[test]
    fn paths_no_match() {
        assert!(!paths_match(r"C:\Users\Test", r"C:\Users\Other"));
    }

    #[test]
    fn is_subpath_basic() {
        assert!(is_subpath(
            r"C:\Users\Test",
            r"C:\Users\Test\file.txt"
        ));
    }

    #[test]
    fn is_subpath_nested() {
        assert!(is_subpath(
            r"C:\Users\Test",
            r"C:\Users\Test\sub\deep\file.txt"
        ));
    }

    #[test]
    fn is_subpath_case_insensitive() {
        assert!(is_subpath(
            r"C:\USERS\TEST",
            r"c:\users\test\file.txt"
        ));
    }

    #[test]
    fn is_subpath_not_prefix_of_name() {
        // `C:\Users\TestExtra\file.txt` should NOT match parent `C:\Users\Test`
        assert!(!is_subpath(
            r"C:\Users\Test",
            r"C:\Users\TestExtra\file.txt"
        ));
    }

    #[test]
    fn is_subpath_same_path_not_sub() {
        assert!(!is_subpath(r"C:\Users\Test", r"C:\Users\Test"));
    }

    #[test]
    fn matcher_exact_file() {
        let m = PathMatcher::new(
            vec![r"\Device\HarddiskVolume2\file.txt".into()],
            vec![],
        );
        assert!(m.matches(r"\Device\HarddiskVolume2\file.txt").is_some());
        assert!(m.matches(r"\Device\HarddiskVolume2\other.txt").is_none());
    }

    #[test]
    fn matcher_dir_prefix() {
        let m = PathMatcher::new(
            vec![],
            vec![r"\Device\HarddiskVolume2\Users".into()],
        );
        assert!(m
            .matches(r"\Device\HarddiskVolume2\Users\test\file.txt")
            .is_some());
        assert!(m
            .matches(r"\Device\HarddiskVolume3\Users\test\file.txt")
            .is_none());
    }

    #[test]
    fn matcher_case_insensitive() {
        let m = PathMatcher::new(
            vec![r"\Device\HarddiskVolume2\FILE.TXT".into()],
            vec![],
        );
        assert!(m.matches(r"\Device\HarddiskVolume2\file.txt").is_some());
    }

    #[test]
    fn matcher_returns_original_path() {
        let original = r"\Device\HarddiskVolume2\MyFile.txt".to_string();
        let m = PathMatcher::new(vec![original.clone()], vec![]);
        let result = m.matches(r"\Device\HarddiskVolume2\myfile.txt");
        assert_eq!(result, Some(original));
    }

    #[test]
    fn normalize_empty_string() {
        assert_eq!(normalize_path(""), "");
    }

    #[test]
    fn normalize_kernel_path() {
        assert_eq!(
            normalize_path(r"\Device\HarddiskVolume2\Users\test"),
            r"\device\harddiskvolume2\users\test"
        );
    }
}
