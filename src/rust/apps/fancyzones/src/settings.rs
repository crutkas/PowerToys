//! Settings and layout data loading for FancyZones.
//!
//! Delegates to `fancyzones_engine::snap::load_all()` which reads:
//!   - settings.json (via powertoys_win32::settings)
//!   - applied-layouts.json
//!   - custom-layouts.json
//!   - default-layouts.json
//!   - layout-hotkeys.json
//!   - layout-templates.json
//!
//! This module re-exports the loader and provides a convenience helper for
//! detecting on-disk changes so the app can reload.

use std::path::PathBuf;
use std::time::SystemTime;

/// Files that should be watched for changes.
const WATCHED_FILES: &[&str] = &[
    "settings.json",
    "applied-layouts.json",
    "custom-layouts.json",
    "default-layouts.json",
    "layout-hotkeys.json",
    "layout-templates.json",
];

/// Snapshot of file modification times used for simple change detection.
pub struct SettingsWatcher {
    dir: Option<PathBuf>,
    stamps: Vec<Option<SystemTime>>,
}

impl SettingsWatcher {
    pub fn new() -> Self {
        let dir = powertoys_win32::settings::module_dir("FancyZones");
        let stamps = Self::read_stamps(&dir);
        Self { dir, stamps }
    }

    /// Returns `true` if any watched file has changed since last check.
    pub fn has_changed(&mut self) -> bool {
        let current = Self::read_stamps(&self.dir);
        if current != self.stamps {
            self.stamps = current;
            return true;
        }
        false
    }

    fn read_stamps(dir: &Option<PathBuf>) -> Vec<Option<SystemTime>> {
        let Some(dir) = dir else {
            return vec![None; WATCHED_FILES.len()];
        };
        WATCHED_FILES
            .iter()
            .map(|name| {
                std::fs::metadata(dir.join(name))
                    .and_then(|m| m.modified())
                    .ok()
            })
            .collect()
    }
}
