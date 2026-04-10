//! PowerToys AlwaysOnTop — Rust implementation
//!
//! Pins windows to stay on top with a colored border indicator.
//! Replaces the C++ PowerToys.AlwaysOnTop.exe.

#![windows_subsystem = "windows"]

mod app;
mod border;
mod settings;

use std::sync::Mutex;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static MAIN_THREAD_ID: Mutex<u32> = Mutex::new(0);

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Parse parent PID from command line (runner passes it)
    let parent_pid: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    // Singleton mutex
    let mutex_name = to_wide("Local\\PowerToys_AlwaysOnTop_InstanceMutex");
    let mutex = unsafe { CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr()) };
    if mutex.is_null() || unsafe { GetLastError() } == 183 {
        // ERROR_ALREADY_EXISTS — another instance is running
        return;
    }

    // Store main thread ID for posting quit messages
    *MAIN_THREAD_ID.lock().unwrap() = unsafe { GetCurrentThreadId() };

    // Watch parent process — exit when runner exits
    if parent_pid != 0 {
        std::thread::spawn(move || {
            let handle = unsafe { OpenProcess(0x00100000, 0, parent_pid) }; // SYNCHRONIZE
            if !handle.is_null() {
                unsafe { WaitForSingleObject(handle, u32::MAX) };
                unsafe { CloseHandle(handle) };
            }
            let tid = *MAIN_THREAD_ID.lock().unwrap();
            unsafe { PostThreadMessageW(tid, WM_QUIT, 0, 0) };
        });
    }

    // Load settings and create the app
    let settings = settings::Settings::load();
    let mut aot = match app::AlwaysOnTop::new(settings) {
        Some(a) => a,
        None => return,
    };

    // Message loop
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if msg.message == app::WM_PRIV_SETTINGS_CHANGED {
                aot.reload_settings();
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        aot.cleanup();
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
