#![windows_subsystem = "windows"]

mod app;
mod hooks;
mod settings;

use powertoys_win32::mutex::AppMutex;
use powertoys_win32::process::ProcessHandle;
use std::thread;
use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows_sys::Win32::UI::WindowsAndMessaging::WM_QUIT;

fn main() {
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2); }

    // Singleton mutex — exit if another instance is already running.
    let _mutex = match AppMutex::create("Local\\PowerToys_FancyZones_InstanceMutex") {
        Some(m) if !m.already_running() => m,
        _ => return,
    };

    // Monitor parent process: when it exits, post WM_QUIT so we shut down.
    if let Some(parent_pid) = parse_parent_pid() {
        let thread_id = unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
        thread::spawn(move || {
            if let Some(parent) = ProcessHandle::open_for_wait(parent_pid) {
                parent.wait_infinite();
            }
            unsafe { windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(thread_id, WM_QUIT, 0, 0); }
        });
    }

    let mut fz_app = app::FancyZonesApp::new();
    fz_app.run();
    fz_app.shutdown();
}

/// Parse `--parent-pid <PID>` from command-line arguments.
fn parse_parent_pid() -> Option<u32> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2).find_map(|pair| {
        if pair[0] == "--parent-pid" { pair[1].parse().ok() } else { None }
    })
}
