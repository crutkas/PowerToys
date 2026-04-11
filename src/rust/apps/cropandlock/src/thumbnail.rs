#![allow(static_mut_refs)]
//! Thumbnail mode — uses DwmRegisterThumbnail to show a live crop of a window.

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::Graphics::Gdi::*;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static THUMB_CLASS_INIT: std::sync::Once = std::sync::Once::new();
const THUMB_CLASS: &str = "CropAndLock_Thumbnail_Rust";

struct ThumbnailState {
    thumbnail: isize, // HTHUMBNAIL
    source_crop: RECT,
}

static mut THUMB_STATE: Option<ThumbnailState> = None;

/// Create a topmost window showing a DWM thumbnail of the cropped region.
pub fn create_thumbnail_window(source_hwnd: HWND, source_screen_rect: RECT, crop: RECT) {
    let hinstance = unsafe {
        windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null())
    } as HINSTANCE;

    let class_name = to_wide(THUMB_CLASS);
    THUMB_CLASS_INIT.call_once(|| unsafe {
        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(thumb_wnd_proc);
        wc.hInstance = hinstance;
        wc.lpszClassName = class_name.as_ptr();
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        wc.hbrBackground = GetStockObject(BLACK_BRUSH) as _;
        RegisterClassExW(&wc);
    });

    let w = crop.right - crop.left;
    let h = crop.bottom - crop.top;

    // Position near where the crop was taken
    let x = source_screen_rect.left + crop.left;
    let y = source_screen_rect.top + crop.top;

    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            to_wide("CropAndLock").as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            x, y, w, h,
            std::ptr::null_mut(), std::ptr::null_mut(), hinstance, std::ptr::null(),
        )
    };

    if hwnd.is_null() { return; }

    // Register DWM thumbnail
    let mut thumbnail: isize = 0;
    let hr = unsafe { DwmRegisterThumbnail(hwnd, source_hwnd, &mut thumbnail) };
    if hr != 0 || thumbnail == 0 {
        unsafe { DestroyWindow(hwnd); }
        return;
    }

    // Configure thumbnail to show the cropped region
    update_thumbnail(hwnd, thumbnail, crop);

    unsafe {
        THUMB_STATE = Some(ThumbnailState {
            thumbnail,
            source_crop: crop,
        });
    }
}

fn update_thumbnail(hwnd: HWND, thumbnail: isize, source_crop: RECT) {
    let mut client: RECT = unsafe { std::mem::zeroed() };
    unsafe { GetClientRect(hwnd, &mut client); }

    let props = DWM_THUMBNAIL_PROPERTIES {
        dwFlags: DWM_TNP_VISIBLE | DWM_TNP_RECTDESTINATION | DWM_TNP_RECTSOURCE | DWM_TNP_OPACITY,
        rcDestination: client,
        rcSource: source_crop,
        opacity: 255,
        fVisible: 1,
        fSourceClientAreaOnly: 0,
    };

    unsafe { DwmUpdateThumbnailProperties(thumbnail, &props); }
}

unsafe extern "system" fn thumb_wnd_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_SIZE => {
                if let Some(state) = THUMB_STATE.as_ref() {
                    update_thumbnail(hwnd, state.thumbnail, state.source_crop);
                }
                0
            }
            WM_DESTROY => {
                if let Some(state) = THUMB_STATE.take() {
                    DwmUnregisterThumbnail(state.thumbnail);
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
