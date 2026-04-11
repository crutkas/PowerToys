//! Popup window showing accent character choices.
//! Uses a layered window with GDI text rendering.

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static POPUP_HWND: std::sync::Mutex<usize> = std::sync::Mutex::new(0);
static POPUP_CLASS_INIT: std::sync::Once = std::sync::Once::new();

const POPUP_CLASS: &str = "PowerAccent_Popup_Rust";
const CELL_W: i32 = 48;
const CELL_H: i32 = 52;
const FONT_SIZE: i32 = 24;
const BG_COLOR: u32 = 0x00302020;     // dark background (BGR)
const SEL_COLOR: u32 = 0x00CC8833;    // accent highlight (BGR)
const TEXT_COLOR: u32 = 0x00FFFFFF;    // white text (BGR)

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn show(chars: &[char], selected: usize) {
    let hinstance = unsafe {
        windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null())
    } as HINSTANCE;

    let class_name = to_wide(POPUP_CLASS);
    POPUP_CLASS_INIT.call_once(|| unsafe {
        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(popup_wnd_proc);
        wc.hInstance = hinstance;
        wc.lpszClassName = class_name.as_ptr();
        RegisterClassExW(&wc);
    });

    let count = chars.len() as i32;
    let width = CELL_W * count + 8;  // 4px padding each side
    let height = CELL_H + 8;

    // Position near the caret/cursor
    let mut cursor_pos: POINT = unsafe { std::mem::zeroed() };
    unsafe { GetCursorPos(&mut cursor_pos); }
    let x = cursor_pos.x - width / 2;
    let y = cursor_pos.y - height - 20;

    let mut hwnd = *POPUP_HWND.lock().unwrap() as HWND;

    if hwnd.is_null() {
        hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                std::ptr::null(),
                WS_POPUP,
                x, y, width, height,
                std::ptr::null_mut(), std::ptr::null_mut(), hinstance, std::ptr::null(),
            )
        };
        *POPUP_HWND.lock().unwrap() = hwnd as usize;
    } else {
        unsafe { SetWindowPos(hwnd, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE); }
    }

    if hwnd.is_null() { return; }

    // Render to a 32-bit ARGB bitmap
    render_popup(hwnd, chars, selected, width, height);

    unsafe { ShowWindow(hwnd, SW_SHOWNA); }
}

pub fn hide() {
    let hwnd = *POPUP_HWND.lock().unwrap() as HWND;
    if !hwnd.is_null() {
        unsafe { ShowWindow(hwnd, SW_HIDE); }
    }
}

fn render_popup(hwnd: HWND, chars: &[char], selected: usize, width: i32, height: i32) {
    unsafe {
        let screen_dc = GetDC(std::ptr::null_mut());
        if screen_dc.is_null() { return; }
        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.is_null() { ReleaseDC(std::ptr::null_mut(), screen_dc); return; }

        // Create 32-bit ARGB bitmap
        let mut bmi: BITMAPINFO = std::mem::zeroed();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let bmp = CreateDIBSection(mem_dc, &bmi, 0, &mut bits, std::ptr::null_mut(), 0);
        if bmp.is_null() { DeleteDC(mem_dc); ReleaseDC(std::ptr::null_mut(), screen_dc); return; }
        let old_bmp = SelectObject(mem_dc, bmp);

        let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (width * height) as usize);

        // Fill background with semi-transparent dark
        let bg_px = premultiply_bgr(BG_COLOR, 230);
        for p in pixels.iter_mut() { *p = bg_px; }

        // Highlight selected cell
        let sel_x_start = 4 + (selected as i32) * CELL_W;
        let sel_px = premultiply_bgr(SEL_COLOR, 255);
        for y in 4..height - 4 {
            for x in sel_x_start..sel_x_start + CELL_W {
                if x >= 0 && x < width {
                    pixels[(y * width + x) as usize] = sel_px;
                }
            }
        }

        // Create font for text
        let font = CreateFontW(
            FONT_SIZE, 0, 0, 0, 400, // FW_NORMAL
            0, 0, 0, 1, // DEFAULT_CHARSET
            0, 0, 4, // ANTIALIASED_QUALITY
            0, to_wide("Segoe UI").as_ptr(),
        );
        let old_font = SelectObject(mem_dc, font);
        SetBkMode(mem_dc, 1); // TRANSPARENT
        SetTextColor(mem_dc, TEXT_COLOR);

        // Draw each character
        for (i, ch) in chars.iter().enumerate() {
            let cx = 4 + (i as i32) * CELL_W;
            let mut s = [0u16; 3];
            let encoded: Vec<u16> = ch.encode_utf16(&mut s).iter().cloned().collect();
            let rc = RECT { left: cx, top: 4, right: cx + CELL_W, bottom: height - 4 };
            DrawTextW(mem_dc, encoded.as_ptr(), encoded.len() as i32, &rc as *const _ as *mut _,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }

        SelectObject(mem_dc, old_font);
        DeleteObject(font);

        // Update layered window
        let pt_src = POINT { x: 0, y: 0 };
        let size = SIZE { cx: width, cy: height };
        let blend = BLENDFUNCTION { BlendOp: 0, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: 1 };
        UpdateLayeredWindow(hwnd, screen_dc, std::ptr::null(), &size, mem_dc, &pt_src, 0, &blend, ULW_ALPHA);

        SelectObject(mem_dc, old_bmp);
        DeleteObject(bmp);
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);
    }
}

fn premultiply_bgr(bgr: u32, alpha: u8) -> u32 {
    let a = alpha as u32;
    let b = (bgr & 0xFF) * a / 255;
    let g = ((bgr >> 8) & 0xFF) * a / 255;
    let r = ((bgr >> 16) & 0xFF) * a / 255;
    (a << 24) | (r << 16) | (g << 8) | b
}

unsafe extern "system" fn popup_wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}
