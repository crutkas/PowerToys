//! Edge detection in pixel buffers — matches C++ `EdgeDetection.cpp`.
//!
//! Given a pixel buffer and a center point, walks outward in each direction
//! comparing adjacent pixels. When the pixel color differs beyond the
//! tolerance threshold, that position is an edge.

use crate::types::{IRect, PixelBuffer, Point};

/// Detect the bounding edges of the contiguous color region around `center`.
///
/// Returns an `IRect` where left/top/right/bottom are the last same-color
/// pixel positions before a color boundary (or the buffer edge).
///
/// Matches C++ `DetectEdges()`.
pub fn detect_edges(
    buffer: &PixelBuffer,
    center: Point,
    per_channel: bool,
    tolerance: u8,
) -> IRect {
    let left = find_edge(buffer, center, tolerance, per_channel, true, false);
    let right = find_edge(buffer, center, tolerance, per_channel, true, true);
    let top = find_edge(buffer, center, tolerance, per_channel, false, false);
    let bottom = find_edge(buffer, center, tolerance, per_channel, false, true);

    IRect::new(left, top, right, bottom)
}

/// Walk from `center` along one axis until a color boundary or the buffer edge.
///
/// - `is_x`: walk along x-axis (horizontal) if true, y-axis (vertical) if false
/// - `increment`: walk toward higher values if true, lower if false
///
/// Returns the coordinate of the last pixel that is still "close" to the center pixel.
fn find_edge(
    buffer: &PixelBuffer,
    center: Point,
    tolerance: u8,
    per_channel: bool,
    is_x: bool,
    increment: bool,
) -> i32 {
    let max_dim = if is_x { buffer.width } else { buffer.height } as i32;

    // Clamp center to valid range [1, max-2] matching C++
    let mut x = center.x.clamp(1, buffer.width as i32 - 2);
    let mut y = center.y.clamp(1, buffer.height as i32 - 2);

    let start_pixel = buffer.get_pixel(x as usize, y as usize);

    loop {
        let old_x = x;
        let old_y = y;

        if is_x {
            if increment {
                x += 1;
                if x >= max_dim {
                    break;
                }
            } else {
                x -= 1;
                if x <= 0 {
                    break;
                }
            }
        } else {
            if increment {
                y += 1;
                if y >= max_dim {
                    break;
                }
            } else {
                y -= 1;
                if y <= 0 {
                    break;
                }
            }
        }

        let next_pixel = buffer.get_pixel(x as usize, y as usize);
        if !PixelBuffer::pixels_close(start_pixel, next_pixel, tolerance, per_channel) {
            return if is_x { old_x } else { old_y };
        }
    }

    // Reached the buffer edge
    if increment {
        max_dim - 1
    } else {
        0
    }
}

/// Find edges in a single row of pixels, returning positions where color changes occur.
///
/// Useful for horizontal line analysis: given a row of pixel values,
/// returns the x-positions where adjacent pixels differ beyond tolerance.
pub fn find_row_edges(row: &[u32], tolerance: u8, per_channel: bool) -> Vec<usize> {
    let mut edges = Vec::new();
    for i in 1..row.len() {
        if !PixelBuffer::pixels_close(row[i - 1], row[i], tolerance, per_channel) {
            edges.push(i);
        }
    }
    edges
}

/// Find edges in a single column of pixels (given as a slice with stride).
///
/// Returns the y-positions where adjacent pixels differ beyond tolerance.
pub fn find_column_edges(
    buffer: &PixelBuffer,
    x: usize,
    tolerance: u8,
    per_channel: bool,
) -> Vec<usize> {
    let mut edges = Vec::new();
    for y in 1..buffer.height {
        let prev = buffer.get_pixel(x, y - 1);
        let curr = buffer.get_pixel(x, y);
        if !PixelBuffer::pixels_close(prev, curr, tolerance, per_channel) {
            edges.push(y);
        }
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a pixel buffer from a 2D grid of u32 colors.
    #[allow(dead_code)]
    fn make_buffer(width: usize, height: usize, pixels: Vec<u32>) -> (Vec<u32>, usize) {
        assert_eq!(pixels.len(), width * height);
        (pixels, width) // pitch = width for simplicity
    }

    // ── Row edge detection tests ──────────────────────────────────────────

    #[test]
    fn test_solid_color_row_no_edges() {
        let row = vec![0xFF0000FF; 100]; // 100 identical red pixels
        let edges = find_row_edges(&row, 30, false);
        assert!(edges.is_empty(), "Solid color row should have no edges");
    }

    #[test]
    fn test_color_change_at_pixel_50() {
        let mut row = vec![0xFF0000FF; 100]; // red
        for px in row.iter_mut().skip(50) {
            *px = 0xFF00FF00; // green from pixel 50 onward
        }
        let edges = find_row_edges(&row, 30, false);
        assert_eq!(edges, vec![50], "Edge should be at pixel 50");
    }

    #[test]
    fn test_multiple_color_changes() {
        // 3 bands: [0..33] red, [33..66] green, [66..100] blue
        let mut row = vec![0u32; 100];
        for (i, px) in row.iter_mut().enumerate() {
            *px = if i < 33 {
                0xFF0000FF // red
            } else if i < 66 {
                0xFF00FF00 // green
            } else {
                0xFFFF0000 // blue
            };
        }
        let edges = find_row_edges(&row, 30, false);
        assert_eq!(edges, vec![33, 66], "Edges at band transitions");
    }

    #[test]
    fn test_tolerance_similar_colors_not_edge() {
        // Two similar colors: differ by 5 in blue channel only
        let mut row = vec![0xFF808080u32; 100];
        for px in row.iter_mut().skip(50) {
            *px = 0xFF808085; // blue channel: 0x85 vs 0x80 = diff 5
        }
        // Tolerance 30 (total mode): diff = 5, well under
        let edges = find_row_edges(&row, 30, false);
        assert!(
            edges.is_empty(),
            "Small color difference within tolerance should not be an edge"
        );
    }

    #[test]
    fn test_tolerance_per_channel_boundary() {
        let mut row = vec![0xFF808080u32; 100];
        for px in row.iter_mut().skip(50) {
            *px = 0xFF808085; // one channel differs by 5
        }
        // Per-channel mode, tolerance exactly 5 — should NOT be an edge
        let edges_at_5 = find_row_edges(&row, 5, true);
        assert!(edges_at_5.is_empty());

        // Per-channel mode, tolerance 4 — SHOULD be an edge
        let edges_at_4 = find_row_edges(&row, 4, true);
        assert_eq!(edges_at_4, vec![50]);
    }

    // ── Column edge detection tests ───────────────────────────────────────

    #[test]
    fn test_column_edges_solid() {
        let pixels = vec![0xFF0000FF; 10 * 10]; // 10x10 solid red
        let buf = PixelBuffer::new(&pixels, 10, 10, 10);
        let edges = find_column_edges(&buf, 5, 30, false);
        assert!(edges.is_empty());
    }

    #[test]
    fn test_column_edges_color_change() {
        let mut pixels = vec![0xFF0000FF; 10 * 10]; // 10x10 red
        // Change rows 5–9 to green at column 3
        for y in 5..10 {
            pixels[3 + 10 * y] = 0xFF00FF00;
        }
        let buf = PixelBuffer::new(&pixels, 10, 10, 10);
        let edges = find_column_edges(&buf, 3, 30, false);
        assert_eq!(edges, vec![5]);
    }

    // ── Full detect_edges tests ───────────────────────────────────────────

    #[test]
    fn test_detect_edges_uniform_buffer() {
        // Entire buffer is same color → edges reach the border
        let pixels = vec![0xFF0000FF; 100 * 100];
        let buf = PixelBuffer::new(&pixels, 100, 100, 100);
        let edges = detect_edges(&buf, Point { x: 50, y: 50 }, false, 30);
        assert_eq!(edges.left, 0);
        assert_eq!(edges.top, 0);
        assert_eq!(edges.right, 99);
        assert_eq!(edges.bottom, 99);
    }

    #[test]
    fn test_detect_edges_centered_box() {
        // 100x100 buffer, white background, red box from (20,30) to (79,69)
        let white = 0xFFFFFFFF;
        let red = 0xFF0000FF;
        let mut pixels = vec![white; 100 * 100];
        for y in 30..=69 {
            for x in 20..=79 {
                pixels[x + 100 * y] = red;
            }
        }
        let buf = PixelBuffer::new(&pixels, 100, 100, 100);
        let edges = detect_edges(&buf, Point { x: 50, y: 50 }, false, 30);
        assert_eq!(edges.left, 20);
        assert_eq!(edges.top, 30);
        assert_eq!(edges.right, 79);
        assert_eq!(edges.bottom, 69);
    }

    #[test]
    fn test_detect_edges_at_corner() {
        // Red box in top-left corner (0,0) to (19,19)
        let white = 0xFFFFFFFF;
        let red = 0xFF0000FF;
        let mut pixels = vec![white; 100 * 100];
        for y in 0..=19 {
            for x in 0..=19 {
                pixels[x + 100 * y] = red;
            }
        }
        let buf = PixelBuffer::new(&pixels, 100, 100, 100);
        let edges = detect_edges(&buf, Point { x: 10, y: 10 }, false, 30);
        // Left and top should be 0 (buffer edge)
        assert_eq!(edges.left, 0);
        assert_eq!(edges.top, 0);
        assert_eq!(edges.right, 19);
        assert_eq!(edges.bottom, 19);
    }

    #[test]
    fn test_detect_edges_per_channel_mode() {
        let bg = 0xFF808080;
        let fg = 0xFF808080 + 0x00000020; // blue channel differs by 32
        let mut pixels = vec![bg; 50 * 50];
        for y in 10..=39 {
            for x in 10..=39 {
                pixels[x + 50 * y] = fg;
            }
        }
        let buf = PixelBuffer::new(&pixels, 50, 50, 50);

        // Per-channel, tolerance 31: blue differs by 32, should detect edges
        let edges = detect_edges(&buf, Point { x: 25, y: 25 }, true, 31);
        assert_eq!(edges.left, 10);
        assert_eq!(edges.right, 39);

        // Per-channel, tolerance 32: diff == tolerance, not an edge
        let edges2 = detect_edges(&buf, Point { x: 25, y: 25 }, true, 32);
        assert_eq!(edges2.left, 0); // extends to border
        assert_eq!(edges2.right, 49);
    }

    #[test]
    fn test_detect_edges_small_region() {
        // 10x10 buffer, single different pixel at (5,5) with tolerance 0
        let bg = 0xFF000000;
        let fg = 0xFF000001; // differs by 1
        let mut pixels = vec![bg; 10 * 10];
        pixels[5 + 10 * 5] = fg;
        let buf = PixelBuffer::new(&pixels, 10, 10, 10);
        let edges = detect_edges(&buf, Point { x: 5, y: 5 }, false, 0);
        assert_eq!(edges.left, 5);
        assert_eq!(edges.top, 5);
        assert_eq!(edges.right, 5);
        assert_eq!(edges.bottom, 5);
    }

    #[test]
    fn test_detect_edges_cursor_clamped() {
        // Cursor at edge of buffer — should clamp and not panic
        let pixels = vec![0xFF0000FF; 20 * 20];
        let buf = PixelBuffer::new(&pixels, 20, 20, 20);
        let edges = detect_edges(&buf, Point { x: 0, y: 0 }, false, 30);
        assert_eq!(edges.left, 0);
        assert_eq!(edges.top, 0);
    }
}
