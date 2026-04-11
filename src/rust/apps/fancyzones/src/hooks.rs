//! WinEvent hooks for drag detection + low-level keyboard hook for Win+Arrow.

use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::app::{WM_FZ_MOVESIZE_START, WM_FZ_MOVESIZE_END, WM_FZ_WINDOW_CREATED, WM_FZ_DESKTOP_CHANGE};

/// Custom message for keyboard snap (Win+Arrow intercepted).
pub const WM_FZ_SNAP_HOTKEY: u32 = WM_APP + 3;

/// Whether to override Windows snap (set from settings).
static OVERRIDE_SNAP: AtomicBool = AtomicBool::new(false);
/// Thread ID to post messages to.
static MAIN_THREAD_ID: AtomicU32 = AtomicU32::new(0);

/// RAII guard that unhooks a WinEvent hook on drop.
pub struct WinEventHookGuard {
    handle: *mut core::ffi::c_void,
}

unsafe impl Send for WinEventHookGuard {}

impl Drop for WinEventHookGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { UnhookWinEvent(self.handle); }
        }
    }
}

/// Install WinEvent hooks for `MOVESIZESTART` and `MOVESIZEEND`.
/// The callback posts custom messages to the calling thread's message loop.
pub fn install_move_size_hooks() -> Vec<WinEventHookGuard> {
    let mut guards = Vec::new();

    let h1 = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_MOVESIZESTART,
            EVENT_SYSTEM_MOVESIZESTART,
            ptr::null_mut(), // no DLL
            Some(win_event_proc),
            0, // all processes
            0, // all threads
            0x0002, // WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS
        )
    };
    if !h1.is_null() {
        guards.push(WinEventHookGuard { handle: h1 });
    }

    let h2 = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_MOVESIZEEND,
            EVENT_SYSTEM_MOVESIZEEND,
            ptr::null_mut(),
            Some(win_event_proc),
            0,
            0,
            0x0002, // WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS
        )
    };
    if !h2.is_null() {
        guards.push(WinEventHookGuard { handle: h2 });
    }

    guards
}

/// Install WinEvent hooks for `EVENT_OBJECT_SHOW` to detect new windows appearing.
/// When a top-level window is shown, we post `WM_FZ_WINDOW_CREATED` so the app can
/// look up zone history and auto-snap.
pub fn install_window_create_hooks() -> Vec<WinEventHookGuard> {
    let mut guards = Vec::new();

    let h = unsafe {
        SetWinEventHook(
            EVENT_OBJECT_SHOW,
            EVENT_OBJECT_SHOW,
            ptr::null_mut(),
            Some(window_show_event_proc),
            0,
            0,
            0x0002, // WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS
        )
    };
    if !h.is_null() {
        guards.push(WinEventHookGuard { handle: h });
    }

    guards
}

/// WinEvent callback for EVENT_OBJECT_SHOW — fires when a window becomes visible.
unsafe extern "system" fn window_show_event_proc(
    _hook: *mut core::ffi::c_void,
    _event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    // OBJID_WINDOW = 0 — only handle top-level window show events
    if id_object != 0 || hwnd.is_null() { return; }
    unsafe {
        PostThreadMessageW(GetCurrentThreadId(), WM_FZ_WINDOW_CREATED, hwnd as usize, 0);
    }
}

/// Install a WinEvent hook for `EVENT_OBJECT_NAMECHANGE` to detect virtual desktop switches.
pub fn install_desktop_hooks() -> Vec<WinEventHookGuard> {
    let mut guards = Vec::new();

    let h = unsafe {
        SetWinEventHook(
            EVENT_OBJECT_NAMECHANGE,
            EVENT_OBJECT_NAMECHANGE,
            ptr::null_mut(),
            Some(desktop_name_change_proc),
            0,
            0,
            0x0002, // WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS
        )
    };
    if !h.is_null() {
        guards.push(WinEventHookGuard { handle: h });
    }

    guards
}

/// WinEvent callback for EVENT_OBJECT_NAMECHANGE — fires when the desktop accessibility
/// name changes, which happens on virtual desktop switches.
unsafe extern "system" fn desktop_name_change_proc(
    _hook: *mut core::ffi::c_void,
    _event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if id_object != 0 || hwnd.is_null() { return; }
    // Only respond to name changes on the desktop window (virtual desktop switch signal)
    if unsafe { hwnd == GetDesktopWindow() } {
        unsafe {
            PostThreadMessageW(GetCurrentThreadId(), WM_FZ_DESKTOP_CHANGE, 0, 0);
        }
    }
}

/// WinEvent callback. Runs on the thread that called `SetWinEventHook` (out-of-context).
/// Posts a custom message so the main loop can handle it synchronously.
unsafe extern "system" fn win_event_proc(
    _hook: *mut core::ffi::c_void,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    let msg = match event {
        EVENT_SYSTEM_MOVESIZESTART => WM_FZ_MOVESIZE_START,
        EVENT_SYSTEM_MOVESIZEEND => WM_FZ_MOVESIZE_END,
        _ => return,
    };

    unsafe {
        PostThreadMessageW(GetCurrentThreadId(), msg, hwnd as usize, 0);
    }
}

// ---- Low-level keyboard hook for Win+Arrow (override Windows snap) ----

/// RAII guard for the keyboard hook.
pub struct KeyboardHookGuard {
    handle: *mut core::ffi::c_void,
}
unsafe impl Send for KeyboardHookGuard {}
impl Drop for KeyboardHookGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { UnhookWindowsHookEx(self.handle); }
        }
    }
}

/// Install a low-level keyboard hook to intercept Win+Arrow.
pub fn install_keyboard_hook(override_snap: bool) -> Option<KeyboardHookGuard> {
    OVERRIDE_SNAP.store(override_snap, Ordering::SeqCst);
    MAIN_THREAD_ID.store(unsafe { GetCurrentThreadId() }, Ordering::SeqCst);

    let handle = unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(low_level_keyboard_proc),
            GetModuleHandleW(ptr::null()),
            0,
        )
    };
    if handle.is_null() { None }
    else { Some(KeyboardHookGuard { handle }) }
}

/// Update the override_snap setting at runtime.
pub fn set_override_snap(val: bool) {
    OVERRIDE_SNAP.store(val, Ordering::SeqCst);
}

unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 && OVERRIDE_SNAP.load(Ordering::SeqCst) {
        let info = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        let vk = info.vkCode;

        // Check if Win key is held
        let win_held = unsafe {
            (GetAsyncKeyState(VK_LWIN as i32) as u16 & 0x8000) != 0 ||
            (GetAsyncKeyState(VK_RWIN as i32) as u16 & 0x8000) != 0
        };

        if win_held {
            let is_arrow = matches!(vk, 0x25 | 0x26 | 0x27 | 0x28); // VK_LEFT, UP, RIGHT, DOWN
            if is_arrow && (wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize) {
                // Post to our message loop and EAT the key
                let tid = MAIN_THREAD_ID.load(Ordering::SeqCst);
                unsafe {
                    PostThreadMessageW(tid, WM_FZ_SNAP_HOTKEY, 0, vk as isize);
                }
                return 1; // Swallow the key — prevents Windows snap
            }
        }
    }

    unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) }
}
