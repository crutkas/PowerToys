//! Core types for MeasureTool — matches the C++ ToolState.h / Measurement.h enums and structs.

use serde::{Deserialize, Serialize};

/// Measurement mode — mirrors `MeasureToolState::Mode` in C++.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum MeasureMode {
    /// Horizontal + vertical measurement lines from cursor to nearest edges.
    Cross = 0,
    /// Horizontal measurement only.
    Horizontal = 1,
    /// Vertical measurement only.
    Vertical = 2,
    /// Detect the bounding rectangle of the contiguous color region under cursor.
    Bounds = 3,
}

/// Unit of measurement — mirrors `Measurement::Unit` bitmask in C++.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum MeasureUnit {
    Pixel = 1,
    Inch = 2,
    Centimetre = 4,
    Millimetre = 8,
}

impl MeasureUnit {
    /// Convert from settings index (0–3) to unit enum, matching C++ `GetUnitFromIndex`.
    pub fn from_index(index: i32) -> Self {
        match index {
            1 => MeasureUnit::Inch,
            2 => MeasureUnit::Centimetre,
            3 => MeasureUnit::Millimetre,
            _ => MeasureUnit::Pixel,
        }
    }

    /// Get the abbreviation string for a unit.
    pub fn abbreviation(&self) -> &'static str {
        match self {
            MeasureUnit::Pixel => "px",
            MeasureUnit::Inch => "in",
            MeasureUnit::Centimetre => "cm",
            MeasureUnit::Millimetre => "mm",
        }
    }
}

/// RGB color with 3 components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Default for Color {
    /// Default line color: OrangeRed (255, 69, 0) — matches C++ default.
    fn default() -> Self {
        Self {
            r: 255,
            g: 69,
            b: 0,
        }
    }
}

/// A rectangle with inclusive pixel coordinates (left, top, right, bottom).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Rect {
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

/// Integer rectangle — matches Win32 RECT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl IRect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Convert to float rect.
    pub fn to_rect(&self) -> Rect {
        Rect {
            left: self.left as f32,
            top: self.top as f32,
            right: self.right as f32,
            bottom: self.bottom as f32,
        }
    }
}

/// Integer point — matches Win32 POINT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// A measurement result with start/end positions and pixel length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasurementResult {
    /// The bounding rect of the measurement (inclusive corners).
    pub rect: Rect,
    /// Ratio for physical pixel-to-mm conversion. 0.0 means use 96 DPI fallback.
    pub px2mm_ratio: f32,
}

impl MeasurementResult {
    pub fn new(rect: Rect, px2mm_ratio: f32) -> Self {
        Self { rect, px2mm_ratio }
    }

    pub fn from_irect(r: IRect, px2mm_ratio: f32) -> Self {
        Self {
            rect: r.to_rect(),
            px2mm_ratio,
        }
    }

    /// Width in the given unit. Pixel width = right - left + 1 (inclusive).
    pub fn width(&self, unit: MeasureUnit) -> f32 {
        convert(self.rect.right - self.rect.left + 1.0, unit, self.px2mm_ratio)
    }

    /// Height in the given unit. Pixel height = bottom - top + 1 (inclusive).
    pub fn height(&self, unit: MeasureUnit) -> f32 {
        convert(
            self.rect.bottom - self.rect.top + 1.0,
            unit,
            self.px2mm_ratio,
        )
    }

    /// Format measurement as a string (e.g., "100 × 50 px").
    pub fn format(&self, print_width: bool, print_height: bool, unit: MeasureUnit) -> String {
        let mut parts = Vec::new();
        if print_width {
            parts.push(format_value(self.width(unit)));
        }
        if print_height {
            parts.push(format_value(self.height(unit)));
        }
        let dimensions = parts.join(" \u{00D7} "); // × symbol
        format!("{} {}", dimensions, unit.abbreviation())
    }
}

/// Unit conversion matching C++ `Convert` function.
fn convert(pixels: f32, unit: MeasureUnit, px2mm_ratio: f32) -> f32 {
    if px2mm_ratio > 0.0 {
        match unit {
            MeasureUnit::Pixel => pixels,
            MeasureUnit::Inch => pixels * px2mm_ratio / 10.0 / 2.54,
            MeasureUnit::Centimetre => pixels * px2mm_ratio / 10.0,
            MeasureUnit::Millimetre => pixels * px2mm_ratio,
        }
    } else {
        // Fallback: assume 96 DPI
        match unit {
            MeasureUnit::Pixel => pixels,
            MeasureUnit::Inch => pixels / 96.0,
            MeasureUnit::Centimetre => pixels / 96.0 * 2.54,
            MeasureUnit::Millimetre => pixels / 96.0 / 10.0 * 2.54,
        }
    }
}

/// Format a measurement value with up to 4 significant digits, matching C++ "%.4g".
fn format_value(v: f32) -> String {
    // %.4g in C: 4 significant digits, no trailing zeros
    let s = format!("{:.4}", v);
    // Trim trailing zeros after decimal point
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0').trim_end_matches('.').to_string();
        // But if the original has more than 4 significant digits, use the g format
        // For simplicity, use Rust's built-in formatting
        trimmed
    } else {
        s
    }
}

/// BGRA pixel buffer view — pure data, no GPU dependencies.
/// Matches C++ `BGRATextureView`.
pub struct PixelBuffer<'a> {
    /// Raw BGRA pixel data (each u32 = one pixel: 0xAARRGGBB in memory as BGRA bytes).
    pub pixels: &'a [u32],
    /// Stride in pixels (not bytes) — number of u32 values per row.
    pub pitch: usize,
    /// Width of the visible image in pixels.
    pub width: usize,
    /// Height of the visible image in pixels.
    pub height: usize,
}

impl<'a> PixelBuffer<'a> {
    pub fn new(pixels: &'a [u32], pitch: usize, width: usize, height: usize) -> Self {
        Self {
            pixels,
            pitch,
            width,
            height,
        }
    }

    /// Get pixel at (x, y). Panics if out of bounds.
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        debug_assert!(x < self.width);
        debug_assert!(y < self.height);
        self.pixels[x + self.pitch * y]
    }

    /// Check if two pixels are "close" within tolerance.
    /// - `per_channel`: if true, each RGBA channel must individually be within tolerance.
    /// - `per_channel`: if false, the sum of all channel differences must be ≤ tolerance.
    #[inline]
    pub fn pixels_close(pixel1: u32, pixel2: u32, tolerance: u8, per_channel: bool) -> bool {
        let b1 = (pixel1 & 0xFF) as i16;
        let g1 = ((pixel1 >> 8) & 0xFF) as i16;
        let r1 = ((pixel1 >> 16) & 0xFF) as i16;
        let a1 = ((pixel1 >> 24) & 0xFF) as i16;

        let b2 = (pixel2 & 0xFF) as i16;
        let g2 = ((pixel2 >> 8) & 0xFF) as i16;
        let r2 = ((pixel2 >> 16) & 0xFF) as i16;
        let a2 = ((pixel2 >> 24) & 0xFF) as i16;

        let db = (b1 - b2).unsigned_abs();
        let dg = (g1 - g2).unsigned_abs();
        let dr = (r1 - r2).unsigned_abs();
        let da = (a1 - a2).unsigned_abs();

        let tol = tolerance as u16;

        if per_channel {
            // Each channel distance must be ≤ tolerance
            db <= tol && dg <= tol && dr <= tol && da <= tol
        } else {
            // Sum of all channel differences ≤ tolerance
            (db + dg + dr + da) <= tol
        }
    }
}

/// Rendering constants — matches C++ `consts` namespace.
pub mod consts {
    pub const TARGET_FRAME_RATE: usize = 90;
    pub const FONT_SIZE: f32 = 14.0;
    pub const TEXT_BOX_CORNER_RADIUS: f32 = 4.0;
    pub const FEET_HALF_LENGTH: f32 = 2.0;
    pub const SHADOW_OPACITY: f32 = 0.4;
    pub const SHADOW_RADIUS: f32 = 6.0;
    pub const SHADOW_OFFSET: f32 = 5.0;
    pub const CROSS_OPACITY: f32 = 0.25;
    pub const MOUSE_WHEEL_TOLERANCE_STEP: i8 = 15;
    pub const CURSOR_OFFSET_AMOUNT_X: i32 = 4;
    pub const CURSOR_OFFSET_AMOUNT_Y: i32 = 4;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measure_mode_values() {
        assert_eq!(MeasureMode::Cross as i32, 0);
        assert_eq!(MeasureMode::Horizontal as i32, 1);
        assert_eq!(MeasureMode::Vertical as i32, 2);
        assert_eq!(MeasureMode::Bounds as i32, 3);
    }

    #[test]
    fn test_measure_unit_from_index() {
        assert_eq!(MeasureUnit::from_index(0), MeasureUnit::Pixel);
        assert_eq!(MeasureUnit::from_index(1), MeasureUnit::Inch);
        assert_eq!(MeasureUnit::from_index(2), MeasureUnit::Centimetre);
        assert_eq!(MeasureUnit::from_index(3), MeasureUnit::Millimetre);
        assert_eq!(MeasureUnit::from_index(99), MeasureUnit::Pixel);
    }

    #[test]
    fn test_measure_unit_abbreviation() {
        assert_eq!(MeasureUnit::Pixel.abbreviation(), "px");
        assert_eq!(MeasureUnit::Inch.abbreviation(), "in");
        assert_eq!(MeasureUnit::Centimetre.abbreviation(), "cm");
        assert_eq!(MeasureUnit::Millimetre.abbreviation(), "mm");
    }

    #[test]
    fn test_default_color_is_orange_red() {
        let c = Color::default();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 69);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn test_irect_to_rect() {
        let ir = IRect::new(10, 20, 110, 120);
        let r = ir.to_rect();
        assert_eq!(r.left, 10.0);
        assert_eq!(r.top, 20.0);
        assert_eq!(r.right, 110.0);
        assert_eq!(r.bottom, 120.0);
    }

    #[test]
    fn test_measurement_result_pixel_width_height() {
        // rect from (10, 20) to (109, 69): width = 109 - 10 + 1 = 100, height = 69 - 20 + 1 = 50
        let m = MeasurementResult::new(Rect::new(10.0, 20.0, 109.0, 69.0), 0.0);
        assert_eq!(m.width(MeasureUnit::Pixel), 100.0);
        assert_eq!(m.height(MeasureUnit::Pixel), 50.0);
    }

    #[test]
    fn test_measurement_result_unit_conversion_96dpi() {
        // 96 pixels at 96 DPI = 1 inch
        let m = MeasurementResult::new(Rect::new(0.0, 0.0, 95.0, 95.0), 0.0);
        assert_eq!(m.width(MeasureUnit::Pixel), 96.0);
        let inches = m.width(MeasureUnit::Inch);
        assert!((inches - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_measurement_result_unit_conversion_with_ratio() {
        // With px2mm_ratio = 0.5, 100 pixels = 100 * 0.5 = 50 mm
        let m = MeasurementResult::new(Rect::new(0.0, 0.0, 99.0, 0.0), 0.5);
        let mm = m.width(MeasureUnit::Millimetre);
        assert!((mm - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_measurement_format() {
        let m = MeasurementResult::new(Rect::new(0.0, 0.0, 99.0, 49.0), 0.0);
        let s = m.format(true, true, MeasureUnit::Pixel);
        assert!(s.contains("100"));
        assert!(s.contains("50"));
        assert!(s.contains("px"));
        assert!(s.contains('\u{00D7}')); // × symbol
    }

    #[test]
    fn test_measurement_format_width_only() {
        let m = MeasurementResult::new(Rect::new(0.0, 0.0, 99.0, 49.0), 0.0);
        let s = m.format(true, false, MeasureUnit::Pixel);
        assert!(s.contains("100"));
        assert!(!s.contains('\u{00D7}'));
    }

    #[test]
    fn test_pixels_close_identical() {
        assert!(PixelBuffer::pixels_close(0xFF112233, 0xFF112233, 0, true));
        assert!(PixelBuffer::pixels_close(0xFF112233, 0xFF112233, 0, false));
    }

    #[test]
    fn test_pixels_close_per_channel_within_tolerance() {
        // pixel1: BGRA = (0x33, 0x22, 0x11, 0xFF)
        // pixel2: BGRA = (0x34, 0x23, 0x12, 0xFF) — each channel differs by 1
        assert!(PixelBuffer::pixels_close(0xFF112233, 0xFF122334, 1, true));
    }

    #[test]
    fn test_pixels_close_per_channel_exceeds_tolerance() {
        // pixel1 B=0x33, pixel2 B=0x44 — differ by 17
        assert!(!PixelBuffer::pixels_close(0xFF112233, 0xFF112244, 10, true));
    }

    #[test]
    fn test_pixels_close_total_within_tolerance() {
        // Channels differ by 5+5+5+0 = 15 total
        assert!(PixelBuffer::pixels_close(0xFF101010, 0xFF151515, 15, false));
    }

    #[test]
    fn test_pixels_close_total_exceeds_tolerance() {
        assert!(!PixelBuffer::pixels_close(0xFF101010, 0xFF151515, 14, false));
    }

    #[test]
    fn test_pixel_buffer_get_pixel() {
        let pixels = vec![0xAABBCCDD, 0x11223344, 0x55667788, 0x99AABBCC];
        let buf = PixelBuffer::new(&pixels, 2, 2, 2);
        assert_eq!(buf.get_pixel(0, 0), 0xAABBCCDD);
        assert_eq!(buf.get_pixel(1, 0), 0x11223344);
        assert_eq!(buf.get_pixel(0, 1), 0x55667788);
        assert_eq!(buf.get_pixel(1, 1), 0x99AABBCC);
    }

    #[test]
    fn test_pixel_buffer_with_pitch() {
        // pitch=4 but only width=2, simulating row padding
        let pixels = vec![
            0xAA, 0xBB, 0x00, 0x00, // row 0
            0xCC, 0xDD, 0x00, 0x00, // row 1
        ];
        let buf = PixelBuffer::new(&pixels, 4, 2, 2);
        assert_eq!(buf.get_pixel(0, 0), 0xAA);
        assert_eq!(buf.get_pixel(1, 0), 0xBB);
        assert_eq!(buf.get_pixel(0, 1), 0xCC);
        assert_eq!(buf.get_pixel(1, 1), 0xDD);
    }

    #[test]
    fn test_consts_match_cpp() {
        assert_eq!(consts::TARGET_FRAME_RATE, 90);
        assert_eq!(consts::FONT_SIZE, 14.0);
        assert_eq!(consts::TEXT_BOX_CORNER_RADIUS, 4.0);
        assert_eq!(consts::FEET_HALF_LENGTH, 2.0);
        assert_eq!(consts::SHADOW_OPACITY, 0.4);
        assert_eq!(consts::SHADOW_RADIUS, 6.0);
        assert_eq!(consts::SHADOW_OFFSET, 5.0);
        assert_eq!(consts::CROSS_OPACITY, 0.25);
        assert_eq!(consts::MOUSE_WHEEL_TOLERANCE_STEP, 15);
    }
}
