#![allow(static_mut_refs)]
//! Region selection overlay — captures mouse drag to define crop rect.
//! Uses a simple transparent GDI window (no WinRT Composition dependency).

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

use crate::thumbnail;
use crate::screenshot;

#[derive(Clone, Copy)]
pub enum CropMode {
    Thumbnail,
    Screenshot,
}

static mut SELECTION_STATE: Option<SelectionState> = None;

struct SelectionState {
    source_hwnd: HWND,
    mode: CropMode,
    start: POINT,
    current: POINT,
    dragging: bool,
    overlay_hwnd: HWND,
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static CLASS_INIT: std::sync::Once = std::sync::Once::new();
const CLASS_NAME: &str = "CropAndLock_Selection_Rust";

/// Start the selection overlay for the given source window.
pub fn start_selection(source_hwnd: HWND, mode: CropMode) {
    let hinstance = unsafe {
        windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null())
    } as HINSTANCE;

    let class_name = to_wide(CLASS_NAME);
    CLASS_INIT.call_once(|| unsafe {
        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(selection_wnd_proc);
        wc.hInstance = hinstance;
        wc.lpszClassName = class_name.as_ptr();
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_CROSS);
        RegisterClassExW(&wc);
    });

    // Get the source window's screen rect
    let mut source_rect: RECT = unsafe { std::mem::zeroed() };
    unsafe { GetWindowRect(source_hwnd, &mut source_rect); }

    // Create a transparent overlay covering the source window
    let overlay = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            source_rect.left, source_rect.top,
            source_rect.right - source_rect.left,
            source_rect.bottom - source_rect.top,
            std::ptr::null_mut(), std::ptr::null_mut(), hinstance, std::ptr::null(),
        )
    };

    if overlay.is_null() { return; }

    // Semi-transparent dark overlay
    unsafe {
        SetLayeredWindowAttributes(overlay, 0, 100, LWA_ALPHA);
        ShowWindow(overlay, SW_SHOW);
        SetForegroundWindow(overlay);
        SetCapture(overlay);
    }

    unsafe {
        SELECTION_STATE = Some(SelectionState {
            source_hwnd,
            mode,
            start: POINT { x: 0, y: 0 },
            current: POINT { x: 0, y: 0 },
            dragging: false,
            overlay_hwnd: overlay,
        });
    }
}

unsafe extern "system" fn selection_wnd_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_LBUTTONDOWN => {
                if let Some(state) = SELECTION_STATE.as_mut() {
                    let x = (lparam & 0xFFFF) as i16 as i32;
                    let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
                    state.start = POINT { x, y };
                    state.current = POINT { x, y };
                    state.dragging = true;
                }
                0
            }
            WM_MOUSEMOVE => {
                if let Some(state) = SELECTION_STATE.as_mut() {
                    if state.dragging {
                        state.current.x = (lparam & 0xFFFF) as i16 as i32;
                        state.current.y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
                        InvalidateRect(hwnd, std::ptr::null(), 1);
                    }
                }
                0
            }
            WM_LBUTTONUP => {
                if let Some(state) = SELECTION_STATE.take() {
                    ReleaseCapture();
                    DestroyWindow(hwnd);

                    if state.dragging {
                        // Calculate crop rect in source window coordinates
                        let mut source_rect: RECT = std::mem::zeroed();
                        GetWindowRect(state.source_hwnd, &mut source_rect);

                        let crop = RECT {
                            left: state.start.x.min(state.current.x),
                            top: state.start.y.min(state.current.y),
                            right: state.start.x.max(state.current.x),
                            bottom: state.start.y.max(state.current.y),
                        };

                        let w = crop.right - crop.left;
                        let h = crop.bottom - crop.top;

                        if w > 10 && h > 10 {
                            match state.mode {
                                CropMode::Thumbnail => {
                                    thumbnail::create_thumbnail_window(
                                        state.source_hwnd, source_rect, crop,
                                    );
                                }
                                CropMode::Screenshot => {
                                    screenshot::create_screenshot_window(
                                        state.source_hwnd, source_rect, crop,
                                    );
                                }
                            }
                        }
                    }
                }
                0
            }
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);

                // Draw selection rectangle
                if let Some(state) = SELECTION_STATE.as_ref() {
                    if state.dragging {
                        let pen = CreatePen(PS_SOLID as i32, 2, 0x000000FF); // Red
                        let old_pen = SelectObject(hdc, pen);
                        let null_brush = GetStockObject(NULL_BRUSH);
                        let old_brush = SelectObject(hdc, null_brush);

                        Rectangle(hdc,
                            state.start.x.min(state.current.x),
                            state.start.y.min(state.current.y),
                            state.start.x.max(state.current.x),
                            state.start.y.max(state.current.y),
                        );

                        SelectObject(hdc, old_pen);
                        SelectObject(hdc, old_brush);
                        DeleteObject(pen);
                    }
                }

                EndPaint(hwnd, &ps);
                0
            }
            WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
                SELECTION_STATE.take();
                ReleaseCapture();
                DestroyWindow(hwnd);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
