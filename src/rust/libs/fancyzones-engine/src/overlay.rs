//! Zone overlay window using Direct2D + DirectWrite.
//!
//! GPU-accelerated rendering matching the C++ FancyZones implementation:
//! ID2D1HwndRenderTarget for zone fills/borders, IDWriteTextFormat for zone numbers.

use std::ptr;

use fancyzones_core::rect::Rect;
use fancyzones_core::util::hex_to_rgb;
use powertoys_win32::string::to_wide;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::*;

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

static OVERLAY_REG: std::sync::Once = std::sync::Once::new();
const OVERLAY_CLASS: &str = "FancyZones_Overlay_Rust";

static D2D_FACTORY: std::sync::OnceLock<ID2D1Factory1> = std::sync::OnceLock::new();
static DW_FACTORY: std::sync::OnceLock<IDWriteFactory> = std::sync::OnceLock::new();

fn get_d2d_factory() -> Option<&'static ID2D1Factory1> {
    D2D_FACTORY.get_or_init(|| unsafe {
        D2D1CreateFactory::<ID2D1Factory1>(D2D1_FACTORY_TYPE_MULTI_THREADED, None).ok()
    }.unwrap());
    D2D_FACTORY.get()
}

fn get_dwrite_factory() -> Option<&'static IDWriteFactory> {
    DW_FACTORY.get_or_init(|| unsafe {
        DWriteCreateFactory::<IDWriteFactory>(DWRITE_FACTORY_TYPE_SHARED).ok()
    }.unwrap());
    DW_FACTORY.get()
}

pub struct ZoneOverlay {
    hwnd: HWND,
    render_target: Option<ID2D1HwndRenderTarget>,
    text_format: Option<IDWriteTextFormat>,
    width: i32,
    height: i32,
}

unsafe impl Send for ZoneOverlay {}

impl ZoneOverlay {
    pub fn create(left: i32, top: i32, width: i32, height: i32) -> Option<Self> {
        OVERLAY_REG.call_once(|| {
            let cn = to_wide(OVERLAY_CLASS);
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(overlay_wnd_proc),
                lpszClassName: cn.as_ptr(),
                hInstance: ptr::null_mut(),
                style: 0, cbClsExtra: 0, cbWndExtra: 0,
                hIcon: ptr::null_mut(), hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(), lpszMenuName: ptr::null(),
                hIconSm: ptr::null_mut(),
            };
            unsafe { RegisterClassExW(&wc); }
        });

        let cn = to_wide(OVERLAY_CLASS);
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                cn.as_ptr(), ptr::null(), WS_POPUP,
                left, top, width, height,
                ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), ptr::null_mut(),
            )
        };
        if hwnd.is_null() { return None; }

        // Make window click-through with full alpha
        unsafe { SetLayeredWindowAttributes(hwnd, 0, 255, 0x02); } // LWA_ALPHA

        let mut overlay = Self { hwnd, render_target: None, text_format: None, width, height };
        overlay.init_d2d();
        Some(overlay)
    }

    fn init_d2d(&mut self) {
        let factory = match get_d2d_factory() { Some(f) => f, None => return };
        let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0, dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: windows::Win32::Foundation::HWND(self.hwnd as _),
            pixelSize: D2D_SIZE_U { width: self.width as u32, height: self.height as u32 },
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };
        self.render_target = unsafe { factory.CreateHwndRenderTarget(&rt_props, &hwnd_props).ok() };
        if let Some(ref rt) = self.render_target {
            unsafe { rt.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE); }
        }

        // Create text format for zone numbers
        if let Some(dw) = get_dwrite_factory() {
            let font = to_wide("Segoe UI");
            self.text_format = unsafe {
                dw.CreateTextFormat(
                    windows::core::PCWSTR(font.as_ptr()),
                    None,
                    DWRITE_FONT_WEIGHT_BOLD,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    48.0,
                    windows::core::PCWSTR(to_wide("en-us").as_ptr()),
                ).ok()
            };
            if let Some(ref tf) = self.text_format {
                unsafe {
                    let _ = tf.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
                    let _ = tf.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
                }
            }
        }
    }

    /// Render zone rectangles with numbers. `zones` are screen-relative,
    /// `wa_rect` is the work area origin for offset calculation.
    pub fn render_zones(
        &self,
        zones: &[Rect],
        wa_rect: &Rect,
        colors: &OverlayColors,
        highlight_zone: Option<usize>,
    ) {
        let rt = match &self.render_target { Some(rt) => rt, None => return };

        unsafe {
            rt.BeginDraw();
            rt.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));

            for (i, zone) in zones.iter().enumerate() {
                let is_hl = highlight_zone == Some(i);
                let (cr, cg, cb) = if is_hl { colors.highlight_color } else { colors.zone_color };
                let alpha = if is_hl { colors.opacity as f32 / 255.0 } else { colors.opacity as f32 / 255.0 / 3.0 };

                let fill_color = D2D1_COLOR_F {
                    r: cr as f32 / 255.0, g: cg as f32 / 255.0, b: cb as f32 / 255.0, a: alpha,
                };
                let (br, bg, bb) = colors.border_color;
                let border_color = D2D1_COLOR_F {
                    r: br as f32 / 255.0, g: bg as f32 / 255.0, b: bb as f32 / 255.0,
                    a: colors.opacity as f32 / 255.0,
                };

                let fill_brush = rt.CreateSolidColorBrush(&fill_color, None).ok();
                let border_brush = rt.CreateSolidColorBrush(&border_color, None).ok();

                // Zone rect relative to overlay origin
                let d2d_rect = D2D_RECT_F {
                    left: (zone.left - wa_rect.left) as f32,
                    top: (zone.top - wa_rect.top) as f32,
                    right: (zone.right - wa_rect.left) as f32,
                    bottom: (zone.bottom - wa_rect.top) as f32,
                };

                if let Some(ref fb) = fill_brush {
                    rt.FillRectangle(&d2d_rect, fb);
                }
                if let Some(ref bb) = border_brush {
                    rt.DrawRectangle(&d2d_rect, bb, 2.0, None);
                }

                // Zone number
                if let (Some(tf), Some(nb)) = (&self.text_format, &border_brush) {
                    let num = format!("{}", i + 1);
                    let wide: Vec<u16> = num.encode_utf16().collect();
                    let rt_base: &ID2D1RenderTarget = rt;
                    rt_base.DrawText(&wide, tf, &d2d_rect, nb, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
                }
            }

            let _ = rt.EndDraw(None, None);
        }
    }

    pub fn show(&self) { unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE); } }
    pub fn hide(&self) { unsafe { ShowWindow(self.hwnd, SW_HIDE); } }
    pub fn hwnd(&self) -> HWND { self.hwnd }

    pub fn rect(&self) -> Option<Rect> {
        let mut r = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if unsafe { GetWindowRect(self.hwnd, &mut r) } != 0 {
            Some(Rect::new(r.left, r.top, r.right, r.bottom))
        } else { None }
    }
}

impl Drop for ZoneOverlay {
    fn drop(&mut self) {
        if !self.hwnd.is_null() { unsafe { DestroyWindow(self.hwnd); } }
    }
}

unsafe extern "system" fn overlay_wnd_proc(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlay_colors_from_hex() {
        let c = OverlayColors::from_hex("#FF0000", "#00FF00", "#0000FF", 50);
        assert_eq!(c.zone_color, (255, 0, 0));
        assert_eq!(c.highlight_color, (0, 0, 255));
        assert_eq!(c.opacity, 127);
    }
    #[test]
    fn d2d_factory_creates() { assert!(get_d2d_factory().is_some()); }
    #[test]
    fn dwrite_factory_creates() { assert!(get_dwrite_factory().is_some()); }
}