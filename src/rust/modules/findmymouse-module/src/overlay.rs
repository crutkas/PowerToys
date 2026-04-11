//! FindMyMouse D2D spotlight overlay.
//!
//! Creates a fullscreen transparent layered window covering the virtual screen,
//! uses Direct2D to render a dim background with a radial gradient spotlight
//! centered on the cursor position.

use findmymouse_core::types::Settings;
use std::sync::OnceLock;

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::core::Interface;

use windows_sys::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;

static D2D_FACTORY: OnceLock<ID2D1Factory1> = OnceLock::new();

fn get_d2d_factory() -> Option<&'static ID2D1Factory1> {
    D2D_FACTORY.get_or_init(|| unsafe {
        D2D1CreateFactory::<ID2D1Factory1>(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)
            .ok()
            .unwrap()
    });
    D2D_FACTORY.get()
}

const OVERLAY_CLASS_NAME: &str = "FindMyMouse_Overlay_Rust";

static OVERLAY_CLASS_REGISTERED: std::sync::Once = std::sync::Once::new();

/// The D2D spotlight overlay state.
pub struct SpotlightOverlay {
    hwnd: windows_sys::Win32::Foundation::HWND,
    render_target: Option<ID2D1HwndRenderTarget>,
    dim_brush: Option<ID2D1SolidColorBrush>,
    gradient_brush: Option<ID2D1RadialGradientBrush>,
    cursor_pos: POINT,
    visible: bool,
    settings: OverlaySettings,
    /// Current animated radius (starts large, shrinks to target).
    current_radius: f32,
    animation_start_ms: u64,
}

/// Rendering-specific settings extracted from the core Settings.
#[derive(Clone)]
struct OverlaySettings {
    spotlight_radius: f32,
    initial_zoom: f32,
    animation_duration_ms: u32,
    bg_color: D2D1_COLOR_F,
    spot_color: D2D1_COLOR_F,
}

impl From<&Settings> for OverlaySettings {
    fn from(s: &Settings) -> Self {
        Self {
            spotlight_radius: s.spotlight_radius as f32,
            initial_zoom: s.spotlight_initial_zoom as f32,
            animation_duration_ms: s.animation_duration_ms as u32,
            bg_color: argb_to_d2d(s.background_color),
            spot_color: argb_to_d2d(s.spotlight_color),
        }
    }
}

fn argb_to_d2d(c: (u8, u8, u8, u8)) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        a: c.0 as f32 / 255.0,
        r: c.1 as f32 / 255.0,
        g: c.2 as f32 / 255.0,
        b: c.3 as f32 / 255.0,
    }
}

unsafe impl Send for SpotlightOverlay {}

impl SpotlightOverlay {
    /// Create a new overlay (window not yet shown).
    pub fn new(hinstance: HINSTANCE, settings: &Settings) -> Option<Self> {
        let class_wide = wide(OVERLAY_CLASS_NAME);

        OVERLAY_CLASS_REGISTERED.call_once(|| unsafe {
            let mut wc: WNDCLASSW = std::mem::zeroed();
            wc.lpfnWndProc = Some(overlay_wnd_proc);
            wc.hInstance = hinstance;
            wc.lpszClassName = class_wide.as_ptr();
            wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
            RegisterClassW(&wc);
        });

        // Cover the entire virtual screen (1px inset to avoid taskbar transparency glitch).
        let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) } + 1;
        let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) } + 1;
        let w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) } - 2;
        let h = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) } - 2;

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                class_wide.as_ptr(),
                std::ptr::null(),
                WS_POPUP,
                x,
                y,
                w,
                h,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null(),
            )
        };
        if hwnd.is_null() {
            return None;
        }

        // Make the window fully transparent initially.
        unsafe {
            SetLayeredWindowAttributes(hwnd, 0, 255, 0x02 /* LWA_ALPHA */);
        }

        let os = OverlaySettings::from(settings);
        let mut overlay = Self {
            hwnd,
            render_target: None,
            dim_brush: None,
            gradient_brush: None,
            cursor_pos: POINT { x: 0, y: 0 },
            visible: false,
            settings: os,
            current_radius: 0.0,
            animation_start_ms: 0,
        };
        overlay.init_d2d(w, h);
        Some(overlay)
    }

    fn init_d2d(&mut self, w: i32, h: i32) {
        let factory = match get_d2d_factory() {
            Some(f) => f,
            None => return,
        };

        let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };

        let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: HWND(self.hwnd as _),
            pixelSize: D2D_SIZE_U {
                width: w as u32,
                height: h as u32,
            },
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };

        let rt = unsafe { factory.CreateHwndRenderTarget(&rt_props, &hwnd_props).ok() };
        if let Some(ref rt) = rt {
            unsafe {
                rt.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            }
        }
        self.render_target = rt;
    }

    /// Show the overlay with spotlight at the current cursor position.
    pub fn activate(&mut self) {
        unsafe {
            GetCursorPos(&mut self.cursor_pos);
        }
        self.current_radius = self.settings.spotlight_radius * self.settings.initial_zoom;
        self.animation_start_ms = tick_count();
        self.visible = true;

        // Reposition to cover virtual screen.
        let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) } + 1;
        let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) } + 1;
        let w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) } - 2;
        let h = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) } - 2;

        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE,
            );
            ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }

        // Resize render target.
        if let Some(ref rt) = self.render_target {
            let new_size = D2D_SIZE_U {
                width: w as u32,
                height: h as u32,
            };
            let _ = unsafe { rt.Resize(&new_size) };
        } else {
            self.init_d2d(w, h);
        }

        self.render();
    }

    /// Hide the overlay.
    pub fn deactivate(&mut self) {
        self.visible = false;
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    /// Update the spotlight position to follow the cursor and re-render.
    pub fn update_cursor(&mut self) {
        if !self.visible {
            return;
        }
        unsafe {
            GetCursorPos(&mut self.cursor_pos);
        }

        // Animate radius: lerp from (radius * zoom) down to radius.
        let elapsed = tick_count().saturating_sub(self.animation_start_ms);
        let duration = self.settings.animation_duration_ms as u64;
        let t = if duration == 0 {
            1.0_f32
        } else {
            (elapsed as f32 / duration as f32).min(1.0)
        };
        let start_r = self.settings.spotlight_radius * self.settings.initial_zoom;
        let end_r = self.settings.spotlight_radius;
        self.current_radius = start_r + (end_r - start_r) * t;

        self.render();
    }

    /// Apply new settings (e.g. after config change).
    pub fn apply_settings(&mut self, settings: &Settings) {
        self.settings = OverlaySettings::from(settings);
    }

    /// Returns true if the overlay is currently visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Destroy the overlay window.
    pub fn destroy(&mut self) {
        self.deactivate();
        if !self.hwnd.is_null() {
            unsafe {
                DestroyWindow(self.hwnd);
            }
            self.hwnd = std::ptr::null_mut();
        }
    }

    /// Render the spotlight effect.
    fn render(&mut self) {
        let rt = match &self.render_target {
            Some(rt) => rt,
            None => return,
        };

        // Convert cursor position to client coordinates.
        let mut client_pt = self.cursor_pos;
        unsafe {
            ScreenToClient(self.hwnd, &mut client_pt);
        }

        let center = D2D_POINT_2F {
            x: client_pt.x as f32,
            y: client_pt.y as f32,
        };
        let radius = self.current_radius;

        // Use windows::core::Interface to get the base ID2D1RenderTarget.
        let base_rt: ID2D1RenderTarget = rt.cast().unwrap();

        unsafe {
            base_rt.BeginDraw();
            base_rt.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            // Get render target size for the full-screen dim fill.
            let size = base_rt.GetSize();

            // Create dim brush (background color).
            let dim_brush = base_rt
                .CreateSolidColorBrush(&self.settings.bg_color, None)
                .ok();
            if let Some(ref brush) = dim_brush {
                let rect = D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: size.width,
                    bottom: size.height,
                };
                base_rt.FillRectangle(&rect, brush);
            }
            self.dim_brush = dim_brush;

            // Create radial gradient brush for the spotlight hole.
            // Center = transparent, edge = dim color (to mask the dim layer).
            let stops = [
                D2D1_GRADIENT_STOP {
                    position: 0.0,
                    color: D2D1_COLOR_F {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    },
                },
                D2D1_GRADIENT_STOP {
                    position: 1.0,
                    color: self.settings.bg_color,
                },
            ];

            let gradient_stops = base_rt.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP);

            if let Ok(ref stop_collection) = gradient_stops {
                let gradient_props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES {
                    center,
                    gradientOriginOffset: D2D_POINT_2F { x: 0.0, y: 0.0 },
                    radiusX: radius,
                    radiusY: radius,
                };

                let gradient_brush = base_rt.CreateRadialGradientBrush(&gradient_props, None, stop_collection);

                if let Ok(ref brush) = gradient_brush {
                    // Draw the spotlight: fill the entire screen with the radial gradient.
                    // The center is transparent, creating a "hole" in the dim layer.
                    let spot_rect = D2D_RECT_F {
                        left: center.x - radius,
                        top: center.y - radius,
                        right: center.x + radius,
                        bottom: center.y + radius,
                    };

                    // Clear the spotlight area, then re-fill with gradient.
                    base_rt.PushAxisAlignedClip(&spot_rect, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
                    base_rt.Clear(Some(&D2D1_COLOR_F {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }));
                    base_rt.FillRectangle(&spot_rect, brush);
                    base_rt.PopAxisAlignedClip();

                    // Draw the spotlight fill (colored ellipse at center).
                    let ellipse = D2D1_ELLIPSE {
                        point: center,
                        radiusX: radius * 0.95,
                        radiusY: radius * 0.95,
                    };
                    if let Ok(spot_brush) =
                        base_rt.CreateSolidColorBrush(&self.settings.spot_color, None)
                    {
                        base_rt.FillEllipse(&ellipse, &spot_brush);
                    }
                }
                self.gradient_brush = gradient_brush.ok();
            }

            let _ = base_rt.EndDraw(None, None);
        }
    }
}

impl Drop for SpotlightOverlay {
    fn drop(&mut self) {
        self.destroy();
    }
}

fn tick_count() -> u64 {
    unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_NCHITTEST => HTTRANSPARENT as LRESULT,
            WM_ERASEBKGND => 1,
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argb_to_d2d_transparent_black() {
        let color = argb_to_d2d((0, 0, 0, 0));
        assert_eq!(color.a, 0.0);
        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
    }

    #[test]
    fn test_argb_to_d2d_opaque_white() {
        let color = argb_to_d2d((255, 255, 255, 255));
        assert!((color.a - 1.0).abs() < 0.01);
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 1.0).abs() < 0.01);
        assert!((color.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_argb_to_d2d_semitransparent() {
        let color = argb_to_d2d((128, 0, 0, 0));
        assert!((color.a - 128.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_overlay_settings_from_defaults() {
        let s = Settings::default();
        let os = OverlaySettings::from(&s);
        assert_eq!(os.spotlight_radius, 100.0);
        assert_eq!(os.initial_zoom, 9.0);
        assert_eq!(os.animation_duration_ms, 500);
    }

    #[test]
    fn test_d2d_factory_creates() {
        let factory = get_d2d_factory();
        assert!(factory.is_some());
    }

    #[test]
    fn test_wide_string() {
        let w = wide("Hello");
        assert_eq!(w, vec![72, 101, 108, 108, 111, 0]);
    }
}
