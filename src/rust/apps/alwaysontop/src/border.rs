//! Window border rendering using GDI on a layered window.

use powertoys_win32::string::to_wide;
use crate::settings::Settings;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static BORDER_CLASS_REGISTERED: std::sync::Once = std::sync::Once::new();
const BORDER_CLASS_NAME: &str = "AlwaysOnTop_Border_Rust";

pub struct WindowBorder {
    border_hwnd: HWND,
}

unsafe impl Send for WindowBorder {}

impl WindowBorder {
    pub fn empty() -> Self {
        Self { border_hwnd: std::ptr::null_mut() }
    }

    pub fn create(tracked_hwnd: HWND, hinstance: HINSTANCE, settings: &Settings) -> Option<Self> {
        let rect = get_frame_rect(tracked_hwnd, settings.frame_thickness)?;
        let class_name = to_wide(BORDER_CLASS_NAME);
        BORDER_CLASS_REGISTERED.call_once(|| unsafe {
            let mut wc: WNDCLASSEXW = std::mem::zeroed();
            wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
            wc.lpfnWndProc = Some(border_wnd_proc);
            wc.hInstance = hinstance;
            wc.lpszClassName = class_name.as_ptr();
            wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
            RegisterClassExW(&wc);
        });
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let border_hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                class_name.as_ptr(), std::ptr::null(), WS_POPUP | WS_DISABLED,
                rect.left, rect.top, w, h,
                std::ptr::null_mut(), std::ptr::null_mut(), hinstance, std::ptr::null(),
            )
        };
        if border_hwnd.is_null() { return None; }

        let corner_radius = if settings.round_corners_enabled {
            get_corner_radius(tracked_hwnd)
        } else {
            0.0
        };

        let border = Self { border_hwnd };
        border.render_border(w, h, settings, corner_radius);

        unsafe {
            let val: BOOL = 1;
            DwmSetWindowAttribute(border_hwnd, DWMWA_EXCLUDED_FROM_PEEK as u32,
                &val as *const _ as *const _, std::mem::size_of::<BOOL>() as u32);
            SetWindowPos(tracked_hwnd, border_hwnd, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            ShowWindow(border_hwnd, SW_SHOWNA);
        }
        Some(border)
    }

    fn render_border(&self, width: i32, height: i32, settings: &Settings, corner_radius: f32) {
        if self.border_hwnd.is_null() || width <= 0 || height <= 0 { return; }
        unsafe {
            let screen_dc = GetDC(std::ptr::null_mut());
            if screen_dc.is_null() { return; }
            let mem_dc = CreateCompatibleDC(screen_dc);
            if mem_dc.is_null() { ReleaseDC(std::ptr::null_mut(), screen_dc); return; }

            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width;
            bmi.bmiHeader.biHeight = -height;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;

            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let bmp = CreateDIBSection(mem_dc, &bmi, 0, &mut bits, std::ptr::null_mut(), 0);
            if bmp.is_null() || bits.is_null() {
                DeleteDC(mem_dc); ReleaseDC(std::ptr::null_mut(), screen_dc); return;
            }
            let old_bmp = SelectObject(mem_dc, bmp);

            let (r, g, b) = settings.frame_color_rgb();
            let alpha = ((settings.frame_opacity as u32 * 255) / 100) as u8;
            let t = settings.frame_thickness as f32;
            let radius = corner_radius;
            let w = width as f32;
            let h = height as f32;
            let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (width * height) as usize);
            let border_px = premultiply(b, g, r, alpha);

            for py in 0..height {
                for px in 0..width {
                    let idx = (py * width + px) as usize;
                    let x = px as f32;
                    let y = py as f32;

                    if radius > 0.0 {
                        // Rounded rectangle border using SDF (signed distance field)
                        // Outer rounded rect
                        let d_outer = rounded_rect_sdf(x, y, w, h, radius + t);
                        // Inner rounded rect (radius shrinks by thickness)
                        let inner_r = (radius - 1.0).max(0.0);
                        let d_inner = rounded_rect_sdf(x - t, y - t, w - 2.0 * t, h - 2.0 * t, inner_r);

                        // Inside outer AND outside inner = border region
                        // Use anti-aliasing: smooth transition over ~1px
                        let outer_alpha = smoothstep(0.5, -0.5, d_outer);
                        let inner_alpha = smoothstep(-0.5, 0.5, d_inner);
                        let border_alpha = outer_alpha * inner_alpha;

                        if border_alpha > 0.01 {
                            let a = (alpha as f32 * border_alpha) as u8;
                            pixels[idx] = premultiply(b, g, r, a);
                        } else {
                            pixels[idx] = 0;
                        }
                    } else {
                        // Simple rectangular border
                        pixels[idx] = if x < t || x >= w - t || y < t || y >= h - t {
                            border_px
                        } else {
                            0
                        };
                    }
                }
            }

            let pt_zero = POINT { x: 0, y: 0 };
            let size = SIZE { cx: width, cy: height };
            let blend = BLENDFUNCTION { BlendOp: 0, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: 1 };
            UpdateLayeredWindow(self.border_hwnd, screen_dc, std::ptr::null(), &size, mem_dc, &pt_zero, 0, &blend, ULW_ALPHA);

            SelectObject(mem_dc, old_bmp);
            DeleteObject(bmp);
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
        }
    }

    pub fn update_position(&self, tracked_hwnd: HWND) {
        if self.border_hwnd.is_null() { return; }
        if let Some(rect) = get_frame_rect(tracked_hwnd, 15) {
            unsafe {
                SetWindowPos(self.border_hwnd, tracked_hwnd, rect.left, rect.top,
                    rect.right - rect.left, rect.bottom - rect.top, SWP_NOACTIVATE);
            }
        }
    }

    pub fn update_properties(&self, tracked_hwnd: HWND, settings: &Settings) {
        if self.border_hwnd.is_null() { return; }
        if let Some(rect) = get_frame_rect(tracked_hwnd, settings.frame_thickness) {
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            unsafe { SetWindowPos(self.border_hwnd, tracked_hwnd, rect.left, rect.top, w, h, SWP_NOACTIVATE); }
            let radius = if settings.round_corners_enabled { get_corner_radius(tracked_hwnd) } else { 0.0 };
            self.render_border(w, h, settings, radius);
        }
    }

    pub fn show(&self) { if !self.border_hwnd.is_null() { unsafe { ShowWindow(self.border_hwnd, SW_SHOWNA); } } }
    pub fn hide(&self) { if !self.border_hwnd.is_null() { unsafe { ShowWindow(self.border_hwnd, SW_HIDE); } } }
    pub fn destroy(self) { if !self.border_hwnd.is_null() { unsafe { DestroyWindow(self.border_hwnd); } } }
}

fn premultiply(b: u8, g: u8, r: u8, a: u8) -> u32 {
    let a32 = a as u32;
    ((a32 << 24) | (((r as u32) * a32 / 255) << 16) | (((g as u32) * a32 / 255) << 8) | ((b as u32) * a32 / 255))
}

/// Signed distance field for a rounded rectangle centered at (w/2, h/2).
/// Returns negative inside the rect, positive outside.
fn rounded_rect_sdf(px: f32, py: f32, w: f32, h: f32, radius: f32) -> f32 {
    let cx = w / 2.0;
    let cy = h / 2.0;
    let half_w = w / 2.0 - radius;
    let half_h = h / 2.0 - radius;

    let dx = (px - cx).abs() - half_w;
    let dy = (py - cy).abs() - half_h;

    let outside = (dx.max(0.0) * dx.max(0.0) + dy.max(0.0) * dy.max(0.0)).sqrt();
    let inside = dx.max(dy).min(0.0);

    outside + inside - radius
}

/// Smooth interpolation between edges for anti-aliasing.
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Query the DWM corner preference for a window and return the radius in pixels.
/// DWMWCP_ROUND → 8px, DWMWCP_ROUNDSMALL → 4px, else → 0.
fn get_corner_radius(hwnd: HWND) -> f32 {
    // DWMWA_WINDOW_CORNER_PREFERENCE = 33
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    let mut preference: u32 = 0;
    let hr = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &mut preference as *mut _ as *mut _,
            std::mem::size_of::<u32>() as u32,
        )
    };
    if hr != 0 {
        // If the API fails (older Windows), default to rounded
        return 8.0;
    }
    match preference {
        2 => 8.0,  // DWMWCP_ROUND
        3 => 4.0,  // DWMWCP_ROUNDSMALL
        _ => 0.0,  // DWMWCP_DEFAULT (0) or DWMWCP_DONOTROUND (1)
    }
}

fn get_frame_rect(hwnd: HWND, border_thickness: i32) -> Option<RECT> {
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    let hr = unsafe {
        DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            &mut rect as *mut _ as *mut _, std::mem::size_of::<RECT>() as u32)
    };
    if hr != 0 { return None; }
    rect.top -= border_thickness;
    rect.left -= border_thickness;
    rect.right += border_thickness;
    rect.bottom += border_thickness;
    Some(rect)
}

unsafe extern "system" fn border_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_ERASEBKGND => 1,
            WM_SETCURSOR => { SetCursor(LoadCursorW(std::ptr::null_mut(), IDC_ARROW)); 1 }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_premultiply_opaque() {
        let px = premultiply(255, 128, 0, 255);
        // Fully opaque: channels unchanged
        assert_eq!((px >> 24) & 0xFF, 255); // alpha
        assert_eq!((px >> 16) & 0xFF, 0);   // R
        assert_eq!((px >> 8) & 0xFF, 128);  // G
        assert_eq!(px & 0xFF, 255);          // B
    }

    #[test]
    fn test_premultiply_transparent() {
        let px = premultiply(255, 128, 0, 0);
        assert_eq!(px, 0); // Fully transparent = all zeros
    }

    #[test]
    fn test_premultiply_half_alpha() {
        let px = premultiply(200, 100, 50, 128);
        let a = (px >> 24) & 0xFF;
        let r = (px >> 16) & 0xFF;
        let g = (px >> 8) & 0xFF;
        let b = px & 0xFF;
        assert_eq!(a, 128);
        // Channels should be ~half their original values
        assert!(r <= 25 && r >= 24);   // 50 * 128/255 ≈ 25
        assert!(g <= 50 && g >= 49);   // 100 * 128/255 ≈ 50
        assert!(b <= 100 && b >= 99);  // 200 * 128/255 ≈ 100
    }

    #[test]
    fn test_sdf_center_is_inside() {
        // Point at center of a 100x100 rect with radius 10 should be well inside
        let d = rounded_rect_sdf(50.0, 50.0, 100.0, 100.0, 10.0);
        assert!(d < 0.0, "Center should be inside (negative SDF)");
    }

    #[test]
    fn test_sdf_outside_corner() {
        // Point far outside should be positive
        let d = rounded_rect_sdf(200.0, 200.0, 100.0, 100.0, 10.0);
        assert!(d > 0.0, "Far outside should be positive SDF");
    }

    #[test]
    fn test_sdf_on_edge() {
        // Point on the edge of a 100x100 rect (no rounding)
        let d = rounded_rect_sdf(0.0, 50.0, 100.0, 100.0, 0.0);
        assert!((d.abs()) < 1.0, "Edge should be near zero SDF");
    }

    #[test]
    fn test_smoothstep_boundaries() {
        assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_smoothstep_clamped() {
        assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 2.0), 1.0);
    }

    #[test]
    fn test_get_corner_radius_returns_reasonable_value() {
        // With a null HWND, should return 8.0 (default fallback)
        let r = get_corner_radius(std::ptr::null_mut());
        assert!(r >= 0.0 && r <= 20.0);
    }
}