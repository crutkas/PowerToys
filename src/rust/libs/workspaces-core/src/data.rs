//! Workspace data structures.
//! Ported from WorkspacesLib/WorkspacesData.h

use serde::{Deserialize, Serialize};

/// Window position (x, y, width, height).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Position {
    /// Convert to a (left, top, right, bottom) rect.
    pub fn to_rect(&self) -> Rect {
        Rect {
            left: self.x,
            top: self.y,
            right: self.x + self.width,
            bottom: self.y + self.height,
        }
    }
}

/// Simple rectangle (left, top, right, bottom).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// An application within a workspace.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Application {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub path: String,
    #[serde(default, rename = "packageFullName")]
    pub package_full_name: String,
    #[serde(default, rename = "appUserModelId")]
    pub app_user_model_id: String,
    #[serde(default, rename = "pwaAppId")]
    pub pwa_app_id: String,
    #[serde(default, rename = "commandLineArgs")]
    pub command_line_args: String,
    #[serde(default, rename = "isElevated")]
    pub is_elevated: bool,
    #[serde(default, rename = "canLaunchElevated")]
    pub can_launch_elevated: bool,
    #[serde(default, rename = "isMinimized")]
    pub is_minimized: bool,
    #[serde(default, rename = "isMaximized")]
    pub is_maximized: bool,
    #[serde(default)]
    pub position: Position,
    #[serde(default)]
    pub monitor: u32,
}

/// A workspace project (collection of apps with positions).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "creationTime")]
    pub creation_time: String,
    #[serde(default, rename = "lastLaunchedTime")]
    pub last_launched_time: String,
    #[serde(default, rename = "isShortcutNeeded")]
    pub is_shortcut_needed: bool,
    #[serde(default, rename = "moveExistingWindows")]
    pub move_existing_windows: bool,
    #[serde(default)]
    pub apps: Vec<Application>,
}

/// File path helpers — parameterized for testability.
pub fn workspaces_file(base_dir: &str) -> String {
    format!("{}\\workspaces.json", base_dir)
}

pub fn temp_workspaces_file(base_dir: &str) -> String {
    format!("{}\\temp-workspaces.json", base_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Position_ToRect tests (3 from C++) ---

    #[test]
    fn position_to_rect_converts_correctly() {
        let pos = Position { x: 100, y: 200, width: 800, height: 600 };
        let rect = pos.to_rect();
        assert_eq!(rect.left, 100);
        assert_eq!(rect.top, 200);
        assert_eq!(rect.right, 900);  // x + width
        assert_eq!(rect.bottom, 800); // y + height
    }

    #[test]
    fn position_to_rect_zero() {
        let pos = Position::default();
        let rect = pos.to_rect();
        assert_eq!(rect.left, 0);
        assert_eq!(rect.top, 0);
        assert_eq!(rect.right, 0);
        assert_eq!(rect.bottom, 0);
    }

    #[test]
    fn position_to_rect_negative_coordinates() {
        let pos = Position { x: -100, y: -50, width: 200, height: 150 };
        let rect = pos.to_rect();
        assert_eq!(rect.left, -100);
        assert_eq!(rect.top, -50);
        assert_eq!(rect.right, 100);  // -100 + 200
        assert_eq!(rect.bottom, 100); // -50 + 150
    }

    // --- Application default values (1 from C++) ---

    #[test]
    fn application_default_values() {
        let app = Application::default();
        assert!(app.id.is_empty());
        assert!(app.name.is_empty());
        assert!(app.title.is_empty());
        assert!(app.path.is_empty());
        assert!(app.package_full_name.is_empty());
        assert!(app.app_user_model_id.is_empty());
        assert!(app.pwa_app_id.is_empty());
        assert!(app.command_line_args.is_empty());
        assert!(!app.is_elevated);
        assert!(!app.can_launch_elevated);
        assert!(!app.is_minimized);
        assert!(!app.is_maximized);
        assert_eq!(app.position.x, 0);
        assert_eq!(app.position.y, 0);
        assert_eq!(app.position.width, 0);
        assert_eq!(app.position.height, 0);
        assert_eq!(app.monitor, 0);
    }

    // --- Application comparison (2 from C++) ---

    #[test]
    fn application_comparison_equal() {
        let app1 = Application {
            id: "test-id".into(),
            name: "Test App".into(),
            position: Position { x: 100, y: 200, ..Default::default() },
            ..Default::default()
        };
        let app2 = app1.clone();
        assert_eq!(app1, app2);
    }

    #[test]
    fn application_comparison_different() {
        let app1 = Application { id: "test-id-1".into(), name: "Test App 1".into(), ..Default::default() };
        let app2 = Application { id: "test-id-2".into(), name: "Test App 2".into(), ..Default::default() };
        assert_ne!(app1, app2);
    }

    // --- Position comparison (2 from C++) ---

    #[test]
    fn position_comparison_equal() {
        let pos1 = Position { x: 100, y: 200, width: 800, height: 600 };
        let pos2 = Position { x: 100, y: 200, width: 800, height: 600 };
        assert_eq!(pos1, pos2);
    }

    #[test]
    fn position_comparison_different() {
        let pos1 = Position { x: 100, y: 200, width: 800, height: 600 };
        let pos2 = Position { x: 150, y: 200, width: 800, height: 600 };
        assert_ne!(pos1, pos2);
    }

    // --- File path tests (3 from C++) ---

    #[test]
    fn workspaces_file_returns_valid_path() {
        let path = workspaces_file(r"C:\test");
        assert!(path.contains("workspaces.json"));
    }

    #[test]
    fn temp_workspaces_file_returns_valid_path() {
        let path = temp_workspaces_file(r"C:\test");
        assert!(path.contains("temp-workspaces.json"));
    }

    #[test]
    fn workspaces_file_and_temp_are_different() {
        let base = r"C:\test";
        assert_ne!(workspaces_file(base), temp_workspaces_file(base));
    }
}
