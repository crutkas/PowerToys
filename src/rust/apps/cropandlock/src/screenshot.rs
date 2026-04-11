#![allow(static_mut_refs)]
//! Screenshot mode — captures a static bitmap of the cropped region.

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static SHOT_CLASS_INIT: std::sync::Once = std::sync::Once::new();
const SHOT_CLASS: &str = "CropAndLock_Screenshot_Rust";

static mut SCREENSHOT_BMP: HBITMAP = std::ptr::null_mut();
static mut SCREENSHOT_W: i32 = 0;
static mut SCREENSHOT_H: i32 = 0;

/// Capture a screenshot of the cropped region and display in a topmost window.
pub fn create_screenshot_window(source_hwnd: HWND, source_screen_rect: RECT, crop: RECT) {
    let hinstance = unsafe {
        windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null())
    } as HINSTANCE;

    let class_name = to_wide(SHOT_CLASS);
    SHOT_CLASS_INIT.call_once(|| unsafe {
        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(shot_wnd_proc);
        wc.hInstance = hinstance;
        wc.lpszClassName = class_name.as_ptr();
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        RegisterClassExW(&wc);
    });

    let w = crop.right - crop.left;
    let h = crop.bottom - crop.top;

    // Capture the source window to a bitmap
    unsafe {
        let source_dc = GetDC(source_hwnd);
        if source_dc.is_null() { return; }

        let mem_dc = CreateCompatibleDC(source_dc);
        let bmp = CreateCompatibleBitmap(source_dc, w, h);
        let old = SelectObject(mem_dc, bmp);

        // BitBlt the crop region from the source window
        BitBlt(mem_dc, 0, 0, w, h, source_dc, crop.left, crop.top, SRCCOPY);

        SelectObject(mem_dc, old);
        DeleteDC(mem_dc);
        ReleaseDC(source_hwnd, source_dc);

        SCREENSHOT_BMP = bmp;
        SCREENSHOT_W = w;
        SCREENSHOT_H = h;
    }

    let x = source_screen_rect.left + crop.left;
    let y = source_screen_rect.top + crop.top;

    unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            to_wide("CropAndLock").as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            x, y, w, h,
            std::ptr::null_mut(), std::ptr::null_mut(), hinstance, std::ptr::null(),
        );
    }
}

unsafe extern "system" fn shot_wnd_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);

                if !SCREENSHOT_BMP.is_null() {
                    let mem_dc = CreateCompatibleDC(hdc);
                    let old = SelectObject(mem_dc, SCREENSHOT_BMP);

                    let mut client: RECT = std::mem::zeroed();
                    GetClientRect(hwnd, &mut client);

                    // Stretch to fill the window (letterbox could be added)
                    StretchBlt(
                        hdc, 0, 0, client.right, client.bottom,
                        mem_dc, 0, 0, SCREENSHOT_W, SCREENSHOT_H,
                        SRCCOPY,
                    );

                    SelectObject(mem_dc, old);
                    DeleteDC(mem_dc);
                }

                EndPaint(hwnd, &ps);
                0
            }
            WM_DESTROY => {
                if !SCREENSHOT_BMP.is_null() {
                    DeleteObject(SCREENSHOT_BMP);
                    SCREENSHOT_BMP = std::ptr::null_mut();
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
