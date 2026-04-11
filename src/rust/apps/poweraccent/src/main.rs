//! PowerToys PowerAccent — Rust implementation
//!
//! Shows accent character popup when user holds a letter key
//! and presses space or arrow keys.

#![windows_subsystem = "windows"]

mod accents;
mod hook;
mod popup;
mod input;

use std::sync::Mutex;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static MAIN_THREAD_ID: Mutex<u32> = Mutex::new(0);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let parent_pid: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    // Singleton
    let mutex_name = to_wide("Local\\PowerToys_PowerAccent_InstanceMutex");
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

    // Install keyboard hook
    hook::install_hook();

    // Message loop
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    hook::uninstall_hook();
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
