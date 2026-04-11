use powertoys_win32::string::to_wide;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static REG: std::sync::Once = std::sync::Once::new();
const CLS: &str = "FZ_Overlay_Rust";

pub struct ZoneOverlay { hwnd: HWND }
unsafe impl Send for ZoneOverlay {}
impl ZoneOverlay {
    pub fn create(l: i32, t: i32, w: i32, h: i32) -> Option<Self> {
        REG.call_once(|| {
            let cn = to_wide(CLS);
            let wc = WNDCLASSEXW { cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(dp), lpszClassName: cn.as_ptr(),
                ..unsafe { std::mem::zeroed() } };
            unsafe { RegisterClassExW(&wc); }
        });
        let cn = to_wide(CLS);
        let hwnd = unsafe { CreateWindowExW(
            WS_EX_LAYERED|WS_EX_TRANSPARENT|WS_EX_TOOLWINDOW|WS_EX_TOPMOST|WS_EX_NOACTIVATE,
            cn.as_ptr(), std::ptr::null(), WS_POPUP, l, t, w, h,
            std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()) };
        if hwnd.is_null() { None } else { Some(Self { hwnd }) }
    }
    pub fn show(&self) { unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE); } }
    pub fn hide(&self) { unsafe { ShowWindow(self.hwnd, SW_HIDE); } }
}
impl Drop for ZoneOverlay { fn drop(&mut self) { unsafe { DestroyWindow(self.hwnd); } } }
unsafe extern "system" fn dp(h: HWND, m: u32, w: usize, l: isize) -> isize { unsafe { DefWindowProcW(h,m,w,l) } }
