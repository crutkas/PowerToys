// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! Monitor topology — edge detection, outer-edge identification, and wrap destination.
//!
//! This is the pure-logic port of `MonitorTopology.h/cpp` from C++.
//! No Win32 dependencies — all functions operate on [`MonitorInfo`] and plain coordinates.

use std::collections::BTreeMap;

use crate::types::*;

/// Tolerance in pixels for edge adjacency detection (matches C++ `tolerance = 50`).
const ADJACENCY_TOLERANCE: i32 = 50;

/// Threshold for cursor being "on" an edge (within 1 pixel of boundary).
const EDGE_THRESHOLD: i32 = 1;

/// Monitor topology: manages edge-based monitor layout for cursor wrapping.
#[derive(Debug, Clone)]
pub struct MonitorTopology {
    monitors: Vec<MonitorInfo>,
    outer_edges: Vec<MonitorEdge>,
    /// (monitor_index, edge_type) → edge info
    edge_map: BTreeMap<(i32, EdgeType), MonitorEdge>,
}

impl MonitorTopology {
    pub fn new() -> Self {
        Self {
            monitors: Vec::new(),
            outer_edges: Vec::new(),
            edge_map: BTreeMap::new(),
        }
    }

    /// Initialize topology from a list of monitors (call after display change).
    pub fn initialize(&mut self, monitors: &[MonitorInfo]) {
        self.monitors = monitors.to_vec();
        self.outer_edges.clear();
        self.edge_map.clear();

        if monitors.is_empty() {
            return;
        }

        self.build_edge_map();
        self.identify_outer_edges();
    }

    /// Check if cursor at `cursor_pos` is on an outer edge of the monitor at index `monitor_index`.
    ///
    /// Returns `Some(EdgeType)` if on an outer edge (filtered by `wrap_mode` and prioritized
    /// by `direction`), or `None` if not on any outer edge.
    pub fn is_on_outer_edge(
        &self,
        monitor_index: i32,
        cursor_pos: Point,
        wrap_mode: WrapMode,
        direction: Option<&CursorDirection>,
    ) -> Option<EdgeType> {
        let monitor_rect = self.get_monitor_rect_by_index(monitor_index)?;

        let mut candidates = Vec::new();

        // Left edge
        if (wrap_mode == WrapMode::Both || wrap_mode == WrapMode::HorizontalOnly)
            && cursor_pos.x <= monitor_rect.left + EDGE_THRESHOLD
        {
            if let Some(edge) = self.edge_map.get(&(monitor_index, EdgeType::Left)) {
                if edge.is_outer {
                    candidates.push(EdgeType::Left);
                }
            }
        }

        // Right edge
        if (wrap_mode == WrapMode::Both || wrap_mode == WrapMode::HorizontalOnly)
            && cursor_pos.x >= monitor_rect.right - 1 - EDGE_THRESHOLD
        {
            if let Some(edge) = self.edge_map.get(&(monitor_index, EdgeType::Right)) {
                if edge.is_outer {
                    candidates.push(EdgeType::Right);
                }
            }
        }

        // Top edge
        if (wrap_mode == WrapMode::Both || wrap_mode == WrapMode::VerticalOnly)
            && cursor_pos.y <= monitor_rect.top + EDGE_THRESHOLD
        {
            if let Some(edge) = self.edge_map.get(&(monitor_index, EdgeType::Top)) {
                if edge.is_outer {
                    candidates.push(EdgeType::Top);
                }
            }
        }

        // Bottom edge
        if (wrap_mode == WrapMode::Both || wrap_mode == WrapMode::VerticalOnly)
            && cursor_pos.y >= monitor_rect.bottom - 1 - EDGE_THRESHOLD
        {
            if let Some(edge) = self.edge_map.get(&(monitor_index, EdgeType::Bottom)) {
                if edge.is_outer {
                    candidates.push(EdgeType::Bottom);
                }
            }
        }

        if candidates.is_empty() {
            return None;
        }

        // Prioritize by direction at corners
        let prioritized = self.prioritize_edge_by_direction(&candidates, direction);

        // Verify this edge has a valid wrap destination
        let source = self.edge_map.get(&(monitor_index, prioritized))?;
        let cursor_coord = if prioritized == EdgeType::Left || prioritized == EdgeType::Right {
            cursor_pos.y
        } else {
            cursor_pos.x
        };

        let result = self.find_nearest_opposite_edge(prioritized, cursor_coord, source);
        if result.found {
            return Some(prioritized);
        }

        // Try other candidates
        for &candidate in &candidates {
            if candidate == prioritized {
                continue;
            }
            let source = match self.edge_map.get(&(monitor_index, candidate)) {
                Some(s) => s,
                None => continue,
            };
            let coord = if candidate == EdgeType::Left || candidate == EdgeType::Right {
                cursor_pos.y
            } else {
                cursor_pos.x
            };
            let alt_result = self.find_nearest_opposite_edge(candidate, coord, source);
            if alt_result.found {
                return Some(candidate);
            }
        }

        None
    }

    /// Calculate the wrap destination for a cursor on the outer edge of `monitor_index`.
    pub fn get_wrap_destination(
        &self,
        monitor_index: i32,
        cursor_pos: Point,
        edge_type: EdgeType,
    ) -> Point {
        let from_edge = match self.edge_map.get(&(monitor_index, edge_type)) {
            Some(e) => e,
            None => return cursor_pos,
        };

        let cursor_coord = if edge_type == EdgeType::Left || edge_type == EdgeType::Right {
            cursor_pos.y
        } else {
            cursor_pos.x
        };

        let opposite = self.find_nearest_opposite_edge(edge_type, cursor_coord, from_edge);

        if !opposite.found {
            // No opposite edge — wrap within same monitor
            if let Some(rect) = self.get_monitor_rect_by_index(monitor_index) {
                let mut result = cursor_pos;
                match edge_type {
                    EdgeType::Left => result.x = rect.right - 2,
                    EdgeType::Right => result.x = rect.left + 1,
                    EdgeType::Top => result.y = rect.bottom - 2,
                    EdgeType::Bottom => result.y = rect.top + 1,
                }
                return result;
            }
            return cursor_pos;
        }

        if edge_type == EdgeType::Left || edge_type == EdgeType::Right {
            Point {
                x: opposite.edge.position,
                y: if opposite.requires_projection {
                    opposite.projected_coordinate
                } else {
                    cursor_pos.y
                },
            }
        } else {
            Point {
                x: if opposite.requires_projection {
                    opposite.projected_coordinate
                } else {
                    cursor_pos.x
                },
                y: opposite.edge.position,
            }
        }
    }

    /// Get outer edges (for debugging/inspection).
    pub fn outer_edges(&self) -> &[MonitorEdge] {
        &self.outer_edges
    }

    /// Find the monitor index whose rect contains the given point.
    pub fn monitor_index_at_point(&self, pt: Point) -> Option<i32> {
        for (i, m) in self.monitors.iter().enumerate() {
            if pt.x >= m.rect.left
                && pt.x < m.rect.right
                && pt.y >= m.rect.top
                && pt.y < m.rect.bottom
            {
                return Some(i as i32);
            }
        }
        None
    }

    /// Get the rectangle for a monitor by its index.
    pub fn get_monitor_rect_by_index(&self, index: i32) -> Option<Rect> {
        self.monitors.get(index as usize).map(|m| m.rect)
    }

    /// Get number of monitors.
    pub fn monitor_count(&self) -> usize {
        self.monitors.len()
    }

    // ── Private helpers ────────────────────────────────────────────────────

    fn build_edge_map(&mut self) {
        for (idx, monitor) in self.monitors.iter().enumerate() {
            let i = idx as i32;
            let r = &monitor.rect;

            // Left edge
            self.edge_map.insert(
                (i, EdgeType::Left),
                MonitorEdge {
                    monitor_index: i,
                    edge_type: EdgeType::Left,
                    position: r.left,
                    start: r.top,
                    end: r.bottom,
                    is_outer: true,
                },
            );

            // Right edge (position = right - 1, matching C++)
            self.edge_map.insert(
                (i, EdgeType::Right),
                MonitorEdge {
                    monitor_index: i,
                    edge_type: EdgeType::Right,
                    position: r.right - 1,
                    start: r.top,
                    end: r.bottom,
                    is_outer: true,
                },
            );

            // Top edge
            self.edge_map.insert(
                (i, EdgeType::Top),
                MonitorEdge {
                    monitor_index: i,
                    edge_type: EdgeType::Top,
                    position: r.top,
                    start: r.left,
                    end: r.right,
                    is_outer: true,
                },
            );

            // Bottom edge (position = bottom - 1, matching C++)
            self.edge_map.insert(
                (i, EdgeType::Bottom),
                MonitorEdge {
                    monitor_index: i,
                    edge_type: EdgeType::Bottom,
                    position: r.bottom - 1,
                    start: r.left,
                    end: r.right,
                    is_outer: true,
                },
            );
        }
    }

    fn identify_outer_edges(&mut self) {
        // Collect all keys first, then mutate
        let keys: Vec<(i32, EdgeType)> = self.edge_map.keys().cloned().collect();

        for key in &keys {
            let edge = self.edge_map[key].clone();
            let mut is_outer = true;

            for other_key in &keys {
                if edge.monitor_index == other_key.0 {
                    continue; // Same monitor
                }
                let other = &self.edge_map[other_key];
                if self.edges_are_adjacent(&edge, other, ADJACENCY_TOLERANCE) {
                    is_outer = false;
                    break;
                }
            }

            self.edge_map.get_mut(key).unwrap().is_outer = is_outer;
            if is_outer {
                let mut e = self.edge_map[key].clone();
                e.is_outer = true;
                self.outer_edges.push(e);
            }
        }
    }

    fn edges_are_adjacent(&self, edge1: &MonitorEdge, edge2: &MonitorEdge, tolerance: i32) -> bool {
        // Must be opposite types
        let opposite = matches!(
            (edge1.edge_type, edge2.edge_type),
            (EdgeType::Left, EdgeType::Right)
                | (EdgeType::Right, EdgeType::Left)
                | (EdgeType::Top, EdgeType::Bottom)
                | (EdgeType::Bottom, EdgeType::Top)
        );

        if !opposite {
            return false;
        }

        // Positions within tolerance
        if (edge1.position - edge2.position).abs() > tolerance {
            return false;
        }

        // Perpendicular ranges must overlap by more than tolerance
        let overlap_start = edge1.start.max(edge2.start);
        let overlap_end = edge1.end.min(edge2.end);

        overlap_end > overlap_start + tolerance
    }

    fn prioritize_edge_by_direction(
        &self,
        candidates: &[EdgeType],
        direction: Option<&CursorDirection>,
    ) -> EdgeType {
        if candidates.len() == 1 || direction.is_none() {
            return candidates[0];
        }

        let dir = direction.unwrap();

        if dir.is_primarily_horizontal() {
            // Prefer Left if moving left, Right if moving right
            if dir.is_moving_left() {
                if let Some(&e) = candidates.iter().find(|&&e| e == EdgeType::Left) {
                    return e;
                }
            } else if dir.is_moving_right() {
                if let Some(&e) = candidates.iter().find(|&&e| e == EdgeType::Right) {
                    return e;
                }
            }
            // Fall back to any horizontal edge
            if let Some(&e) = candidates
                .iter()
                .find(|&&e| e == EdgeType::Left || e == EdgeType::Right)
            {
                return e;
            }
        } else {
            // Prefer Top if moving up, Bottom if moving down
            if dir.is_moving_up() {
                if let Some(&e) = candidates.iter().find(|&&e| e == EdgeType::Top) {
                    return e;
                }
            } else if dir.is_moving_down() {
                if let Some(&e) = candidates.iter().find(|&&e| e == EdgeType::Bottom) {
                    return e;
                }
            }
            // Fall back to any vertical edge
            if let Some(&e) = candidates
                .iter()
                .find(|&&e| e == EdgeType::Top || e == EdgeType::Bottom)
            {
                return e;
            }
        }

        candidates[0]
    }

    /// Find an opposite outer edge whose range directly overlaps `cursor_coord`.
    fn find_opposite_outer_edge(&self, from_edge: EdgeType, cursor_coord: i32) -> Option<MonitorEdge> {
        let (target_type, find_max) = match from_edge {
            EdgeType::Left => (EdgeType::Right, true),
            EdgeType::Right => (EdgeType::Left, false),
            EdgeType::Top => (EdgeType::Bottom, true),
            EdgeType::Bottom => (EdgeType::Top, false),
        };

        let mut best: Option<MonitorEdge> = None;
        let mut extreme = if find_max { i32::MIN } else { i32::MAX };

        for edge in &self.outer_edges {
            if edge.edge_type != target_type {
                continue;
            }
            if cursor_coord >= edge.start && cursor_coord <= edge.end {
                let dominated = if find_max {
                    edge.position > extreme
                } else {
                    edge.position < extreme
                };
                if dominated {
                    extreme = edge.position;
                    best = Some(edge.clone());
                }
            }
        }

        best
    }

    /// Find the nearest opposite outer edge, including projection for non-overlapping regions.
    fn find_nearest_opposite_edge(
        &self,
        from_edge: EdgeType,
        cursor_coord: i32,
        source_edge: &MonitorEdge,
    ) -> OppositeEdgeResult {
        let (target_type, find_max) = match from_edge {
            EdgeType::Left => (EdgeType::Right, true),
            EdgeType::Right => (EdgeType::Left, false),
            EdgeType::Top => (EdgeType::Bottom, true),
            EdgeType::Bottom => (EdgeType::Top, false),
        };

        // First try direct overlap
        if let Some(direct) = self.find_opposite_outer_edge(from_edge, cursor_coord) {
            return OppositeEdgeResult {
                edge: direct,
                found: true,
                requires_projection: false,
                projected_coordinate: cursor_coord,
            };
        }

        // No direct overlap — find nearest by coordinate distance
        let mut best_distance = i32::MAX;
        let mut best_edge: Option<MonitorEdge> = None;

        for edge in &self.outer_edges {
            if edge.edge_type != target_type {
                continue;
            }

            let distance = if cursor_coord < edge.start {
                edge.start - cursor_coord
            } else if cursor_coord > edge.end {
                cursor_coord - edge.end
            } else {
                0
            };

            let is_better = if distance < best_distance {
                true
            } else if distance == best_distance {
                if let Some(ref current_best) = best_edge {
                    if find_max {
                        edge.position > current_best.position
                    } else {
                        edge.position < current_best.position
                    }
                } else {
                    true
                }
            } else {
                false
            };

            if is_better {
                best_distance = distance;
                best_edge = Some(edge.clone());
            }
        }

        match best_edge {
            Some(target) => {
                let projected =
                    self.calculate_projected_position(cursor_coord, source_edge, &target);
                OppositeEdgeResult {
                    edge: target,
                    found: true,
                    requires_projection: true,
                    projected_coordinate: projected,
                }
            }
            None => OppositeEdgeResult::not_found(),
        }
    }

    /// Calculate projected position using Windows-like clamp-to-boundary behaviour.
    fn calculate_projected_position(
        &self,
        cursor_coord: i32,
        source_edge: &MonitorEdge,
        target_edge: &MonitorEdge,
    ) -> i32 {
        let shared_start = source_edge.start.max(target_edge.start);
        let shared_end = source_edge.end.min(target_edge.end);

        if cursor_coord >= shared_start && cursor_coord <= shared_end {
            return cursor_coord;
        }

        let projected = if cursor_coord < shared_start {
            target_edge.start + 1
        } else {
            target_edge.end - 1
        };

        // Final bounds check
        projected.max(target_edge.start).min(target_edge.end - 1)
    }
}

impl Default for MonitorTopology {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helper to build monitors for tests ─────────────────────────────────────

#[cfg(test)]
fn make_monitor(id: i32, left: i32, top: i32, right: i32, bottom: i32, primary: bool) -> MonitorInfo {
    MonitorInfo {
        rect: Rect { left, top, right, bottom },
        is_primary: primary,
        monitor_id: id,
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Single monitor ─────────────────────────────────────────────────

    #[test]
    fn single_monitor_no_outer_edges_for_wrapping() {
        // With only 1 monitor, there are 4 outer edges but no other monitor to wrap to.
        // FindNearestOppositeEdge finds the same monitor's opposite outer edge.
        let mut topo = MonitorTopology::new();
        topo.initialize(&[make_monitor(0, 0, 0, 1920, 1080, true)]);

        // All 4 edges are outer (no adjacent monitors)
        assert_eq!(topo.outer_edges().len(), 4);

        // On the left edge → wraps to right outer edge of same monitor
        let cursor = Point { x: 0, y: 540 };
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::Both, None);
        assert!(edge.is_some());

        let dest = topo.get_wrap_destination(0, cursor, EdgeType::Left);
        assert_eq!(dest.x, 1919); // right edge position = right - 1
        assert_eq!(dest.y, 540);
    }

    #[test]
    fn single_monitor_right_edge_wraps_to_left() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[make_monitor(0, 0, 0, 1920, 1080, true)]);

        let cursor = Point { x: 1919, y: 540 };
        let dest = topo.get_wrap_destination(0, cursor, EdgeType::Right);
        assert_eq!(dest.x, 0); // left edge position
        assert_eq!(dest.y, 540);
    }

    #[test]
    fn single_monitor_top_wraps_to_bottom() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[make_monitor(0, 0, 0, 1920, 1080, true)]);

        let cursor = Point { x: 960, y: 0 };
        let dest = topo.get_wrap_destination(0, cursor, EdgeType::Top);
        assert_eq!(dest.x, 960);
        assert_eq!(dest.y, 1079); // bottom edge position = bottom - 1
    }

    #[test]
    fn single_monitor_bottom_wraps_to_top() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[make_monitor(0, 0, 0, 1920, 1080, true)]);

        let cursor = Point { x: 960, y: 1079 };
        let dest = topo.get_wrap_destination(0, cursor, EdgeType::Bottom);
        assert_eq!(dest.x, 960);
        assert_eq!(dest.y, 0); // top edge position
    }

    // ── Two monitors side-by-side ──────────────────────────────────────

    #[test]
    fn two_monitors_side_by_side_outer_edges() {
        // Monitor 0: [0, 0, 1920, 1080]  Monitor 1: [1920, 0, 3840, 1080]
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 1920, 0, 3840, 1080, false),
        ]);

        // Check that the left edge of monitor 0 IS outer
        assert!(topo.edge_map.get(&(0, EdgeType::Left)).unwrap().is_outer);
        // The right edge of monitor 0 is NOT outer (adjacent to monitor 1 left)
        assert!(!topo.edge_map.get(&(0, EdgeType::Right)).unwrap().is_outer);
        // The left edge of monitor 1 is NOT outer (adjacent to monitor 0 right)
        assert!(!topo.edge_map.get(&(1, EdgeType::Left)).unwrap().is_outer);
        // The right edge of monitor 1 IS outer
        assert!(topo.edge_map.get(&(1, EdgeType::Right)).unwrap().is_outer);
    }

    #[test]
    fn two_monitors_side_by_side_left_outer_wrap() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 1920, 0, 3840, 1080, false),
        ]);

        // Cursor on left outer edge of monitor 0 → should wrap to right outer edge of monitor 1
        let cursor = Point { x: 0, y: 540 };
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::Both, None);
        assert_eq!(edge, Some(EdgeType::Left));

        let dest = topo.get_wrap_destination(0, cursor, EdgeType::Left);
        // Right edge position of monitor 1 = 3840 - 1 = 3839
        assert_eq!(dest.x, 3839);
        assert_eq!(dest.y, 540); // Y preserved
    }

    #[test]
    fn two_monitors_side_by_side_right_outer_wrap() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 1920, 0, 3840, 1080, false),
        ]);

        // Cursor on right outer edge of monitor 1 → wrap to left outer edge of monitor 0
        let cursor = Point { x: 3839, y: 540 };
        let edge = topo.is_on_outer_edge(1, cursor, WrapMode::Both, None);
        assert_eq!(edge, Some(EdgeType::Right));

        let dest = topo.get_wrap_destination(1, cursor, EdgeType::Right);
        assert_eq!(dest.x, 0); // Left edge of monitor 0
        assert_eq!(dest.y, 540);
    }

    #[test]
    fn wrap_preserves_y_coordinate_horizontal() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 1920, 0, 3840, 1080, false),
        ]);

        // Various Y positions should be preserved during horizontal wrap
        for y in [0, 100, 540, 1079] {
            let cursor = Point { x: 0, y };
            let dest = topo.get_wrap_destination(0, cursor, EdgeType::Left);
            assert_eq!(dest.y, y, "Y should be preserved for y={}", y);
        }
    }

    // ── Two monitors stacked ───────────────────────────────────────────

    #[test]
    fn two_monitors_stacked_outer_edges() {
        // Monitor 0 on top, Monitor 1 on bottom
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 0, 1080, 1920, 2160, false),
        ]);

        // Top of monitor 0 IS outer
        assert!(topo.edge_map.get(&(0, EdgeType::Top)).unwrap().is_outer);
        // Bottom of monitor 0 is NOT outer (adjacent to top of monitor 1)
        assert!(!topo.edge_map.get(&(0, EdgeType::Bottom)).unwrap().is_outer);
        // Top of monitor 1 is NOT outer
        assert!(!topo.edge_map.get(&(1, EdgeType::Top)).unwrap().is_outer);
        // Bottom of monitor 1 IS outer
        assert!(topo.edge_map.get(&(1, EdgeType::Bottom)).unwrap().is_outer);
    }

    #[test]
    fn two_monitors_stacked_top_wraps_to_bottom() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 0, 1080, 1920, 2160, false),
        ]);

        let cursor = Point { x: 960, y: 0 };
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::Both, None);
        assert_eq!(edge, Some(EdgeType::Top));

        let dest = topo.get_wrap_destination(0, cursor, EdgeType::Top);
        // Bottom edge of monitor 1 = 2160 - 1 = 2159
        assert_eq!(dest.y, 2159);
        assert_eq!(dest.x, 960); // X preserved
    }

    #[test]
    fn wrap_preserves_x_coordinate_vertical() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 0, 1080, 1920, 2160, false),
        ]);

        for x in [0, 500, 960, 1919] {
            let cursor = Point { x, y: 0 };
            let dest = topo.get_wrap_destination(0, cursor, EdgeType::Top);
            assert_eq!(dest.x, x, "X should be preserved for x={}", x);
        }
    }

    // ── L-shaped layout ────────────────────────────────────────────────

    #[test]
    fn l_shaped_layout_outer_edges() {
        // Monitor 0 (center): [1920, 1080, 3840, 2160]
        // Monitor 1 (left):   [0, 1080, 1920, 2160]
        // Monitor 2 (top):    [1920, 0, 3840, 1080]
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 1920, 1080, 3840, 2160, true),
            make_monitor(1, 0, 1080, 1920, 2160, false),
            make_monitor(2, 1920, 0, 3840, 1080, false),
        ]);

        // Monitor 0: right edge IS outer (nothing to its right)
        assert!(topo.edge_map.get(&(0, EdgeType::Right)).unwrap().is_outer);
        // Monitor 0: left is NOT outer (monitor 1 is there)
        assert!(!topo.edge_map.get(&(0, EdgeType::Left)).unwrap().is_outer);
        // Monitor 0: top is NOT outer (monitor 2 is there)
        assert!(!topo.edge_map.get(&(0, EdgeType::Top)).unwrap().is_outer);
        // Monitor 0: bottom IS outer
        assert!(topo.edge_map.get(&(0, EdgeType::Bottom)).unwrap().is_outer);

        // Monitor 1: left IS outer
        assert!(topo.edge_map.get(&(1, EdgeType::Left)).unwrap().is_outer);
        // Monitor 1: right is NOT outer (monitor 0)
        assert!(!topo.edge_map.get(&(1, EdgeType::Right)).unwrap().is_outer);

        // Monitor 2: top IS outer
        assert!(topo.edge_map.get(&(2, EdgeType::Top)).unwrap().is_outer);
        // Monitor 2: bottom is NOT outer (monitor 0)
        assert!(!topo.edge_map.get(&(2, EdgeType::Bottom)).unwrap().is_outer);
    }

    #[test]
    fn l_shaped_horizontal_wrap() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 1920, 1080, 3840, 2160, true),
            make_monitor(1, 0, 1080, 1920, 2160, false),
            make_monitor(2, 1920, 0, 3840, 1080, false),
        ]);

        // Cursor on left outer edge of monitor 1 at y=1500
        let cursor = Point { x: 0, y: 1500 };
        let edge = topo.is_on_outer_edge(1, cursor, WrapMode::Both, None);
        assert_eq!(edge, Some(EdgeType::Left));

        // Should wrap to the right outer edge of monitor 0 (rightmost outer Right edge)
        let dest = topo.get_wrap_destination(1, cursor, EdgeType::Left);
        assert_eq!(dest.x, 3839); // right edge of monitor 0
        assert_eq!(dest.y, 1500); // Y preserved (overlapping region)
    }

    // ── Edge adjacency with tolerance ──────────────────────────────────

    #[test]
    fn edge_adjacency_within_tolerance() {
        // Two monitors with a small gap (< 50px) should still be adjacent
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 1925, 0, 3845, 1080, false), // 5px gap
        ]);

        // Right edge of monitor 0 should NOT be outer (adjacent within tolerance)
        assert!(!topo.edge_map.get(&(0, EdgeType::Right)).unwrap().is_outer);
        // Left edge of monitor 1 should NOT be outer
        assert!(!topo.edge_map.get(&(1, EdgeType::Left)).unwrap().is_outer);
    }

    #[test]
    fn edge_adjacency_beyond_tolerance() {
        // Two monitors with a large gap (> 50px) should NOT be adjacent
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 2000, 0, 3920, 1080, false), // 80px gap
        ]);

        // Right edge of monitor 0 IS outer (gap too large for adjacency)
        assert!(topo.edge_map.get(&(0, EdgeType::Right)).unwrap().is_outer);
        // Left edge of monitor 1 IS outer
        assert!(topo.edge_map.get(&(1, EdgeType::Left)).unwrap().is_outer);
    }

    // ── Projected position for non-overlapping regions ─────────────────

    #[test]
    fn wrap_destination_non_overlapping_projected() {
        // Monitor 0: [0, 0, 1920, 1080]
        // Monitor 1: [1920, 500, 3840, 1580]  (offset vertically)
        // Left edge of monitor 0 (range 0..1080) wrapping to right of monitor 1 (range 500..1580)
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 1920, 500, 3840, 1580, false),
        ]);

        // Cursor at y=200 on left edge of monitor 0
        // This is outside monitor 1's vertical range (500..1580)
        // The nearest opposite Right edge is monitor 1's right
        // Since y=200 < shared_start=500, project to target_edge.start + 1 = 501
        let cursor = Point { x: 0, y: 200 };
        let dest = topo.get_wrap_destination(0, cursor, EdgeType::Left);
        assert_eq!(dest.x, 3839); // right edge of monitor 1
        assert_eq!(dest.y, 501); // projected (clamped to target start + 1)
    }

    #[test]
    fn wrap_destination_overlapping_region_preserves_coord() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 1920, 500, 3840, 1580, false),
        ]);

        // Cursor at y=700 on left edge of monitor 0
        // This is within monitor 1's vertical range → preserved
        let cursor = Point { x: 0, y: 700 };
        let dest = topo.get_wrap_destination(0, cursor, EdgeType::Left);
        assert_eq!(dest.x, 3839);
        assert_eq!(dest.y, 700); // Exact Y preserved (overlapping)
    }

    // ── WrapMode filtering ─────────────────────────────────────────────

    #[test]
    fn wrap_mode_horizontal_only_filters_vertical_edges() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 0, 1080, 1920, 2160, false),
        ]);

        // Top edge of monitor 0 is outer, but HorizontalOnly should filter it out
        let cursor = Point { x: 960, y: 0 };
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::HorizontalOnly, None);
        assert_eq!(edge, None); // Not visible in HorizontalOnly mode

        // Left edge should still work
        let cursor = Point { x: 0, y: 540 };
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::HorizontalOnly, None);
        assert!(edge.is_some());
    }

    #[test]
    fn wrap_mode_vertical_only_filters_horizontal_edges() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 1920, 0, 3840, 1080, false),
        ]);

        // Left edge of monitor 0 is outer, but VerticalOnly should filter it out
        let cursor = Point { x: 0, y: 540 };
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::VerticalOnly, None);
        assert_eq!(edge, None);

        // Top edge should still work (it's an outer edge for vertical wrapping)
        let cursor = Point { x: 960, y: 0 };
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::VerticalOnly, None);
        assert!(edge.is_some());
    }

    #[test]
    fn wrap_mode_both_allows_all_edges() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[make_monitor(0, 0, 0, 1920, 1080, true)]);

        // Both mode: all edges visible
        let cursor_left = Point { x: 0, y: 540 };
        assert!(topo.is_on_outer_edge(0, cursor_left, WrapMode::Both, None).is_some());

        let cursor_top = Point { x: 960, y: 0 };
        assert!(topo.is_on_outer_edge(0, cursor_top, WrapMode::Both, None).is_some());
    }

    // ── Corner prioritization by direction ─────────────────────────────

    #[test]
    fn corner_prefers_horizontal_when_moving_right() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[make_monitor(0, 0, 0, 1920, 1080, true)]);

        // Cursor at top-right corner: both Right and Top are outer edges
        let cursor = Point { x: 1919, y: 0 };
        let dir = CursorDirection { dx: 5, dy: -2 }; // primarily horizontal (right)
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::Both, Some(&dir));
        assert_eq!(edge, Some(EdgeType::Right));
    }

    #[test]
    fn corner_prefers_vertical_when_moving_up() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[make_monitor(0, 0, 0, 1920, 1080, true)]);

        // Cursor at top-right corner: both Right and Top are outer edges
        let cursor = Point { x: 1919, y: 0 };
        let dir = CursorDirection { dx: 2, dy: -5 }; // primarily vertical (up)
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::Both, Some(&dir));
        assert_eq!(edge, Some(EdgeType::Top));
    }

    #[test]
    fn corner_prefers_left_when_moving_left() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[make_monitor(0, 0, 0, 1920, 1080, true)]);

        let cursor = Point { x: 0, y: 0 }; // top-left corner
        let dir = CursorDirection { dx: -5, dy: -2 };
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::Both, Some(&dir));
        assert_eq!(edge, Some(EdgeType::Left));
    }

    #[test]
    fn corner_prefers_bottom_when_moving_down() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[make_monitor(0, 0, 0, 1920, 1080, true)]);

        let cursor = Point { x: 0, y: 1079 }; // bottom-left corner
        let dir = CursorDirection { dx: -2, dy: 5 };
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::Both, Some(&dir));
        assert_eq!(edge, Some(EdgeType::Bottom));
    }

    // ── No direction at corner → first candidate ───────────────────────

    #[test]
    fn corner_no_direction_uses_first_candidate() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[make_monitor(0, 0, 0, 1920, 1080, true)]);

        let cursor = Point { x: 0, y: 0 }; // top-left corner
        let edge = topo.is_on_outer_edge(0, cursor, WrapMode::Both, None);
        // Without direction info, first candidate (Left) wins
        assert!(edge.is_some());
    }

    // ── Three monitors horizontal ──────────────────────────────────────

    #[test]
    fn three_monitors_horizontal_wrap_left_to_right() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 1920, 0, 3840, 1080, false),
            make_monitor(2, 3840, 0, 5760, 1080, false),
        ]);

        // Left edge of monitor 0 → should wrap to right edge of monitor 2
        let cursor = Point { x: 0, y: 540 };
        let dest = topo.get_wrap_destination(0, cursor, EdgeType::Left);
        assert_eq!(dest.x, 5759); // right edge of monitor 2 (5760-1)
        assert_eq!(dest.y, 540);
    }

    #[test]
    fn three_monitors_horizontal_wrap_right_to_left() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 1920, 0, 3840, 1080, false),
            make_monitor(2, 3840, 0, 5760, 1080, false),
        ]);

        // Right edge of monitor 2 → wrap to left edge of monitor 0
        let cursor = Point { x: 5759, y: 540 };
        let dest = topo.get_wrap_destination(2, cursor, EdgeType::Right);
        assert_eq!(dest.x, 0);
        assert_eq!(dest.y, 540);
    }

    // ── monitor_index_at_point ─────────────────────────────────────────

    #[test]
    fn monitor_index_at_point_finds_correct() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 1920, 0, 3840, 1080, false),
        ]);

        assert_eq!(topo.monitor_index_at_point(Point { x: 960, y: 540 }), Some(0));
        assert_eq!(topo.monitor_index_at_point(Point { x: 2500, y: 540 }), Some(1));
        assert_eq!(topo.monitor_index_at_point(Point { x: -100, y: 540 }), None);
    }

    // ── Empty monitors ─────────────────────────────────────────────────

    #[test]
    fn empty_monitors_no_crash() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[]);
        assert_eq!(topo.monitor_count(), 0);
        assert!(topo.outer_edges().is_empty());
    }

    // ── Three monitors vertical ────────────────────────────────────────

    #[test]
    fn three_monitors_vertical_wrap_top_to_bottom() {
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 1080, true),
            make_monitor(1, 0, 1080, 1920, 2160, false),
            make_monitor(2, 0, 2160, 1920, 3240, false),
        ]);

        // Top of monitor 0 → bottom of monitor 2
        let cursor = Point { x: 960, y: 0 };
        let dest = topo.get_wrap_destination(0, cursor, EdgeType::Top);
        assert_eq!(dest.y, 3239); // bottom-1 of monitor 2
        assert_eq!(dest.x, 960);
    }

    // ── Adjacency overlap requirement ──────────────────────────────────

    #[test]
    fn adjacency_requires_sufficient_overlap() {
        // Two monitors side-by-side but barely overlapping vertically (< tolerance)
        // Monitor 0: [0, 0, 1920, 60]
        // Monitor 1: [1920, 0, 3840, 60]
        // The vertical overlap is 60px, but the overlap_end > overlap_start + tolerance
        // requires overlap > 50, which 60 > 0 + 50 = true → adjacent
        let mut topo = MonitorTopology::new();
        topo.initialize(&[
            make_monitor(0, 0, 0, 1920, 60, true),
            make_monitor(1, 1920, 0, 3840, 60, false),
        ]);

        // Should be adjacent (overlap = 60 > tolerance = 50)
        assert!(!topo.edge_map.get(&(0, EdgeType::Right)).unwrap().is_outer);

        // Now with insufficient overlap
        let mut topo2 = MonitorTopology::new();
        topo2.initialize(&[
            make_monitor(0, 0, 0, 1920, 40, true),
            make_monitor(1, 1920, 0, 3840, 40, false),
        ]);

        // overlap = 40 ≤ tolerance = 50, so NOT adjacent → both are outer
        assert!(topo2.edge_map.get(&(0, EdgeType::Right)).unwrap().is_outer);
    }
}
