#![windows_subsystem = "windows"]

//! WorkspacesSnapshotTool — captures current window layout as a workspace project.

use powertoys_win32::monitor::{self, MonitorInfo};
use powertoys_win32::settings;
use powertoys_win32::string::from_wide;
use powertoys_win32::window;
use workspaces_core::data::{Application, Position, Workspace};

use windows_sys::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, GetWindowPlacement, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    IsZoomed, GWL_STYLE, WINDOWPLACEMENT, WS_THICKFRAME,
};

unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(value: isize) -> i32;
}
const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

unsafe extern "system" {
    fn CoInitializeEx(reserved: *const std::ffi::c_void, coinit: u32) -> i32;
    fn CoUninitialize();
    fn CoCreateGuid(guid: *mut Guid) -> i32;
}
const COINIT_MULTITHREADED: u32 = 0;

#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetPackageFullName(
        hprocess: *mut std::ffi::c_void,
        package_full_name_length: *mut u32,
        package_full_name: *mut u16,
    ) -> u32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(
        process_handle: *mut std::ffi::c_void,
        desired_access: u32,
        token_handle: *mut *mut std::ffi::c_void,
    ) -> i32;
    fn GetTokenInformation(
        token_handle: *mut std::ffi::c_void,
        token_information_class: u32,
        token_information: *mut std::ffi::c_void,
        token_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHGetPropertyStoreForWindow(
        hwnd: HWND,
        riid: *const Guid,
        ppv: *mut *mut std::ffi::c_void,
    ) -> i32;
}

#[link(name = "ole32")]
unsafe extern "system" {
    fn PropVariantClear(pvar: *mut PropVariantRaw) -> i32;
}

const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
const ERROR_SUCCESS: u32 = 0;
const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_ELEVATION_CLASS: u32 = 20; // TokenElevation

const IID_IPROPERTY_STORE: Guid = Guid {
    data1: 0x886D8EEB,
    data2: 0x8CF2,
    data3: 0x4446,
    data4: [0x8D, 0x02, 0xCD, 0xBA, 0x1D, 0xBD, 0xCF, 0x99],
};

const PKEY_APP_USER_MODEL_ID: PropertyKey = PropertyKey {
    fmtid: Guid {
        data1: 0x9F4C2855,
        data2: 0x9F79,
        data3: 0x4B39,
        data4: [0xA8, 0xD0, 0xE1, 0xD4, 0x2D, 0xE1, 0xD5, 0xF3],
    },
    pid: 5,
};

const VT_LPWSTR: u16 = 31;

#[repr(C)]
struct PropertyKey {
    fmtid: Guid,
    pid: u32,
}

#[repr(C, align(8))]
struct PropVariantRaw {
    vt: u16,
    _reserved: [u16; 3],
    data: [u8; 16],
}

fn main() -> std::process::ExitCode {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hr = CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED);
        if hr < 0 {
            eprintln!("Workspaces Snapshot: COM init failed: 0x{hr:08x}");
            return std::process::ExitCode::FAILURE;
        }
    }

    let args: Vec<String> = std::env::args().collect();
    let invoke_point = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let is_launch_and_edit = invoke_point == "LaunchAndEdit";

    let workspace_id = generate_guid();
    let creation_time = current_timestamp();
    let apps = snapshot_apps(is_launch_and_edit);

    let workspace = Workspace {
        id: workspace_id,
        name: String::new(),
        creation_time,
        last_launched_time: String::new(),
        is_shortcut_needed: false,
        move_existing_windows: false,
        apps,
    };

    let base_dir = settings::module_dir("Workspaces")
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp_path = workspaces_core::data::temp_workspaces_file(&base_dir);

    if let Some(parent) = std::path::Path::new(&temp_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let workspaces = vec![workspace];
    if let Err(e) = workspaces_core::json_utils::write_workspaces_file(&temp_path, &workspaces) {
        eprintln!("Workspaces Snapshot: failed to write {temp_path}: {e:?}");
        unsafe { CoUninitialize() };
        return std::process::ExitCode::FAILURE;
    }

    eprintln!(
        "Workspaces Snapshot: wrote {} apps to {}",
        workspaces[0].apps.len(),
        temp_path
    );
    unsafe { CoUninitialize() };
    std::process::ExitCode::SUCCESS
}

/// Snapshot all visible windows and build Application structs.
fn snapshot_apps(is_guid_needed: bool) -> Vec<Application> {
    let monitors = monitor::enum_monitors();
    let windows = window::enum_visible_windows();
    let mut apps = Vec::new();

    for hwnd in windows {
        let title = window::get_window_text(hwnd);
        if title.is_empty() {
            continue;
        }

        let rect = match get_window_rect_safe(hwnd) {
            Some(r) => r,
            None => continue,
        };

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let is_minimized = unsafe { IsIconic(hwnd) != 0 };
        if (width <= 0 || height <= 0) && !is_minimized {
            continue;
        }

        let process_path = match get_process_path(hwnd) {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };

        if is_excluded(&process_path) {
            continue;
        }

        let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
        if (style & WS_THICKFRAME) == 0 {
            continue;
        }

        let is_maximized = unsafe { IsZoomed(hwnd) != 0 };

        // Use restored placement for minimized/maximized windows.
        let position = if is_minimized || is_maximized {
            let mut placement: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
            placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
            if unsafe { GetWindowPlacement(hwnd, &mut placement) } != 0 {
                let rc = placement.rcNormalPosition;
                Position {
                    x: rc.left,
                    y: rc.top,
                    width: rc.right - rc.left,
                    height: rc.bottom - rc.top,
                }
            } else {
                Position { x: rect.left, y: rect.top, width, height }
            }
        } else {
            Position { x: rect.left, y: rect.top, width, height }
        };

        let monitor_num = monitor_for_window(hwnd, &monitors);
        let name = file_name_from_path(&process_path);
        let id = if is_guid_needed { generate_guid() } else { String::new() };

        let pid = get_pid(hwnd);
        let is_elevated = is_process_elevated(pid);
        let package_full_name = get_package_full_name(pid).unwrap_or_default();
        let app_user_model_id = get_app_user_model_id_from_window(hwnd).unwrap_or_default();

        // PWA detection for Chrome (pattern: _crx_{app_id} in AUMID)
        let mut pwa_app_id = String::new();
        let lower_path = process_path.to_lowercase();
        if lower_path.contains("chrome.exe") {
            if let Some(pos) = app_user_model_id.find("_crx_") {
                pwa_app_id = app_user_model_id[pos + 5..].to_string();
            }
        }

        // Non-packaged (Win32) apps can be launched elevated; packaged (UWP) apps cannot.
        let can_launch_elevated = package_full_name.is_empty();

        apps.push(Application {
            id,
            name,
            title,
            path: process_path,
            package_full_name,
            app_user_model_id,
            pwa_app_id,
            command_line_args: String::new(),
            is_elevated,
            can_launch_elevated,
            is_minimized,
            is_maximized,
            position,
            monitor: monitor_num,
        });
    }

    eprintln!("Workspaces Snapshot: found {} apps", apps.len());
    apps
}

fn get_process_path(hwnd: HWND) -> Option<String> {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if pid == 0 {
        return None;
    }
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let mut buf = [0u16; 1024];
    let mut size = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };
    unsafe { CloseHandle(handle) };
    if ok != 0 && size > 0 {
        Some(from_wide(&buf[..size as usize]))
    } else {
        None
    }
}

fn get_window_rect_safe(hwnd: HWND) -> Option<RECT> {
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(hwnd, &mut rect) } != 0 {
        Some(rect)
    } else {
        None
    }
}

fn monitor_for_window(hwnd: HWND, monitors: &[MonitorInfo]) -> u32 {
    use windows_sys::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
    let hmon = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    monitors.iter().position(|m| m.handle == hmon).unwrap_or(0) as u32
}

fn is_excluded(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("applicationframehost.exe")
        || lower.contains("shellexperiencehost.exe")
        || lower.contains("startmenuexperiencehost.exe")
        || lower.contains("searchhost.exe")
        || lower.contains("lockapp.exe")
        || lower.contains("textinputhost.exe")
}

fn file_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn generate_guid() -> String {
    unsafe {
        let mut guid = Guid { data1: 0, data2: 0, data3: 0, data4: [0; 8] };
        CoCreateGuid(&mut guid);
        format!(
            "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
            guid.data1, guid.data2, guid.data3,
            guid.data4[0], guid.data4[1], guid.data4[2], guid.data4[3],
            guid.data4[4], guid.data4[5], guid.data4[6], guid.data4[7],
        )
    }
}

fn current_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    format!("{}", duration.as_secs())
}

// ── Process info helpers ──

fn get_pid(hwnd: HWND) -> u32 {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    pid
}

/// Check whether the process is running elevated (admin).
/// Mirrors C++ `IsProcessElevated` in SnapshotUtils.cpp.
fn is_process_elevated(pid: u32) -> bool {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut token: *mut std::ffi::c_void = std::ptr::null_mut();
        if OpenProcessToken(handle, TOKEN_QUERY, &mut token) == 0 {
            CloseHandle(handle);
            return false;
        }
        let mut elevation: u32 = 0; // TOKEN_ELEVATION.TokenIsElevated
        let mut size: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION_CLASS,
            &mut elevation as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
            &mut size,
        );
        CloseHandle(token);
        CloseHandle(handle);
        ok != 0 && elevation != 0
    }
}

/// Get the package full name for a packaged (UWP/Store) app process.
/// Returns `None` for non-packaged (Win32) processes.
/// Mirrors C++ `GetPackageFullName` usage in WindowUtils.cpp.
fn get_package_full_name(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut length: u32 = 0;
        let rc = GetPackageFullName(handle, &mut length, std::ptr::null_mut());
        if rc != ERROR_INSUFFICIENT_BUFFER {
            CloseHandle(handle);
            return None;
        }
        let mut buf = vec![0u16; length as usize];
        let rc = GetPackageFullName(handle, &mut length, buf.as_mut_ptr());
        CloseHandle(handle);
        if rc != ERROR_SUCCESS {
            return None;
        }
        // length includes null terminator
        let s = from_wide(&buf[..length.saturating_sub(1) as usize]);
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

/// Get the AppUserModelId from the window property store.
/// Mirrors C++ `GetAUMIDFromWindow` in WindowUtils.cpp via `SHGetPropertyStoreForWindow`.
fn get_app_user_model_id_from_window(hwnd: HWND) -> Option<String> {
    if hwnd.is_null() {
        return None;
    }
    unsafe {
        let mut store_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let hr = SHGetPropertyStoreForWindow(hwnd, &IID_IPROPERTY_STORE, &mut store_ptr);
        if hr < 0 || store_ptr.is_null() {
            return None;
        }

        // Raw COM vtable: IPropertyStore inherits IUnknown
        // [0] QueryInterface [1] AddRef [2] Release [3] GetCount [4] GetAt [5] GetValue
        let vtbl = *(store_ptr as *const *const usize);

        let get_value: unsafe extern "system" fn(
            *mut std::ffi::c_void,
            *const PropertyKey,
            *mut PropVariantRaw,
        ) -> i32 = std::mem::transmute(*vtbl.add(5));

        let mut pv: PropVariantRaw = std::mem::zeroed();
        let hr = get_value(store_ptr, &PKEY_APP_USER_MODEL_ID, &mut pv);

        let result = if hr >= 0 && pv.vt == VT_LPWSTR {
            let ptr = *(pv.data.as_ptr() as *const *const u16);
            if !ptr.is_null() {
                let mut len = 0;
                while *ptr.add(len) != 0 {
                    len += 1;
                }
                if len > 0 {
                    Some(from_wide(std::slice::from_raw_parts(ptr, len)))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        PropVariantClear(&mut pv);

        let release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32 =
            std::mem::transmute(*vtbl.add(2));
        release(store_ptr);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_process_elevated_returns_bool_without_crash() {
        let pid = std::process::id();
        // Must not panic; result depends on whether test runner is elevated.
        let _elevated = is_process_elevated(pid);
    }

    #[test]
    fn is_process_elevated_invalid_pid() {
        assert!(!is_process_elevated(0));
        assert!(!is_process_elevated(u32::MAX));
    }

    #[test]
    fn get_package_full_name_none_for_non_packaged() {
        // The test runner itself is a non-packaged Win32 process.
        let pid = std::process::id();
        assert_eq!(get_package_full_name(pid), None);
    }

    #[test]
    fn get_package_full_name_invalid_pid() {
        assert_eq!(get_package_full_name(0), None);
        assert_eq!(get_package_full_name(u32::MAX), None);
    }
}
