//! PowerToys AlwaysOnTop — Rust implementation
//!
//! Pins windows to stay on top with a colored border indicator.
//! Replaces the C++ PowerToys.AlwaysOnTop.exe.

#![windows_subsystem = "windows"]

mod app;
mod border;
mod settings;

use std::sync::Mutex;
use powertoys_win32::string::to_wide;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static MAIN_THREAD_ID: Mutex<u32> = Mutex::new(0);

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Parse parent PID from command line (runner passes it)
    let parent_pid: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    // Singleton mutex
    let app_mutex = powertoys_win32::mutex::AppMutex::create("Local\\PowerToys_AlwaysOnTop_InstanceMutex");
    match &app_mutex {
        Some(m) if m.already_running() => {
            eprintln!("[AlwaysOnTop] Another instance already running, exiting");
            return;
        }
        None => {
            eprintln!("[AlwaysOnTop] Failed to create mutex");
            return;
        }
        _ => {}
    }

    // Store main thread ID for posting quit messages
    *MAIN_THREAD_ID.lock().unwrap() = unsafe { GetCurrentThreadId() };

    // Watch parent process — exit when runner exits
    if parent_pid != 0 {
        std::thread::spawn(move || {
            if let Some(proc) = powertoys_win32::process::ProcessHandle::open_for_wait(parent_pid) {
                proc.wait_infinite();
            }
            let tid = *MAIN_THREAD_ID.lock().unwrap();
            unsafe { PostThreadMessageW(tid, WM_QUIT, 0, 0) };
        });
    }

    // Load settings and create the app
    let settings = settings::Settings::load();
    let mut aot = match app::AlwaysOnTop::new(settings) {
        Some(a) => a,
        None => {
            eprintln!("[AlwaysOnTop] Failed to create app");
            return;
        }
    };

    // IMPORTANT: Set the global instance pointer AFTER the value is in its
    // final stack location. AlwaysOnTop::new() can't do this because the
    // value moves when returned.
    app::set_instance(&mut aot);

    // Reset the terminate event in case it was left signaled from a previous run
    app::reset_terminate_event(&aot);

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
