//! App Zone History — persists which zones each application was last snapped to.
//!
//! Compatible with the C++ `app-zone-history.json` format:
//! ```json
//! {
//!   "app-zone-history": [
//!     {
//!       "app-path": "C:\\Windows\\notepad.exe",
//!       "history": [{ "zone-index-set": [0], "device-id": "key", "layout-id": "uuid" }]
//!     }
//!   ]
//! }
//! ```

use std::collections::HashMap;
use std::path::Path;

use fancyzones_core::zone::ZoneIndexSet;
use serde::{Deserialize, Serialize};

/// A single history entry for one device+layout combination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub device_key: String,
    pub layout_id: String,
    pub zone_index_set: ZoneIndexSet,
}

/// Tracks which zones each application was last assigned to.
pub struct AppZoneHistory {
    /// Key = lowercase exe path, Value = list of entries (one per device+layout pair).
    history: HashMap<String, Vec<HistoryEntry>>,
}

impl AppZoneHistory {
    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
        }
    }

    /// Load history from a JSON file on disk.
    pub fn load(path: &Path) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return Self::new(),
        };
        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return Self::new(),
        };
        Self::load_from_json(&json)
    }

    /// Parse from the C++ JSON format.
    pub fn load_from_json(json: &serde_json::Value) -> Self {
        let mut history = HashMap::new();
        if let Some(arr) = json.get("app-zone-history").and_then(|v| v.as_array()) {
            for item in arr {
                let app_path = item
                    .get("app-path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                if app_path.is_empty() {
                    continue;
                }
                if let Some(entries_arr) = item.get("history").and_then(|v| v.as_array()) {
                    let entries: Vec<HistoryEntry> = entries_arr
                        .iter()
                        .filter_map(|e| {
                            let layout_id =
                                e.get("layout-id").and_then(|v| v.as_str())?.to_string();
                            let device_key = e
                                .get("device-id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let zone_index_set: ZoneIndexSet = e
                                .get("zone-index-set")
                                .and_then(|v| v.as_array())
                                .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
                                .unwrap_or_default();
                            Some(HistoryEntry {
                                layout_id,
                                device_key,
                                zone_index_set,
                            })
                        })
                        .collect();
                    if !entries.is_empty() {
                        history.insert(app_path, entries);
                    }
                }
            }
        }
        Self { history }
    }

    /// Save history to a JSON file on disk.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = self.to_json();
        std::fs::write(
            path,
            serde_json::to_string_pretty(&json).unwrap_or_default(),
        )
    }

    /// Record a snap: overwrites any previous entry for the same app+device+layout.
    pub fn record(
        &mut self,
        app_path: &str,
        device_key: &str,
        layout_id: &str,
        zones: ZoneIndexSet,
    ) {
        let key = app_path.to_lowercase();
        let entries = self.history.entry(key).or_default();

        // Overwrite existing entry for same device+layout, or append.
        if let Some(existing) = entries
            .iter_mut()
            .find(|e| e.device_key == device_key && e.layout_id == layout_id)
        {
            existing.zone_index_set = zones;
        } else {
            entries.push(HistoryEntry {
                device_key: device_key.to_string(),
                layout_id: layout_id.to_string(),
                zone_index_set: zones,
            });
        }
    }

    /// Look up the most recent zone assignment for an app on a given device.
    /// Returns the zone indices if found.
    pub fn lookup(&self, app_path: &str, device_key: &str) -> Option<Vec<i64>> {
        let key = app_path.to_lowercase();
        let entries = self.history.get(&key)?;
        entries
            .iter()
            .rev()
            .find(|e| e.device_key == device_key)
            .map(|e| e.zone_index_set.clone())
    }

    /// Serialize to the C++ compatible JSON format.
    pub fn to_json(&self) -> serde_json::Value {
        let entries: Vec<serde_json::Value> = self
            .history
            .iter()
            .map(|(path, entries)| {
                let history: Vec<serde_json::Value> = entries
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "layout-id": e.layout_id,
                            "device-id": e.device_key,
                            "zone-index-set": e.zone_index_set,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "app-path": path,
                    "history": history,
                })
            })
            .collect();
        serde_json::json!({ "app-zone-history": entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let h = AppZoneHistory::new();
        assert!(h.lookup("anything.exe", "dev1").is_none());
    }

    #[test]
    fn record_and_lookup() {
        let mut h = AppZoneHistory::new();
        h.record("notepad.exe", "dev1", "layout1", vec![0, 1]);
        let zones = h.lookup("notepad.exe", "dev1");
        assert_eq!(zones, Some(vec![0, 1]));
    }

    #[test]
    fn record_overwrites_previous() {
        let mut h = AppZoneHistory::new();
        h.record("app.exe", "dev1", "layout1", vec![0]);
        h.record("app.exe", "dev1", "layout1", vec![2, 3]);
        let zones = h.lookup("app.exe", "dev1");
        assert_eq!(zones, Some(vec![2, 3]));
        // Should only have one entry, not two.
        assert_eq!(h.history.get("app.exe").unwrap().len(), 1);
    }

    #[test]
    fn record_different_devices_coexist() {
        let mut h = AppZoneHistory::new();
        h.record("app.exe", "dev1", "L1", vec![0]);
        h.record("app.exe", "dev2", "L1", vec![1]);
        assert_eq!(h.lookup("app.exe", "dev1"), Some(vec![0]));
        assert_eq!(h.lookup("app.exe", "dev2"), Some(vec![1]));
    }

    #[test]
    fn lookup_miss_returns_none() {
        let h = AppZoneHistory::new();
        assert!(h.lookup("missing.exe", "dev1").is_none());
    }

    #[test]
    fn lookup_wrong_device_returns_none() {
        let mut h = AppZoneHistory::new();
        h.record("app.exe", "dev1", "L1", vec![0]);
        assert!(h.lookup("app.exe", "dev999").is_none());
    }

    #[test]
    fn case_insensitive_app_path() {
        let mut h = AppZoneHistory::new();
        h.record("C:\\Windows\\Notepad.EXE", "dev1", "L1", vec![0]);
        let zones = h.lookup("c:\\windows\\notepad.exe", "dev1");
        assert_eq!(zones, Some(vec![0]));
    }

    #[test]
    fn save_load_roundtrip() {
        let mut h = AppZoneHistory::new();
        h.record("app1.exe", "dev1", "layout1", vec![0, 1]);
        h.record("app2.exe", "dev2", "layout2", vec![2]);

        // Roundtrip through JSON
        let json = h.to_json();
        let h2 = AppZoneHistory::load_from_json(&json);
        assert_eq!(h2.lookup("app1.exe", "dev1"), Some(vec![0, 1]));
        assert_eq!(h2.lookup("app2.exe", "dev2"), Some(vec![2]));
    }

    #[test]
    fn save_load_file_roundtrip() {
        let dir = std::env::current_dir().unwrap().join("test_app_history");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("app-zone-history-test.json");

        let mut h = AppZoneHistory::new();
        h.record("notepad.exe", "monitor1", "layout-abc", vec![0, 2]);
        h.save(&path).expect("save failed");

        let h2 = AppZoneHistory::load(&path);
        assert_eq!(h2.lookup("notepad.exe", "monitor1"), Some(vec![0, 2]));

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn load_from_cpp_format() {
        let json = serde_json::json!({
            "app-zone-history": [
                {
                    "app-path": "C:\\Windows\\notepad.exe",
                    "history": [
                        {
                            "zone-index-set": [0],
                            "device-id": "monitor_key",
                            "layout-id": "layout_uuid"
                        }
                    ]
                }
            ]
        });
        let h = AppZoneHistory::load_from_json(&json);
        let zones = h.lookup("c:\\windows\\notepad.exe", "monitor_key");
        assert_eq!(zones, Some(vec![0]));
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let h = AppZoneHistory::load(Path::new("nonexistent_file_12345.json"));
        assert!(h.lookup("any.exe", "dev").is_none());
    }
}
