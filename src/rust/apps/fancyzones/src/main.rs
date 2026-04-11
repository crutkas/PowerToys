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

    // COM initialization needed for WinEvent hooks  
    unsafe {
        windows_sys::Win32::System::Com::CoInitializeEx(
            core::ptr::null(),
            windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED as u32,
        );
    }

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

    // File watcher: monitor FancyZones layout data dir for changes (editor saves)
    {
        let thread_id = unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
        std::thread::spawn(move || {
            watch_layout_files(thread_id);
        });
    }

    fz_app.run();
    fz_app.shutdown();
}

/// Watch the FancyZones data directory for file changes.
/// Posts WM_FZ_LAYOUTS_CHANGED to the main thread when layout files are modified.
fn watch_layout_files(main_thread_id: u32) {
    let dir = match powertoys_win32::settings::module_dir("FancyZones") {
        Some(d) => d,
        None => return,
    };
    let dir_wide = powertoys_win32::string::to_wide(&dir.to_string_lossy());
    unsafe {
        let handle = windows_sys::Win32::Storage::FileSystem::FindFirstChangeNotificationW(
            dir_wide.as_ptr(),
            0, // don't watch subtree
            windows_sys::Win32::Storage::FileSystem::FILE_NOTIFY_CHANGE_LAST_WRITE,
        );
        if handle.is_null() || handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return;
        }
        loop {
            let wait = windows_sys::Win32::System::Threading::WaitForSingleObject(handle, u32::MAX);
            if wait != 0 { break; } // WAIT_OBJECT_0 = 0
            windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                main_thread_id,
                app::WM_FZ_LAYOUTS_CHANGED,
                0,
                0,
            );
            // Debounce rapid writes (editor may write multiple files)
            std::thread::sleep(std::time::Duration::from_millis(300));
            if windows_sys::Win32::Storage::FileSystem::FindNextChangeNotification(handle) == 0 {
                break;
            }
        }
        windows_sys::Win32::Foundation::CloseHandle(handle);
    }
}

/// Parse `--parent-pid <PID>` from command-line arguments.
fn parse_parent_pid() -> Option<u32> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2).find_map(|pair| {
        if pair[0] == "--parent-pid" { pair[1].parse().ok() } else { None }
    })
}
