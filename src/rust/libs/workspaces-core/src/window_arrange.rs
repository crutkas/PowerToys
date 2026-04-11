//! Window arrangement helpers.
//! GAP AREA: C++ had no tests for WindowArranger distance/matching logic.

/// Distance between two points.
pub fn distance(x1: i32, y1: i32, x2: i32, y2: i32) -> f64 {
    let dx = (x2 - x1) as f64;
    let dy = (y2 - y1) as f64;
    (dx * dx + dy * dy).sqrt()
}

/// Center of a rect (x, y, width, height).
pub fn rect_center(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    (x + w / 2, y + h / 2)
}

/// Distance between centers of two rects.
pub fn rect_distance(x1: i32, y1: i32, w1: i32, h1: i32, x2: i32, y2: i32, w2: i32, h2: i32) -> f64 {
    let (cx1, cy1) = rect_center(x1, y1, w1, h1);
    let (cx2, cy2) = rect_center(x2, y2, w2, h2);
    distance(cx1, cy1, cx2, cy2)
}

/// Overlap area between two rects (x, y, w, h format). Returns 0 if no overlap.
pub fn overlap_area(x1: i32, y1: i32, w1: i32, h1: i32, x2: i32, y2: i32, w2: i32, h2: i32) -> i64 {
    let left = x1.max(x2);
    let top = y1.max(y2);
    let right = (x1 + w1).min(x2 + w2);
    let bottom = (y1 + h1).min(y2 + h2);
    if right > left && bottom > top {
        (right - left) as i64 * (bottom - top) as i64
    } else {
        0
    }
}

/// Find the index of the nearest target rect to the given source rect.
/// Returns None if targets is empty.
pub fn find_nearest(
    src_x: i32, src_y: i32, src_w: i32, src_h: i32,
    targets: &[(i32, i32, i32, i32)],
) -> Option<usize> {
    targets.iter().enumerate().min_by(|(_, a), (_, b)| {
        let da = rect_distance(src_x, src_y, src_w, src_h, a.0, a.1, a.2, a.3);
        let db = rect_distance(src_x, src_y, src_w, src_h, b.0, b.1, b.2, b.3);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    }).map(|(i, _)| i)
}

/// Find the index of the target with the most overlap with the source rect.
/// Returns None if no target overlaps.
pub fn find_best_overlap(
    src_x: i32, src_y: i32, src_w: i32, src_h: i32,
    targets: &[(i32, i32, i32, i32)],
) -> Option<usize> {
    let mut best_idx = None;
    let mut best_area: i64 = 0;
    for (i, t) in targets.iter().enumerate() {
        let area = overlap_area(src_x, src_y, src_w, src_h, t.0, t.1, t.2, t.3);
        if area > best_area {
            best_area = area;
            best_idx = Some(i);
        }
    }
    best_idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_zero() {
        assert_eq!(distance(0, 0, 0, 0), 0.0);
    }

    #[test]
    fn distance_horizontal() {
        assert_eq!(distance(0, 0, 3, 0), 3.0);
    }

    #[test]
    fn distance_diagonal() {
        let d = distance(0, 0, 3, 4);
        assert!((d - 5.0).abs() < 0.001);
    }

    #[test]
    fn rect_center_simple() {
        assert_eq!(rect_center(0, 0, 100, 200), (50, 100));
    }

    #[test]
    fn rect_center_offset() {
        assert_eq!(rect_center(100, 200, 50, 50), (125, 225));
    }

    #[test]
    fn overlap_full() {
        let area = overlap_area(0, 0, 100, 100, 0, 0, 100, 100);
        assert_eq!(area, 10_000);
    }

    #[test]
    fn overlap_partial() {
        let area = overlap_area(0, 0, 100, 100, 50, 50, 100, 100);
        assert_eq!(area, 2_500); // 50×50
    }

    #[test]
    fn overlap_none() {
        let area = overlap_area(0, 0, 50, 50, 100, 100, 50, 50);
        assert_eq!(area, 0);
    }

    #[test]
    fn overlap_adjacent_no_overlap() {
        let area = overlap_area(0, 0, 100, 100, 100, 0, 100, 100);
        assert_eq!(area, 0);
    }

    #[test]
    fn find_nearest_single() {
        let targets = vec![(100, 100, 50, 50)];
        assert_eq!(find_nearest(0, 0, 50, 50, &targets), Some(0));
    }

    #[test]
    fn find_nearest_picks_closest() {
        let targets = vec![
            (500, 500, 50, 50), // far
            (60, 60, 50, 50),   // close
            (200, 200, 50, 50), // medium
        ];
        assert_eq!(find_nearest(0, 0, 50, 50, &targets), Some(1));
    }

    #[test]
    fn find_nearest_empty() {
        assert_eq!(find_nearest(0, 0, 50, 50, &[]), None);
    }

    #[test]
    fn find_best_overlap_picks_largest() {
        let targets = vec![
            (80, 80, 50, 50),  // small overlap with (0,0,100,100)
            (25, 25, 100, 100), // large overlap
        ];
        assert_eq!(find_best_overlap(0, 0, 100, 100, &targets), Some(1));
    }

    #[test]
    fn find_best_overlap_no_overlap() {
        let targets = vec![(200, 200, 50, 50)];
        assert_eq!(find_best_overlap(0, 0, 50, 50, &targets), None);
    }
}
