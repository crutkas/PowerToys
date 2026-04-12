use crate::types::{CrosshairsOrientation, Settings};

/// Source of cursor position — either from the mouse hook or externally provided.
///
/// Mirrors the C++ `SetExternalControl(bool)` flag: when external control is
/// active the WH_MOUSE_LL hook is unhooked and position is provided by an
/// external caller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorSource {
    /// Position reported by the low-level mouse hook (normal mode).
    Hook { x: i32, y: i32 },
    /// Position provided by an external module (external-control mode).
    External { x: i32, y: i32 },
}

impl CursorSource {
    /// Extract the (x, y) pair regardless of source.
    pub fn position(&self) -> (i32, i32) {
        match *self {
            CursorSource::Hook { x, y } | CursorSource::External { x, y } => (x, y),
        }
    }
}

/// Axis-aligned rectangle used for crosshair line segments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
}

/// Monitor/screen bounds in absolute coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenBounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// The 8 rectangles that make up a crosshair overlay (4 inner lines + 4 border lines).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrosshairLayout {
    pub left_inner: Rect,
    pub right_inner: Rect,
    pub top_inner: Rect,
    pub bottom_inner: Rect,
    pub left_border: Rect,
    pub right_border: Rect,
    pub top_border: Rect,
    pub bottom_border: Rect,
}

/// Convert an integer opacity percentage (0–100) to a normalized 0.0–1.0 float,
/// matching the C++ `max(0.f, min(1.f, opacity / 100.0f))`.
pub fn opacity_to_normalized(opacity_percent: i32) -> f32 {
    (opacity_percent as f32 / 100.0).clamp(0.0, 1.0)
}

/// Compute the 8 crosshair rectangles in screen-space coordinates.
///
/// This mirrors `InclusiveCrosshairs::UpdateCrosshairsPosition()` but resolves
/// the composition anchor-point offsets into absolute (x, y, width, height) rects.
pub fn calculate_crosshair_layout(
    cursor_x: i32,
    cursor_y: i32,
    screen_bounds: ScreenBounds,
    settings: &Settings,
) -> CrosshairLayout {
    let cx = cursor_x as f32;
    let cy = cursor_y as f32;
    let sx = screen_bounds.left as f32;
    let sy = screen_bounds.top as f32;
    let sx2 = screen_bounds.right as f32;
    let sy2 = screen_bounds.bottom as f32;

    let r = settings.radius as f32;
    let t = settings.thickness as f32;
    let bs = settings.border_size as f32;
    let fl = settings.fixed_length as f32;
    let fixed = settings.is_fixed_length_enabled;

    // Matches the C++ half-pixel adjustment for odd thickness values.
    let hpa = if settings.thickness % 2 == 1 { 0.5 } else { 0.0_f32 };
    let bsp = bs * 2.0; // border size padding

    // --- Horizontal crosshairs (left + right) ---
    let (left_inner, right_inner, left_border, right_border) =
        if settings.orientation == CrosshairsOrientation::Both
            || settings.orientation == CrosshairsOrientation::HorizontalOnly
        {
            // Left arm
            let left_full = cx - sx - r + hpa * 2.0;
            let left_len = if fixed { fl } else { left_full };
            let left_border_len = if fixed { fl + bsp } else { left_full + bs };

            // C++ anchor (1.0, 0.5): right edge at offset, vertically centered
            let left_anchor_x = cx - r + hpa * 2.0;
            let left_anchor_y = cy + hpa;
            let li = Rect {
                x: left_anchor_x - left_len,
                y: left_anchor_y - t / 2.0,
                width: left_len,
                height: t,
            };

            let left_b_anchor_x = cx - r + bs + hpa * 2.0;
            let lb = Rect {
                x: left_b_anchor_x - left_border_len,
                y: left_anchor_y - (t + bsp) / 2.0,
                width: left_border_len,
                height: t + bsp,
            };

            // Right arm
            let right_full = sx2 - cx - r;
            let right_len = if fixed { fl } else { right_full };
            let right_border_len = if fixed { fl + bsp } else { right_full + bs };

            // C++ anchor (0.0, 0.5): left edge at offset, vertically centered
            let ri = Rect {
                x: cx + r,
                y: cy + hpa - t / 2.0,
                width: right_len,
                height: t,
            };
            let rb = Rect {
                x: cx + r - bs,
                y: cy + hpa - (t + bsp) / 2.0,
                width: right_border_len,
                height: t + bsp,
            };

            (li, ri, lb, rb)
        } else {
            (Rect::ZERO, Rect::ZERO, Rect::ZERO, Rect::ZERO)
        };

    // --- Vertical crosshairs (top + bottom) ---
    let (top_inner, bottom_inner, top_border, bottom_border) =
        if settings.orientation == CrosshairsOrientation::Both
            || settings.orientation == CrosshairsOrientation::VerticalOnly
        {
            // Top arm
            let top_full = cy - sy - r + hpa * 2.0;
            let top_len = if fixed { fl } else { top_full };
            let top_border_len = if fixed { fl + bsp } else { top_full + bs };

            // C++ anchor (0.5, 1.0): horizontally centered, bottom edge at offset
            let top_anchor_x = cx + hpa;
            let top_anchor_y = cy - r + hpa * 2.0;
            let ti = Rect {
                x: top_anchor_x - t / 2.0,
                y: top_anchor_y - top_len,
                width: t,
                height: top_len,
            };

            let top_b_anchor_y = cy - r + bs + hpa * 2.0;
            let tb = Rect {
                x: top_anchor_x - (t + bsp) / 2.0,
                y: top_b_anchor_y - top_border_len,
                width: t + bsp,
                height: top_border_len,
            };

            // Bottom arm
            let bottom_full = sy2 - cy - r;
            let bottom_len = if fixed { fl } else { bottom_full };
            let bottom_border_len = if fixed { fl + bsp } else { bottom_full + bs };

            // C++ anchor (0.5, 0.0): horizontally centered, top edge at offset
            let bi = Rect {
                x: cx + hpa - t / 2.0,
                y: cy + r,
                width: t,
                height: bottom_len,
            };
            let bb = Rect {
                x: cx + hpa - (t + bsp) / 2.0,
                y: cy + r - bs,
                width: t + bsp,
                height: bottom_border_len,
            };

            (ti, bi, tb, bb)
        } else {
            (Rect::ZERO, Rect::ZERO, Rect::ZERO, Rect::ZERO)
        };

    CrosshairLayout {
        left_inner,
        right_inner,
        top_inner,
        bottom_inner,
        left_border,
        right_border,
        top_border,
        bottom_border,
    }
}

/// Extended layout calculation that accepts a [`CursorSource`].
///
/// When `settings.external_control` is `true` callers should pass
/// `CursorSource::External`; when `false`, `CursorSource::Hook`.
/// The resulting layout is identical — the distinction exists so
/// upper layers can assert correctness at the call-site.
pub fn calculate_crosshair_layout_ext(
    cursor: CursorSource,
    screen_bounds: ScreenBounds,
    settings: &Settings,
) -> CrosshairLayout {
    let (x, y) = cursor.position();
    calculate_crosshair_layout(x, y, screen_bounds, settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Settings;

    /// Even thickness so hpa == 0, keeping the math simple.
    fn even_settings() -> Settings {
        Settings {
            thickness: 4,
            radius: 20,
            border_size: 1,
            ..Settings::default()
        }
    }

    fn screen_1000() -> ScreenBounds {
        ScreenBounds { left: 0, top: 0, right: 1000, bottom: 1000 }
    }

    // ---- center of screen (4 equal) ----

    #[test]
    fn center_left_equals_right() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        assert_eq!(l.left_inner.width, l.right_inner.width);
    }

    #[test]
    fn center_top_equals_bottom() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        assert_eq!(l.top_inner.height, l.bottom_inner.height);
    }

    #[test]
    fn center_left_length() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        assert_eq!(l.left_inner.width, 480.0);
    }

    #[test]
    fn center_right_length() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        assert_eq!(l.right_inner.width, 480.0);
    }

    // ---- near edges (shorter lines) ----

    #[test]
    fn near_left_edge_shorter_left() {
        let l = calculate_crosshair_layout(30, 500, screen_1000(), &even_settings());
        assert!(l.left_inner.width < l.right_inner.width);
        assert_eq!(l.left_inner.width, 10.0); // 30 - 0 - 20
    }

    #[test]
    fn near_right_edge_shorter_right() {
        let l = calculate_crosshair_layout(970, 500, screen_1000(), &even_settings());
        assert!(l.right_inner.width < l.left_inner.width);
        assert_eq!(l.right_inner.width, 10.0); // 1000 - 970 - 20
    }

    #[test]
    fn near_top_edge_shorter_top() {
        let l = calculate_crosshair_layout(500, 30, screen_1000(), &even_settings());
        assert!(l.top_inner.height < l.bottom_inner.height);
        assert_eq!(l.top_inner.height, 10.0);
    }

    #[test]
    fn near_bottom_edge_shorter_bottom() {
        let l = calculate_crosshair_layout(500, 970, screen_1000(), &even_settings());
        assert!(l.bottom_inner.height < l.top_inner.height);
        assert_eq!(l.bottom_inner.height, 10.0);
    }

    // ---- corners ----

    #[test]
    fn top_left_corner() {
        let l = calculate_crosshair_layout(30, 30, screen_1000(), &even_settings());
        assert_eq!(l.left_inner.width, 10.0);
        assert_eq!(l.top_inner.height, 10.0);
    }

    #[test]
    fn top_right_corner() {
        let l = calculate_crosshair_layout(970, 30, screen_1000(), &even_settings());
        assert_eq!(l.right_inner.width, 10.0);
        assert_eq!(l.top_inner.height, 10.0);
    }

    #[test]
    fn bottom_left_corner() {
        let l = calculate_crosshair_layout(30, 970, screen_1000(), &even_settings());
        assert_eq!(l.left_inner.width, 10.0);
        assert_eq!(l.bottom_inner.height, 10.0);
    }

    #[test]
    fn bottom_right_corner() {
        let l = calculate_crosshair_layout(970, 970, screen_1000(), &even_settings());
        assert_eq!(l.right_inner.width, 10.0);
        assert_eq!(l.bottom_inner.height, 10.0);
    }

    // ---- fixed length ----

    fn fixed_settings() -> Settings {
        Settings {
            thickness: 4,
            radius: 20,
            border_size: 1,
            is_fixed_length_enabled: true,
            fixed_length: 100,
            ..Settings::default()
        }
    }

    #[test]
    fn fixed_length_left() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &fixed_settings());
        assert_eq!(l.left_inner.width, 100.0);
    }

    #[test]
    fn fixed_length_right() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &fixed_settings());
        assert_eq!(l.right_inner.width, 100.0);
    }

    #[test]
    fn fixed_length_top() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &fixed_settings());
        assert_eq!(l.top_inner.height, 100.0);
    }

    #[test]
    fn fixed_length_bottom() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &fixed_settings());
        assert_eq!(l.bottom_inner.height, 100.0);
    }

    // ---- VerticalOnly (left/right zero) ----

    fn vertical_only_settings() -> Settings {
        Settings {
            thickness: 4,
            radius: 20,
            border_size: 1,
            orientation: CrosshairsOrientation::VerticalOnly,
            ..Settings::default()
        }
    }

    #[test]
    fn vertical_only_left_zero_width() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &vertical_only_settings());
        assert_eq!(l.left_inner, Rect::ZERO);
    }

    #[test]
    fn vertical_only_right_zero_width() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &vertical_only_settings());
        assert_eq!(l.right_inner, Rect::ZERO);
    }

    #[test]
    fn vertical_only_top_has_height() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &vertical_only_settings());
        assert_eq!(l.top_inner.height, 480.0);
    }

    #[test]
    fn vertical_only_bottom_has_height() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &vertical_only_settings());
        assert_eq!(l.bottom_inner.height, 480.0);
    }

    // ---- HorizontalOnly (top/bottom zero) ----

    fn horizontal_only_settings() -> Settings {
        Settings {
            thickness: 4,
            radius: 20,
            border_size: 1,
            orientation: CrosshairsOrientation::HorizontalOnly,
            ..Settings::default()
        }
    }

    #[test]
    fn horizontal_only_top_zero_height() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &horizontal_only_settings());
        assert_eq!(l.top_inner, Rect::ZERO);
    }

    #[test]
    fn horizontal_only_bottom_zero_height() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &horizontal_only_settings());
        assert_eq!(l.bottom_inner, Rect::ZERO);
    }

    #[test]
    fn horizontal_only_left_has_width() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &horizontal_only_settings());
        assert_eq!(l.left_inner.width, 480.0);
    }

    #[test]
    fn horizontal_only_right_has_width() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &horizontal_only_settings());
        assert_eq!(l.right_inner.width, 480.0);
    }

    // ---- radius gap ----

    #[test]
    fn radius_gap_left_ends_before_cursor() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        // left inner right edge must be at cursor_x - radius
        assert_eq!(l.left_inner.x + l.left_inner.width, 480.0);
    }

    #[test]
    fn radius_gap_right_starts_after_cursor() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        assert_eq!(l.right_inner.x, 520.0);
    }

    #[test]
    fn radius_gap_top_ends_before_cursor() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        assert_eq!(l.top_inner.y + l.top_inner.height, 480.0);
    }

    #[test]
    fn radius_gap_bottom_starts_after_cursor() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        assert_eq!(l.bottom_inner.y, 520.0);
    }

    // ---- border offset ----

    #[test]
    fn border_extends_beyond_left_inner() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        let border_right = l.left_border.x + l.left_border.width;
        let inner_right = l.left_inner.x + l.left_inner.width;
        assert_eq!(border_right, inner_right + 1.0); // border_size = 1
    }

    #[test]
    fn border_extends_beyond_right_inner() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        assert_eq!(l.right_border.x, l.right_inner.x - 1.0); // border_size = 1
    }

    #[test]
    fn border_extends_beyond_top_inner() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        let border_bottom = l.top_border.y + l.top_border.height;
        let inner_bottom = l.top_inner.y + l.top_inner.height;
        assert_eq!(border_bottom, inner_bottom + 1.0);
    }

    #[test]
    fn border_extends_beyond_bottom_inner() {
        let l = calculate_crosshair_layout(500, 500, screen_1000(), &even_settings());
        assert_eq!(l.bottom_border.y, l.bottom_inner.y - 1.0);
    }

    // ---- opacity conversion ----

    #[test]
    fn opacity_default_75() {
        assert_eq!(opacity_to_normalized(75), 0.75);
    }

    #[test]
    fn opacity_max_100() {
        assert_eq!(opacity_to_normalized(100), 1.0);
    }

    #[test]
    fn opacity_min_0() {
        assert_eq!(opacity_to_normalized(0), 0.0);
    }

    #[test]
    fn opacity_50_percent() {
        assert_eq!(opacity_to_normalized(50), 0.5);
    }

    // ---- external control mode ----

    #[test]
    fn external_control_layout_matches_hook_layout() {
        let settings = even_settings();
        let hook = calculate_crosshair_layout(500, 500, screen_1000(), &settings);
        let ext = calculate_crosshair_layout_ext(
            CursorSource::External { x: 500, y: 500 },
            screen_1000(),
            &settings,
        );
        assert_eq!(hook, ext);
    }

    #[test]
    fn external_control_different_position() {
        let settings = even_settings();
        let ext_a = calculate_crosshair_layout_ext(
            CursorSource::External { x: 100, y: 100 },
            screen_1000(),
            &settings,
        );
        let ext_b = calculate_crosshair_layout_ext(
            CursorSource::External { x: 800, y: 800 },
            screen_1000(),
            &settings,
        );
        // Different positions must produce different layouts
        assert_ne!(ext_a, ext_b);
    }

    #[test]
    fn cursor_source_position_hook() {
        let src = CursorSource::Hook { x: 42, y: 99 };
        assert_eq!(src.position(), (42, 99));
    }

    #[test]
    fn cursor_source_position_external() {
        let src = CursorSource::External { x: 7, y: 13 };
        assert_eq!(src.position(), (7, 13));
    }

    #[test]
    fn external_control_flag_default_false() {
        let s = Settings::default();
        assert!(!s.external_control);
    }

    #[test]
    fn external_control_flag_enables() {
        let s = Settings {
            external_control: true,
            ..Settings::default()
        };
        assert!(s.external_control);
    }
}
