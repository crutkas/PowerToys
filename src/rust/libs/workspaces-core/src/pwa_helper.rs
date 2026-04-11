//! PWA (Progressive Web App) helper.
//! Ported from WorkspacesLib/PwaHelper.h
//!
//! Detects Edge/Chrome PWAs by looking up app IDs in browser profiles.

/// PWA info extracted from a browser profile.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PwaInfo {
    pub app_id: String,
    pub name: String,
    pub browser: PwaBrowser,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PwaBrowser {
    #[default]
    Unknown,
    Edge,
    Chrome,
}

/// Search for a PWA by app_user_model_id in a list of known PWAs.
pub fn find_pwa_by_aumid<'a>(pwa_list: &'a [PwaInfo], aumid: &str) -> Option<&'a PwaInfo> {
    if aumid.is_empty() {
        return None;
    }
    pwa_list.iter().find(|p| p.app_id == aumid)
}

/// Search for a PWA by name (case-insensitive).
pub fn find_pwa_by_name<'a>(pwa_list: &'a [PwaInfo], name: &str) -> Option<&'a PwaInfo> {
    if name.is_empty() {
        return None;
    }
    pwa_list.iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

/// Determine browser type from install path.
pub fn browser_from_path(path: &str) -> PwaBrowser {
    let lower = path.to_lowercase();
    if lower.contains("msedge.exe") {
        PwaBrowser::Edge
    } else if lower.contains("chrome.exe") {
        PwaBrowser::Chrome
    } else {
        PwaBrowser::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pwa_list() -> Vec<PwaInfo> {
        vec![
            PwaInfo { app_id: "edge-pwa-123".into(), name: "Outlook".into(), browser: PwaBrowser::Edge },
            PwaInfo { app_id: "chrome-pwa-456".into(), name: "Gmail".into(), browser: PwaBrowser::Chrome },
            PwaInfo { app_id: "edge-pwa-789".into(), name: "Teams".into(), browser: PwaBrowser::Edge },
        ]
    }

    #[test]
    fn find_by_aumid_found() {
        let list = sample_pwa_list();
        let result = find_pwa_by_aumid(&list, "edge-pwa-123");
        assert_eq!(result.unwrap().name, "Outlook");
    }

    #[test]
    fn find_by_aumid_not_found() {
        let list = sample_pwa_list();
        assert!(find_pwa_by_aumid(&list, "nonexistent").is_none());
    }

    #[test]
    fn find_by_aumid_empty() {
        let list = sample_pwa_list();
        assert!(find_pwa_by_aumid(&list, "").is_none());
    }

    #[test]
    fn find_by_name_found() {
        let list = sample_pwa_list();
        let result = find_pwa_by_name(&list, "gmail");
        assert_eq!(result.unwrap().app_id, "chrome-pwa-456");
    }

    #[test]
    fn find_by_name_not_found() {
        let list = sample_pwa_list();
        assert!(find_pwa_by_name(&list, "Slack").is_none());
    }

    #[test]
    fn find_by_name_empty() {
        let list = sample_pwa_list();
        assert!(find_pwa_by_name(&list, "").is_none());
    }

    #[test]
    fn browser_from_path_detection() {
        assert_eq!(browser_from_path(r"C:\msedge.exe"), PwaBrowser::Edge);
        assert_eq!(browser_from_path(r"C:\chrome.exe"), PwaBrowser::Chrome);
        assert_eq!(browser_from_path(r"C:\firefox.exe"), PwaBrowser::Unknown);
    }
}
