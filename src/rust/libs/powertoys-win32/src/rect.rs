//! Simple rectangle type for zone/monitor math.
//!
//! Platform-independent — no Win32 dependency. Matches the layout of
//! `RECT` (left, top, right, bottom) so it can be transmuted when needed.

/// Axis-aligned rectangle with left/top/right/bottom edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self { left, top, right, bottom }
    }

    /// Create from (x, y, width, height) — the format used in Workspaces data.
    pub const fn from_xywh(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        }
    }

    pub const fn width(&self) -> i32 {
        self.right - self.left
    }

    pub const fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub const fn area(&self) -> i64 {
        self.width() as i64 * self.height() as i64
    }

    pub const fn is_empty(&self) -> bool {
        self.left >= self.right || self.top >= self.bottom
    }

    /// Returns true if `point` (x, y) is inside this rect (inclusive of edges).
    pub const fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }

    /// Returns the intersection of two rects, or `None` if they don't overlap.
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let r = Rect {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        };
        if r.is_empty() { None } else { Some(r) }
    }

    /// Returns the smallest rect containing both `self` and `other`.
    pub fn union(&self, other: &Rect) -> Rect {
        Rect {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    /// Center point (x, y).
    pub const fn center(&self) -> (i32, i32) {
        ((self.left + self.right) / 2, (self.top + self.bottom) / 2)
    }

    /// Inset (shrink) all edges by `amount`. Can produce an empty rect.
    pub fn inset(&self, amount: i32) -> Rect {
        Rect {
            left: self.left + amount,
            top: self.top + amount,
            right: self.right - amount,
            bottom: self.bottom - amount,
        }
    }
}

impl std::fmt::Display for Rect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {}, {})", self.left, self.top, self.right, self.bottom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_dimensions() {
        let r = Rect::new(10, 20, 110, 220);
        assert_eq!(r.width(), 100);
        assert_eq!(r.height(), 200);
        assert_eq!(r.area(), 20_000);
    }

    #[test]
    fn from_xywh() {
        let r = Rect::from_xywh(100, 200, 300, 400);
        assert_eq!(r.left, 100);
        assert_eq!(r.top, 200);
        assert_eq!(r.right, 400);
        assert_eq!(r.bottom, 600);
        assert_eq!(r.width(), 300);
        assert_eq!(r.height(), 400);
    }

    #[test]
    fn empty_rect() {
        assert!(Rect::new(0, 0, 0, 0).is_empty());
        assert!(Rect::new(10, 10, 5, 5).is_empty()); // inverted
        assert!(!Rect::new(0, 0, 1, 1).is_empty());
    }

    #[test]
    fn contains_point() {
        let r = Rect::new(0, 0, 100, 100);
        assert!(r.contains_point(50, 50));
        assert!(r.contains_point(0, 0)); // edge
        assert!(r.contains_point(100, 100)); // edge
        assert!(!r.contains_point(-1, 50));
        assert!(!r.contains_point(101, 50));
    }

    #[test]
    fn intersect() {
        let a = Rect::new(0, 0, 100, 100);
        let b = Rect::new(50, 50, 150, 150);
        let i = a.intersect(&b).unwrap();
        assert_eq!(i, Rect::new(50, 50, 100, 100));
    }

    #[test]
    fn no_intersect() {
        let a = Rect::new(0, 0, 50, 50);
        let b = Rect::new(100, 100, 200, 200);
        assert!(a.intersect(&b).is_none());
    }

    #[test]
    fn union_rects() {
        let a = Rect::new(10, 20, 30, 40);
        let b = Rect::new(0, 0, 50, 50);
        assert_eq!(a.union(&b), Rect::new(0, 0, 50, 50));
    }

    #[test]
    fn center() {
        let r = Rect::new(0, 0, 100, 200);
        assert_eq!(r.center(), (50, 100));
    }

    #[test]
    fn inset() {
        let r = Rect::new(0, 0, 100, 100);
        let i = r.inset(10);
        assert_eq!(i, Rect::new(10, 10, 90, 90));
    }

    #[test]
    fn default_is_empty() {
        assert!(Rect::default().is_empty());
    }
}
