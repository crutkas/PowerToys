//! Fullscreen D2D overlay with WH_MOUSE_LL hook for mouse highlighting.
//!
//! Spawns a dedicated thread that owns:
//! - A transparent layered popup covering the virtual screen
//! - An ID2D1DCRenderTarget rendering to a 32-bit DIB for per-pixel alpha
//! - A low-level mouse hook forwarding events via PostMessage
//! - A 16 ms timer driving the render loop (~60 FPS)

use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use highlighter_core::highlight_manager::HighlightManager;
use highlighter_core::types::{MouseButton, Settings};

use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Dxgi::Common::*;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
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

/// Stores the overlay HWND as isize for cross-thread access.
static OVERLAY_HWND: AtomicIsize = AtomicIsize::new(0);
static EPOCH: OnceLock<Instant> = OnceLock::new();

fn now_ms() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn store_hwnd(h: HWND) { OVERLAY_HWND.store(h as isize, Ordering::SeqCst); }
fn load_hwnd() -> HWND { OVERLAY_HWND.load(Ordering::SeqCst) as HWND }

const WM_HL_LDOWN: u32 = WM_APP + 1;
const WM_HL_LUP: u32 = WM_APP + 2;
const WM_HL_RDOWN: u32 = WM_APP + 3;
const WM_HL_RUP: u32 = WM_APP + 4;
const WM_HL_MOVE: u32 = WM_APP + 5;

const TIMER_ID: usize = 1;
const TIMER_MS: u32 = 16;

// ── Public handle ──────────────────────────────────────────────────────────

pub struct OverlayHandle {
    thread: Option<std::thread::JoinHandle<()>>,
}

impl OverlayHandle {
    pub fn start(settings: Settings) -> Self {
        let thread = std::thread::spawn(move || overlay_thread(settings));
        Self {
            thread: Some(thread),
        }
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

struct OverlayData {
    manager: HighlightManager,
    render_target: Option<ID2D1DCRenderTarget>,
    hook: HHOOK,
    vx: i32,
    vy: i32,
    vw: i32,
    vh: i32,
}

// ── Thread entry ───────────────────────────────────────────────────────────

fn overlay_thread(settings: Settings) {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let class: Vec<u16> = "PowerToys_Highlighter_Overlay\0"
            .encode_utf16()
            .collect();

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

        // Per-pixel alpha via UpdateLayeredWindow — do NOT call SetLayeredWindowAttributes

        let rt = create_dc_render_target();
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), hinst, 0);

        let data = Box::new(OverlayData {
            manager: HighlightManager::new(settings),
            render_target: rt,
            hook,
            vx, vy, vw, vh,
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(data) as isize);

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

fn create_dc_render_target() -> Option<ID2D1DCRenderTarget> {
    let factory = get_d2d_factory()?;
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
    unsafe { factory.CreateDCRenderTarget(&rt_props).ok() }
}

/// Render highlights onto a memory DC, then call UpdateLayeredWindow for per-pixel alpha.
fn render(hwnd: HWND, data: &OverlayData) {
    let rt = match &data.render_target {
        Some(rt) => rt,
        None => return,
    };

    let now = now_ms();
    let highlights = data.manager.get_visible_highlights(now);
    let radius = data.manager.settings().radius as f32;
    let w = data.vw;
    let h = data.vh;

    unsafe {
        // Create memory DC backed by a 32-bit ARGB DIB section
        let screen_dc = GetDC(std::ptr::null_mut());
        let mem_dc = CreateCompatibleDC(screen_dc);

        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let dib = CreateDIBSection(
            mem_dc, &bmi, DIB_RGB_COLORS, &mut bits,
            std::ptr::null_mut(), 0,
        );
        if dib.is_null() {
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return;
        }
        let old_bmp = SelectObject(mem_dc, dib as _);

        // Bind D2D DC render target to the memory DC
        let bind_rect = windows::Win32::Foundation::RECT {
            left: 0, top: 0, right: w, bottom: h,
        };
        let hdc = windows::Win32::Graphics::Gdi::HDC(mem_dc as *mut _);
        if rt.BindDC(hdc, &bind_rect).is_err() {
            SelectObject(mem_dc, old_bmp);
            DeleteObject(dib as _);
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return;
        }

        rt.BeginDraw();
        rt.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));

        for hl in &highlights {
            let color = D2D1_COLOR_F {
                r: hl.color.r as f32 / 255.0,
                g: hl.color.g as f32 / 255.0,
                b: hl.color.b as f32 / 255.0,
                a: (hl.color.a as f32 / 255.0) * hl.opacity as f32,
            };
            if let Ok(brush) = rt.CreateSolidColorBrush(&color, None) {
                let ellipse = D2D1_ELLIPSE {
                    point: D2D_POINT_2F {
                        x: hl.x as f32 - data.vx as f32,
                        y: hl.y as f32 - data.vy as f32,
                    },
                    radiusX: radius,
                    radiusY: radius,
                };
                rt.FillEllipse(&ellipse, &brush);
            }
        }

        let _ = rt.EndDraw(None, None);

        // Update the layered window with per-pixel alpha from the DIB
        let pt_src = POINT { x: 0, y: 0 };
        let pt_dst = POINT { x: data.vx, y: data.vy };
        let size = SIZE { cx: w, cy: h };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        UpdateLayeredWindow(
            hwnd, screen_dc, &pt_dst, &size,
            mem_dc, &pt_src, 0, &blend, ULW_ALPHA,
        );

        // Cleanup GDI objects
        SelectObject(mem_dc, old_bmp);
        DeleteObject(dib as _);
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);
    }
}

// ── Coordinate packing (two i32 in one isize, works on x64) ───────────────

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
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayData;
        if ptr.is_null() {
            return DefWindowProcW(hwnd, msg, wp, lp);
        }
        let data = &mut *ptr;

        match msg {
            WM_TIMER if wp == TIMER_ID => {
                data.manager.cleanup(now_ms());
                render(hwnd, data);
                0
            }
            WM_HL_LDOWN => {
                let (x, y) = unpack_coords(lp);
                data.manager.on_mouse_down(MouseButton::Left, x, y, now_ms());
                0
            }
            WM_HL_LUP => {
                data.manager.on_mouse_up(MouseButton::Left, now_ms());
                0
            }
            WM_HL_RDOWN => {
                let (x, y) = unpack_coords(lp);
                data.manager.on_mouse_down(MouseButton::Right, x, y, now_ms());
                0
            }
            WM_HL_RUP => {
                data.manager.on_mouse_up(MouseButton::Right, now_ms());
                0
            }
            WM_HL_MOVE => {
                let (x, y) = unpack_coords(lp);
                data.manager.on_mouse_move(x, y);
                0
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                store_hwnd(std::ptr::null_mut());
                if !data.hook.is_null() {
                    UnhookWindowsHookEx(data.hook);
                    data.hook = std::ptr::null_mut();
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
        if code >= 0 {
            let h = load_hwnd();
            if !h.is_null() {
                let ms = &*(lparam as *const MSLLHOOKSTRUCT);
                let lp = pack_coords(ms.pt.x, ms.pt.y);
                match wparam as u32 {
                    WM_LBUTTONDOWN => { PostMessageW(h, WM_HL_LDOWN, 0, lp); }
                    WM_LBUTTONUP   => { PostMessageW(h, WM_HL_LUP,   0, lp); }
                    WM_RBUTTONDOWN => { PostMessageW(h, WM_HL_RDOWN, 0, lp); }
                    WM_RBUTTONUP   => { PostMessageW(h, WM_HL_RUP,   0, lp); }
                    WM_MOUSEMOVE   => { PostMessageW(h, WM_HL_MOVE,  0, lp); }
                    _ => {}
                }
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }
}
