use serde::{Deserialize, Serialize};
use std::fmt;

/// Simple rectangle struct replacing Win32 RECT.
/// Uses the same convention: left/top are inclusive, right/bottom are exclusive boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub fn area(&self) -> i64 {
        let w = (self.right - self.left).max(0) as i64;
        let h = (self.bottom - self.top).max(0) as i64;
        w * h
    }

    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        self.left <= x && x < self.right && self.top <= y && y < self.bottom
    }

    pub fn center_x(&self) -> f64 {
        0.5 * self.left as f64 + 0.5 * self.right as f64
    }

    pub fn center_y(&self) -> f64 {
        0.5 * self.top as f64 + 0.5 * self.bottom as f64
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rect({}, {}, {}, {})", self.left, self.top, self.right, self.bottom)
    }
}

/// Simple point struct replacing Win32 POINT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_width_height() {
        let r = Rect::new(10, 20, 110, 220);
        assert_eq!(r.width(), 100);
        assert_eq!(r.height(), 200);
    }

    #[test]
    fn rect_area() {
        let r = Rect::new(0, 0, 100, 50);
        assert_eq!(r.area(), 5000);
    }

    #[test]
    fn rect_area_zero() {
        let r = Rect::new(0, 0, 0, 0);
        assert_eq!(r.area(), 0);
    }

    #[test]
    fn rect_area_negative_dimensions() {
        let r = Rect::new(100, 100, 50, 50);
        assert_eq!(r.area(), 0);
    }

    #[test]
    fn rect_contains_point() {
        let r = Rect::new(0, 0, 100, 100);
        assert!(r.contains_point(0, 0));
        assert!(r.contains_point(50, 50));
        assert!(r.contains_point(99, 99));
        assert!(!r.contains_point(100, 100));
        assert!(!r.contains_point(-1, 0));
    }
}
