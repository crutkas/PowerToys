//! PowerToys ActionRunner — launches processes at non-elevated privilege level.
//!
//! When PowerToys runs elevated but needs to launch a user-level process,
//! it uses ActionRunner as a bridge. ActionRunner inherits the shell window's
//! process token (non-elevated) via PROC_THREAD_ATTRIBUTE_PARENT_PROCESS,
//! then creates the target process under that token.
//!
//! Usage: PowerToys.ActionRunner.exe -run-non-elevated -target <exe> [-pidFile <name>] [params...]

#![windows_subsystem = "windows"]

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        std::process::exit(1);
    }

    if args[1] != "-run-non-elevated" {
        std::process::exit(1);
    }

    // Parse remaining args
    let mut target: Option<String> = None;
    let mut pid_file: Option<String> = None;
    let mut params: Vec<String> = Vec::new();
    let mut i = 2;

    while i < args.len() {
        match args[i].as_str() {
            "-target" if i + 1 < args.len() => {
                target = Some(args[i + 1].clone());
                i += 2;
            }
            "-pidFile" if i + 1 < args.len() => {
                pid_file = Some(args[i + 1].clone());
                i += 2;
            }
            _ => {
                params.push(args[i].clone());
                i += 1;
            }
        }
    }

    let target = match target {
        Some(t) => t,
        None => std::process::exit(1),
    };

    let params_str = params.join(" ");

    // Open PID file mapping if requested
    let (h_map, pid_buffer) = if let Some(ref name) = pid_file {
        open_pid_mapping(name)
    } else {
        (std::ptr::null_mut(), std::ptr::null_mut())
    };

    // Zero the PID buffer before launch
    if !pid_buffer.is_null() {
        unsafe { *(pid_buffer as *mut u32) = 0; }
    }

    // Launch the target process at non-elevated level
    run_non_elevated(&target, &params_str, pid_buffer as *mut u32);

    // Flush and close the mapping
    if !pid_buffer.is_null() {
        unsafe {
            FlushViewOfFile(pid_buffer, std::mem::size_of::<u32>());
            let addr = MEMORY_MAPPED_VIEW_ADDRESS { Value: pid_buffer };
            UnmapViewOfFile(addr);
        }
    }
    if !h_map.is_null() {
        unsafe {
            windows_sys::Win32::Storage::FileSystem::FlushFileBuffers(h_map);
            CloseHandle(h_map);
        }
    }
}

/// Launch a process with the shell window as parent (inherits non-elevated token).
fn run_non_elevated(file: &str, params: &str, return_pid: *mut u32) {
    let file_wide = to_wide(file);
    let mut cmd = to_wide(&format!("\"{}\" {}", file, params));

    unsafe {
        // Get the shell window (Explorer) — always runs non-elevated
        let shell_hwnd = GetShellWindow();
        if shell_hwnd.is_null() {
            return;
        }

        let mut shell_pid: u32 = 0;
        GetWindowThreadProcessId(shell_hwnd, &mut shell_pid);

        let shell_process = OpenProcess(PROCESS_CREATE_PROCESS, 0, shell_pid);
        if shell_process.is_null() {
            return;
        }

        // Set up PROC_THREAD_ATTRIBUTE_PARENT_PROCESS to inherit shell's token
        let mut attr_size: usize = 0;
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut attr_size);

        let mut attr_buf = vec![0u8; attr_size];
        let attr_list = attr_buf.as_mut_ptr() as *mut LPPROC_THREAD_ATTRIBUTE_LIST;

        if InitializeProcThreadAttributeList(attr_list as _, 1, 0, &mut attr_size) == 0 {
            CloseHandle(shell_process);
            return;
        }

        let mut proc_handle = shell_process;
        if UpdateProcThreadAttribute(
            attr_list as _,
            0,
            PROC_THREAD_ATTRIBUTE_PARENT_PROCESS as usize,
            &mut proc_handle as *mut _ as *mut _,
            std::mem::size_of::<HANDLE>(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) == 0 {
            CloseHandle(shell_process);
            return;
        }

        let mut si: STARTUPINFOEXW = std::mem::zeroed();
        si.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        si.lpAttributeList = attr_list as _;
        let mut pi: PROCESS_INFORMATION = std::mem::zeroed();

        let result = CreateProcessW(
            file_wide.as_ptr(),
            cmd.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT,
            std::ptr::null(),
            std::ptr::null(),
            &si.StartupInfo,
            &mut pi,
        );

        if result != 0 {
            if !return_pid.is_null() && !pi.hProcess.is_null() {
                *return_pid = GetProcessId(pi.hProcess);
            }
            if !pi.hProcess.is_null() { CloseHandle(pi.hProcess); }
            if !pi.hThread.is_null() { CloseHandle(pi.hThread); }
        }

        CloseHandle(shell_process);
    }
}

/// Open a named file mapping for PID communication.
fn open_pid_mapping(name: &str) -> (HANDLE, *mut std::ffi::c_void) {
    let name_wide = to_wide(name);
    unsafe {
        let h_map = OpenFileMappingW(FILE_MAP_WRITE, 0, name_wide.as_ptr());
        if h_map.is_null() {
            return (std::ptr::null_mut(), std::ptr::null_mut());
        }
        let buf = MapViewOfFile(h_map, FILE_MAP_ALL_ACCESS, 0, 0, std::mem::size_of::<u32>());
        (h_map, buf.Value)
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
