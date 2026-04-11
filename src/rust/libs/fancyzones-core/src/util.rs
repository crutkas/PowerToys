use crate::rect::Rect;

/// Hex color string to RGB tuple (r, g, b).
/// Accepts "#RGB" (6 hex digits) and "#AARRGGBB" (8 hex digits) formats.
/// Returns (255, 255, 255) on invalid input.
pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim().trim_start_matches('#');
    match u64::from_str_radix(hex, 16) {
        Ok(val) => {
            let r = ((val & 0xFF0000) >> 16) as u8;
            let g = ((val & 0xFF00) >> 8) as u8;
            let b = (val & 0xFF) as u8;
            (r, g, b)
        }
        Err(_) => (255, 255, 255),
    }
}

/// Order monitors using a topological sort based on blocking relationships.
/// A monitor i "blocks" monitor j if i.top < j.bottom AND i.left < j.right.
/// Among unblocked monitors, pick the one with lexicographically smallest (top, left).
///
/// The `monitors` parameter is a vector of (ID, Rect) pairs where ID is a unique identifier.
pub fn order_monitors<T: Clone>(monitors: &mut Vec<(T, Rect)>) {
    let n = monitors.len();
    if n <= 1 {
        return;
    }

    // blocking[i][j] = true means monitor i should go before monitor j
    let mut blocking = vec![vec![false; n]; n];
    let mut blocking_count = vec![0usize; n];

    for i in 0..n {
        let ri = monitors[i].1;
        for j in 0..n {
            if i == j {
                continue;
            }
            let rj = monitors[j].1;
            blocking[i][j] = ri.top < rj.bottom && ri.left < rj.right;
            if blocking[i][j] {
                blocking_count[j] += 1;
            }
        }
    }

    let mut used = vec![false; n];
    let mut sorted = Vec::with_capacity(n);

    for _iteration in 0..n {
        // Find unblocked candidates
        let mut candidates: Vec<usize> = Vec::new();
        for i in 0..n {
            if blocking_count[i] == 0 && !used[i] {
                candidates.push(i);
            }
        }

        // Fallback: use all unused
        if candidates.is_empty() {
            for i in 0..n {
                if !used[i] {
                    candidates.push(i);
                }
            }
        }

        // Pick lexicographically smallest by (top, left)
        let mut smallest = candidates[0];
        for &c in &candidates[1..] {
            let (ct, cl) = (monitors[c].1.top, monitors[c].1.left);
            let (st, sl) = (monitors[smallest].1.top, monitors[smallest].1.left);
            if (ct, cl) < (st, sl) {
                smallest = c;
            }
        }

        used[smallest] = true;
        sorted.push(monitors[smallest].clone());

        for i in 0..n {
            if blocking[smallest][i] {
                blocking_count[i] -= 1;
            }
        }
    }

    *monitors = sorted;
}

/// Choose the next zone by position using an ellipse-based distance metric.
/// Returns the index of the closest zone in the given direction, or `zone_rects.len()` if none found.
///
/// `vk_code` should be one of `VK_UP`, `VK_DOWN`, `VK_LEFT`, `VK_RIGHT`.
pub const VK_UP: u32 = 0x26;
pub const VK_DOWN: u32 = 0x28;
pub const VK_LEFT: u32 = 0x25;
pub const VK_RIGHT: u32 = 0x27;

pub fn choose_next_zone_by_position(vk_code: u32, window_rect: Rect, zone_rects: &[Rect]) -> usize {
    let invalid_result = zone_rects.len();
    let inf: f64 = 1e100;
    let eccentricity: f64 = 2.0;

    let rect_center = |r: &Rect| -> (f64, f64) {
        (0.5 * r.left as f64 + 0.5 * r.right as f64,
         0.5 * r.top as f64 + 0.5 * r.bottom as f64)
    };

    // Complex number multiplication: (a+bi)(c-di) = (ac+bd) + (bc-ad)i
    // conj: (c+di) -> (c-di)
    // We compute scalar product = Re(direction * conj(zone_direction))
    let distance = |dir: (f64, f64), zone_dir: (f64, f64)| -> f64 {
        // scalar_product = Re(dir * conj(zone_dir)) = dir.0 * zone_dir.0 + dir.1 * zone_dir.1
        let scalar_product = dir.0 * zone_dir.0 + dir.1 * zone_dir.1;
        if scalar_product <= 0.0 {
            return inf;
        }

        let zone_abs = (zone_dir.0 * zone_dir.0 + zone_dir.1 * zone_dir.1).sqrt();
        if zone_abs == 0.0 {
            return inf;
        }

        let cos_angle = scalar_product / zone_abs;
        let cos_angle_clamped = cos_angle.clamp(-1.0, 1.0);
        let tan_angle = cos_angle_clamped.acos().tan().abs();

        if tan_angle > 10.0 {
            return inf;
        }

        let intersect_y = 2.0 * eccentricity / (1.0 + eccentricity * eccentricity * tan_angle * tan_angle);
        let dist_estimate = scalar_product / intersect_y;

        if dist_estimate.is_finite() {
            dist_estimate
        } else {
            inf
        }
    };

    let window_center = rect_center(&window_rect);

    let direction: (f64, f64) = match vk_code {
        VK_UP => (0.0, -1.0),
        VK_DOWN => (0.0, 1.0),
        VK_LEFT => (-1.0, 0.0),
        VK_RIGHT => (1.0, 0.0),
        _ => return invalid_result,
    };

    let mut closest_idx = invalid_result;
    let mut smallest_distance = inf;

    for (i, zone_rect) in zone_rects.iter().enumerate() {
        let mut center = rect_center(zone_rect);
        // Offset slightly to differentiate overlapping zones (real part only, matching C++)
        center.0 += 0.001 * (i + 1) as f64;

        let zone_dir = (center.0 - window_center.0, center.1 - window_center.1);
        let dist = distance(direction, zone_dir);
        if dist < smallest_distance {
            smallest_distance = dist;
            closest_idx = i;
        }
    }

    closest_idx
}

/// Prepare a window rect for cycling by shifting it to the opposite edge of the work area.
pub fn prepare_rect_for_cycling(mut window_rect: Rect, work_area: Rect, vk_code: u32) -> Rect {
    let (dx, dy) = match vk_code {
        VK_UP => (0, work_area.bottom - work_area.top),
        VK_DOWN => (0, work_area.top - work_area.bottom),
        VK_LEFT => (work_area.right - work_area.left, 0),
        VK_RIGHT => (work_area.left - work_area.right, 0),
        _ => (0, 0),
    };
    window_rect.left += dx;
    window_rect.right += dx;
    window_rect.top += dy;
    window_rect.bottom += dy;
    window_rect
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- HexToRGB tests (from Util.Spec.cpp) ----

    #[test]
    fn test_hex_to_rgb_rgb() {
        let (r, g, b) = hex_to_rgb("#A3F6FF");
        assert_eq!((r, g, b), (163, 246, 255));
    }

    #[test]
    fn test_hex_to_rgb_argb() {
        let (r, g, b) = hex_to_rgb("#FFA3F6FF");
        assert_eq!((r, g, b), (163, 246, 255));
    }

    #[test]
    fn test_hex_to_rgb_invalid() {
        let (r, g, b) = hex_to_rgb("zzz");
        assert_eq!((r, g, b), (255, 255, 255));
    }

    // ---- Monitor ordering tests (from Util.Spec.cpp) ----

    fn test_monitor_set_permutations(expected: &[(u64, Rect)]) {
        let n = expected.len();
        let mut perm: Vec<(u64, Rect)> = expected.to_vec();

        // Test all permutations (or subset for large sets)
        loop {
            let mut copy = perm.clone();
            order_monitors(&mut copy);
            for i in 0..n {
                assert_eq!(expected[i].0, copy[i].0, "Monitor ordering mismatch at position {}", i);
                assert_eq!(expected[i].1, copy[i].1, "Rect mismatch at position {}", i);
            }
            if !next_permutation_by_first(&mut perm) {
                break;
            }
        }
    }

    fn next_permutation_by_first(data: &mut [(u64, Rect)]) -> bool {
        let n = data.len();
        if n <= 1 {
            return false;
        }
        let mut i = n - 1;
        while i > 0 && data[i - 1].0 >= data[i].0 {
            i -= 1;
        }
        if i == 0 {
            return false;
        }
        let mut j = n - 1;
        while data[j].0 <= data[i - 1].0 {
            j -= 1;
        }
        data.swap(i - 1, j);
        data[i..].reverse();
        true
    }

    fn test_monitor_set_permutations_offsets(monitors: &[(u64, Rect)]) {
        let offsets = [-3000, -2000, -1000, 0, 1000, 2000, 3000];
        for &ox in &offsets {
            for &oy in &offsets {
                let shifted: Vec<(u64, Rect)> = monitors.iter().map(|(id, r)| {
                    (*id, Rect::new(r.left + ox, r.top + oy, r.right + ox, r.bottom + oy))
                }).collect();
                test_monitor_set_permutations(&shifted);
            }
        }
    }

    #[test]
    fn test_monitor_ordering_01() {
        let monitors = vec![
            (1, Rect::new(0, 200, 1600, 1100)),
            (2, Rect::new(1600, 100, 3300, 1100)),
            (3, Rect::new(3300, 0, 5100, 1100)),
        ];
        test_monitor_set_permutations_offsets(&monitors);
    }

    #[test]
    fn test_monitor_ordering_02() {
        let monitors = vec![
            (1, Rect::new(0, 0, 1600, 900)),
            (2, Rect::new(1600, 0, 3200, 900)),
            (3, Rect::new(3200, 0, 4800, 900)),
        ];
        test_monitor_set_permutations_offsets(&monitors);
    }

    #[test]
    fn test_monitor_ordering_03() {
        let monitors = vec![
            (1, Rect::new(0, 0, 1800, 1100)),
            (2, Rect::new(1800, 100, 3500, 1100)),
            (3, Rect::new(3500, 200, 5100, 1100)),
        ];
        test_monitor_set_permutations_offsets(&monitors);
    }

    #[test]
    fn test_monitor_ordering_04() {
        let monitors = vec![
            (1, Rect::new(0, 0, 1600, 900)),
            (2, Rect::new(1600, 0, 3300, 1000)),
            (3, Rect::new(3300, 0, 5100, 1100)),
        ];
        test_monitor_set_permutations_offsets(&monitors);
    }

    #[test]
    fn test_monitor_ordering_05() {
        let monitors = vec![
            (1, Rect::new(0, 0, 1600, 900)),
            (2, Rect::new(1600, 0, 3200, 900)),
            (3, Rect::new(3200, 0, 4800, 900)),
        ];
        test_monitor_set_permutations_offsets(&monitors);
    }

    #[test]
    fn test_monitor_ordering_06() {
        let monitors = vec![
            (1, Rect::new(0, 0, 1800, 1100)),
            (2, Rect::new(1800, 0, 3500, 1000)),
            (3, Rect::new(3500, 0, 5100, 900)),
        ];
        test_monitor_set_permutations_offsets(&monitors);
    }

    #[test]
    fn test_monitor_ordering_07() {
        let monitors = vec![
            (1, Rect::new(100, 0, 1700, 900)),
            (2, Rect::new(0, 900, 1800, 1800)),
            (3, Rect::new(100, 1800, 1700, 2700)),
        ];
        test_monitor_set_permutations_offsets(&monitors);
    }

    #[test]
    fn test_monitor_ordering_08() {
        let monitors = vec![
            (1, Rect::new(0, 0, 600, 400)),
            (2, Rect::new(600, 0, 1200, 400)),
            (3, Rect::new(1200, 0, 1800, 400)),
            (4, Rect::new(0, 400, 900, 800)),
            (5, Rect::new(900, 400, 1800, 800)),
        ];
        test_monitor_set_permutations_offsets(&monitors);
    }

    #[test]
    fn test_monitor_ordering_09() {
        let monitors = vec![
            (1, Rect::new(0, 0, 400, 300)),
            (2, Rect::new(400, 0, 800, 300)),
            (3, Rect::new(800, 0, 1200, 300)),
            (4, Rect::new(0, 300, 400, 600)),
            (5, Rect::new(400, 300, 800, 600)),
            (6, Rect::new(800, 300, 1200, 600)),
            (7, Rect::new(0, 600, 400, 900)),
            (8, Rect::new(400, 600, 800, 900)),
            (9, Rect::new(800, 600, 1200, 900)),
        ];

        // Test only rotations for 9 monitors (full permutation too expensive)
        for start in 0..9 {
            let mut rotated = monitors.clone();
            rotated.rotate_left(start);
            let mut copy = rotated;
            order_monitors(&mut copy);
            for i in 0..9 {
                assert_eq!(monitors[i].0, copy[i].0, "Rotation {} mismatch at {}", start, i);
                assert_eq!(monitors[i].1, copy[i].1);
            }
        }
    }

    #[test]
    fn test_monitor_ordering_10() {
        let monitors = vec![
            (1, Rect::new(0, 0, 900, 400)),
            (2, Rect::new(900, 0, 1800, 400)),
            (3, Rect::new(0, 400, 600, 800)),
            (4, Rect::new(600, 400, 1200, 800)),
            (5, Rect::new(1200, 400, 1800, 800)),
        ];
        test_monitor_set_permutations_offsets(&monitors);
    }

    #[test]
    fn test_monitor_ordering_11() {
        // Random/overlapping monitors — just verify determinism
        let monitors = vec![
            (1, Rect::new(410, 630, 988, 631)),
            (2, Rect::new(302, 189, 550, 714)),
            (3, Rect::new(158, 115, 657, 499)),
            (4, Rect::new(341, 340, 723, 655)),
            (5, Rect::new(433, 393, 846, 544)),
        ];

        let mut first_time = monitors.clone();
        order_monitors(&mut first_time);

        let mut perm = monitors.clone();
        loop {
            let mut copy = perm.clone();
            order_monitors(&mut copy);
            for i in 0..5 {
                assert_eq!(first_time[i].0, copy[i].0);
                assert_eq!(first_time[i].1, copy[i].1);
            }
            if !next_permutation_by_first(&mut perm) {
                break;
            }
        }
    }

    // ---- ChooseNextZoneByPosition tests ----

    #[test]
    fn choose_next_zone_right() {
        // Window is between two zones, to the left of zone 1
        let window = Rect::new(50, 0, 150, 100);
        let zones = vec![
            Rect::new(200, 0, 300, 100),
            Rect::new(400, 0, 500, 100),
        ];
        let result = choose_next_zone_by_position(VK_RIGHT, window, &zones);
        assert_eq!(result, 0); // nearest zone to the right
    }

    #[test]
    fn choose_next_zone_left() {
        let window = Rect::new(350, 0, 450, 100);
        let zones = vec![
            Rect::new(0, 0, 100, 100),
            Rect::new(200, 0, 300, 100),
        ];
        let result = choose_next_zone_by_position(VK_LEFT, window, &zones);
        assert_eq!(result, 1); // nearest zone to the left
    }

    #[test]
    fn choose_next_zone_down() {
        let window = Rect::new(0, 0, 100, 100);
        let zones = vec![
            Rect::new(0, 200, 100, 300),
            Rect::new(0, 400, 100, 500),
        ];
        let result = choose_next_zone_by_position(VK_DOWN, window, &zones);
        assert_eq!(result, 0);
    }

    #[test]
    fn choose_next_zone_up() {
        let window = Rect::new(0, 400, 100, 500);
        let zones = vec![
            Rect::new(0, 0, 100, 100),
            Rect::new(0, 200, 100, 300),
        ];
        let result = choose_next_zone_by_position(VK_UP, window, &zones);
        assert_eq!(result, 1); // nearest zone upward
    }

    #[test]
    fn choose_next_zone_no_match() {
        // No zones at all
        let window = Rect::new(0, 0, 100, 100);
        let zones: Vec<Rect> = vec![];
        let result = choose_next_zone_by_position(VK_RIGHT, window, &zones);
        assert_eq!(result, zones.len()); // invalid
    }

    #[test]
    fn prepare_rect_for_cycling_left() {
        let window = Rect::new(0, 0, 100, 100);
        let work_area = Rect::new(0, 0, 1920, 1080);
        let result = prepare_rect_for_cycling(window, work_area, VK_LEFT);
        assert_eq!(result, Rect::new(1920, 0, 2020, 100));
    }

    #[test]
    fn prepare_rect_for_cycling_right() {
        let window = Rect::new(1820, 0, 1920, 100);
        let work_area = Rect::new(0, 0, 1920, 1080);
        let result = prepare_rect_for_cycling(window, work_area, VK_RIGHT);
        assert_eq!(result, Rect::new(-100, 0, 0, 100));
    }
}
