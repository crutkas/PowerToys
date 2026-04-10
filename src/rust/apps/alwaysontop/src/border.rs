//! Window border rendering using GDI on a layered window.
//!
//! Creates a transparent topmost window that draws a colored border frame
//! around the tracked (pinned) window. Uses GDI instead of D2D1 for
//! simplicity — visually equivalent for non-rounded borders.

use crate::settings::Settings;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static BORDER_CLASS_REGISTERED: std::sync::Once = std::sync::Once::new();
const BORDER_CLASS_NAME: &str = "AlwaysOnTop_Border_Rust";

pub struct WindowBorder {
    border_hwnd: HWND,
}

unsafe impl Send for WindowBorder {}

impl WindowBorder {
    /// Create an empty (no-op) border for tracking without visual.
    pub fn empty() -> Self {
        Self {
            border_hwnd: std::ptr::null_mut(),
        }
    }

    /// Create a visible border window around `tracked_hwnd`.
    pub fn create(tracked_hwnd: HWND, hinstance: HINSTANCE, settings: &Settings) -> Option<Self> {
        let rect = get_frame_rect(tracked_hwnd, settings.frame_thickness)?;

        // Register the border window class once
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
                class_name.as_ptr(),
                std::ptr::null(),
                WS_POPUP | WS_DISABLED,
                rect.left,
                rect.top,
                w,
                h,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null(),
            )
        };

        if border_hwnd.is_null() {
            return None;
        }

        // Make window click-through transparent, then draw the border
        unsafe {
            SetLayeredWindowAttributes(border_hwnd, 0, 0, LWA_COLORKEY);
        }

        let border = Self { border_hwnd };
        border.draw_border(w, h, settings);

        // Exclude from peek
        unsafe {
            let val: BOOL = 1;
            DwmSetWindowAttribute(
                border_hwnd,
                DWMWA_EXCLUDED_FROM_PEEK as u32,
                &val as *const _ as *const _,
                std::mem::size_of::<BOOL>() as u32,
            );
        }

        // Position behind the tracked window but show it
        unsafe {
            SetWindowPos(
                tracked_hwnd,
                border_hwnd,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            ShowWindow(border_hwnd, SW_SHOWNA);
        }

        Some(border)
    }

    fn draw_border(&self, width: i32, height: i32, settings: &Settings) {
        if self.border_hwnd.is_null() {
            return;
        }

        unsafe {
            let hdc = GetDC(self.border_hwnd);
            if hdc.is_null() {
                return;
            }

            // Create memory DC and bitmap for layered window update
            let mem_dc = CreateCompatibleDC(hdc);
            let bmp = CreateCompatibleBitmap(hdc, width, height);
            let old_bmp = SelectObject(mem_dc, bmp);

            // Fill with the color key (black = transparent)
            let black_brush = GetStockObject(BLACK_BRUSH);
            let rc = RECT { left: 0, top: 0, right: width, bottom: height };
            FillRect(mem_dc, &rc, black_brush);

            // Draw the colored border frame
            let (r, g, b) = settings.frame_color_rgb();
            let color = (r as u32) | ((g as u32) << 8) | ((b as u32) << 16);
            let pen = CreatePen(PS_SOLID as i32, settings.frame_thickness, color);
            let old_pen = SelectObject(mem_dc, pen);
            let null_brush = GetStockObject(NULL_BRUSH);
            let old_brush = SelectObject(mem_dc, null_brush);

            // Draw rectangle border
            let half = settings.frame_thickness / 2;
            Rectangle(mem_dc, half, half, width - half, height - half);

            // Clean up GDI objects
            SelectObject(mem_dc, old_pen);
            SelectObject(mem_dc, old_brush);
            DeleteObject(pen);

            // Update the layered window with our drawn content
            let blend = BLENDFUNCTION {
                BlendOp: 0, // AC_SRC_OVER
                BlendFlags: 0,
                SourceConstantAlpha: ((settings.frame_opacity as u32 * 255) / 100) as u8,
                AlphaFormat: 0,
            };
            let pt_src = POINT { x: 0, y: 0 };
            let pt_dst = POINT { x: 0, y: 0 };
            let size = SIZE { cx: width, cy: height };

            // Use UpdateLayeredWindow for proper alpha
            UpdateLayeredWindow(
                self.border_hwnd,
                hdc,
                &pt_dst,
                &size,
                mem_dc,
                &pt_src,
                0,
                &blend,
                ULW_ALPHA,
            );

            SelectObject(mem_dc, old_bmp);
            DeleteObject(bmp);
            DeleteDC(mem_dc);
            ReleaseDC(self.border_hwnd, hdc);
        }
    }

    pub fn update_position(&self, tracked_hwnd: HWND) {
        if self.border_hwnd.is_null() {
            return;
        }

        // Use a default thickness for positioning
        if let Some(rect) = get_frame_rect(tracked_hwnd, 15) {
            unsafe {
                SetWindowPos(
                    self.border_hwnd,
                    tracked_hwnd,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOACTIVATE | SWP_NOREDRAW,
                );
            }
        }
    }

    pub fn update_properties(&self, tracked_hwnd: HWND, settings: &Settings) {
        if self.border_hwnd.is_null() {
            return;
        }

        if let Some(rect) = get_frame_rect(tracked_hwnd, settings.frame_thickness) {
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;
            unsafe {
                SetWindowPos(
                    self.border_hwnd,
                    tracked_hwnd,
                    rect.left, rect.top, w, h,
                    SWP_NOACTIVATE,
                );
            }
            self.draw_border(w, h, settings);
        }
    }

    pub fn show(&self) {
        if !self.border_hwnd.is_null() {
            unsafe { ShowWindow(self.border_hwnd, SW_SHOWNA) };
        }
    }

    pub fn hide(&self) {
        if !self.border_hwnd.is_null() {
            unsafe { ShowWindow(self.border_hwnd, SW_HIDE) };
        }
    }

    pub fn destroy(self) {
        if !self.border_hwnd.is_null() {
            unsafe { DestroyWindow(self.border_hwnd) };
        }
    }
}

fn get_frame_rect(hwnd: HWND, border_thickness: i32) -> Option<RECT> {
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    let hr = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            &mut rect as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
    };
    if hr != 0 {
        return None;
    }

    rect.top -= border_thickness;
    rect.left -= border_thickness;
    rect.right += border_thickness;
    rect.bottom += border_thickness;

    Some(rect)
}

unsafe extern "system" fn border_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_ERASEBKGND => 1,
            WM_SETCURSOR => {
                let cursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
                if !cursor.is_null() {
                    SetCursor(cursor);
                }
                1
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
