//! App detection helpers.
//! Ported from WorkspacesLib/AppUtils.h

/// Application data for workspace capture/launch.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AppData {
    pub name: String,
    pub install_path: String,
    pub package_full_name: String,
    pub app_user_model_id: String,
    pub pwa_app_id: String,
    pub protocol_path: String,
    pub can_launch_elevated: bool,
}

impl AppData {
    /// Is this Microsoft Edge?
    pub fn is_edge(&self) -> bool {
        let lower = self.install_path.to_lowercase();
        lower.contains("msedge.exe")
    }

    /// Is this Google Chrome?
    pub fn is_chrome(&self) -> bool {
        let lower = self.install_path.to_lowercase();
        lower.contains("chrome.exe")
    }

    /// Is this a Steam game (steam:// protocol)?
    pub fn is_steam_game(&self) -> bool {
        self.protocol_path.starts_with("steam://")
    }

    /// Is this a browser (Edge or Chrome)?
    pub fn is_browser(&self) -> bool {
        self.is_edge() || self.is_chrome()
    }

    /// Classify what type of app this is.
    pub fn app_type(&self) -> AppType {
        if self.is_steam_game() {
            AppType::SteamGame
        } else if !self.package_full_name.is_empty() {
            AppType::Packaged
        } else if self.is_edge() {
            AppType::Edge
        } else if self.is_chrome() {
            AppType::Chrome
        } else {
            AppType::Win32
        }
    }
}

/// Classification of app types for launch strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppType {
    Win32,
    Packaged,
    Edge,
    Chrome,
    SteamGame,
}

/// Build command line arguments for launching an app.
pub fn build_launch_args(app: &AppData, command_line_args: &str) -> LaunchCommand {
    match app.app_type() {
        AppType::SteamGame => LaunchCommand {
            executable: "steam://".to_string(),
            args: app.protocol_path.clone(),
            use_shell_execute: true,
        },
        AppType::Packaged => LaunchCommand {
            executable: app.app_user_model_id.clone(),
            args: command_line_args.to_string(),
            use_shell_execute: true,
        },
        AppType::Edge | AppType::Chrome => {
            let mut args = String::new();
            if !app.pwa_app_id.is_empty() {
                args.push_str(&format!("--app-id={}", app.pwa_app_id));
            }
            if !command_line_args.is_empty() {
                if !args.is_empty() {
                    args.push(' ');
                }
                args.push_str(command_line_args);
            }
            LaunchCommand {
                executable: app.install_path.clone(),
                args,
                use_shell_execute: false,
            }
        }
        AppType::Win32 => LaunchCommand {
            executable: app.install_path.clone(),
            args: command_line_args.to_string(),
            use_shell_execute: false,
        },
    }
}

/// Command to launch an application.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchCommand {
    pub executable: String,
    pub args: String,
    pub use_shell_execute: bool,
}

/// Get the folder name as uppercase (for case-insensitive path matching).
pub fn to_upper(s: &str) -> String {
    s.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- IsEdge tests (3 from C++) ---

    #[test]
    fn is_edge_edge_path_returns_true() {
        let app = AppData {
            install_path: r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".into(),
            ..Default::default()
        };
        assert!(app.is_edge());
    }

    #[test]
    fn is_edge_non_edge_path_returns_false() {
        let app = AppData {
            install_path: r"C:\Program Files\Google\Chrome\Application\chrome.exe".into(),
            ..Default::default()
        };
        assert!(!app.is_edge());
    }

    #[test]
    fn is_edge_empty_path_returns_false() {
        let app = AppData::default();
        assert!(!app.is_edge());
    }

    // --- IsChrome tests (3 from C++) ---

    #[test]
    fn is_chrome_chrome_path_returns_true() {
        let app = AppData {
            install_path: r"C:\Program Files\Google\Chrome\Application\chrome.exe".into(),
            ..Default::default()
        };
        assert!(app.is_chrome());
    }

    #[test]
    fn is_chrome_non_chrome_path_returns_false() {
        let app = AppData {
            install_path: r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".into(),
            ..Default::default()
        };
        assert!(!app.is_chrome());
    }

    #[test]
    fn is_chrome_empty_path_returns_false() {
        let app = AppData::default();
        assert!(!app.is_chrome());
    }

    // --- IsSteamGame tests (4 from C++) ---

    #[test]
    fn is_steam_game_steam_protocol_returns_true() {
        let app = AppData {
            protocol_path: "steam://run/123456".into(),
            ..Default::default()
        };
        assert!(app.is_steam_game());
    }

    #[test]
    fn is_steam_game_non_steam_returns_false() {
        let app = AppData {
            protocol_path: "https://example.com".into(),
            ..Default::default()
        };
        assert!(!app.is_steam_game());
    }

    #[test]
    fn is_steam_game_empty_returns_false() {
        let app = AppData::default();
        assert!(!app.is_steam_game());
    }

    #[test]
    fn is_steam_game_partial_steam_returns_false() {
        let app = AppData {
            protocol_path: "http://run/123456".into(),
            ..Default::default()
        };
        assert!(!app.is_steam_game());
    }

    // --- DefaultValues test (1 from C++) ---

    #[test]
    fn default_values() {
        let app = AppData::default();
        assert!(app.name.is_empty());
        assert!(app.install_path.is_empty());
        assert!(app.package_full_name.is_empty());
        assert!(app.app_user_model_id.is_empty());
        assert!(app.pwa_app_id.is_empty());
        assert!(app.protocol_path.is_empty());
        assert!(!app.can_launch_elevated);
    }

    // --- MultipleBrowserDetection test (1 from C++) ---

    #[test]
    fn multiple_browser_detection() {
        let edge = AppData {
            install_path: r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".into(),
            ..Default::default()
        };
        let chrome = AppData {
            install_path: r"C:\Program Files\Google\Chrome\Application\chrome.exe".into(),
            ..Default::default()
        };
        let other = AppData {
            install_path: r"C:\Program Files\Firefox\firefox.exe".into(),
            ..Default::default()
        };

        assert!(edge.is_edge());
        assert!(!edge.is_chrome());
        assert!(!edge.is_steam_game());

        assert!(!chrome.is_edge());
        assert!(chrome.is_chrome());
        assert!(!chrome.is_steam_game());

        assert!(!other.is_edge());
        assert!(!other.is_chrome());
        assert!(!other.is_steam_game());
    }

    // --- Folder helpers (4 from C++) ---

    #[test]
    fn to_upper_works() {
        let lower = "c:\\program files\\test";
        let upper = to_upper(lower);
        assert_eq!(upper, "C:\\PROGRAM FILES\\TEST");
    }

    #[test]
    fn to_upper_consistent() {
        let s = "Hello World";
        assert_eq!(to_upper(s), to_upper(s));
    }

    #[test]
    fn to_upper_preserves_length() {
        let s = "test string";
        assert_eq!(to_upper(s).len(), s.len());
    }

    #[test]
    fn to_upper_empty() {
        assert_eq!(to_upper(""), "");
    }

    // --- GAP: App type classification ---

    #[test]
    fn app_type_win32() {
        let app = AppData {
            install_path: r"C:\notepad.exe".into(),
            ..Default::default()
        };
        assert_eq!(app.app_type(), AppType::Win32);
    }

    #[test]
    fn app_type_packaged() {
        let app = AppData {
            package_full_name: "Microsoft.WindowsCalculator_8wekyb3d8bbwe".into(),
            ..Default::default()
        };
        assert_eq!(app.app_type(), AppType::Packaged);
    }

    #[test]
    fn app_type_edge() {
        let app = AppData {
            install_path: r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".into(),
            ..Default::default()
        };
        assert_eq!(app.app_type(), AppType::Edge);
    }

    #[test]
    fn app_type_steam() {
        let app = AppData {
            protocol_path: "steam://run/440".into(),
            ..Default::default()
        };
        assert_eq!(app.app_type(), AppType::SteamGame);
    }

    // --- GAP: Launch command building ---

    #[test]
    fn launch_cmd_win32() {
        let app = AppData {
            install_path: r"C:\notepad.exe".into(),
            ..Default::default()
        };
        let cmd = build_launch_args(&app, "--file test.txt");
        assert_eq!(cmd.executable, r"C:\notepad.exe");
        assert_eq!(cmd.args, "--file test.txt");
        assert!(!cmd.use_shell_execute);
    }

    #[test]
    fn launch_cmd_edge_pwa() {
        let app = AppData {
            install_path: r"C:\msedge.exe".into(),
            pwa_app_id: "abcdef123".into(),
            ..Default::default()
        };
        let cmd = build_launch_args(&app, "");
        assert!(cmd.args.contains("--app-id=abcdef123"));
        assert!(!cmd.use_shell_execute);
    }

    #[test]
    fn launch_cmd_steam() {
        let app = AppData {
            protocol_path: "steam://run/440".into(),
            ..Default::default()
        };
        let cmd = build_launch_args(&app, "");
        assert!(cmd.use_shell_execute);
        assert_eq!(cmd.args, "steam://run/440");
    }
}
