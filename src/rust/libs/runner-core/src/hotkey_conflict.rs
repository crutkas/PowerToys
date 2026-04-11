use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Represents a keyboard hotkey with modifier keys and a virtual key code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hotkey {
    pub win: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: u8,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            win: false,
            ctrl: false,
            shift: false,
            alt: false,
            key: 0,
        }
    }
}

/// Associates a hotkey with its owning module and a unique ID within that module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HotkeyInfo {
    pub hotkey: Hotkey,
    pub module: String,
    pub id: i32,
}

/// The kind of conflict detected for a hotkey registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    NoConflict,
    InAppConflict,
    SystemConflict,
}

/// Tracks hotkey registrations and detects conflicts between modules.
///
/// Mirrors the C++ `HotkeyConflictManager` from `hotkey_conflict_detector.cpp`.
///
/// Three internal maps:
/// - `hotkey_map`            – successfully registered (no conflict) hotkeys
/// - `in_app_conflict_map`   – hotkeys that collide with another module
/// - `sys_conflict_map`      – hotkeys that collide with a system-level registration
/// - `disabled_hotkeys`      – hotkeys belonging to disabled modules
pub struct HotkeyConflictManager {
    hotkey_map: HashMap<u16, HotkeyInfo>,
    in_app_conflict_map: HashMap<u16, HashSet<HotkeyInfo>>,
    sys_conflict_map: HashMap<u16, HashSet<HotkeyInfo>>,
    disabled_hotkeys: HashMap<String, Vec<HotkeyInfo>>,
    /// Pluggable predicate that decides if a hotkey conflicts at the OS level.
    /// Defaults to always returning `false` (no OS registration available in
    /// pure Rust). Tests can inject a custom function.
    system_conflict_checker: Box<dyn Fn(&Hotkey) -> bool>,
}

impl Default for HotkeyConflictManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyConflictManager {
    pub fn new() -> Self {
        Self {
            hotkey_map: HashMap::new(),
            in_app_conflict_map: HashMap::new(),
            sys_conflict_map: HashMap::new(),
            disabled_hotkeys: HashMap::new(),
            system_conflict_checker: Box::new(|_| false),
        }
    }

    /// Create a manager with a custom system-conflict checker (useful for tests).
    pub fn with_system_checker<F: Fn(&Hotkey) -> bool + 'static>(checker: F) -> Self {
        Self {
            hotkey_map: HashMap::new(),
            in_app_conflict_map: HashMap::new(),
            sys_conflict_map: HashMap::new(),
            disabled_hotkeys: HashMap::new(),
            system_conflict_checker: Box::new(checker),
        }
    }

    /// Compute a 12-bit handle: bits [7:0] = key, bits [11:8] = win|ctrl|shift|alt.
    pub fn get_hotkey_handle(hotkey: &Hotkey) -> u16 {
        let mut handle: u16 = hotkey.key as u16;
        if hotkey.win {
            handle |= 1 << 8;
        }
        if hotkey.ctrl {
            handle |= 1 << 9;
        }
        if hotkey.shift {
            handle |= 1 << 10;
        }
        if hotkey.alt {
            handle |= 1 << 11;
        }
        handle
    }

    /// Check if registering `(hotkey, module, id)` would conflict.
    pub fn has_conflict(&self, hotkey: &Hotkey, module: &str, id: i32) -> ConflictType {
        // Disabled modules never conflict.
        if self.disabled_hotkeys.contains_key(module) {
            return ConflictType::NoConflict;
        }

        let handle = Self::get_hotkey_handle(hotkey);
        if handle == 0 {
            return ConflictType::NoConflict;
        }

        // System conflict takes priority.
        if self.sys_conflict_map.contains_key(&handle) {
            return ConflictType::SystemConflict;
        }

        if self.in_app_conflict_map.contains_key(&handle) {
            return ConflictType::InAppConflict;
        }

        if let Some(existing) = self.hotkey_map.get(&handle) {
            // Same module + same id → re-registration, not a conflict.
            if existing.module == module && existing.id == id {
                return ConflictType::NoConflict;
            }
            return ConflictType::InAppConflict;
        }

        if (self.system_conflict_checker)(hotkey) {
            return ConflictType::SystemConflict;
        }

        ConflictType::NoConflict
    }

    /// Register a hotkey. Returns `true` when it is placed in the main map
    /// (no conflict). Returns `false` when a conflict is detected.
    pub fn add_hotkey(&mut self, hotkey: &Hotkey, module: &str, id: i32, is_enabled: bool) -> bool {
        if !is_enabled {
            self.disabled_hotkeys
                .entry(module.to_string())
                .or_default()
                .push(HotkeyInfo {
                    hotkey: *hotkey,
                    module: module.to_string(),
                    id,
                });
            return true;
        }

        let handle = Self::get_hotkey_handle(hotkey);
        if handle == 0 {
            return false;
        }

        let conflict = self.has_conflict(hotkey, module, id);
        if conflict != ConflictType::NoConflict {
            let info = HotkeyInfo {
                hotkey: *hotkey,
                module: module.to_string(),
                id,
            };
            match conflict {
                ConflictType::InAppConflict => {
                    let set = self.in_app_conflict_map.entry(handle).or_default();
                    set.insert(info);
                    // Move the current main-map occupant into the conflict set.
                    if let Some(existing) = self.hotkey_map.remove(&handle) {
                        set.insert(existing);
                    }
                }
                ConflictType::SystemConflict => {
                    self.sys_conflict_map.entry(handle).or_default().insert(info);
                }
                ConflictType::NoConflict => unreachable!(),
            }
            return false;
        }

        self.hotkey_map.insert(
            handle,
            HotkeyInfo {
                hotkey: *hotkey,
                module: module.to_string(),
                id,
            },
        );
        true
    }

    /// Remove all hotkeys belonging to `module`. Returns the removed entries.
    /// When an in-app conflict set drops to a single entry, promote the
    /// survivor back into the main map.
    pub fn remove_by_module(&mut self, module: &str) -> Vec<HotkeyInfo> {
        let mut removed = Vec::new();

        // Remove from disabled.
        self.disabled_hotkeys.remove(module);

        // Remove from sys conflict map.
        self.sys_conflict_map.retain(|_handle, set| {
            set.retain(|info| {
                if info.module == module {
                    removed.push(info.clone());
                    false
                } else {
                    true
                }
            });
            !set.is_empty()
        });

        // Remove from in-app conflict map; promote lone survivors.
        let mut promote: Vec<(u16, HotkeyInfo)> = Vec::new();
        self.in_app_conflict_map.retain(|&handle, set| {
            set.retain(|info| {
                if info.module == module {
                    removed.push(info.clone());
                    false
                } else {
                    true
                }
            });
            if set.is_empty() {
                return false;
            }
            if set.len() == 1 {
                let survivor = set.iter().next().unwrap().clone();
                promote.push((handle, survivor));
                return false; // remove from conflict map
            }
            true
        });
        for (handle, info) in promote {
            self.hotkey_map.insert(handle, info);
        }

        // Remove from main hotkey map; check if remaining in-app conflicts
        // should be promoted.
        let mut promote2: Vec<(u16, HotkeyInfo)> = Vec::new();
        self.hotkey_map.retain(|&handle, info| {
            if info.module == module {
                removed.push(info.clone());
                if let Some(set) = self.in_app_conflict_map.get(&handle) {
                    if set.len() == 1 {
                        promote2.push((handle, set.iter().next().unwrap().clone()));
                    }
                }
                false
            } else {
                true
            }
        });
        for (handle, info) in promote2 {
            self.in_app_conflict_map.remove(&handle);
            self.hotkey_map.insert(handle, info);
        }

        removed
    }

    /// Disable a module: remove its hotkeys and stash them.
    pub fn disable_module(&mut self, module: &str) {
        let hotkeys = self.remove_by_module(module);
        self.disabled_hotkeys.insert(module.to_string(), hotkeys);
    }

    /// Re-enable a module: re-add its stashed hotkeys as enabled.
    pub fn enable_module(&mut self, module: &str) {
        let Some(hotkeys) = self.disabled_hotkeys.remove(module) else {
            return;
        };
        for info in hotkeys {
            self.add_hotkey(&info.hotkey, module, info.id, true);
        }
    }

    /// Return all entries that conflict with the given hotkey.
    pub fn get_all_conflicts(&self, hotkey: &Hotkey) -> Vec<HotkeyInfo> {
        let handle = Self::get_hotkey_handle(hotkey);

        // In-app conflicts first.
        if let Some(set) = self.in_app_conflict_map.get(&handle) {
            return set.iter().cloned().collect();
        }

        // System conflicts.
        if self.sys_conflict_map.contains_key(&handle) {
            return vec![HotkeyInfo {
                hotkey: *hotkey,
                module: "System".to_string(),
                id: 0,
            }];
        }

        // Main-map occupant.
        if let Some(info) = self.hotkey_map.get(&handle) {
            return vec![info.clone()];
        }

        // Fallback: if the system checker says conflict, report System.
        if (self.system_conflict_checker)(hotkey) {
            return vec![HotkeyInfo {
                hotkey: *hotkey,
                module: "System".to_string(),
                id: 0,
            }];
        }

        Vec::new()
    }

    /// Serialize the current conflict state to JSON.
    pub fn to_json(&self) -> serde_json::Value {
        let serialize_hotkey = |hk: &Hotkey| -> serde_json::Value {
            serde_json::json!({
                "win": hk.win,
                "ctrl": hk.ctrl,
                "shift": hk.shift,
                "alt": hk.alt,
                "key": hk.key,
            })
        };

        let mut in_app: Vec<serde_json::Value> = Vec::new();
        for set in self.in_app_conflict_map.values() {
            if set.is_empty() {
                continue;
            }
            let first = set.iter().next().unwrap();
            let modules: Vec<serde_json::Value> = set
                .iter()
                .map(|info| {
                    serde_json::json!({
                        "moduleName": info.module,
                        "hotkeyID": info.id,
                    })
                })
                .collect();
            in_app.push(serde_json::json!({
                "hotkey": serialize_hotkey(&first.hotkey),
                "modules": modules,
            }));
        }

        let mut sys: Vec<serde_json::Value> = Vec::new();
        for set in self.sys_conflict_map.values() {
            if set.is_empty() {
                continue;
            }
            let first = set.iter().next().unwrap();
            let modules: Vec<serde_json::Value> = set
                .iter()
                .map(|info| {
                    serde_json::json!({
                        "moduleName": info.module,
                        "hotkeyID": info.id,
                    })
                })
                .collect();
            sys.push(serde_json::json!({
                "hotkey": serialize_hotkey(&first.hotkey),
                "modules": modules,
            }));
        }

        serde_json::json!({
            "inAppConflicts": in_app,
            "sysConflicts": sys,
        })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn hotkey(win: bool, ctrl: bool, shift: bool, alt: bool, key: u8) -> Hotkey {
        Hotkey {
            win,
            ctrl,
            shift,
            alt,
            key,
        }
    }

    // 1. Same hotkey produces the same handle.
    #[test]
    fn get_hotkey_handle_same_hotkey_same_handle() {
        let hk = hotkey(true, false, false, false, 0x41); // Win+A
        assert_eq!(
            HotkeyConflictManager::get_hotkey_handle(&hk),
            HotkeyConflictManager::get_hotkey_handle(&hk),
        );
    }

    // 2. Different hotkeys produce different handles.
    #[test]
    fn get_hotkey_handle_different_hotkeys_different_handles() {
        let a = hotkey(true, false, false, false, 0x41); // Win+A
        let b = hotkey(false, true, false, false, 0x42); // Ctrl+B
        assert_ne!(
            HotkeyConflictManager::get_hotkey_handle(&a),
            HotkeyConflictManager::get_hotkey_handle(&b),
        );
    }

    // 3. No conflict when manager is empty.
    #[test]
    fn has_conflict_no_conflict_when_empty() {
        let mgr = HotkeyConflictManager::new();
        let hk = hotkey(true, false, false, false, 0x41);
        assert_eq!(mgr.has_conflict(&hk, "ModA", 1), ConflictType::NoConflict);
    }

    // 4. Conflict when same hotkey from a different module.
    #[test]
    fn has_conflict_same_hotkey_different_module() {
        let mut mgr = HotkeyConflictManager::new();
        let hk = hotkey(true, false, false, false, 0x41);
        mgr.add_hotkey(&hk, "ModA", 1, true);
        assert_eq!(
            mgr.has_conflict(&hk, "ModB", 1),
            ConflictType::InAppConflict,
        );
    }

    // 5. No conflict when same module re-registers.
    #[test]
    fn has_conflict_same_module_re_registers() {
        let mut mgr = HotkeyConflictManager::new();
        let hk = hotkey(true, false, false, false, 0x41);
        mgr.add_hotkey(&hk, "ModA", 1, true);
        assert_eq!(mgr.has_conflict(&hk, "ModA", 1), ConflictType::NoConflict);
    }

    // 6. add_hotkey returns true on success and tracks entry.
    #[test]
    fn add_hotkey_success_tracks_in_map() {
        let mut mgr = HotkeyConflictManager::new();
        let hk = hotkey(false, true, false, false, 0x42);
        assert!(mgr.add_hotkey(&hk, "ModA", 1, true));
        let handle = HotkeyConflictManager::get_hotkey_handle(&hk);
        assert!(mgr.hotkey_map.contains_key(&handle));
    }

    // 7. add_hotkey returns false on in-app conflict.
    #[test]
    fn add_hotkey_returns_false_on_conflict() {
        let mut mgr = HotkeyConflictManager::new();
        let hk = hotkey(false, true, false, false, 0x42);
        assert!(mgr.add_hotkey(&hk, "ModA", 1, true));
        assert!(!mgr.add_hotkey(&hk, "ModB", 2, true));
    }

    // 8. remove_by_module clears all hotkeys for a module.
    #[test]
    fn remove_by_module_clears_all() {
        let mut mgr = HotkeyConflictManager::new();
        let hk1 = hotkey(true, false, false, false, 0x41);
        let hk2 = hotkey(false, true, false, false, 0x42);
        mgr.add_hotkey(&hk1, "ModA", 1, true);
        mgr.add_hotkey(&hk2, "ModA", 2, true);
        let removed = mgr.remove_by_module("ModA");
        assert_eq!(removed.len(), 2);
        assert!(mgr.hotkey_map.is_empty());
    }

    // 9. remove_by_module promotes survivor from in-app conflict set.
    #[test]
    fn remove_by_module_promotes_survivor() {
        let mut mgr = HotkeyConflictManager::new();
        let hk = hotkey(true, false, false, false, 0x41);
        mgr.add_hotkey(&hk, "ModA", 1, true);
        mgr.add_hotkey(&hk, "ModB", 1, true); // conflict
        // Both should be in in_app_conflict_map now.
        let handle = HotkeyConflictManager::get_hotkey_handle(&hk);
        assert!(mgr.in_app_conflict_map.contains_key(&handle));

        mgr.remove_by_module("ModA");
        // ModB's entry should be promoted to main map.
        assert!(!mgr.in_app_conflict_map.contains_key(&handle));
        assert_eq!(mgr.hotkey_map.get(&handle).unwrap().module, "ModB");
    }

    // 10. disable_module moves hotkeys out, they no longer conflict.
    #[test]
    fn disable_module_no_longer_conflicts() {
        let mut mgr = HotkeyConflictManager::new();
        let hk = hotkey(true, false, false, false, 0x41);
        mgr.add_hotkey(&hk, "ModA", 1, true);
        mgr.disable_module("ModA");
        assert_eq!(mgr.has_conflict(&hk, "ModB", 1), ConflictType::NoConflict);
    }

    // 11. enable_module re-adds hotkeys and re-checks conflicts.
    #[test]
    fn enable_module_readds_hotkeys() {
        let mut mgr = HotkeyConflictManager::new();
        let hk = hotkey(true, false, false, false, 0x41);
        mgr.add_hotkey(&hk, "ModA", 1, true);
        mgr.disable_module("ModA");
        assert!(mgr.hotkey_map.is_empty());
        mgr.enable_module("ModA");
        let handle = HotkeyConflictManager::get_hotkey_handle(&hk);
        assert!(mgr.hotkey_map.contains_key(&handle));
    }

    // 12. get_all_conflicts returns all matching hotkeys.
    #[test]
    fn get_all_conflicts_returns_all() {
        let mut mgr = HotkeyConflictManager::new();
        let hk = hotkey(true, false, false, false, 0x41);
        mgr.add_hotkey(&hk, "ModA", 1, true);
        mgr.add_hotkey(&hk, "ModB", 1, true);
        mgr.add_hotkey(&hk, "ModC", 1, true);
        let conflicts = mgr.get_all_conflicts(&hk);
        assert_eq!(conflicts.len(), 3);
    }

    // 13. to_json has correct format.
    #[test]
    fn to_json_correct_format() {
        let mut mgr = HotkeyConflictManager::new();
        let hk = hotkey(true, false, false, false, 0x41);
        mgr.add_hotkey(&hk, "ModA", 1, true);
        mgr.add_hotkey(&hk, "ModB", 2, true); // in-app conflict
        let json = mgr.to_json();
        assert!(json["inAppConflicts"].is_array());
        assert!(json["sysConflicts"].is_array());
        let in_app = json["inAppConflicts"].as_array().unwrap();
        assert_eq!(in_app.len(), 1);
        assert!(in_app[0]["hotkey"].is_object());
        assert!(in_app[0]["modules"].is_array());
    }

    // 14. System conflict priority over in-app.
    #[test]
    fn system_conflict_priority() {
        // Create manager where system checker always says "conflict" for key 0x41.
        let mut mgr =
            HotkeyConflictManager::with_system_checker(|hk| hk.key == 0x41);
        let hk = hotkey(true, false, false, false, 0x41);
        // First add succeeds (system conflict not checked until has_conflict).
        // Actually the first add calls has_conflict internally which checks system.
        let result = mgr.add_hotkey(&hk, "ModA", 1, true);
        // The system conflict should be detected.
        assert!(!result);
        assert_eq!(
            mgr.has_conflict(&hk, "ModB", 1),
            ConflictType::SystemConflict,
        );
    }

    // 15. Multiple modules same hotkey: all tracked in conflict map.
    #[test]
    fn multiple_modules_same_hotkey_all_tracked() {
        let mut mgr = HotkeyConflictManager::new();
        let hk = hotkey(false, true, true, false, 0x50);
        mgr.add_hotkey(&hk, "A", 1, true);
        mgr.add_hotkey(&hk, "B", 1, true);
        mgr.add_hotkey(&hk, "C", 1, true);
        let handle = HotkeyConflictManager::get_hotkey_handle(&hk);
        let set = mgr.in_app_conflict_map.get(&handle).unwrap();
        assert_eq!(set.len(), 3);
    }

    // 16. Empty module name edge case.
    #[test]
    fn empty_module_name_edge_case() {
        let mut mgr = HotkeyConflictManager::new();
        let hk = hotkey(true, false, false, false, 0x41);
        assert!(mgr.add_hotkey(&hk, "", 1, true));
        assert_eq!(mgr.has_conflict(&hk, "", 1), ConflictType::NoConflict);
        assert_eq!(
            mgr.has_conflict(&hk, "Other", 1),
            ConflictType::InAppConflict,
        );
    }

    // 17. Handle with zero key returns 0 → treated as no-op.
    #[test]
    fn handle_zero_key_is_zero() {
        let hk = hotkey(false, false, false, false, 0);
        assert_eq!(HotkeyConflictManager::get_hotkey_handle(&hk), 0);
        let mut mgr = HotkeyConflictManager::new();
        assert!(!mgr.add_hotkey(&hk, "Mod", 1, true));
    }

    // 18. Many hotkeys do not overflow the handle.
    #[test]
    fn handle_overflow_with_many_hotkeys() {
        let mut mgr = HotkeyConflictManager::new();
        for key in 1u8..=255 {
            let hk = hotkey(true, true, true, true, key);
            mgr.add_hotkey(&hk, &format!("Mod{key}"), 1, true);
        }
        // All 255 hotkeys should be in the main map (unique handles).
        assert_eq!(mgr.hotkey_map.len(), 255);
    }
}
