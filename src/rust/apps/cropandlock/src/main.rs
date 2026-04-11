//! PowerToys CropAndLock — crop a window region into a pinned mini-window.
//!
//! Three modes: Thumbnail (DWM), Screenshot (GDI), Reparent (SetParent).
//! The module DLL signals named events when the user presses hotkeys.

#![windows_subsystem = "windows"]

mod selection;
mod thumbnail;
mod screenshot;

use std::sync::Mutex;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;

static MAIN_THREAD_ID: Mutex<u32> = Mutex::new(0);

// Named events from CommonSharedConstants
const REPARENT_EVENT: &str = "Local\\PowerToysCropAndLockReparentEvent-6060860a-76a1-44e8-8d0e-63578885e9c36";
const THUMBNAIL_EVENT: &str = "Local\\PowerToysCropAndLockThumbnailEvent-1637be50-da72-46b2-9220-b32bb206b2434";
const SCREENSHOT_EVENT: &str = "Local\\PowerToysCropAndLockScreenshotEvent-ff077ab2-8360-4bd1-864a-6337389d35593";
const EXIT_EVENT: &str = "Local\\PowerToysCropAndLockExitEvent-d995d409-7b70-482b-bad6-e7c8666f375a";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let parent_pid: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    // Singleton
    let mutex_name = to_wide("Local\\PowerToys_CropAndLock_InstanceMutex");
    let mutex = unsafe { CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr()) };
    if mutex.is_null() || unsafe { GetLastError() } == 183 { return; }

    *MAIN_THREAD_ID.lock().unwrap() = unsafe { GetCurrentThreadId() };

    // Watch parent process
    if parent_pid != 0 {
        std::thread::spawn(move || {
            let h = unsafe { OpenProcess(0x00100000, 0, parent_pid) };
            if !h.is_null() {
                unsafe { WaitForSingleObject(h, u32::MAX); CloseHandle(h); }
            }
            let tid = *MAIN_THREAD_ID.lock().unwrap();
            unsafe { PostThreadMessageW(tid, WM_QUIT, 0, 0); }
        });
    }

    // Create events
    let events = [
        create_event(REPARENT_EVENT),
        create_event(THUMBNAIL_EVENT),
        create_event(SCREENSHOT_EVENT),
        create_event(EXIT_EVENT),
    ];

    // Event loop — wait for hotkey signals from the module DLL
    loop {
        let result = unsafe {
            MsgWaitForMultipleObjects(4, events.as_ptr(), 0, u32::MAX, QS_ALLINPUT)
        };

        match result {
            0 => {
                // Reparent mode — use thumbnail as fallback for now
                unsafe { ResetEvent(events[0]); }
                let fg = unsafe { GetForegroundWindow() };
                if !fg.is_null() {
                    selection::start_selection(fg, selection::CropMode::Thumbnail);
                }
            }
            1 => {
                // Thumbnail mode
                unsafe { ResetEvent(events[1]); }
                let fg = unsafe { GetForegroundWindow() };
                if !fg.is_null() {
                    selection::start_selection(fg, selection::CropMode::Thumbnail);
                }
            }
            2 => {
                // Screenshot mode
                unsafe { ResetEvent(events[2]); }
                let fg = unsafe { GetForegroundWindow() };
                if !fg.is_null() {
                    selection::start_selection(fg, selection::CropMode::Screenshot);
                }
            }
            3 => {
                // Exit
                break;
            }
            4 => {
                // Windows message — pump it
                unsafe {
                    let mut msg: MSG = std::mem::zeroed();
                    while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                        if msg.message == WM_QUIT { return; }
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
            _ => break,
        }
    }
}

fn create_event(name: &str) -> *mut std::ffi::c_void {
    let wide = to_wide(name);
    unsafe {
        let mut sa: SECURITY_ATTRIBUTES = std::mem::zeroed();
        sa.nLength = std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32;
        CreateEventW(&sa, 0, 0, wide.as_ptr())
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
