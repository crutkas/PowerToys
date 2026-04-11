use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::zone::ZoneIndexSet;

// ---- Layout types ----

/// Layout type enum matching C++ ZoneSetLayoutType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZoneSetLayoutType {
    Blank,
    Focus,
    Columns,
    Rows,
    Grid,
    PriorityGrid,
    Custom,
}

impl ZoneSetLayoutType {
    pub fn to_string_repr(&self) -> &'static str {
        match self {
            Self::Blank => "blank",
            Self::Focus => "focus",
            Self::Columns => "columns",
            Self::Rows => "rows",
            Self::Grid => "grid",
            Self::PriorityGrid => "priority-grid",
            Self::Custom => "custom",
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "blank" => Some(Self::Blank),
            "focus" => Some(Self::Focus),
            "columns" => Some(Self::Columns),
            "rows" => Some(Self::Rows),
            "grid" => Some(Self::Grid),
            "priority-grid" => Some(Self::PriorityGrid),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// All non-custom layout types (for iteration in tests).
    pub fn template_types() -> &'static [Self] {
        &[Self::Focus, Self::Columns, Self::Rows, Self::Grid, Self::PriorityGrid]
    }

    /// All grid-based layout types.
    pub fn grid_types() -> &'static [Self] {
        &[Self::Columns, Self::Rows, Self::Grid, Self::PriorityGrid]
    }
}

// ---- Custom layout types ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustomLayoutType {
    Grid,
    Canvas,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasZoneRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasLayoutInfo {
    pub last_work_area_width: i32,
    pub last_work_area_height: i32,
    pub zones: Vec<CanvasZoneRect>,
    pub sensitivity_radius: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridLayoutInfo {
    pub rows: i32,
    pub columns: i32,
    pub rows_percents: Vec<i32>,
    pub columns_percents: Vec<i32>,
    pub cell_child_map: Vec<Vec<i32>>,
    pub show_spacing: bool,
    pub spacing: i32,
    pub sensitivity_radius: i32,
}

impl GridLayoutInfo {
    /// Create a minimal grid layout info (used for auto-generated grids).
    pub fn minimal(rows: i32, columns: i32) -> Self {
        Self {
            rows,
            columns,
            rows_percents: vec![0; rows as usize],
            columns_percents: vec![0; columns as usize],
            cell_child_map: vec![vec![0; columns as usize]; rows as usize],
            show_spacing: false,
            spacing: 0,
            sensitivity_radius: 0,
        }
    }

    pub fn zone_count(&self) -> i32 {
        let mut max_val = -1i32;
        for row in &self.cell_child_map {
            for &cell in row {
                if cell > max_val {
                    max_val = cell;
                }
            }
        }
        max_val + 1
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CustomLayoutInfo {
    Canvas(CanvasLayoutInfo),
    Grid(GridLayoutInfo),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomLayoutData {
    pub name: String,
    pub layout_type: CustomLayoutType,
    pub info: CustomLayoutInfo,
}

// ---- Layout defaults ----

pub mod defaults {
    pub const ZONE_COUNT: i32 = 3;
    pub const SHOW_SPACING: bool = true;
    pub const SPACING: i32 = 16;
    pub const SENSITIVITY_RADIUS: i32 = 20;
}

// ---- Layout data ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutData {
    pub uuid: String,
    pub layout_type: ZoneSetLayoutType,
    pub show_spacing: bool,
    pub spacing: i32,
    pub zone_count: i32,
    pub sensitivity_radius: i32,
}

impl Default for LayoutData {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            layout_type: ZoneSetLayoutType::PriorityGrid,
            show_spacing: defaults::SHOW_SPACING,
            spacing: defaults::SPACING,
            zone_count: defaults::ZONE_COUNT,
            sensitivity_radius: defaults::SENSITIVITY_RADIUS,
        }
    }
}

// ---- Device / Monitor / WorkArea IDs ----

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DeviceId {
    pub id: String,
    pub instance_id: String,
    pub number: i32,
}

impl DeviceId {
    pub fn is_default(&self) -> bool {
        self.id == "Default_Monitor"
    }
}

/// Comparison matching C++ operator==:
/// If id differs, not equal.
/// If instance_id differs, compare by number.
/// Otherwise equal.
impl DeviceId {
    pub fn eq_cpp(&self, other: &DeviceId) -> bool {
        if self.id != other.id {
            return false;
        }
        if self.instance_id != other.instance_id {
            return self.number == other.number;
        }
        true
    }
}

/// A monitor handle abstraction (since we don't have Win32 HMONITOR).
/// In tests, each monitor gets a unique ID.
pub type MonitorHandle = u64;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct MonitorId {
    pub monitor: MonitorHandle,
    pub device_id: DeviceId,
    pub serial_number: String,
}

/// Comparison matching C++ operator==:
/// If both monitors are non-zero, compare handles only.
/// Otherwise check serial numbers then device_id.
impl MonitorId {
    pub fn eq_cpp(&self, other: &MonitorId) -> bool {
        if self.monitor != 0 && other.monitor != 0 {
            return self.monitor == other.monitor;
        }
        if !self.serial_number.is_empty() && !other.serial_number.is_empty() {
            if self.serial_number != other.serial_number {
                return false;
            }
        }
        self.device_id.eq_cpp(&other.device_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct WorkAreaId {
    pub monitor_id: MonitorId,
    pub virtual_desktop_id: String,
}

/// Comparison matching C++ operator==.
impl WorkAreaId {
    pub fn eq_cpp(&self, other: &WorkAreaId) -> bool {
        self.virtual_desktop_id == other.virtual_desktop_id
            && self.monitor_id.eq_cpp(&other.monitor_id)
    }
}

// ---- ZoneSetData ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneSetData {
    pub uuid: String,
    pub layout_type: ZoneSetLayoutType,
}

// ---- DeviceInfoData ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfoData {
    pub active_zone_set: ZoneSetData,
    pub show_spacing: bool,
    pub spacing: i32,
    pub zone_count: i32,
    pub sensitivity_radius: i32,
}

// ---- AppZoneHistoryData ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppZoneHistoryData {
    pub layout_id: String,
    pub work_area_id: WorkAreaId,
    pub zone_index_set: ZoneIndexSet,
}

// ---- LayoutHotkeys ----

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayoutHotkeys {
    hotkeys: HashMap<i32, String>,
}

impl LayoutHotkeys {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_json(&mut self, json: &serde_json::Value) {
        self.hotkeys.clear();
        if let Some(arr) = json.get("layout-hotkeys").and_then(|v| v.as_array()) {
            for item in arr {
                let uuid = item.get("layout-id").and_then(|v| v.as_str()).unwrap_or("");
                let key = item.get("key").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
                if key >= 0 && !uuid.is_empty() {
                    self.hotkeys.insert(key, uuid.to_string());
                }
            }
        }
    }

    pub fn get_hotkeys_count(&self) -> usize {
        self.hotkeys.len()
    }

    pub fn get_layout_id(&self, key: i32) -> Option<&String> {
        self.hotkeys.get(&key)
    }
}

// ---- DefaultLayouts ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonitorConfigurationType {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DefaultLayouts {
    horizontal: Option<LayoutData>,
    vertical: Option<LayoutData>,
}

impl DefaultLayouts {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_json(&mut self, json: &serde_json::Value) {
        self.horizontal = None;
        self.vertical = None;

        if let Some(arr) = json.get("default-layouts").and_then(|v| v.as_array()) {
            for item in arr {
                let config_type = item.get("monitor-configuration-type").and_then(|v| v.as_str()).unwrap_or("");
                let layout_json = item.get("layout");

                if let Some(layout_val) = layout_json {
                    let layout_data = parse_layout_data(layout_val);
                    match config_type {
                        "horizontal" => self.horizontal = Some(layout_data),
                        "vertical" => self.vertical = Some(layout_data),
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn get_default_layout(&self, config: MonitorConfigurationType) -> LayoutData {
        match config {
            MonitorConfigurationType::Horizontal => {
                self.horizontal.clone().unwrap_or(LayoutData {
                    layout_type: ZoneSetLayoutType::PriorityGrid,
                    ..LayoutData::default()
                })
            }
            MonitorConfigurationType::Vertical => {
                self.vertical.clone().unwrap_or(LayoutData {
                    layout_type: ZoneSetLayoutType::Rows,
                    ..LayoutData::default()
                })
            }
        }
    }
}

fn parse_layout_data(val: &serde_json::Value) -> LayoutData {
    let type_str = val.get("type").and_then(|v| v.as_str()).unwrap_or("priority-grid");
    let layout_type = ZoneSetLayoutType::from_string(type_str).unwrap_or(ZoneSetLayoutType::PriorityGrid);
    let uuid = val.get("uuid").and_then(|v| v.as_str()).unwrap_or("").to_string();

    if layout_type == ZoneSetLayoutType::Custom {
        LayoutData {
            uuid,
            layout_type,
            ..LayoutData::default()
        }
    } else {
        LayoutData {
            uuid: String::new(),
            layout_type,
            show_spacing: val.get("show-spacing").and_then(|v| v.as_bool()).unwrap_or(defaults::SHOW_SPACING),
            spacing: val.get("spacing").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(defaults::SPACING),
            zone_count: val.get("zone-count").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(defaults::ZONE_COUNT),
            sensitivity_radius: val.get("sensitivity-radius").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(defaults::SENSITIVITY_RADIUS),
        }
    }
}

// ---- CustomLayouts store ----

#[derive(Debug, Clone, Default)]
pub struct CustomLayoutsStore {
    layouts: HashMap<String, CustomLayoutData>,
}

impl CustomLayoutsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_json(&mut self, json: &serde_json::Value) {
        self.layouts.clear();
        if let Some(arr) = json.get("custom-layouts").and_then(|v| v.as_array()) {
            for item in arr {
                if let Some((uuid, data)) = parse_custom_layout(item) {
                    self.layouts.insert(uuid, data);
                }
            }
        }
    }

    pub fn get_all_layouts(&self) -> &HashMap<String, CustomLayoutData> {
        &self.layouts
    }

    pub fn get_layout(&self, uuid: &str) -> Option<&CustomLayoutData> {
        self.layouts.get(uuid)
    }
}

fn parse_custom_layout(val: &serde_json::Value) -> Option<(String, CustomLayoutData)> {
    let uuid = val.get("uuid")?.as_str()?.to_string();
    let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let type_str = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let info_val = val.get("info")?;

    let (layout_type, info) = match type_str {
        "canvas" => {
            let width = info_val.get("ref-width").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let height = info_val.get("ref-height").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let zones = info_val.get("zones").and_then(|v| v.as_array()).map(|arr| {
                arr.iter().filter_map(|z| {
                    Some(CanvasZoneRect {
                        x: z.get("X")?.as_i64()? as i32,
                        y: z.get("Y")?.as_i64()? as i32,
                        width: z.get("width")?.as_i64()? as i32,
                        height: z.get("height")?.as_i64()? as i32,
                    })
                }).collect()
            }).unwrap_or_default();
            let sensitivity_radius = info_val.get("sensitivity-radius").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

            (CustomLayoutType::Canvas, CustomLayoutInfo::Canvas(CanvasLayoutInfo {
                last_work_area_width: width,
                last_work_area_height: height,
                zones,
                sensitivity_radius,
            }))
        }
        "grid" => {
            let rows = info_val.get("rows").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let columns = info_val.get("columns").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let rows_percents = info_val.get("rows-percentage").and_then(|v| v.as_array()).map(|a| {
                a.iter().filter_map(|v| v.as_i64().map(|n| n as i32)).collect()
            }).unwrap_or_default();
            let columns_percents = info_val.get("columns-percentage").and_then(|v| v.as_array()).map(|a| {
                a.iter().filter_map(|v| v.as_i64().map(|n| n as i32)).collect()
            }).unwrap_or_default();
            let cell_child_map = info_val.get("cell-child-map").and_then(|v| v.as_array()).map(|rows_arr| {
                rows_arr.iter().map(|row| {
                    row.as_array().map(|cells| {
                        cells.iter().filter_map(|v| v.as_i64().map(|n| n as i32)).collect()
                    }).unwrap_or_default()
                }).collect()
            }).unwrap_or_default();
            let show_spacing = info_val.get("show-spacing").and_then(|v| v.as_bool()).unwrap_or(false);
            let spacing = info_val.get("spacing").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let sensitivity_radius = info_val.get("sensitivity-radius").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

            (CustomLayoutType::Grid, CustomLayoutInfo::Grid(GridLayoutInfo {
                rows,
                columns,
                rows_percents,
                columns_percents,
                cell_child_map,
                show_spacing,
                spacing,
                sensitivity_radius,
            }))
        }
        _ => return None,
    };

    Some((uuid, CustomLayoutData { name, layout_type, info }))
}

// ---- AppliedLayouts ----

#[derive(Debug, Clone, Default)]
pub struct AppliedLayouts {
    layouts: HashMap<String, LayoutData>,
}

impl AppliedLayouts {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_json(&mut self, json: &serde_json::Value) {
        self.layouts.clear();
        if let Some(arr) = json.get("applied-layouts").and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(device_json) = item.get("device") {
                    let monitor_id = device_json.get("monitor").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let vd_id = device_json.get("virtual-desktop").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let key = format!("{}_{}", monitor_id, vd_id);

                    if let Some(layout_json) = item.get("applied-layout") {
                        let layout_data = parse_applied_layout(layout_json);
                        self.layouts.insert(key, layout_data);
                    }
                } else if let Some(device_id_str) = item.get("device-id").and_then(|v| v.as_str()) {
                    if let Some(layout_json) = item.get("applied-layout") {
                        let layout_data = parse_applied_layout(layout_json);
                        self.layouts.insert(device_id_str.to_string(), layout_data);
                    }
                }
            }
        }
    }

    pub fn get_applied_layout_map(&self) -> &HashMap<String, LayoutData> {
        &self.layouts
    }

    pub fn get_device_layout(&self, key: &str) -> Option<&LayoutData> {
        self.layouts.get(key)
    }

    pub fn is_layout_applied(&self, key: &str) -> bool {
        self.layouts.contains_key(key)
    }

    pub fn apply_layout(&mut self, key: String, layout: LayoutData) {
        self.layouts.insert(key, layout);
    }
}

fn parse_applied_layout(val: &serde_json::Value) -> LayoutData {
    let uuid = val.get("uuid").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let type_str = val.get("type").and_then(|v| v.as_str()).unwrap_or("priority-grid");
    let layout_type = ZoneSetLayoutType::from_string(type_str).unwrap_or(ZoneSetLayoutType::PriorityGrid);
    let show_spacing = val.get("show-spacing").and_then(|v| v.as_bool()).unwrap_or(defaults::SHOW_SPACING);
    let spacing = val.get("spacing").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(defaults::SPACING);
    let zone_count = val.get("zone-count").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(defaults::ZONE_COUNT);
    let sensitivity_radius = val.get("sensitivity-radius").and_then(|v| v.as_i64()).map(|v| v as i32).unwrap_or(defaults::SENSITIVITY_RADIUS);

    LayoutData {
        uuid,
        layout_type,
        show_spacing,
        spacing,
        zone_count,
        sensitivity_radius,
    }
}

// ---- Backwards compatibility: DeviceIdData ----

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceIdData {
    pub device_name: String,
    pub width: i32,
    pub height: i32,
    pub virtual_desktop_id: String,
    pub monitor_id: String,
}

impl DeviceIdData {
    /// Parse a device ID string matching the C++ logic:
    /// 1. If '#' found: name = everything up to '#' + '#' + part until next '_'
    /// 2. If '#' not found: name = first token before '_'
    /// 3. Remaining parts split by '_' should be: width, height, {GUID} [, monitorId]
    pub fn parse_device_id(input: &str) -> Option<Self> {
        let device_name;
        let remaining;

        if let Some(hash_pos) = input.find('#') {
            let before_hash = &input[..hash_pos];
            let after_hash = &input[hash_pos + 1..];
            let underscore_pos = after_hash.find('_')?;
            let after_hash_part = &after_hash[..underscore_pos];
            device_name = format!("{}#{}", before_hash, after_hash_part);
            remaining = &after_hash[underscore_pos + 1..];
        } else {
            let underscore_pos = input.find('_')?;
            device_name = input[..underscore_pos].to_string();
            if device_name.is_empty() {
                return None;
            }
            remaining = &input[underscore_pos + 1..];
        }

        // Split remaining by '_'
        let parts: Vec<&str> = remaining.split('_').collect();

        // Expected: width, height, {GUID-part1, GUID-part2, ...}
        // The GUID contains dashes and braces, so when split by '_', it stays as one piece
        // Actually the GUID is enclosed in {}, so it's: width_height_{GUID} or width_height_{GUID}_monitorId
        // But parts split by '_' would break the GUID if it has underscores... No, GUIDs don't have underscores.

        // We need at least 3 parts: width, height, guid
        if parts.len() < 3 {
            return None;
        }

        let width: i32 = parts[0].parse().ok()?;
        let height: i32 = parts[1].parse().ok()?;

        // Reconstruct the GUID - it might be split if there are extra underscores... 
        // Actually GUIDs look like {xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx} with no underscores
        let guid_str = parts[2];
        if !is_valid_guid(guid_str) {
            return None;
        }

        let monitor_id = if parts.len() > 3 {
            parts[3..].join("_")
        } else {
            String::new()
        };

        Some(DeviceIdData {
            device_name,
            width,
            height,
            virtual_desktop_id: guid_str.to_string(),
            monitor_id,
        })
    }

    /// Validate a device ID string using the C++ IsValidDeviceId logic:
    /// If '#' present: parse name with #, then 3 more parts = 4 total
    /// If '#' absent: split all by '_', expect exactly 4 parts
    pub fn is_valid_device_id(input: &str) -> bool {
        let mut parts: Vec<String> = Vec::new();
        let remaining;

        if let Some(hash_pos) = input.find('#') {
            let before_hash = &input[..hash_pos];
            let after_hash = &input[hash_pos + 1..];
            let Some(underscore_pos) = after_hash.find('_') else { return false };
            let after_hash_part = &after_hash[..underscore_pos];
            let monitor_name = format!("{}#{}", before_hash, after_hash_part);
            parts.push(monitor_name);
            remaining = &after_hash[underscore_pos + 1..];
        } else {
            remaining = input;
        }

        // Split remaining by '_'
        for part in remaining.split('_') {
            parts.push(part.to_string());
        }

        if parts.len() != 4 {
            return false;
        }

        // parts[0] = name, must be non-empty
        if parts[0].is_empty() {
            return false;
        }

        // parts[0] = name (may or may not have #)
        // parts[1] = width (numeric)
        // parts[2] = height (numeric)
        // parts[3] = GUID

        let width_idx = if input.contains('#') { 1 } else { 1 };
        let height_idx = width_idx + 1;
        let guid_idx = height_idx + 1;

        // Validate width and height are numeric
        if parts[width_idx].parse::<i32>().is_err() {
            return false;
        }
        if parts[height_idx].parse::<i32>().is_err() {
            return false;
        }

        // Validate GUID
        is_valid_guid(&parts[guid_idx])
    }
}

/// Simple GUID validation (checks format `{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}`).
pub fn is_valid_guid(s: &str) -> bool {
    if s.len() != 38 {
        return false;
    }
    let bytes = s.as_bytes();
    if bytes[0] != b'{' || bytes[37] != b'}' {
        return false;
    }
    // Check dash positions
    if bytes[9] != b'-' || bytes[14] != b'-' || bytes[19] != b'-' || bytes[24] != b'-' {
        return false;
    }
    // Check hex digits
    for (i, &b) in bytes.iter().enumerate() {
        if i == 0 || i == 37 || i == 9 || i == 14 || i == 19 || i == 24 {
            continue;
        }
        if !b.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

// ---- LayoutAssignedWindows ----

/// Tracks window-to-zone assignments (pure logic, uses u64 as window handle stand-in).
pub type WindowHandle = u64;

#[derive(Debug, Clone, Default)]
pub struct LayoutAssignedWindows {
    window_index_set: HashMap<WindowHandle, ZoneIndexSet>,
}

impl LayoutAssignedWindows {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn assign(&mut self, window: WindowHandle, zones: ZoneIndexSet) {
        self.window_index_set.insert(window, zones);
    }

    pub fn dismiss(&mut self, window: WindowHandle) {
        self.window_index_set.remove(&window);
    }

    pub fn get_zone_index_set_from_window(&self, window: WindowHandle) -> ZoneIndexSet {
        if window == 0 {
            return vec![];
        }
        self.window_index_set.get(&window).cloned().unwrap_or_default()
    }

    pub fn is_zone_empty(&self, zone_index: i64) -> bool {
        !self.window_index_set.values().any(|zones| zones.contains(&zone_index))
    }
}

// ---- LayoutTemplates ----

/// A layout template definition parsed from JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutTemplate {
    pub layout_type: ZoneSetLayoutType,
    pub show_spacing: bool,
    pub spacing: i32,
    pub zone_count: i32,
    pub sensitivity_radius: i32,
}

#[derive(Debug, Clone, Default)]
pub struct LayoutTemplatesStore {
    templates: Vec<LayoutTemplate>,
}

impl LayoutTemplatesStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_json(&mut self, json: &serde_json::Value) {
        self.templates.clear();
        if let Some(arr) = json.get("layout-templates").and_then(|v| v.as_array()) {
            for item in arr {
                let type_str = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let layout_type = ZoneSetLayoutType::from_string(type_str);
                if let Some(lt) = layout_type {
                    self.templates.push(LayoutTemplate {
                        layout_type: lt,
                        show_spacing: item.get("show-spacing").and_then(|v| v.as_bool()).unwrap_or(false),
                        spacing: item.get("spacing").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        zone_count: item.get("zone-count").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                        sensitivity_radius: item.get("sensitivity-radius").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    });
                }
            }
        }
    }

    pub fn get_templates(&self) -> &[LayoutTemplate] {
        &self.templates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- WorkAreaId comparison tests (from WorkAreaIdTests.Spec.cpp) ----

    static NEXT_MONITOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

    fn mock_monitor() -> MonitorHandle {
        NEXT_MONITOR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    #[test]
    fn monitor_handle_same() {
        let monitor = mock_monitor();
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor, device_id: DeviceId { id: "device-1".into(), instance_id: "instance-id-1".into(), number: 0 }, serial_number: "serial-number-1".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor, device_id: DeviceId { id: "device-2".into(), instance_id: "instance-id-2".into(), number: 0 }, serial_number: "serial-number-2".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(id1.eq_cpp(&id2));
    }

    #[test]
    fn monitor_handle_different() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: mock_monitor(), device_id: DeviceId { id: "device".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: mock_monitor(), device_id: DeviceId { id: "device".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(!id1.eq_cpp(&id2));
    }

    #[test]
    fn virtual_desktop_different() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "{F21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(!id1.eq_cpp(&id2));
    }

    #[test]
    fn virtual_desktop_null() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(!id1.eq_cpp(&id2));
    }

    #[test]
    fn different_serial_number() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "another-serial-number".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(!id1.eq_cpp(&id2));
    }

    #[test]
    fn default_monitor_id_different_instance_id_same_number() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "Default_Monitor".into(), instance_id: "instance-id".into(), number: 1 }, serial_number: "".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "Default_Monitor".into(), instance_id: "another-instance-id".into(), number: 1 }, serial_number: "".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(id1.eq_cpp(&id2));
    }

    #[test]
    fn default_monitor_id_different_instance_id_different_number() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "Default_Monitor".into(), instance_id: "instance-id".into(), number: 1 }, serial_number: "".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "Default_Monitor".into(), instance_id: "another-instance-id".into(), number: 2 }, serial_number: "".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(!id1.eq_cpp(&id2));
    }

    #[test]
    fn default_monitor_id_same_instance_id() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "Default_Monitor".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "Default_Monitor".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(id1.eq_cpp(&id2));
    }

    #[test]
    fn different_id() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device-1".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device-2".into(), instance_id: "instance-id".into(), number: 0 }, serial_number: "".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(!id1.eq_cpp(&id2));
    }

    #[test]
    fn same_id_different_serial_numbers() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device-1".into(), instance_id: "instance-id-1".into(), number: 0 }, serial_number: "serial-number-1".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device-1".into(), instance_id: "instance-id-2".into(), number: 0 }, serial_number: "serial-number-2".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(!id1.eq_cpp(&id2));
    }

    #[test]
    fn different_id_same_serial_numbers() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device-1".into(), instance_id: "instance-id-1".into(), number: 0 }, serial_number: "serial-number-1".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device-2".into(), instance_id: "instance-id-2".into(), number: 0 }, serial_number: "serial-number-1".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(!id1.eq_cpp(&id2));
    }

    #[test]
    fn monitor_reconnect() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "4&125707d6&0&UID1".into(), number: 1 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "4&125707d6&0&UID2".into(), number: 1 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(id1.eq_cpp(&id2));
    }

    #[test]
    fn same_monitor_models() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "4&125707d6&0&UID1".into(), number: 1 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "4&125707d6&0&UID2".into(), number: 2 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(!id1.eq_cpp(&id2));
    }

    #[test]
    fn serial_number_not_found_error() {
        let id1 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "instance-id".into(), number: 1 }, serial_number: "serial-number".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        let id2 = WorkAreaId {
            monitor_id: MonitorId { monitor: 0, device_id: DeviceId { id: "device".into(), instance_id: "instance-id".into(), number: 1 }, serial_number: "".into() },
            virtual_desktop_id: "{E21F6F29-76FD-4FC1-8970-17AB8AD64847}".into(),
        };
        assert!(id1.eq_cpp(&id2));
    }

    // ---- LayoutAssignedWindows tests (from LayoutAssignedWindows.Spec.cpp) ----

    static NEXT_WINDOW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(100);

    fn mock_window() -> WindowHandle {
        NEXT_WINDOW.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    #[test]
    fn zone_index_from_window_unknown() {
        let mut lw = LayoutAssignedWindows::new();
        lw.assign(mock_window(), vec![0]);
        let actual = lw.get_zone_index_set_from_window(mock_window());
        assert_eq!(actual, vec![] as ZoneIndexSet);
    }

    #[test]
    fn zone_index_from_window_null() {
        let mut lw = LayoutAssignedWindows::new();
        lw.assign(mock_window(), vec![0]);
        let actual = lw.get_zone_index_set_from_window(0); // 0 = null
        assert_eq!(actual, vec![] as ZoneIndexSet);
    }

    #[test]
    fn assign() {
        let window = mock_window();
        let mut lw = LayoutAssignedWindows::new();
        lw.assign(window, vec![1, 2, 3]);
        assert_eq!(lw.get_zone_index_set_from_window(window), vec![1, 2, 3]);
    }

    #[test]
    fn assign_empty() {
        let window = mock_window();
        let mut lw = LayoutAssignedWindows::new();
        lw.assign(window, vec![]);
        assert_eq!(lw.get_zone_index_set_from_window(window), vec![] as ZoneIndexSet);
    }

    #[test]
    fn assign_several_times_same_window() {
        let mut lw = LayoutAssignedWindows::new();
        let window = mock_window();

        lw.assign(window, vec![0]);
        assert_eq!(lw.get_zone_index_set_from_window(window), vec![0]);

        lw.assign(window, vec![1]);
        assert_eq!(lw.get_zone_index_set_from_window(window), vec![1]);

        lw.assign(window, vec![2]);
        assert_eq!(lw.get_zone_index_set_from_window(window), vec![2]);
    }

    #[test]
    fn dismiss_window() {
        let mut lw = LayoutAssignedWindows::new();
        let window = mock_window();
        lw.assign(window, vec![0]);
        lw.dismiss(window);
        assert_eq!(lw.get_zone_index_set_from_window(window), vec![] as ZoneIndexSet);
    }

    #[test]
    fn empty() {
        let mut lw = LayoutAssignedWindows::new();
        let window = mock_window();
        lw.assign(window, vec![0]);
        assert!(!lw.is_zone_empty(0));
        assert!(lw.is_zone_empty(1));
    }

    // ---- GUID validation tests (from JsonHelpers.Tests.cpp) ----

    #[test]
    fn guid_valid() {
        assert!(is_valid_guid("{33A2B101-06E0-437B-A61E-CDBECF502906}"));
    }

    #[test]
    fn guid_invalid_form() {
        assert!(!is_valid_guid("33A2B101-06E0-437B-A61E-CDBECF502906"));
    }

    #[test]
    fn guid_invalid_symbols() {
        assert!(!is_valid_guid("{33A2B101-06E0-437B-A61E-CDBECF50290*}"));
    }

    #[test]
    fn guid_invalid() {
        assert!(!is_valid_guid("guid"));
    }

    // ---- DeviceIdData validation tests (from JsonHelpers.Tests.cpp) ----

    #[test]
    fn device_id_valid() {
        assert!(DeviceIdData::is_valid_device_id("AOC2460#4&fe3a015&0&UID65793_1920_1200_{39B25DD2-130D-4B5D-8851-4791D66B1539}"));
    }

    #[test]
    fn device_id_without_hash_in_name() {
        assert!(DeviceIdData::is_valid_device_id("LOCALDISPLAY_5120_1440_{00000000-0000-0000-0000-000000000000}"));
    }

    #[test]
    fn device_id_without_hash_in_name_but_with_underscores() {
        // "LOCAL_DISPLAY" has extra underscore making ambiguous parse
        assert!(!DeviceIdData::is_valid_device_id("LOCAL_DISPLAY_5120_1440_{00000000-0000-0000-0000-000000000000}"));
    }

    #[test]
    fn device_id_with_underscores_in_name() {
        assert!(DeviceIdData::is_valid_device_id("Default_Monitor#1&1f0c3c2f&0&UID256_5120_1440_{00000000-0000-0000-0000-000000000000}"));
    }

    #[test]
    fn device_id_invalid_format() {
        assert!(!DeviceIdData::is_valid_device_id("_1920_1200_{39B25DD2-130D-4B5D-8851-4791D66B1539}"));
    }

    #[test]
    fn device_id_invalid_format2() {
        // missing underscore between width and height
        assert!(!DeviceIdData::is_valid_device_id("AOC2460#4&fe3a015&0&UID65793_19201200_{39B25DD2-130D-4B5D-8851-4791D66B1539}"));
    }

    #[test]
    fn device_id_invalid_decimals() {
        assert!(!DeviceIdData::is_valid_device_id("AOC2460#4&fe3a015&0&UID65793_aaaa_1200_{39B25DD2-130D-4B5D-8851-4791D66B1539}"));
    }

    #[test]
    fn device_id_invalid_decimals2() {
        assert!(!DeviceIdData::is_valid_device_id("AOC2460#4&fe3a015&0&UID65793_19a0_1200_{39B25DD2-130D-4B5D-8851-4791D66B1539}"));
    }

    // ---- DeviceIdData parsing tests (from Util.Spec.cpp) ----

    #[test]
    fn test_parse_device_id_01() {
        let input = "AOC0001#5&37ac4db&0&UID160002_1536_960_{E0A2904E-889C-4532-95B1-28FE15C16F66}";
        let actual = DeviceIdData::parse_device_id(input).unwrap();
        assert_eq!(actual.device_name, "AOC0001#5&37ac4db&0&UID160002");
        assert_eq!(actual.width, 1536);
        assert_eq!(actual.height, 960);
        assert_eq!(actual.virtual_desktop_id, "{E0A2904E-889C-4532-95B1-28FE15C16F66}");
        assert_eq!(actual.monitor_id, "");
    }

    #[test]
    fn test_parse_device_id_02() {
        let input = "AOC0001#5&37ac4db&0&UID160002_1536_960_{E0A2904E-889C-4532-95B1-28FE15C16F66}_monitorId";
        let actual = DeviceIdData::parse_device_id(input).unwrap();
        assert_eq!(actual.device_name, "AOC0001#5&37ac4db&0&UID160002");
        assert_eq!(actual.width, 1536);
        assert_eq!(actual.height, 960);
        assert_eq!(actual.virtual_desktop_id, "{E0A2904E-889C-4532-95B1-28FE15C16F66}");
        assert_eq!(actual.monitor_id, "monitorId");
    }

    #[test]
    fn test_parse_device_id_03() {
        let input = "AOC00015&37ac4db&0&UID160002_1536_960_{E0A2904E-889C-4532-95B1-28FE15C16F66}";
        let actual = DeviceIdData::parse_device_id(input).unwrap();
        assert_eq!(actual.device_name, "AOC00015&37ac4db&0&UID160002");
        assert_eq!(actual.width, 1536);
        assert_eq!(actual.height, 960);
    }

    #[test]
    fn test_parse_device_id_invalid_01() {
        let input = "AOC00015&37ac4db&0&UID160002_1536960_{E0A2904E-889C-4532-95B1-28FE15C16F66}";
        assert!(DeviceIdData::parse_device_id(input).is_none());
    }

    #[test]
    fn test_parse_device_id_invalid_02() {
        let input = "AOC00015&37ac4db&0&UID160002_{E0A2904E-889C-4532-95B1-28FE15C16F66}_monitorId";
        assert!(DeviceIdData::parse_device_id(input).is_none());
    }

    #[test]
    fn test_parse_device_id_invalid_03() {
        let input = "AOC00015&37ac4db&0&UID160002_1536960_";
        assert!(DeviceIdData::parse_device_id(input).is_none());
    }

    #[test]
    fn test_parse_device_id_invalid_04() {
        let input = "AOC00015&37ac4db&0&UID160002_1536960_{asdf}";
        assert!(DeviceIdData::parse_device_id(input).is_none());
    }

    #[test]
    fn test_parse_device_id_invalid_05() {
        let input = "AOC00015&37ac4db&0&UID160002_15a6_960_{E0A2904E-889C-4532-95B1-28FE15C16F66}";
        assert!(DeviceIdData::parse_device_id(input).is_none());
    }

    #[test]
    fn test_parse_device_id_invalid_06() {
        let input = "AOC00015&37ac4db&0&UID160002_15a6_960_monitorId_{E0A2904E-889C-4532-95B1-28FE15C16F66}";
        assert!(DeviceIdData::parse_device_id(input).is_none());
    }

    // ---- LayoutHotkeys tests (from LayoutHotkeysTests.Spec.cpp) ----

    #[test]
    fn layout_hotkeys_parse() {
        let json = serde_json::json!({
            "layout-hotkeys": [
                { "layout-id": "{33A2B101-06E0-437B-A61E-CDBECF502906}", "key": 1 },
                { "layout-id": "{33A2B101-06E0-437B-A61E-CDBECF502907}", "key": 2 },
            ]
        });
        let mut hotkeys = LayoutHotkeys::new();
        hotkeys.load_from_json(&json);
        assert_eq!(hotkeys.get_hotkeys_count(), 2);
        assert_eq!(hotkeys.get_layout_id(1).unwrap(), "{33A2B101-06E0-437B-A61E-CDBECF502906}");
        assert_eq!(hotkeys.get_layout_id(2).unwrap(), "{33A2B101-06E0-437B-A61E-CDBECF502907}");
    }

    #[test]
    fn layout_hotkeys_parse_empty() {
        let json = serde_json::json!({
            "layout-hotkeys": []
        });
        let mut hotkeys = LayoutHotkeys::new();
        hotkeys.load_from_json(&json);
        assert_eq!(hotkeys.get_hotkeys_count(), 0);
    }

    #[test]
    fn layout_hotkeys_no_file() {
        let json = serde_json::json!({});
        let mut hotkeys = LayoutHotkeys::new();
        hotkeys.load_from_json(&json);
        assert_eq!(hotkeys.get_hotkeys_count(), 0);
    }

    // ---- DefaultLayouts tests (from DefaultLayoutsTests.Spec.cpp) ----

    #[test]
    fn default_layouts_parse() {
        let json = serde_json::json!({
            "default-layouts": [
                {
                    "monitor-configuration-type": "horizontal",
                    "layout": {
                        "uuid": "{ACE817FD-2C51-4E13-903A-84CAB86FD17C}",
                        "type": "custom"
                    }
                },
                {
                    "monitor-configuration-type": "vertical",
                    "layout": {
                        "type": "grid",
                        "show-spacing": true,
                        "spacing": 1,
                        "zone-count": 4,
                        "sensitivity-radius": 30
                    }
                }
            ]
        });
        let mut dl = DefaultLayouts::new();
        dl.load_from_json(&json);

        let horizontal = dl.get_default_layout(MonitorConfigurationType::Horizontal);
        assert_eq!(horizontal.uuid, "{ACE817FD-2C51-4E13-903A-84CAB86FD17C}");
        assert_eq!(horizontal.layout_type, ZoneSetLayoutType::Custom);

        let vertical = dl.get_default_layout(MonitorConfigurationType::Vertical);
        assert_eq!(vertical.uuid, "");
        assert_eq!(vertical.layout_type, ZoneSetLayoutType::Grid);
        assert!(vertical.show_spacing);
        assert_eq!(vertical.spacing, 1);
        assert_eq!(vertical.zone_count, 4);
        assert_eq!(vertical.sensitivity_radius, 30);
    }

    #[test]
    fn default_layouts_parse_empty() {
        let json = serde_json::json!({
            "default-layouts": []
        });
        let mut dl = DefaultLayouts::new();
        dl.load_from_json(&json);

        let h = dl.get_default_layout(MonitorConfigurationType::Horizontal);
        assert_eq!(h.layout_type, ZoneSetLayoutType::PriorityGrid);
        assert_eq!(h.zone_count, defaults::ZONE_COUNT);

        let v = dl.get_default_layout(MonitorConfigurationType::Vertical);
        assert_eq!(v.layout_type, ZoneSetLayoutType::Rows);
        assert_eq!(v.zone_count, defaults::ZONE_COUNT);
    }

    #[test]
    fn default_layouts_no_file() {
        let json = serde_json::json!({});
        let mut dl = DefaultLayouts::new();
        dl.load_from_json(&json);

        let h = dl.get_default_layout(MonitorConfigurationType::Horizontal);
        assert_eq!(h.layout_type, ZoneSetLayoutType::PriorityGrid);

        let v = dl.get_default_layout(MonitorConfigurationType::Vertical);
        assert_eq!(v.layout_type, ZoneSetLayoutType::Rows);
    }

    // ---- CustomLayouts tests (from CustomLayoutsTests.Spec.cpp) ----

    #[test]
    fn custom_layouts_parse() {
        let json = serde_json::json!({
            "custom-layouts": [
                {
                    "uuid": "{ACE817FD-2C51-4E13-903A-84CAB86FD17C}",
                    "name": "Custom canvas layout",
                    "type": "canvas",
                    "info": {
                        "ref-width": 1920,
                        "ref-height": 1080,
                        "zones": [
                            { "X": 0, "Y": 0, "width": 1140, "height": 1040 },
                            { "X": 1140, "Y": 649, "width": 780, "height": 391 }
                        ]
                    }
                },
                {
                    "uuid": "{ACE817FD-2C51-4E13-903A-84CAB86FD17D}",
                    "name": "Custom grid layout",
                    "type": "grid",
                    "info": {
                        "rows": 2,
                        "columns": 3,
                        "rows-percentage": [5000, 5000],
                        "columns-percentage": [3333, 5000, 1667],
                        "cell-child-map": [[0, 1, 2], [3, 1, 4]],
                        "sensitivity-radius": 20,
                        "show-spacing": false,
                        "spacing": 16
                    }
                }
            ]
        });
        let mut store = CustomLayoutsStore::new();
        store.load_from_json(&json);
        assert_eq!(store.get_all_layouts().len(), 2);
        assert!(store.get_layout("{ACE817FD-2C51-4E13-903A-84CAB86FD17C}").is_some());
        assert!(store.get_layout("{ACE817FD-2C51-4E13-903A-84CAB86FD17D}").is_some());
    }

    #[test]
    fn custom_layouts_parse_empty() {
        let json = serde_json::json!({ "custom-layouts": [] });
        let mut store = CustomLayoutsStore::new();
        store.load_from_json(&json);
        assert!(store.get_all_layouts().is_empty());
    }

    #[test]
    fn custom_layouts_no_file() {
        let json = serde_json::json!({});
        let mut store = CustomLayoutsStore::new();
        store.load_from_json(&json);
        assert!(store.get_all_layouts().is_empty());
    }

    // ---- AppliedLayouts tests (from AppliedLayoutsTests.Spec.cpp) ----

    #[test]
    fn applied_layouts_parse() {
        let json = serde_json::json!({
            "applied-layouts": [
                {
                    "device": {
                        "monitor": "DELA026#5&10a58c63&0&UID16777488",
                        "virtual-desktop": "{61FA9FC0-26A6-4B37-A834-491C148DFC57}"
                    },
                    "applied-layout": {
                        "uuid": "{ACE817FD-2C51-4E13-903A-84CAB86FD17C}",
                        "type": "rows",
                        "show-spacing": true,
                        "spacing": 3,
                        "zone-count": 4,
                        "sensitivity-radius": 22
                    }
                }
            ]
        });
        let mut al = AppliedLayouts::new();
        al.load_from_json(&json);
        assert_eq!(al.get_applied_layout_map().len(), 1);
    }

    #[test]
    fn applied_layouts_parse_empty() {
        let json = serde_json::json!({ "applied-layouts": [] });
        let mut al = AppliedLayouts::new();
        al.load_from_json(&json);
        assert_eq!(al.get_applied_layout_map().len(), 0);
    }

    #[test]
    fn applied_layouts_no_file() {
        let json = serde_json::json!({});
        let mut al = AppliedLayouts::new();
        al.load_from_json(&json);
        assert_eq!(al.get_applied_layout_map().len(), 0);
    }

    #[test]
    fn applied_layouts_apply_layout() {
        let mut al = AppliedLayouts::new();
        let layout = LayoutData {
            uuid: "{ACE817FD-2C51-4E13-903A-84CAB86FD17C}".into(),
            layout_type: ZoneSetLayoutType::Grid,
            ..LayoutData::default()
        };
        al.apply_layout("test-key".into(), layout.clone());
        assert!(al.is_layout_applied("test-key"));
        assert_eq!(al.get_device_layout("test-key"), Some(&layout));
    }

    // ---- LayoutTemplates tests (from LayoutTemplatesTests.Spec.cpp) ----

    #[test]
    fn layout_templates_parse() {
        let json = serde_json::json!({
            "layout-templates": [
                { "type": "blank", "show-spacing": false, "spacing": 0, "zone-count": 0, "sensitivity-radius": 0 },
                { "type": "grid", "show-spacing": true, "spacing": -10, "zone-count": 4, "sensitivity-radius": 30 },
            ]
        });
        let mut store = LayoutTemplatesStore::new();
        store.load_from_json(&json);
        assert_eq!(store.get_templates().len(), 2);
        assert_eq!(store.get_templates()[0].layout_type, ZoneSetLayoutType::Blank);
        assert_eq!(store.get_templates()[1].layout_type, ZoneSetLayoutType::Grid);
        assert_eq!(store.get_templates()[1].zone_count, 4);
    }

    #[test]
    fn layout_templates_parse_empty() {
        let json = serde_json::json!({ "layout-templates": [] });
        let mut store = LayoutTemplatesStore::new();
        store.load_from_json(&json);
        assert!(store.get_templates().is_empty());
    }

    #[test]
    fn layout_templates_no_file() {
        let json = serde_json::json!({});
        let mut store = LayoutTemplatesStore::new();
        store.load_from_json(&json);
        assert!(store.get_templates().is_empty());
    }

    // ---- ZoneSetLayoutType string conversion tests (from JsonHelpers.Tests.cpp) ----

    #[test]
    fn zone_set_layout_type_to_string() {
        assert_eq!(ZoneSetLayoutType::Blank.to_string_repr(), "blank");
        assert_eq!(ZoneSetLayoutType::Focus.to_string_repr(), "focus");
        assert_eq!(ZoneSetLayoutType::Columns.to_string_repr(), "columns");
        assert_eq!(ZoneSetLayoutType::Rows.to_string_repr(), "rows");
        assert_eq!(ZoneSetLayoutType::Grid.to_string_repr(), "grid");
        assert_eq!(ZoneSetLayoutType::PriorityGrid.to_string_repr(), "priority-grid");
        assert_eq!(ZoneSetLayoutType::Custom.to_string_repr(), "custom");
    }

    #[test]
    fn zone_set_layout_type_from_string() {
        assert_eq!(ZoneSetLayoutType::from_string("blank"), Some(ZoneSetLayoutType::Blank));
        assert_eq!(ZoneSetLayoutType::from_string("focus"), Some(ZoneSetLayoutType::Focus));
        assert_eq!(ZoneSetLayoutType::from_string("columns"), Some(ZoneSetLayoutType::Columns));
        assert_eq!(ZoneSetLayoutType::from_string("rows"), Some(ZoneSetLayoutType::Rows));
        assert_eq!(ZoneSetLayoutType::from_string("grid"), Some(ZoneSetLayoutType::Grid));
        assert_eq!(ZoneSetLayoutType::from_string("priority-grid"), Some(ZoneSetLayoutType::PriorityGrid));
        assert_eq!(ZoneSetLayoutType::from_string("custom"), Some(ZoneSetLayoutType::Custom));
        assert_eq!(ZoneSetLayoutType::from_string("invalid"), None);
    }
}
