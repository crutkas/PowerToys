//! WinEvent hooks for drag detection (EVENT_SYSTEM_MOVESIZESTART / END).

use std::ptr;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent};
use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, EVENT_SYSTEM_MOVESIZESTART, EVENT_SYSTEM_MOVESIZEEND};

use crate::app::{WM_FZ_MOVESIZE_START, WM_FZ_MOVESIZE_END};

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
            0, // WINEVENT_OUTOFCONTEXT (default)
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
            0,
        )
    };
    if !h2.is_null() {
        guards.push(WinEventHookGuard { handle: h2 });
    }

    guards
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
