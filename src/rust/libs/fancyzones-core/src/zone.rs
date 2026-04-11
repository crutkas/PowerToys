use crate::rect::Rect;

/// Maximum negative spacing allowed for zone coordinates.
pub const MAX_NEGATIVE_SPACING: i32 = -20;

/// Index type for zones (matches C++ ZoneIndex = int64_t).
pub type ZoneIndex = i64;

/// A set of zone indices.
pub type ZoneIndexSet = Vec<ZoneIndex>;

/// A zone: a rectangle with an index.
#[derive(Debug, Clone)]
pub struct Zone {
    rect: Rect,
    index: ZoneIndex,
}

impl Zone {
    pub fn new(rect: Rect, index: ZoneIndex) -> Self {
        Self { rect, index }
    }

    pub fn id(&self) -> ZoneIndex {
        self.index
    }

    pub fn is_valid(&self) -> bool {
        if self.index < 0 {
            return false;
        }
        let width = self.rect.right - self.rect.left;
        let height = self.rect.bottom - self.rect.top;
        self.rect.left >= MAX_NEGATIVE_SPACING
            && self.rect.right >= MAX_NEGATIVE_SPACING
            && self.rect.top >= MAX_NEGATIVE_SPACING
            && self.rect.bottom >= MAX_NEGATIVE_SPACING
            && width >= 0
            && height >= 0
    }

    pub fn get_zone_rect(&self) -> Rect {
        self.rect
    }

    pub fn get_zone_area(&self) -> i64 {
        self.rect.area()
    }
}

/// Bitmask representation of a ZoneIndexSet, supporting up to 128 zones.
/// part1 covers zones 0..=63, part2 covers zones 64..=127.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ZoneIndexSetBitmask {
    pub part1: u64,
    pub part2: u64,
}

impl ZoneIndexSetBitmask {
    /// In C++, `std::numeric_limits<ZoneIndex>::digits` for int64_t is 63.
    const DIGITS: ZoneIndex = 63;

    pub fn from_index_set(set: &ZoneIndexSet) -> Self {
        let mut bitmask = Self::default();
        for &zone_index in set {
            if zone_index <= Self::DIGITS {
                bitmask.part1 |= 1u64 << zone_index;
            } else {
                let index = zone_index - Self::DIGITS - 1;
                bitmask.part2 |= 1u64 << index;
            }
        }
        bitmask
    }

    pub fn to_index_set(&self) -> ZoneIndexSet {
        let mut set = ZoneIndexSet::new();
        if self.part1 != 0 {
            for i in 0..=Self::DIGITS {
                if (1u64 << i) & self.part1 != 0 {
                    set.push(i);
                }
            }
        }
        if self.part2 != 0 {
            for i in 0..=Self::DIGITS {
                if (1u64 << i) & self.part2 != 0 {
                    set.push(i + Self::DIGITS + 1);
                }
            }
        }
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Zone tests (from Zone.Spec.cpp) ----

    #[test]
    fn test_create_zone() {
        let rect = Rect::new(10, 10, 200, 200);
        let zone = Zone::new(rect, 1);
        assert!(zone.is_valid());
        assert_eq!(zone.get_zone_rect(), rect);
    }

    #[test]
    fn test_create_zone_zero_rect() {
        let rect = Rect::new(0, 0, 0, 0);
        let zone = Zone::new(rect, 1);
        assert!(zone.is_valid());
        assert_eq!(zone.get_zone_rect(), rect);
    }

    #[test]
    fn get_set_id() {
        let zone_id: ZoneIndex = 123;
        let zone = Zone::new(Rect::new(10, 10, 200, 200), zone_id);
        assert!(zone.is_valid());
        assert_eq!(zone.id(), zone_id);
    }

    #[test]
    fn invalid_id() {
        let zone = Zone::new(Rect::new(10, 10, 200, 200), -1);
        assert!(!zone.is_valid());
    }

    #[test]
    fn invalid_rect() {
        let zone = Zone::new(Rect::new(100, 100, 99, 101), 1);
        assert!(!zone.is_valid());
    }

    #[test]
    fn zone_area() {
        let zone = Zone::new(Rect::new(0, 0, 100, 50), 0);
        assert_eq!(zone.get_zone_area(), 5000);
    }

    #[test]
    fn zone_area_inverted() {
        let zone = Zone::new(Rect::new(100, 100, 50, 50), 0);
        assert_eq!(zone.get_zone_area(), 0);
    }

    // ---- ZoneIndexSetBitmask tests (from Layout.Spec.cpp ZoneIndexSetUnitTests) ----

    #[test]
    fn bitmask_from_index_set_test() {
        let set: ZoneIndexSet = vec![0, 64];
        let bitmask = ZoneIndexSetBitmask::from_index_set(&set);
        assert_eq!(bitmask.part1, 1u64);
        assert_eq!(bitmask.part2, 1u64);
    }

    #[test]
    fn bitmask_to_index_set() {
        let bitmask = ZoneIndexSetBitmask { part1: 1, part2: 1 };
        let set = bitmask.to_index_set();
        assert_eq!(set.len(), 2);
        assert_eq!(set[0], 0);
        assert_eq!(set[1], 64);
    }

    #[test]
    fn bitmask_convert_test() {
        let set: ZoneIndexSet = vec![53, 54, 55, 65, 66, 67];
        let bitmask = ZoneIndexSetBitmask::from_index_set(&set);
        let actual = bitmask.to_index_set();
        assert_eq!(set.len(), actual.len());
        for i in 0..set.len() {
            assert_eq!(set[i], actual[i]);
        }
    }

    #[test]
    fn bitmask_convert2_test() {
        let set: ZoneIndexSet = (0..128).collect();
        let bitmask = ZoneIndexSetBitmask::from_index_set(&set);
        let actual = bitmask.to_index_set();
        assert_eq!(set.len(), actual.len());
        for i in 0..set.len() {
            assert_eq!(set[i], actual[i]);
        }
    }
}
