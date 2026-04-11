//! Zone overlay window for FancyZones visual feedback.

use std::ptr;

use fancyzones_core::rect::Rect;
use fancyzones_core::util::hex_to_rgb;
use powertoys_win32::string::to_wide;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Colors used by the overlay.
#[derive(Debug, Clone)]
pub struct OverlayColors {
    pub zone_color: (u8, u8, u8),
    pub border_color: (u8, u8, u8),
    pub highlight_color: (u8, u8, u8),
    pub opacity: u8,
}

impl OverlayColors {
    pub fn from_hex(zone: &str, border: &str, highlight: &str, opacity_percent: i32) -> Self {
        Self {
            zone_color: hex_to_rgb(zone),
            border_color: hex_to_rgb(border),
            highlight_color: hex_to_rgb(highlight),
            opacity: ((opacity_percent.clamp(0, 100) as u32 * 255) / 100) as u8,
        }
    }
}

/// Create an ARGB pixel buffer for a zone rectangle (BGRA format for Win32).
pub fn create_zone_pixel_buffer(
    width: u32,
    height: u32,
    fill: (u8, u8, u8),
    border: (u8, u8, u8),
    alpha: u8,
    border_width: u32,
) -> Vec<u8> {
    let mut buf = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let is_border = x < border_width
                || x >= width - border_width
                || y < border_width
                || y >= height - border_width;
            let (r, g, b) = if is_border { border } else { fill };
            buf[idx] = b;
            buf[idx + 1] = g;
            buf[idx + 2] = r;
            buf[idx + 3] = alpha;
        }
    }
    buf
}

static OVERLAY_REG: std::sync::Once = std::sync::Once::new();
const OVERLAY_CLASS: &str = "FancyZones_Overlay_Rust";

/// A transparent overlay window that displays zone highlights.
pub struct ZoneOverlay {
    hwnd: HWND,
}

unsafe impl Send for ZoneOverlay {}

impl ZoneOverlay {
    pub fn create(left: i32, top: i32, width: i32, height: i32) -> Option<Self> {
        OVERLAY_REG.call_once(|| {
            let class_name = to_wide(OVERLAY_CLASS);
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(overlay_wnd_proc),
                lpszClassName: class_name.as_ptr(),
                hInstance: ptr::null_mut(),
                style: 0,
                cbClsExtra: 0,
                cbWndExtra: 0,
                hIcon: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                hIconSm: ptr::null_mut(),
            };
            unsafe {
                RegisterClassExW(&wc);
            }
        });

        let class_name = to_wide(OVERLAY_CLASS);
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                ptr::null(),
                WS_POPUP,
                left,
                top,
                width,
                height,
                ptr::null_mut(), // parent HWND
                ptr::null_mut(), // menu
                ptr::null_mut(), // hInstance
                ptr::null_mut(), // lpParam
            )
        };

        if hwnd.is_null() {
            None
        } else {
            Some(Self { hwnd })
        }
    }

    pub fn show(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
    }

    pub fn hide(&self) {
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn rect(&self) -> Option<Rect> {
        let mut r = windows_sys::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let ok = unsafe { GetWindowRect(self.hwnd, &mut r) };
        if ok != 0 {
            Some(Rect::new(r.left, r.top, r.right, r.bottom))
        } else {
            None
        }
    }
}

impl Drop for ZoneOverlay {
    fn drop(&mut self) {
        if !self.hwnd.is_null() {
            unsafe {
                DestroyWindow(self.hwnd);
            }
        }
    }
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_colors_from_hex() {
        let c = OverlayColors::from_hex("#FF0000", "#00FF00", "#0000FF", 50);
        assert_eq!(c.zone_color, (255, 0, 0));
        assert_eq!(c.border_color, (0, 255, 0));
        assert_eq!(c.highlight_color, (0, 0, 255));
        assert_eq!(c.opacity, 127); // 50% of 255
    }

    #[test]
    fn overlay_colors_clamp_opacity() {
        let c = OverlayColors::from_hex("#000000", "#000000", "#000000", 200);
        assert_eq!(c.opacity, 255);
        let c2 = OverlayColors::from_hex("#000000", "#000000", "#000000", -10);
        assert_eq!(c2.opacity, 0);
    }

    #[test]
    fn pixel_buffer_dimensions() {
        let buf = create_zone_pixel_buffer(100, 50, (255, 0, 0), (0, 0, 0), 200, 2);
        assert_eq!(buf.len(), 100 * 50 * 4);
    }

    #[test]
    fn pixel_buffer_border_and_fill() {
        let buf = create_zone_pixel_buffer(10, 10, (255, 0, 0), (0, 255, 0), 128, 1);
        // Corner pixel (0,0) should be border color green (BGRA: B=0, G=255, R=0, A=128)
        assert_eq!(buf[0], 0); // B
        assert_eq!(buf[1], 255); // G
        assert_eq!(buf[2], 0); // R
        assert_eq!(buf[3], 128); // A
        // Center pixel (5,5) should be fill color red (BGRA: B=0, G=0, R=255, A=128)
        let idx = (5 * 10 + 5) * 4;
        assert_eq!(buf[idx], 0); // B
        assert_eq!(buf[idx + 1], 0); // G
        assert_eq!(buf[idx + 2], 255); // R
        assert_eq!(buf[idx + 3], 128); // A
    }
}
