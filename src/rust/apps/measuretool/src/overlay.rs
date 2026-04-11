//! Win32 overlay window with D2D rendering and screen capture for MeasureTool.
//!
//! Flow:
//! 1. Capture the screen into a PixelBuffer via BitBlt
//! 2. Create a fullscreen transparent overlay with D2D
//! 3. On mouse move: run edge detection + compute measurements
//! 4. Render measurement lines (DrawLine) and dimension labels (DirectWrite)
//! 5. Esc exits the tool

use std::sync::OnceLock;

use measuretool_core::measurement::{compute_cross_measurements, compute_measurement};
use measuretool_core::settings::MeasureToolSettings;
use measuretool_core::types::{consts, MeasureMode, MeasurementResult, PixelBuffer, Point};

use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::*;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── D2D / DirectWrite factories ────────────────────────────────────────────

static D2D_FACTORY: OnceLock<ID2D1Factory1> = OnceLock::new();
static DW_FACTORY: OnceLock<IDWriteFactory> = OnceLock::new();

fn get_d2d_factory() -> Option<&'static ID2D1Factory1> {
    D2D_FACTORY.get_or_init(|| unsafe {
        D2D1CreateFactory::<ID2D1Factory1>(D2D1_FACTORY_TYPE_MULTI_THREADED, None)
            .expect("D2D1CreateFactory failed")
    });
    D2D_FACTORY.get()
}

fn get_dw_factory() -> Option<&'static IDWriteFactory> {
    DW_FACTORY.get_or_init(|| unsafe {
        DWriteCreateFactory::<IDWriteFactory>(DWRITE_FACTORY_TYPE_SHARED)
            .expect("DWriteCreateFactory failed")
    });
    DW_FACTORY.get()
}

// ── Per-window state ───────────────────────────────────────────────────────

struct OverlayState {
    pixels: Vec<u32>,
    pitch: usize,
    width: usize,
    height: usize,
    mode: MeasureMode,
    settings: MeasureToolSettings,
    cursor: Point,
    measurement: Option<MeasurementResult>,
    h_measurement: Option<MeasurementResult>,
    v_measurement: Option<MeasurementResult>,
    render_target: Option<ID2D1HwndRenderTarget>,
    text_format: Option<IDWriteTextFormat>,
}

// ── Public entry point ─────────────────────────────────────────────────────

pub fn run_overlay(mode: MeasureMode, settings: MeasureToolSettings) -> i32 {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let class: Vec<u16> = "PowerToys_MeasureTool_Overlay\0".encode_utf16().collect();

        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance = hinst;
        wc.lpszClassName = class.as_ptr();
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_CROSS);
        RegisterClassExW(&wc);

        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        // Capture screen via BitBlt
        let (pixels, pitch) = capture_screen(vx, vy, vw, vh);

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            vx, vy, vw, vh,
            std::ptr::null_mut(), std::ptr::null_mut(), hinst, std::ptr::null(),
        );
        if hwnd.is_null() { return 1; }

        let rt = create_render_target(hwnd, vw, vh);
        let tf = create_text_format();

        let state = Box::new(OverlayState {
            pixels,
            pitch,
            width: vw as usize,
            height: vh as usize,
            mode,
            settings,
            cursor: Point { x: vw / 2, y: vh / 2 },
            measurement: None,
            h_measurement: None,
            v_measurement: None,
            render_target: rt,
            text_format: tf,
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnregisterClassW(class.as_ptr(), hinst);
        0
    }
}

// ── Screen capture ─────────────────────────────────────────────────────────

unsafe fn capture_screen(x: i32, y: i32, w: i32, h: i32) -> (Vec<u32>, usize) {
    unsafe {
        let hdc_screen = GetDC(std::ptr::null_mut());
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbmp = CreateCompatibleBitmap(hdc_screen, w, h);
        let old = SelectObject(hdc_mem, hbmp);

        BitBlt(hdc_mem, 0, 0, w, h, hdc_screen, x, y, SRCCOPY);

        let pixel_count = w as usize * h as usize;
        let mut pixels = vec![0u32; pixel_count];

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        GetDIBits(
            hdc_mem, hbmp, 0, h as u32,
            pixels.as_mut_ptr() as *mut _,
            &mut bmi, DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old);
        DeleteObject(hbmp);
        DeleteDC(hdc_mem);
        ReleaseDC(std::ptr::null_mut(), hdc_screen);

        (pixels, w as usize)
    }
}

// ── D2D / DWrite setup ────────────────────────────────────────────────────

fn create_render_target(hwnd: HWND, w: i32, h: i32) -> Option<ID2D1HwndRenderTarget> {
    let factory = get_d2d_factory()?;
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
        hwnd: windows::Win32::Foundation::HWND(hwnd as *mut _),
        pixelSize: D2D_SIZE_U { width: w as u32, height: h as u32 },
        presentOptions: D2D1_PRESENT_OPTIONS_NONE,
    };
    unsafe {
        let rt = factory.CreateHwndRenderTarget(&rt_props, &hwnd_props).ok()?;
        rt.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        Some(rt)
    }
}

fn create_text_format() -> Option<IDWriteTextFormat> {
    let dw = get_dw_factory()?;
    let font: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
    unsafe {
        dw.CreateTextFormat(
            windows::core::PCWSTR(font.as_ptr()),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            consts::FONT_SIZE,
            windows::core::PCWSTR(std::ptr::null()),
        ).ok()
    }
}

// ── Rendering ──────────────────────────────────────────────────────────────

fn render(state: &OverlayState) {
    let rt = match &state.render_target { Some(r) => r, None => return };

    let lc = &state.settings.line_color;
    let line_color = D2D1_COLOR_F {
        r: lc.r as f32 / 255.0, g: lc.g as f32 / 255.0,
        b: lc.b as f32 / 255.0, a: 1.0,
    };

    unsafe {
        rt.BeginDraw();
        rt.Clear(Some(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }));

        if let Ok(brush) = rt.CreateSolidColorBrush(&line_color, None) {
            let cx = state.cursor.x as f32;
            let cy = state.cursor.y as f32;

            match state.mode {
                MeasureMode::Cross => {
                    if let (Some(hm), Some(vm)) = (&state.h_measurement, &state.v_measurement) {
                        draw_line(rt, &brush, hm.rect.left, cy, hm.rect.right, cy, 1.0);
                        draw_line(rt, &brush, cx, vm.rect.top, cx, vm.rect.bottom, 1.0);

                        if state.settings.draw_feet_on_cross {
                            let f = consts::FEET_HALF_LENGTH;
                            draw_line(rt, &brush, hm.rect.left, cy - f, hm.rect.left, cy + f, 1.0);
                            draw_line(rt, &brush, hm.rect.right, cy - f, hm.rect.right, cy + f, 1.0);
                            draw_line(rt, &brush, cx - f, vm.rect.top, cx + f, vm.rect.top, 1.0);
                            draw_line(rt, &brush, cx - f, vm.rect.bottom, cx + f, vm.rect.bottom, 1.0);
                        }

                        let unit = state.settings.units;
                        let h_text = hm.format(true, false, unit);
                        let v_text = vm.format(false, true, unit);
                        draw_label(rt, state, &brush, &h_text, cx + 8.0, cy - 20.0);
                        draw_label(rt, state, &brush, &v_text, cx + 8.0, cy + 4.0);
                    }
                }
                MeasureMode::Horizontal => {
                    if let Some(m) = &state.measurement {
                        draw_line(rt, &brush, m.rect.left, cy, m.rect.right, cy, 1.0);
                        let unit = state.settings.units;
                        let text = m.format(true, false, unit);
                        draw_label(rt, state, &brush, &text, cx + 8.0, cy - 20.0);
                    }
                }
                MeasureMode::Vertical => {
                    if let Some(m) = &state.measurement {
                        draw_line(rt, &brush, cx, m.rect.top, cx, m.rect.bottom, 1.0);
                        let unit = state.settings.units;
                        let text = m.format(false, true, unit);
                        draw_label(rt, state, &brush, &text, cx + 8.0, cy - 20.0);
                    }
                }
                MeasureMode::Bounds => {
                    if let Some(m) = &state.measurement {
                        let r = &m.rect;
                        draw_line(rt, &brush, r.left, r.top, r.right, r.top, 1.0);
                        draw_line(rt, &brush, r.left, r.bottom, r.right, r.bottom, 1.0);
                        draw_line(rt, &brush, r.left, r.top, r.left, r.bottom, 1.0);
                        draw_line(rt, &brush, r.right, r.top, r.right, r.bottom, 1.0);
                        let unit = state.settings.units;
                        let text = m.format(true, true, unit);
                        draw_label(rt, state, &brush, &text, r.right + 4.0, r.top - 20.0);
                    }
                }
            }
        }

        let _ = rt.EndDraw(None, None);
    }
}

fn draw_line(
    rt: &ID2D1HwndRenderTarget, brush: &ID2D1SolidColorBrush,
    x1: f32, y1: f32, x2: f32, y2: f32, width: f32,
) {
    let p0 = D2D_POINT_2F { x: x1, y: y1 };
    let p1 = D2D_POINT_2F { x: x2, y: y2 };
    unsafe { rt.DrawLine(p0, p1, brush, width, None); }
}

fn draw_label(
    rt: &ID2D1HwndRenderTarget, state: &OverlayState,
    brush: &ID2D1SolidColorBrush, text: &str, x: f32, y: f32,
) {
    let tf = match &state.text_format { Some(f) => f, None => return };
    let wide: Vec<u16> = text.encode_utf16().collect();
    let rect = D2D_RECT_F { left: x, top: y, right: x + 200.0, bottom: y + 30.0 };
    unsafe {
        rt.DrawText(&wide, tf, &rect, brush, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
    }
}

// ── Measurement update ─────────────────────────────────────────────────────

fn update_measurement(state: &mut OverlayState) {
    let buf = PixelBuffer::new(&state.pixels, state.pitch, state.width, state.height);
    let per_ch = state.settings.per_color_channel_edge_detection;
    let tol = state.settings.pixel_tolerance as u8;

    match state.mode {
        MeasureMode::Cross => {
            let (h, v) = compute_cross_measurements(&buf, state.cursor, per_ch, tol, 0.0);
            state.h_measurement = Some(h);
            state.v_measurement = Some(v);
        }
        _ => {
            state.measurement = Some(compute_measurement(
                &buf, state.cursor, state.mode, per_ch, tol, 0.0,
            ));
        }
    }
}

// ── Window procedure ───────────────────────────────────────────────────────

unsafe extern "system" fn wnd_proc(
    hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM,
) -> LRESULT {
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayState;
        if ptr.is_null() {
            return DefWindowProcW(hwnd, msg, wp, lp);
        }
        let state = &mut *ptr;

        match msg {
            WM_MOUSEMOVE => {
                let x = (lp & 0xFFFF) as i16 as i32;
                let y = ((lp >> 16) & 0xFFFF) as i16 as i32;
                state.cursor = Point { x, y };
                update_measurement(state);
                render(state);
                0
            }
            WM_KEYDOWN => {
                if wp == VK_ESCAPE as usize {
                    DestroyWindow(hwnd);
                }
                0
            }
            WM_CLOSE => { DestroyWindow(hwnd); 0 }
            WM_DESTROY => {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let _ = Box::from_raw(ptr);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}
