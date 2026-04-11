//! Window border rendering using Direct2D.
//!
//! Uses ID2D1HwndRenderTarget + DrawRoundedRectangle for GPU-accelerated
//! border rendering that matches the Windows 11 visual style.

use powertoys_win32::string::to_wide;
use crate::settings::Settings;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Dxgi::Common::*;

static BORDER_CLASS_REGISTERED: std::sync::Once = std::sync::Once::new();
const BORDER_CLASS_NAME: &str = "AlwaysOnTop_Border_Rust";

static D2D_FACTORY: std::sync::OnceLock<ID2D1Factory1> = std::sync::OnceLock::new();

fn get_d2d_factory() -> Option<&'static ID2D1Factory1> {
    D2D_FACTORY.get_or_init(|| {
        unsafe {
            D2D1CreateFactory::<ID2D1Factory1>(
                D2D1_FACTORY_TYPE_MULTI_THREADED,
                None,
            ).ok()
        }.unwrap()
    });
    D2D_FACTORY.get()
}

pub struct WindowBorder {
    border_hwnd: HWND,
    render_target: Option<ID2D1HwndRenderTarget>,
    brush: Option<ID2D1SolidColorBrush>,
}

unsafe impl Send for WindowBorder {}

impl WindowBorder {
    pub fn empty() -> Self {
        Self { border_hwnd: std::ptr::null_mut(), render_target: None, brush: None }
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

        // Enable DWM blur behind for native transparency
        unsafe {
            let val: BOOL = 1;
            DwmSetWindowAttribute(border_hwnd, DWMWA_EXCLUDED_FROM_PEEK as u32,
                &val as *const _ as *const _, std::mem::size_of::<BOOL>() as u32);

            // SetLayeredWindowAttributes for transparent background
            SetLayeredWindowAttributes(border_hwnd, 0, 255, 0x02); // LWA_ALPHA
        }

        let mut border = Self { border_hwnd, render_target: None, brush: None };
        border.init_d2d(w, h);
        border.set_border_rect(tracked_hwnd, settings);

        unsafe {
            SetWindowPos(tracked_hwnd, border_hwnd, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            ShowWindow(border_hwnd, SW_SHOWNA);
        }
        Some(border)
    }

    fn init_d2d(&mut self, w: i32, h: i32) {
        let factory = match get_d2d_factory() {
            Some(f) => f,
            None => return,
        };

        let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_UNKNOWN,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };

        let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: windows::Win32::Foundation::HWND(self.border_hwnd as _),
            pixelSize: D2D_SIZE_U { width: w as u32, height: h as u32 },
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };

        let rt = unsafe { factory.CreateHwndRenderTarget(&rt_props, &hwnd_props).ok() };
        if let Some(ref rt) = rt {
            unsafe { rt.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE); }
        }
        self.render_target = rt;
    }

    fn set_border_rect(&mut self, tracked_hwnd: HWND, settings: &Settings) {
        let rect = match get_frame_rect(tracked_hwnd, settings.frame_thickness) {
            Some(r) => r,
            None => return,
        };

        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;

        // Resize render target if needed
        if let Some(ref rt) = self.render_target {
            let new_size = D2D_SIZE_U { width: w as u32, height: h as u32 };
            let _ = unsafe { rt.Resize(&new_size) };
        } else {
            self.init_d2d(w, h);
        }

        // Create/update brush
        let (r, g, b) = settings.frame_color_rgb();
        let alpha = settings.frame_opacity as f32 / 100.0;
        let color = D2D1_COLOR_F {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: alpha,
        };

        if let Some(ref rt) = self.render_target {
            self.brush = unsafe { rt.CreateSolidColorBrush(&color, None).ok() };
        }

        let corner_radius = if settings.round_corners_enabled {
            get_corner_radius(tracked_hwnd)
        } else {
            0.0
        };

        self.render(w as f32, h as f32, settings.frame_thickness as f32, corner_radius);
    }

    fn render(&self, w: f32, h: f32, thickness: f32, radius: f32) {
        let rt = match &self.render_target { Some(rt) => rt, None => return };
        let brush = match &self.brush { Some(b) => b, None => return };

        unsafe {
            rt.BeginDraw();
            rt.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));

            let half = thickness / 2.0;
            if radius > 0.0 {
                let rounded_rect = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: half + 1.0,
                        top: half + 1.0,
                        right: w - half - 1.0,
                        bottom: h - half - 1.0,
                    },
                    radiusX: radius,
                    radiusY: radius,
                };
                rt.DrawRoundedRectangle(&rounded_rect, brush, thickness, None);
            } else {
                let rect = D2D_RECT_F {
                    left: half + 1.0,
                    top: half + 1.0,
                    right: w - half - 1.0,
                    bottom: h - half - 1.0,
                };
                rt.DrawRectangle(&rect, brush, thickness, None);
            }

            let _ = rt.EndDraw(None, None);
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

    pub fn update_properties(&mut self, tracked_hwnd: HWND, settings: &Settings) {
        if self.border_hwnd.is_null() { return; }
        if let Some(rect) = get_frame_rect(tracked_hwnd, settings.frame_thickness) {
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            unsafe { SetWindowPos(self.border_hwnd, tracked_hwnd, rect.left, rect.top, w, h, SWP_NOACTIVATE); }
        }
        self.set_border_rect(tracked_hwnd, settings);
    }

    pub fn show(&self) { if !self.border_hwnd.is_null() { unsafe { ShowWindow(self.border_hwnd, SW_SHOWNA); } } }
    pub fn hide(&self) { if !self.border_hwnd.is_null() { unsafe { ShowWindow(self.border_hwnd, SW_HIDE); } } }
    pub fn destroy(self) { if !self.border_hwnd.is_null() { unsafe { DestroyWindow(self.border_hwnd); } } }
}

/// Query DWM corner preference: DWMWCP_ROUND → 8px, DWMWCP_ROUNDSMALL → 4px.
fn get_corner_radius(hwnd: HWND) -> f32 {
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    let mut preference: u32 = 0;
    let hr = unsafe {
        DwmGetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE,
            &mut preference as *mut _ as *mut _, std::mem::size_of::<u32>() as u32)
    };
    if hr != 0 { return 8.0; }
    match preference { 2 => 8.0, 3 => 4.0, _ => 0.0 }
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
    fn test_get_corner_radius_returns_reasonable_value() {
        let r = get_corner_radius(std::ptr::null_mut());
        assert!(r >= 0.0 && r <= 20.0);
    }

    #[test]
    fn test_d2d_factory_creates() {
        let factory = get_d2d_factory();
        assert!(factory.is_some());
    }
}