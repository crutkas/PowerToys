use serde::{Deserialize, Serialize};

/// Crosshair orientation matching the C++ `CrosshairsOrientation` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum CrosshairsOrientation {
    Both = 0,
    VerticalOnly = 1,
    HorizontalOnly = 2,
}

impl Default for CrosshairsOrientation {
    fn default() -> Self {
        Self::Both
    }
}

/// ARGB color matching the WinRT `Windows::UI::Color` layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// All configurable crosshair settings with C++ defaults from `InclusiveCrosshairs.h`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub color: Color,
    pub border_color: Color,
    pub radius: i32,
    pub thickness: i32,
    pub opacity: i32,
    pub border_size: i32,
    pub auto_hide: bool,
    pub is_fixed_length_enabled: bool,
    pub fixed_length: i32,
    pub orientation: CrosshairsOrientation,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            color: Color { a: 255, r: 255, g: 0, b: 0 },
            border_color: Color { a: 255, r: 255, g: 255, b: 255 },
            radius: 20,
            thickness: 5,
            opacity: 75,
            border_size: 1,
            auto_hide: false,
            is_fixed_length_enabled: false,
            fixed_length: 1,
            orientation: CrosshairsOrientation::Both,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Enum discriminant tests ---

    #[test]
    fn both_discriminant_is_zero() {
        assert_eq!(CrosshairsOrientation::Both as i32, 0);
    }

    #[test]
    fn vertical_only_discriminant_is_one() {
        assert_eq!(CrosshairsOrientation::VerticalOnly as i32, 1);
    }

    #[test]
    fn horizontal_only_discriminant_is_two() {
        assert_eq!(CrosshairsOrientation::HorizontalOnly as i32, 2);
    }

    // --- Settings default tests ---

    #[test]
    fn default_color() {
        let s = Settings::default();
        assert_eq!(s.color, Color { a: 255, r: 255, g: 0, b: 0 });
    }

    #[test]
    fn default_border_color() {
        let s = Settings::default();
        assert_eq!(s.border_color, Color { a: 255, r: 255, g: 255, b: 255 });
    }

    #[test]
    fn default_radius() {
        assert_eq!(Settings::default().radius, 20);
    }

    #[test]
    fn default_thickness() {
        assert_eq!(Settings::default().thickness, 5);
    }

    #[test]
    fn default_opacity() {
        assert_eq!(Settings::default().opacity, 75);
    }

    #[test]
    fn default_border_size() {
        assert_eq!(Settings::default().border_size, 1);
    }

    #[test]
    fn default_auto_hide() {
        assert!(!Settings::default().auto_hide);
    }

    #[test]
    fn default_is_fixed_length_enabled() {
        assert!(!Settings::default().is_fixed_length_enabled);
    }

    #[test]
    fn default_fixed_length() {
        assert_eq!(Settings::default().fixed_length, 1);
    }

    #[test]
    fn default_orientation() {
        assert_eq!(Settings::default().orientation, CrosshairsOrientation::Both);
    }
}
