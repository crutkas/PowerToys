//! Measurement calculation from cursor position and detected edges.
//!
//! Given a cursor position, a pixel buffer, and the current mode, computes
//! the measurement lines (horizontal, vertical, or both) with pixel dimensions.

use crate::edge_detector::detect_edges;
use crate::types::{IRect, MeasureMode, MeasurementResult, PixelBuffer, Point, Rect};

/// Compute measurement lines for the given mode at the cursor position.
///
/// Returns a `MeasurementResult` with the bounding rect of the measurement.
///
/// - **Cross**: detect edges in all 4 directions, return full bounding rect
/// - **Horizontal**: detect left/right edges only (top/bottom = cursor y)
/// - **Vertical**: detect top/bottom edges only (left/right = cursor x)
/// - **Bounds**: detect all 4 edges (same as Cross for edge detection)
pub fn compute_measurement(
    buffer: &PixelBuffer,
    cursor: Point,
    mode: MeasureMode,
    per_channel: bool,
    tolerance: u8,
    px2mm_ratio: f32,
) -> MeasurementResult {
    let edges = detect_edges(buffer, cursor, per_channel, tolerance);

    let rect = match mode {
        MeasureMode::Cross | MeasureMode::Bounds => edges,
        MeasureMode::Horizontal => IRect::new(edges.left, cursor.y, edges.right, cursor.y),
        MeasureMode::Vertical => IRect::new(cursor.x, edges.top, cursor.x, edges.bottom),
    };

    MeasurementResult::from_irect(rect, px2mm_ratio)
}

/// For Cross mode, compute separate horizontal and vertical measurements.
///
/// Returns (horizontal_measurement, vertical_measurement).
pub fn compute_cross_measurements(
    buffer: &PixelBuffer,
    cursor: Point,
    per_channel: bool,
    tolerance: u8,
    px2mm_ratio: f32,
) -> (MeasurementResult, MeasurementResult) {
    let edges = detect_edges(buffer, cursor, per_channel, tolerance);

    let h_rect = IRect::new(edges.left, cursor.y, edges.right, cursor.y);
    let v_rect = IRect::new(cursor.x, edges.top, cursor.x, edges.bottom);

    (
        MeasurementResult::from_irect(h_rect, px2mm_ratio),
        MeasurementResult::from_irect(v_rect, px2mm_ratio),
    )
}

/// Compute measurement for a manually dragged bounds rectangle.
///
/// Used in Bounds tool mode where the user drags to select a region.
pub fn compute_bounds_measurement(
    start: Point,
    end: Point,
    px2mm_ratio: f32,
) -> MeasurementResult {
    let left = start.x.min(end.x) as f32;
    let top = start.y.min(end.y) as f32;
    let right = start.x.max(end.x) as f32;
    let bottom = start.y.max(end.y) as f32;
    MeasurementResult::new(Rect::new(left, top, right, bottom), px2mm_ratio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MeasureUnit;

    /// Helper: create a 100x100 buffer with a colored box on white background.
    fn make_box_buffer(
        box_left: usize,
        box_top: usize,
        box_right: usize,
        box_bottom: usize,
    ) -> Vec<u32> {
        let white = 0xFFFFFFFF;
        let red = 0xFF0000FF;
        let mut pixels = vec![white; 100 * 100];
        for y in box_top..=box_bottom {
            for x in box_left..=box_right {
                pixels[x + 100 * y] = red;
            }
        }
        pixels
    }

    #[test]
    fn test_cross_mode_center_of_100px_region() {
        // Red box from (0,0) to (99,99) on 100x100 buffer — fills the entire buffer
        let pixels = vec![0xFF0000FF; 100 * 100];
        let buf = PixelBuffer::new(&pixels, 100, 100, 100);
        let result = compute_measurement(
            &buf,
            Point { x: 50, y: 50 },
            MeasureMode::Cross,
            false,
            30,
            0.0,
        );
        assert_eq!(result.width(MeasureUnit::Pixel), 100.0);
        assert_eq!(result.height(MeasureUnit::Pixel), 100.0);
    }

    #[test]
    fn test_bounds_mode_returns_bounding_rect() {
        // 100x100 buffer with red box at (20,30)-(79,69)
        let pixels = make_box_buffer(20, 30, 79, 69);
        let buf = PixelBuffer::new(&pixels, 100, 100, 100);
        let result = compute_measurement(
            &buf,
            Point { x: 50, y: 50 },
            MeasureMode::Bounds,
            false,
            30,
            0.0,
        );
        // Box is 60 wide (20..79 inclusive) and 40 tall (30..69 inclusive)
        assert_eq!(result.width(MeasureUnit::Pixel), 60.0);
        assert_eq!(result.height(MeasureUnit::Pixel), 40.0);
    }

    #[test]
    fn test_horizontal_mode_returns_width_only() {
        let pixels = make_box_buffer(20, 30, 79, 69);
        let buf = PixelBuffer::new(&pixels, 100, 100, 100);
        let result = compute_measurement(
            &buf,
            Point { x: 50, y: 50 },
            MeasureMode::Horizontal,
            false,
            30,
            0.0,
        );
        // Width: from edge left=20 to right=79 = 60 pixels
        assert_eq!(result.width(MeasureUnit::Pixel), 60.0);
        // Height: cursor.y to cursor.y = 1 pixel (single line)
        assert_eq!(result.height(MeasureUnit::Pixel), 1.0);
    }

    #[test]
    fn test_vertical_mode_returns_height_only() {
        let pixels = make_box_buffer(20, 30, 79, 69);
        let buf = PixelBuffer::new(&pixels, 100, 100, 100);
        let result = compute_measurement(
            &buf,
            Point { x: 50, y: 50 },
            MeasureMode::Vertical,
            false,
            30,
            0.0,
        );
        // Height: from edge top=30 to bottom=69 = 40 pixels
        assert_eq!(result.height(MeasureUnit::Pixel), 40.0);
        // Width: cursor.x to cursor.x = 1 pixel
        assert_eq!(result.width(MeasureUnit::Pixel), 1.0);
    }

    #[test]
    fn test_cross_measurements_separate() {
        let pixels = make_box_buffer(10, 20, 89, 79);
        let buf = PixelBuffer::new(&pixels, 100, 100, 100);
        let (h, v) = compute_cross_measurements(
            &buf,
            Point { x: 50, y: 50 },
            false,
            30,
            0.0,
        );
        // Horizontal: 10..89 = 80 pixels wide
        assert_eq!(h.width(MeasureUnit::Pixel), 80.0);
        // Vertical: 20..79 = 60 pixels tall
        assert_eq!(v.height(MeasureUnit::Pixel), 60.0);
    }

    #[test]
    fn test_bounds_drag_measurement() {
        let result = compute_bounds_measurement(
            Point { x: 10, y: 20 },
            Point { x: 110, y: 70 },
            0.0,
        );
        assert_eq!(result.width(MeasureUnit::Pixel), 101.0); // 110-10+1
        assert_eq!(result.height(MeasureUnit::Pixel), 51.0); // 70-20+1
    }

    #[test]
    fn test_bounds_drag_reversed_coordinates() {
        // Drag from bottom-right to top-left
        let result = compute_bounds_measurement(
            Point { x: 110, y: 70 },
            Point { x: 10, y: 20 },
            0.0,
        );
        assert_eq!(result.width(MeasureUnit::Pixel), 101.0);
        assert_eq!(result.height(MeasureUnit::Pixel), 51.0);
    }

    #[test]
    fn test_measurement_with_unit_conversion() {
        let pixels = vec![0xFF0000FF; 100 * 100];
        let buf = PixelBuffer::new(&pixels, 100, 100, 100);
        let result = compute_measurement(
            &buf,
            Point { x: 50, y: 50 },
            MeasureMode::Cross,
            false,
            30,
            0.0,
        );
        // 100 pixels at 96 DPI ≈ 1.042 inches
        let inches = result.width(MeasureUnit::Inch);
        assert!((inches - 100.0 / 96.0).abs() < 0.001);
    }

    #[test]
    fn test_cross_mode_asymmetric_region() {
        // Red box from (10,5) to (49,94): 40 wide, 90 tall
        let pixels = make_box_buffer(10, 5, 49, 94);
        let buf = PixelBuffer::new(&pixels, 100, 100, 100);
        let result = compute_measurement(
            &buf,
            Point { x: 30, y: 50 },
            MeasureMode::Cross,
            false,
            30,
            0.0,
        );
        assert_eq!(result.width(MeasureUnit::Pixel), 40.0);
        assert_eq!(result.height(MeasureUnit::Pixel), 90.0);
    }

    #[test]
    fn test_measurement_at_buffer_edge() {
        // Cursor at (1,1) in a uniform buffer — should extend to full buffer
        let pixels = vec![0xFF0000FF; 50 * 50];
        let buf = PixelBuffer::new(&pixels, 50, 50, 50);
        let result = compute_measurement(
            &buf,
            Point { x: 1, y: 1 },
            MeasureMode::Cross,
            false,
            30,
            0.0,
        );
        assert_eq!(result.width(MeasureUnit::Pixel), 50.0);
        assert_eq!(result.height(MeasureUnit::Pixel), 50.0);
    }

    #[test]
    fn test_measurement_tolerance_affects_result() {
        // Create gradient: pixel values increase by 1 each column
        let mut pixels = vec![0xFF808080u32; 100 * 100];
        for y in 0..100 {
            for x in 0..100 {
                pixels[x + 100 * y] = 0xFF808000 + x as u32;
            }
        }
        let buf = PixelBuffer::new(&pixels, 100, 100, 100);

        // Low tolerance: narrow region
        let r1 = compute_measurement(
            &buf,
            Point { x: 50, y: 50 },
            MeasureMode::Horizontal,
            false,
            5,
            0.0,
        );

        // High tolerance: wider region
        let r2 = compute_measurement(
            &buf,
            Point { x: 50, y: 50 },
            MeasureMode::Horizontal,
            false,
            50,
            0.0,
        );

        assert!(
            r2.width(MeasureUnit::Pixel) > r1.width(MeasureUnit::Pixel),
            "Higher tolerance should yield wider measurement"
        );
    }
}
