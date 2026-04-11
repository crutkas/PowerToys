//! Workspace JSON serialization/deserialization.
//! Ported from WorkspacesLib/JsonUtils.h

use crate::data::Workspace;

/// Read workspaces from a JSON string.
pub fn read_workspaces(json: &str) -> Result<Vec<Workspace>, serde_json::Error> {
    serde_json::from_str(json)
}

/// Write workspaces to a JSON string.
pub fn write_workspaces(workspaces: &[Workspace]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(workspaces)
}

/// Read workspaces from a file path.
pub fn read_workspaces_file(path: &str) -> Result<Vec<Workspace>, JsonError> {
    let content = std::fs::read_to_string(path).map_err(JsonError::Io)?;
    read_workspaces(&content).map_err(JsonError::Parse)
}

/// Write workspaces to a file path.
pub fn write_workspaces_file(path: &str, workspaces: &[Workspace]) -> Result<(), JsonError> {
    let json = write_workspaces(workspaces).map_err(JsonError::Parse)?;
    std::fs::write(path, json).map_err(JsonError::Io)
}

#[derive(Debug)]
pub enum JsonError {
    Io(std::io::Error),
    Parse(serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Application, Position};

    #[test]
    fn read_empty_array() {
        let workspaces = read_workspaces("[]").unwrap();
        assert!(workspaces.is_empty());
    }

    #[test]
    fn read_invalid_json() {
        assert!(read_workspaces("not json").is_err());
    }

    #[test]
    fn read_single_workspace() {
        let json = r#"[{
            "id": "ws-1",
            "name": "Dev Setup",
            "creationTime": "2024-01-01",
            "apps": []
        }]"#;
        let workspaces = read_workspaces(json).unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].id, "ws-1");
        assert_eq!(workspaces[0].name, "Dev Setup");
    }

    #[test]
    fn read_multiple_workspaces() {
        let json = r#"[
            {"id": "ws-1", "name": "Work", "apps": []},
            {"id": "ws-2", "name": "Gaming", "apps": []}
        ]"#;
        let workspaces = read_workspaces(json).unwrap();
        assert_eq!(workspaces.len(), 2);
    }

    #[test]
    fn read_workspace_with_apps() {
        let json = r#"[{
            "id": "ws-1",
            "name": "Dev",
            "apps": [{
                "id": "app-1",
                "name": "Notepad",
                "path": "C:\\Windows\\notepad.exe",
                "position": {"x": 100, "y": 200, "width": 800, "height": 600},
                "monitor": 0
            }]
        }]"#;
        let workspaces = read_workspaces(json).unwrap();
        assert_eq!(workspaces[0].apps.len(), 1);
        assert_eq!(workspaces[0].apps[0].name, "Notepad");
        assert_eq!(workspaces[0].apps[0].position.x, 100);
    }

    #[test]
    fn write_and_read_roundtrip() {
        let workspaces = vec![Workspace {
            id: "ws-1".into(),
            name: "Test".into(),
            apps: vec![Application {
                id: "app-1".into(),
                name: "App".into(),
                position: Position { x: 10, y: 20, width: 300, height: 400 },
                ..Default::default()
            }],
            ..Default::default()
        }];
        let json = write_workspaces(&workspaces).unwrap();
        let parsed = read_workspaces(&json).unwrap();
        assert_eq!(workspaces, parsed);
    }

    #[test]
    fn write_empty_list() {
        let json = write_workspaces(&[]).unwrap();
        let parsed = read_workspaces(&json).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn read_file_nonexistent() {
        assert!(read_workspaces_file("nonexistent_file_12345.json").is_err());
    }
}
