//! Fullscreen D2D overlay with WH_MOUSE_LL hook for crosshair rendering.
//!
//! Draws 8 filled rectangles (4 inner + 4 border) centred on the cursor,
//! computed by `crosshairs_core::line_calculator::calculate_crosshair_layout`.

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::OnceLock;

use crosshairs_core::line_calculator::{
    calculate_crosshair_layout, opacity_to_normalized, CrosshairLayout, ScreenBounds,
};
use crosshairs_core::types::Settings;

use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Dxgi::Common::*;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// ── D2D factory singleton ──────────────────────────────────────────────────

static D2D_FACTORY: OnceLock<ID2D1Factory1> = OnceLock::new();

fn get_d2d_factory() -> Option<&'static ID2D1Factory1> {
    D2D_FACTORY.get_or_init(|| unsafe {
        D2D1CreateFactory::<ID2D1Factory1>(D2D1_FACTORY_TYPE_MULTI_THREADED, None)
            .expect("D2D1CreateFactory failed")
    });
    D2D_FACTORY.get()
}

// ── Hook ↔ window communication ────────────────────────────────────────────

static OVERLAY_HWND: AtomicIsize = AtomicIsize::new(0);

fn store_hwnd(h: HWND) { OVERLAY_HWND.store(h as isize, Ordering::SeqCst); }
fn load_hwnd() -> HWND { OVERLAY_HWND.load(Ordering::SeqCst) as HWND }

const WM_CH_MOVE: u32 = WM_APP + 10;
const TIMER_ID: usize = 2;
const TIMER_MS: u32 = 16;

// ── Public handle ──────────────────────────────────────────────────────────

pub struct OverlayHandle {
    thread: Option<std::thread::JoinHandle<()>>,
}

impl OverlayHandle {
    pub fn start(settings: Settings) -> Self {
        let thread = std::thread::spawn(move || overlay_thread(settings));
        Self { thread: Some(thread) }
    }

    pub fn stop(&mut self) {
        let h = load_hwnd();
        store_hwnd(std::ptr::null_mut());
        if !h.is_null() {
            unsafe { PostMessageW(h, WM_CLOSE, 0, 0); }
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

// ── Per-window state ───────────────────────────────────────────────────────

struct OverlayState {
    settings: Settings,
    layout: CrosshairLayout,
    screen: ScreenBounds,
    render_target: Option<ID2D1HwndRenderTarget>,
    hook: HHOOK,
    vx: i32,
    vy: i32,
    cursor_visible: bool,
}

// ── Thread entry ───────────────────────────────────────────────────────────

fn overlay_thread(settings: Settings) {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let class: Vec<u16> = "PowerToys_Crosshairs_Overlay\0".encode_utf16().collect();

        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance = hinst;
        wc.lpszClassName = class.as_ptr();
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        RegisterClassExW(&wc);

        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let screen = ScreenBounds {
            left: vx,
            top: vy,
            right: vx + vw,
            bottom: vy + vh,
        };

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW
                | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
            class.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            vx, vy, vw, vh,
            std::ptr::null_mut(), std::ptr::null_mut(), hinst, std::ptr::null(),
        );
        if hwnd.is_null() { return; }

        SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);

        let rt = create_render_target(hwnd, vw, vh);
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), hinst, 0);

        let initial_layout = calculate_crosshair_layout(vw / 2, vh / 2, screen, &settings);

        let state = Box::new(OverlayState {
            settings,
            layout: initial_layout,
            screen,
            render_target: rt,
            hook,
            vx,
            vy,
            cursor_visible: true,
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

        store_hwnd(hwnd);
        SetTimer(hwnd, TIMER_ID, TIMER_MS, None);
        ShowWindow(hwnd, SW_SHOWNA);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnregisterClassW(class.as_ptr(), hinst);
    }
}

// ── D2D helpers ────────────────────────────────────────────────────────────

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

fn fill_rect(
    rt: &ID2D1HwndRenderTarget,
    brush: &ID2D1SolidColorBrush,
    r: &crosshairs_core::line_calculator::Rect,
    ox: f32,
    oy: f32,
) {
    if r.width <= 0.0 || r.height <= 0.0 { return; }
    let rect = D2D_RECT_F {
        left: r.x - ox,
        top: r.y - oy,
        right: r.x + r.width - ox,
        bottom: r.y + r.height - oy,
    };
    unsafe { rt.FillRectangle(&rect, brush); }
}

fn render(state: &OverlayState) {
    let rt = match &state.render_target { Some(r) => r, None => return };

    let alpha = opacity_to_normalized(state.settings.opacity);
    let c = &state.settings.color;
    let bc = &state.settings.border_color;
    let ox = state.vx as f32;
    let oy = state.vy as f32;

    unsafe {
        rt.BeginDraw();
        rt.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));

        if !state.cursor_visible && state.settings.auto_hide {
            let _ = rt.EndDraw(None, None);
            return;
        }

        let border_color = D2D1_COLOR_F {
            r: bc.r as f32 / 255.0, g: bc.g as f32 / 255.0,
            b: bc.b as f32 / 255.0, a: alpha,
        };
        let inner_color = D2D1_COLOR_F {
            r: c.r as f32 / 255.0, g: c.g as f32 / 255.0,
            b: c.b as f32 / 255.0, a: alpha,
        };

        if let (Ok(bb), Ok(ib)) = (
            rt.CreateSolidColorBrush(&border_color, None),
            rt.CreateSolidColorBrush(&inner_color, None),
        ) {
            let l = &state.layout;
            // Border rects first (behind inner)
            fill_rect(rt, &bb, &l.left_border, ox, oy);
            fill_rect(rt, &bb, &l.right_border, ox, oy);
            fill_rect(rt, &bb, &l.top_border, ox, oy);
            fill_rect(rt, &bb, &l.bottom_border, ox, oy);
            // Inner rects on top
            fill_rect(rt, &ib, &l.left_inner, ox, oy);
            fill_rect(rt, &ib, &l.right_inner, ox, oy);
            fill_rect(rt, &ib, &l.top_inner, ox, oy);
            fill_rect(rt, &ib, &l.bottom_inner, ox, oy);
        }

        let _ = rt.EndDraw(None, None);
    }
}

// ── Coordinate packing ─────────────────────────────────────────────────────

fn pack_coords(x: i32, y: i32) -> LPARAM {
    ((x as u32 as u64) | ((y as u32 as u64) << 32)) as LPARAM
}

fn unpack_coords(lp: LPARAM) -> (i32, i32) {
    let x = lp as u32 as i32;
    let y = ((lp as u64) >> 32) as u32 as i32;
    (x, y)
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
            WM_TIMER if wp == TIMER_ID => {
                // Auto-hide: check if cursor is suppressed
                if state.settings.auto_hide {
                    let mut ci: CURSORINFO = std::mem::zeroed();
                    ci.cbSize = std::mem::size_of::<CURSORINFO>() as u32;
                    if GetCursorInfo(&mut ci) != 0 {
                        state.cursor_visible = (ci.flags & CURSOR_SHOWING) != 0;
                    }
                }
                render(state);
                0
            }
            WM_CH_MOVE => {
                let (x, y) = unpack_coords(lp);
                state.layout = calculate_crosshair_layout(
                    x, y, state.screen, &state.settings,
                );
                0
            }
            WM_CLOSE => { DestroyWindow(hwnd); 0 }
            WM_DESTROY => {
                store_hwnd(std::ptr::null_mut());
                if !state.hook.is_null() {
                    UnhookWindowsHookEx(state.hook);
                    state.hook = std::ptr::null_mut();
                }
                KillTimer(hwnd, TIMER_ID);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let _ = Box::from_raw(ptr);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

// ── Low-level mouse hook ───────────────────────────────────────────────────

unsafe extern "system" fn mouse_hook_proc(
    code: i32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if code >= 0 && wparam as u32 == WM_MOUSEMOVE {
            let h = load_hwnd();
            if !h.is_null() {
                let ms = &*(lparam as *const MSLLHOOKSTRUCT);
                PostMessageW(h, WM_CH_MOVE, 0, pack_coords(ms.pt.x, ms.pt.y));
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }
}
