use std::collections::BTreeMap;

use crate::data::{GridLayoutInfo, ZoneSetLayoutType};
use crate::rect::{Point, Rect};
use crate::settings::OverlappingZonesAlgorithm;
use crate::zone::{Zone, ZoneIndex, ZoneIndexSet};

/// Mapping zone id to zone, ordered by index.
pub type ZonesMap = BTreeMap<ZoneIndex, Zone>;

/// Layout data used to initialize a layout.
#[derive(Debug, Clone)]
pub struct LayoutData {
    pub uuid: String,
    pub layout_type: ZoneSetLayoutType,
    pub show_spacing: bool,
    pub spacing: i32,
    pub zone_count: i32,
    pub sensitivity_radius: i32,
}

/// A layout is a collection of zones created from a layout type and work area.
#[derive(Debug)]
pub struct Layout {
    data: LayoutData,
    zones: ZonesMap,
}

impl Layout {
    pub fn new(data: LayoutData) -> Self {
        Self { data, zones: ZonesMap::new() }
    }

    /// Initialize the layout for a given work area.
    /// Returns true on success.
    pub fn init(&mut self, work_area: Rect) -> bool {
        // Invalid work area
        if work_area.width() == 0 || work_area.height() == 0 {
            return false;
        }

        let is_grid_type = matches!(
            self.data.layout_type,
            ZoneSetLayoutType::Columns | ZoneSetLayoutType::Rows | ZoneSetLayoutType::Grid | ZoneSetLayoutType::PriorityGrid
        );

        if self.data.zone_count < 0 || (self.data.zone_count == 0 && is_grid_type) {
            return false;
        }

        let spacing = if self.data.show_spacing { self.data.spacing } else { 0 };

        self.zones = match self.data.layout_type {
            ZoneSetLayoutType::Blank => ZonesMap::new(),
            ZoneSetLayoutType::Focus => layout_focus(work_area, self.data.zone_count),
            ZoneSetLayoutType::Columns => layout_columns(work_area, self.data.zone_count, spacing),
            ZoneSetLayoutType::Rows => layout_rows(work_area, self.data.zone_count, spacing),
            ZoneSetLayoutType::Grid => layout_grid(work_area, self.data.zone_count, spacing),
            ZoneSetLayoutType::PriorityGrid => layout_priority_grid(work_area, self.data.zone_count, spacing),
            ZoneSetLayoutType::Custom => return false, // Custom requires external data
        };

        self.zones.len() == self.data.zone_count as usize
    }

    /// Initialize with custom zones (canvas layout, provided externally).
    pub fn init_custom(&mut self, zones: Vec<Rect>) -> bool {
        self.zones.clear();
        for (i, rect) in zones.iter().enumerate() {
            let zone = Zone::new(*rect, i as ZoneIndex);
            if !zone.is_valid() {
                self.zones.clear();
                return false;
            }
            self.zones.insert(i as ZoneIndex, zone);
        }
        true
    }

    pub fn id(&self) -> &str {
        &self.data.uuid
    }

    pub fn layout_type(&self) -> ZoneSetLayoutType {
        self.data.layout_type
    }

    pub fn zones(&self) -> &ZonesMap {
        &self.zones
    }

    /// Find zones containing the given point.
    /// Handles overlapping zone resolution using the specified algorithm.
    pub fn zones_from_point(&self, pt: Point, algorithm: OverlappingZonesAlgorithm) -> ZoneIndexSet {
        let mut captured_zones = ZoneIndexSet::new();
        let mut strictly_captured = ZoneIndexSet::new();
        let sr = self.data.sensitivity_radius;

        for (&zone_id, zone) in &self.zones {
            let r = zone.get_zone_rect();
            if r.left - sr <= pt.x && pt.x <= r.right + sr
                && r.top - sr <= pt.y && pt.y <= r.bottom + sr
            {
                captured_zones.push(zone_id);
            }
            if r.left <= pt.x && pt.x < r.right && r.top <= pt.y && pt.y < r.bottom {
                strictly_captured.push(zone_id);
            }
        }

        // If only one zone captured but not strictly, ignore it
        if captured_zones.len() == 1 && strictly_captured.is_empty() {
            return vec![];
        }

        // Check for overlapping zones
        let mut overlap = false;
        'outer: for i in 0..captured_zones.len() {
            for j in (i + 1)..captured_zones.len() {
                let ri = self.zones[&captured_zones[i]].get_zone_rect();
                let rj = self.zones[&captured_zones[j]].get_zone_rect();
                if ri.top.max(rj.top) + sr < ri.bottom.min(rj.bottom)
                    && ri.left.max(rj.left) + sr < ri.right.min(rj.right)
                {
                    overlap = true;
                    break 'outer;
                }
            }
        }

        if overlap {
            match algorithm {
                OverlappingZonesAlgorithm::Smallest => {
                    zone_select_priority(&self.zones, &captured_zones, |z1, z2| {
                        z1.get_zone_area() < z2.get_zone_area()
                    })
                }
                OverlappingZonesAlgorithm::Largest => {
                    zone_select_priority(&self.zones, &captured_zones, |z1, z2| {
                        z1.get_zone_area() > z2.get_zone_area()
                    })
                }
                OverlappingZonesAlgorithm::Positional => {
                    zone_select_subregion(&self.zones, &captured_zones, pt, sr)
                }
                OverlappingZonesAlgorithm::ClosestCenter => {
                    zone_select_closest_center(&self.zones, &captured_zones, pt)
                }
            }
        } else {
            captured_zones
        }
    }

    /// Returns all zones spanned by the minimum bounding rectangle of two zone sets.
    pub fn get_combined_zone_range(&self, initial_zones: &ZoneIndexSet, final_zones: &ZoneIndexSet) -> ZoneIndexSet {
        let mut combined: ZoneIndexSet = initial_zones.clone();
        for z in final_zones {
            if !combined.contains(z) {
                combined.push(*z);
            }
        }
        combined.sort();

        let mut bounding_rect: Option<Rect> = None;
        for &zone_id in &combined {
            if let Some(zone) = self.zones.get(&zone_id) {
                let r = zone.get_zone_rect();
                bounding_rect = Some(match bounding_rect {
                    None => r,
                    Some(br) => Rect {
                        left: br.left.min(r.left),
                        top: br.top.min(r.top),
                        right: br.right.max(r.right),
                        bottom: br.bottom.max(r.bottom),
                    },
                });
            }
        }

        let Some(br) = bounding_rect else { return vec![] };

        let mut result = ZoneIndexSet::new();
        for (&zone_id, zone) in &self.zones {
            let r = zone.get_zone_rect();
            if br.left <= r.left && r.right <= br.right && br.top <= r.top && r.bottom <= br.bottom {
                result.push(zone_id);
            }
        }
        result
    }

    /// Get the bounding rectangle of a set of zones.
    pub fn get_combined_zones_rect(&self, zones: &ZoneIndexSet) -> Rect {
        let mut result: Option<Rect> = None;
        for &id in zones {
            if let Some(zone) = self.zones.get(&id) {
                let r = zone.get_zone_rect();
                result = Some(match result {
                    None => r,
                    Some(br) => Rect {
                        left: br.left.min(r.left),
                        top: br.top.min(r.top),
                        right: br.right.max(r.right),
                        bottom: br.bottom.max(r.bottom),
                    },
                });
            }
        }
        result.unwrap_or_default()
    }
}

// ---- Zone selection algorithms ----

const OVERLAPPING_CENTERS_SENSITIVITY: i64 = 75;

fn zone_select_priority<F>(zones: &ZonesMap, captured: &ZoneIndexSet, compare: F) -> ZoneIndexSet
where
    F: Fn(&Zone, &Zone) -> bool,
{
    if captured.is_empty() {
        return vec![];
    }
    let mut chosen = 0usize;
    for i in 1..captured.len() {
        if compare(&zones[&captured[i]], &zones[&captured[chosen]]) {
            chosen = i;
        }
    }
    vec![captured[chosen]]
}

fn zone_select_subregion(zones: &ZonesMap, captured: &ZoneIndexSet, pt: Point, sensitivity_radius: i32) -> ZoneIndexSet {
    if captured.is_empty() {
        return vec![];
    }

    let expand = |r: &Rect| -> Rect {
        Rect {
            left: r.left - sensitivity_radius / 2,
            top: r.top - sensitivity_radius / 2,
            right: r.right + sensitivity_radius / 2,
            bottom: r.bottom + sensitivity_radius / 2,
        }
    };

    let mut overlap = expand(&zones[&captured[0]].get_zone_rect());
    for i in 1..captured.len() {
        let current = expand(&zones[&captured[i]].get_zone_rect());
        overlap.top = overlap.top.max(current.top);
        overlap.left = overlap.left.max(current.left);
        overlap.bottom = overlap.bottom.min(current.bottom);
        overlap.right = overlap.right.min(current.right);
    }

    let width = (overlap.right - overlap.left).max(1);
    let height = (overlap.bottom - overlap.top).max(1);
    let vertical_split = height > width;

    let zone_index = if vertical_split {
        (pt.y as i64 - overlap.top as i64) * captured.len() as i64 / height as i64
    } else {
        (pt.x as i64 - overlap.left as i64) * captured.len() as i64 / width as i64
    };

    let zone_index = zone_index.clamp(0, captured.len() as i64 - 1) as usize;
    vec![captured[zone_index]]
}

fn zone_select_closest_center(zones: &ZonesMap, captured: &ZoneIndexSet, pt: Point) -> ZoneIndexSet {
    let get_center = |zone: &Zone| -> (i32, i32) {
        let r = zone.get_zone_rect();
        ((r.right + r.left) / 2, (r.top + r.bottom) / 2)
    };
    let point_diff = |p1: (i32, i32), p2: (i32, i32)| -> i64 {
        let dx = p1.0 as i64 - p2.0 as i64;
        let dy = p1.1 as i64 - p2.1 as i64;
        dx * dx + dy * dy
    };
    let pt_tuple = (pt.x, pt.y);

    zone_select_priority(zones, captured, |z1, z2| {
        let c1 = get_center(z1);
        let c2 = get_center(z2);
        if point_diff(c1, c2) > OVERLAPPING_CENTERS_SENSITIVITY {
            point_diff(c1, pt_tuple) < point_diff(c2, pt_tuple)
        } else {
            z1.get_zone_area() < z2.get_zone_area()
        }
    })
}

// ---- Layout configurators ----

fn add_zone(zone: Zone, zones: &mut ZonesMap) -> bool {
    let id = zone.id();
    if zones.contains_key(&id) {
        return false;
    }
    zones.insert(id, zone);
    true
}

pub fn layout_focus(work_area: Rect, zone_count: i32) -> ZonesMap {
    let mut zones = ZonesMap::new();
    let mut left = 100i32;
    let mut top = 100i32;
    let mut right = left + (work_area.width() as f64 * 0.4) as i32;
    let mut bottom = top + (work_area.height() as f64 * 0.4) as i32;

    let increment = if zone_count <= 1 { 0 } else { 50 };

    for _i in 0..zone_count {
        let zone = Zone::new(Rect::new(left, top, right, bottom), zones.len() as ZoneIndex);
        if !zone.is_valid() {
            return ZonesMap::new();
        }
        if !add_zone(zone, &mut zones) {
            return ZonesMap::new();
        }
        left += increment;
        right += increment;
        top += increment;
        bottom += increment;
    }
    zones
}

pub fn layout_rows(work_area: Rect, zone_count: i32, spacing: i32) -> ZonesMap {
    if zone_count == 0 {
        return ZonesMap::new();
    }
    let mut zones = ZonesMap::new();
    let total_width = work_area.width() - spacing * 2;
    let total_height = work_area.height() - spacing * (zone_count + 1);
    let mut top = spacing;
    let left = spacing;

    for i in 0..zone_count {
        let right = total_width + spacing;
        let bottom = top + (i + 1) as i32 * total_height / zone_count - i as i32 * total_height / zone_count;
        let zone = Zone::new(Rect::new(left, top, right, bottom), zones.len() as ZoneIndex);
        if !zone.is_valid() {
            return ZonesMap::new();
        }
        if !add_zone(zone, &mut zones) {
            return ZonesMap::new();
        }
        top = bottom + spacing;
    }
    zones
}

pub fn layout_columns(work_area: Rect, zone_count: i32, spacing: i32) -> ZonesMap {
    if zone_count == 0 {
        return ZonesMap::new();
    }
    let mut zones = ZonesMap::new();
    let total_width = work_area.width() - spacing * (zone_count + 1);
    let total_height = work_area.height() - spacing * 2;
    let top = spacing;
    let mut left = spacing;

    for i in 0..zone_count {
        let right = left + (i + 1) as i32 * total_width / zone_count - i as i32 * total_width / zone_count;
        let bottom = total_height + spacing;
        let zone = Zone::new(Rect::new(left, top, right, bottom), zones.len() as ZoneIndex);
        if !zone.is_valid() {
            return ZonesMap::new();
        }
        if !add_zone(zone, &mut zones) {
            return ZonesMap::new();
        }
        left = right + spacing;
    }
    zones
}

pub fn calculate_grid_zones(work_area: Rect, grid: &GridLayoutInfo, spacing: i32) -> ZonesMap {
    let mut zones = ZonesMap::new();
    let total_width = work_area.width() as i64;
    let total_height = work_area.height() as i64;
    const C_MULTIPLIER: i64 = 10000;

    struct Info {
        start: i64,
        end: i64,
    }

    let mut row_info: Vec<Info> = Vec::new();
    let mut col_info: Vec<Info> = Vec::new();

    let mut total_percents: i64 = 0;
    for row in 0..grid.rows as usize {
        let start = total_percents * total_height / C_MULTIPLIER;
        total_percents += grid.rows_percents[row] as i64;
        let end = total_percents * total_height / C_MULTIPLIER;
        row_info.push(Info { start, end });
    }

    total_percents = 0;
    for col in 0..grid.columns as usize {
        let start = total_percents * total_width / C_MULTIPLIER;
        total_percents += grid.columns_percents[col] as i64;
        let end = total_percents * total_width / C_MULTIPLIER;
        col_info.push(Info { start, end });
    }

    for row in 0..grid.rows as usize {
        for col in 0..grid.columns as usize {
            let i = grid.cell_child_map[row][col];
            if (row == 0 || grid.cell_child_map[row - 1][col] != i)
                && (col == 0 || grid.cell_child_map[row][col - 1] != i)
            {
                let mut left = col_info[col].start;
                let mut top = row_info[row].start;

                let mut max_row = row;
                while max_row + 1 < grid.rows as usize && grid.cell_child_map[max_row + 1][col] == i {
                    max_row += 1;
                }
                let mut max_col = col;
                while max_col + 1 < grid.columns as usize && grid.cell_child_map[row][max_col + 1] == i {
                    max_col += 1;
                }

                let mut right = col_info[max_col].end;
                let mut bottom = row_info[max_row].end;

                let spacing = spacing as i64;
                top += if row == 0 { spacing } else { spacing / 2 };
                bottom -= if max_row == grid.rows as usize - 1 { spacing } else { spacing / 2 };
                left += if col == 0 { spacing } else { spacing / 2 };
                right -= if max_col == grid.columns as usize - 1 { spacing } else { spacing / 2 };

                let zone = Zone::new(Rect::new(left as i32, top as i32, right as i32, bottom as i32), i as ZoneIndex);
                if !zone.is_valid() {
                    return ZonesMap::new();
                }
                if !add_zone(zone, &mut zones) {
                    return ZonesMap::new();
                }
            }
        }
    }
    zones
}

pub fn layout_grid(work_area: Rect, zone_count: i32, spacing: i32) -> ZonesMap {
    if zone_count == 0 {
        return ZonesMap::new();
    }

    let mut rows = 1i32;
    while zone_count / rows >= rows {
        rows += 1;
    }
    rows -= 1;
    let mut columns = zone_count / rows;
    if zone_count % rows != 0 {
        columns += 1;
    }

    let mut grid = GridLayoutInfo::minimal(rows, columns);
    const C_MULTIPLIER: i32 = 10000;

    for row in 0..rows as usize {
        grid.rows_percents[row] = C_MULTIPLIER * (row as i32 + 1) / rows - C_MULTIPLIER * row as i32 / rows;
    }
    for col in 0..columns as usize {
        grid.columns_percents[col] = C_MULTIPLIER * (col as i32 + 1) / columns - C_MULTIPLIER * col as i32 / columns;
    }

    let mut index = 0;
    for row in 0..rows as usize {
        for col in 0..columns as usize {
            grid.cell_child_map[row][col] = index;
            index += 1;
            if index == zone_count {
                index -= 1;
            }
        }
    }

    calculate_grid_zones(work_area, &grid, spacing)
}

fn predefined_priority_grid_layouts() -> Vec<GridLayoutInfo> {
    vec![
        /* 1 */ GridLayoutInfo { rows: 1, columns: 1, rows_percents: vec![10000], columns_percents: vec![10000], cell_child_map: vec![vec![0]], show_spacing: false, spacing: 0, sensitivity_radius: 0 },
        /* 2 */ GridLayoutInfo { rows: 1, columns: 2, rows_percents: vec![10000], columns_percents: vec![6667, 3333], cell_child_map: vec![vec![0, 1]], show_spacing: false, spacing: 0, sensitivity_radius: 0 },
        /* 3 */ GridLayoutInfo { rows: 1, columns: 3, rows_percents: vec![10000], columns_percents: vec![2500, 5000, 2500], cell_child_map: vec![vec![0, 1, 2]], show_spacing: false, spacing: 0, sensitivity_radius: 0 },
        /* 4 */ GridLayoutInfo { rows: 2, columns: 3, rows_percents: vec![5000, 5000], columns_percents: vec![2500, 5000, 2500], cell_child_map: vec![vec![0, 1, 2], vec![0, 1, 3]], show_spacing: false, spacing: 0, sensitivity_radius: 0 },
        /* 5 */ GridLayoutInfo { rows: 2, columns: 3, rows_percents: vec![5000, 5000], columns_percents: vec![2500, 5000, 2500], cell_child_map: vec![vec![0, 1, 2], vec![3, 1, 4]], show_spacing: false, spacing: 0, sensitivity_radius: 0 },
        /* 6 */ GridLayoutInfo { rows: 3, columns: 3, rows_percents: vec![3333, 3334, 3333], columns_percents: vec![2500, 5000, 2500], cell_child_map: vec![vec![0, 1, 2], vec![0, 1, 3], vec![4, 1, 5]], show_spacing: false, spacing: 0, sensitivity_radius: 0 },
        /* 7 */ GridLayoutInfo { rows: 3, columns: 3, rows_percents: vec![3333, 3334, 3333], columns_percents: vec![2500, 5000, 2500], cell_child_map: vec![vec![0, 1, 2], vec![3, 1, 4], vec![5, 1, 6]], show_spacing: false, spacing: 0, sensitivity_radius: 0 },
        /* 8 */ GridLayoutInfo { rows: 3, columns: 4, rows_percents: vec![3333, 3334, 3333], columns_percents: vec![2500, 2500, 2500, 2500], cell_child_map: vec![vec![0, 1, 2, 3], vec![4, 1, 2, 5], vec![6, 1, 2, 7]], show_spacing: false, spacing: 0, sensitivity_radius: 0 },
        /* 9 */ GridLayoutInfo { rows: 3, columns: 4, rows_percents: vec![3333, 3334, 3333], columns_percents: vec![2500, 2500, 2500, 2500], cell_child_map: vec![vec![0, 1, 2, 3], vec![4, 1, 2, 5], vec![6, 1, 7, 8]], show_spacing: false, spacing: 0, sensitivity_radius: 0 },
        /* 10 */ GridLayoutInfo { rows: 3, columns: 4, rows_percents: vec![3333, 3334, 3333], columns_percents: vec![2500, 2500, 2500, 2500], cell_child_map: vec![vec![0, 1, 2, 3], vec![4, 1, 5, 6], vec![7, 1, 8, 9]], show_spacing: false, spacing: 0, sensitivity_radius: 0 },
        /* 11 */ GridLayoutInfo { rows: 3, columns: 4, rows_percents: vec![3333, 3334, 3333], columns_percents: vec![2500, 2500, 2500, 2500], cell_child_map: vec![vec![0, 1, 2, 3], vec![4, 1, 5, 6], vec![7, 8, 9, 10]], show_spacing: false, spacing: 0, sensitivity_radius: 0 },
    ]
}

pub fn layout_priority_grid(work_area: Rect, zone_count: i32, spacing: i32) -> ZonesMap {
    if zone_count <= 0 {
        return ZonesMap::new();
    }

    let predefined = predefined_priority_grid_layouts();
    if (zone_count as usize) <= predefined.len() {
        return calculate_grid_zones(work_area, &predefined[zone_count as usize - 1], spacing);
    }

    layout_grid(work_area, zone_count, spacing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone::MAX_NEGATIVE_SPACING;

    fn check_zones(layout: &Layout, layout_type: ZoneSetLayoutType, expected_count: usize, rect: Rect) {
        let zones = layout.zones();
        assert_eq!(expected_count, zones.len(), "zone count mismatch for {:?}", layout_type);

        for (_zone_id, zone) in zones {
            let r = zone.get_zone_rect();
            assert!(r.left >= 0, "left border is less than zero");
            assert!(r.top >= 0, "top border is less than zero");
            assert!(r.left < r.right, "rect.left >= rect.right");
            assert!(r.top < r.bottom, "rect.top >= rect.bottom");

            if layout_type != ZoneSetLayoutType::Focus {
                assert!(r.right <= rect.right, "right border ({}) > work area right ({})", r.right, rect.right);
                assert!(r.bottom <= rect.bottom, "bottom border ({}) > work area bottom ({})", r.bottom, rect.bottom);
            }
        }
    }

    const WORK_AREA_RECTS: &[Rect] = &[
        Rect { left: 0, top: 0, right: 1024, bottom: 768 },
        Rect { left: 0, top: 0, right: 1280, bottom: 720 },
        Rect { left: 0, top: 0, right: 1280, bottom: 800 },
        Rect { left: 0, top: 0, right: 1280, bottom: 1024 },
        Rect { left: 0, top: 0, right: 1366, bottom: 768 },
        Rect { left: 0, top: 0, right: 1440, bottom: 900 },
        Rect { left: 0, top: 0, right: 1536, bottom: 864 },
        Rect { left: 0, top: 0, right: 1600, bottom: 900 },
        Rect { left: 0, top: 0, right: 1920, bottom: 1080 },
    ];

    // ---- LayoutUnitTests (from Layout.Spec.cpp) ----

    #[test]
    fn test_create_layout() {
        let data = LayoutData {
            uuid: "{F762BAD6-DAA1-4997-9497-E11DFEB72F21}".into(),
            layout_type: ZoneSetLayoutType::Grid,
            show_spacing: true,
            spacing: 17,
            zone_count: 4,
            sensitivity_radius: 33,
        };
        let layout = Layout::new(data.clone());
        assert_eq!(layout.id(), data.uuid);
        assert_eq!(layout.layout_type(), data.layout_type);
    }

    #[test]
    fn empty_zones() {
        let data = LayoutData {
            uuid: "{F762BAD6-DAA1-4997-9497-E11DFEB72F21}".into(),
            layout_type: ZoneSetLayoutType::Grid,
            show_spacing: true,
            spacing: 17,
            zone_count: 4,
            sensitivity_radius: 33,
        };
        let layout = Layout::new(data);
        assert_eq!(layout.zones().len(), 0);
    }

    #[test]
    fn zone_from_point_empty() {
        let data = LayoutData {
            uuid: "".into(),
            layout_type: ZoneSetLayoutType::Grid,
            show_spacing: true,
            spacing: 17,
            zone_count: 4,
            sensitivity_radius: 33,
        };
        let layout = Layout::new(data);
        let actual = layout.zones_from_point(Point::new(0, 0), OverlappingZonesAlgorithm::Smallest);
        assert_eq!(actual.len(), 0);
    }

    #[test]
    fn zone_from_point_inner() {
        let data = LayoutData {
            uuid: "".into(),
            layout_type: ZoneSetLayoutType::Grid,
            show_spacing: true,
            spacing: 0,
            zone_count: 4,
            sensitivity_radius: 33,
        };
        let mut layout = Layout::new(data);
        assert!(layout.init(Rect::new(0, 0, 1920, 1080)));
        let actual = layout.zones_from_point(Point::new(1, 1), OverlappingZonesAlgorithm::Smallest);
        assert_eq!(actual.len(), 1);
    }

    #[test]
    fn zone_from_point_border() {
        let data = LayoutData {
            uuid: "".into(),
            layout_type: ZoneSetLayoutType::Grid,
            show_spacing: true,
            spacing: 0,
            zone_count: 4,
            sensitivity_radius: 33,
        };
        let mut layout = Layout::new(data);
        assert!(layout.init(Rect::new(0, 0, 1920, 1080)));

        assert_eq!(layout.zones_from_point(Point::new(0, 0), OverlappingZonesAlgorithm::Smallest).len(), 1);
        assert_eq!(layout.zones_from_point(Point::new(1920, 1080), OverlappingZonesAlgorithm::Smallest).len(), 0);
    }

    #[test]
    fn zone_from_point_outer() {
        let data = LayoutData {
            uuid: "".into(),
            layout_type: ZoneSetLayoutType::Grid,
            show_spacing: true,
            spacing: 17,
            zone_count: 4,
            sensitivity_radius: 33,
        };
        let mut layout = Layout::new(data);
        layout.init(Rect::new(0, 0, 1920, 1080));
        let actual = layout.zones_from_point(Point::new(1921, 1080), OverlappingZonesAlgorithm::Smallest);
        assert_eq!(actual.len(), 0);
    }

    #[test]
    fn zone_from_point_overlapping() {
        let data = LayoutData {
            uuid: "".into(),
            layout_type: ZoneSetLayoutType::Custom,
            show_spacing: true,
            spacing: 0,
            zone_count: 4,
            sensitivity_radius: 33,
        };
        let mut layout = Layout::new(data);
        layout.init_custom(vec![
            Rect::new(0, 0, 100, 100),
            Rect::new(10, 10, 90, 90),
            Rect::new(10, 10, 150, 150),
            Rect::new(10, 10, 50, 50),
        ]);

        let zones = layout.zones_from_point(Point::new(50, 50), OverlappingZonesAlgorithm::Smallest);
        assert_eq!(zones.len(), 1);

        // Zone 3 (index 3) is the smallest overlapping zone containing (50,50): {10,10,50,50}
        let actual_zone = &layout.zones()[&zones[0]];
        assert_eq!(actual_zone.id(), 3);
        assert_eq!(actual_zone.get_zone_rect(), Rect::new(10, 10, 50, 50));
    }

    #[test]
    fn zone_from_point_multizone() {
        let data = LayoutData {
            uuid: "".into(),
            layout_type: ZoneSetLayoutType::Custom,
            show_spacing: true,
            spacing: 0,
            zone_count: 4,
            sensitivity_radius: 33,
        };
        let mut layout = Layout::new(data);
        layout.init_custom(vec![
            Rect::new(0, 0, 100, 100),
            Rect::new(100, 0, 200, 100),
            Rect::new(0, 100, 100, 200),
            Rect::new(100, 100, 200, 200),
        ]);

        // Point (50, 100) is on the border of zone 0 and zone 2
        let actual = layout.zones_from_point(Point::new(50, 100), OverlappingZonesAlgorithm::Smallest);
        assert_eq!(actual.len(), 2);

        let z0 = &layout.zones()[&actual[0]];
        assert_eq!(z0.id(), 0);
        assert_eq!(z0.get_zone_rect(), Rect::new(0, 0, 100, 100));

        let z2 = &layout.zones()[&actual[1]];
        assert_eq!(z2.id(), 2);
        assert_eq!(z2.get_zone_rect(), Rect::new(0, 100, 100, 200));
    }

    // ---- LayoutInitUnitTests (from Layout.Spec.cpp) ----

    #[test]
    fn valid_values() {
        let zone_count = 10;
        for &lt in ZoneSetLayoutType::template_types() {
            for &rect in WORK_AREA_RECTS {
                let data = LayoutData {
                    uuid: "".into(),
                    layout_type: lt,
                    show_spacing: true,
                    spacing: 10,
                    zone_count,
                    sensitivity_radius: 33,
                };
                let mut layout = Layout::new(data);
                assert!(layout.init(rect), "Init failed for {:?} with rect {:?}", lt, rect);
                check_zones(&layout, lt, zone_count as usize, rect);
            }
        }
    }

    #[test]
    fn invalid_monitor_info() {
        for &lt in ZoneSetLayoutType::template_types() {
            let data = LayoutData {
                uuid: "".into(),
                layout_type: lt,
                show_spacing: true,
                spacing: 10,
                zone_count: 10,
                sensitivity_radius: 33,
            };
            let mut layout = Layout::new(data);
            assert!(!layout.init(Rect::new(0, 0, 0, 0)));
        }
    }

    #[test]
    fn zero_spacing() {
        for &lt in ZoneSetLayoutType::template_types() {
            for &rect in WORK_AREA_RECTS {
                let data = LayoutData {
                    uuid: "".into(),
                    layout_type: lt,
                    show_spacing: true,
                    spacing: 0,
                    zone_count: 10,
                    sensitivity_radius: 33,
                };
                let mut layout = Layout::new(data);
                assert!(layout.init(rect), "Zero spacing init failed for {:?}", lt);
                check_zones(&layout, lt, 10, rect);
            }
        }
    }

    #[test]
    fn large_negative_spacing() {
        for &lt in ZoneSetLayoutType::template_types() {
            let data = LayoutData {
                uuid: "".into(),
                layout_type: lt,
                show_spacing: true,
                spacing: MAX_NEGATIVE_SPACING - 1,
                zone_count: 10,
                sensitivity_radius: 33,
            };
            let mut layout = Layout::new(data);

            for &rect in WORK_AREA_RECTS {
                let result = layout.init(rect);
                if lt == ZoneSetLayoutType::Focus {
                    assert!(result, "Focus should succeed regardless of spacing");
                } else {
                    assert!(!result, "Non-focus should fail with large negative spacing");
                }
            }
        }
    }

    #[test]
    fn horizontally_big_spacing() {
        for &lt in ZoneSetLayoutType::template_types() {
            for &rect in WORK_AREA_RECTS {
                let data = LayoutData {
                    uuid: "".into(),
                    layout_type: lt,
                    show_spacing: true,
                    spacing: rect.right,
                    zone_count: 10,
                    sensitivity_radius: 33,
                };
                let mut layout = Layout::new(data);
                let result = layout.init(rect);
                if lt == ZoneSetLayoutType::Focus {
                    assert!(result);
                } else {
                    assert!(!result);
                }
            }
        }
    }

    #[test]
    fn vertically_big_spacing() {
        for &lt in ZoneSetLayoutType::template_types() {
            for &rect in WORK_AREA_RECTS {
                let data = LayoutData {
                    uuid: "".into(),
                    layout_type: lt,
                    show_spacing: true,
                    spacing: rect.bottom,
                    zone_count: 10,
                    sensitivity_radius: 33,
                };
                let mut layout = Layout::new(data);
                let result = layout.init(rect);
                if lt == ZoneSetLayoutType::Focus {
                    assert!(result);
                } else {
                    assert!(!result);
                }
            }
        }
    }

    #[test]
    fn zero_zone_count() {
        // Grid types with 0 zones should fail
        for &lt in ZoneSetLayoutType::grid_types() {
            let data = LayoutData {
                uuid: "".into(),
                layout_type: lt,
                show_spacing: true,
                spacing: 17,
                zone_count: 0,
                sensitivity_radius: 33,
            };
            let mut layout = Layout::new(data);
            for &rect in WORK_AREA_RECTS {
                assert!(!layout.init(rect), "Grid type {:?} should fail with 0 zones", lt);
            }
        }

        // Blank with 0 zones should succeed
        {
            let data = LayoutData {
                uuid: "".into(),
                layout_type: ZoneSetLayoutType::Blank,
                show_spacing: true,
                spacing: 17,
                zone_count: 0,
                sensitivity_radius: 33,
            };
            let mut layout = Layout::new(data);
            for &rect in WORK_AREA_RECTS {
                assert!(layout.init(rect));
            }
        }

        // Focus with 0 zones should succeed
        {
            let data = LayoutData {
                uuid: "".into(),
                layout_type: ZoneSetLayoutType::Focus,
                show_spacing: true,
                spacing: 17,
                zone_count: 0,
                sensitivity_radius: 33,
            };
            let mut layout = Layout::new(data);
            for &rect in WORK_AREA_RECTS {
                assert!(layout.init(rect));
            }
        }
    }

    #[test]
    fn big_zone_count() {
        let zone_count = 128;
        for &lt in ZoneSetLayoutType::template_types() {
            for &rect in WORK_AREA_RECTS {
                let data = LayoutData {
                    uuid: "".into(),
                    layout_type: lt,
                    show_spacing: true,
                    spacing: 0,
                    zone_count,
                    sensitivity_radius: 33,
                };
                let mut layout = Layout::new(data);
                assert!(layout.init(rect), "Big zone count failed for {:?} with {:?}", lt, rect);
                check_zones(&layout, lt, zone_count as usize, rect);
            }
        }
    }
}
