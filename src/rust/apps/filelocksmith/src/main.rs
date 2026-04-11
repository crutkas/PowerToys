//! FileLocksmith CLI — finds which processes hold open handles to a file.
//!
//! Usage:
//!     filelocksmith [--json] [--kill] <path> [<path> ...]

fn main() {
    #[cfg(windows)]
    {
        windows_main::run();
    }

    #[cfg(not(windows))]
    {
        eprintln!("filelocksmith only runs on Windows");
        std::process::exit(1);
    }
}

#[cfg(windows)]
mod windows_main {
    use std::collections::{HashMap, HashSet};

    use filelocksmith_core::handle_info::ProcessResult;
    use filelocksmith_core::path::PathMatcher;
    use filelocksmith_core::process;

    // NT status codes
    const STATUS_INFO_LENGTH_MISMATCH: u32 = 0xC000_0004;
    const STATUS_SUCCESS: u32 = 0;

    // Information classes
    const SYSTEM_EXTENDED_HANDLE_INFORMATION: u32 = 64;
    const OBJECT_TYPE_INFORMATION: u32 = 2;
    const OBJECT_NAME_INFORMATION: u32 = 1;

    // Initial / max buffer sizes
    const DEFAULT_BUF: usize = 64 * 1024;
    const MAX_BUF: usize = 1024 * 1024 * 1024;

    // SYSTEM_HANDLE_TABLE_ENTRY_INFO_EX layout
    #[repr(C)]
    struct SystemHandleEntryEx {
        object: usize,
        unique_process_id: usize,
        handle_value: usize,
        granted_access: u32,
        creator_back_trace_index: u16,
        object_type_index: u16,
        handle_attributes: u32,
        reserved: u32,
    }

    #[repr(C)]
    struct SystemHandleInformationEx {
        number_of_handles: usize,
        reserved: usize,
        // Followed by `number_of_handles` entries.
    }

    #[repr(C)]
    struct UnicodeString {
        length: u16,
        maximum_length: u16,
        buffer: *const u16,
    }

    #[repr(C)]
    struct ObjectTypeInformationData {
        type_name: UnicodeString,
        // Remaining fields omitted — we only need the name.
    }

    type NtQuerySystemInformationFn = unsafe extern "system" fn(
        system_information_class: u32,
        system_information: *mut u8,
        system_information_length: u32,
        return_length: *mut u32,
    ) -> u32;

    type NtQueryObjectFn = unsafe extern "system" fn(
        handle: *mut core::ffi::c_void,
        object_information_class: u32,
        object_information: *mut u8,
        object_information_length: u32,
        return_length: *mut u32,
    ) -> u32;

    pub fn run() {
        let args: Vec<String> = std::env::args().collect();
        let mut json_output = false;
        let mut paths: Vec<String> = Vec::new();

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--json" => json_output = true,
                "--help" | "-h" => {
                    println!("Usage: filelocksmith [--json] <path> [<path> ...]");
                    return;
                }
                other => paths.push(other.to_string()),
            }
            i += 1;
        }

        if paths.is_empty() {
            eprintln!("Error: no paths specified. Use --help for usage.");
            std::process::exit(1);
        }

        let results = find_locking_processes(&paths);

        let output = if json_output {
            process::format_json(&results)
        } else {
            process::format_text(&results)
        };
        print!("{output}");
    }

    fn find_locking_processes(paths: &[String]) -> Vec<ProcessResult> {
        // Convert user paths to kernel paths and build matcher.
        let mut file_targets = Vec::new();
        let mut dir_targets = Vec::new();

        for p in paths {
            if let Some(kernel) = user_path_to_kernel(p) {
                if is_directory(p) {
                    dir_targets.push(kernel);
                } else {
                    file_targets.push(kernel);
                }
            } else {
                eprintln!("Warning: could not resolve kernel path for {p}");
            }
        }

        let matcher = PathMatcher::new(file_targets, dir_targets);

        // Enumerate all handles and find matches.
        let handles = enumerate_handles();
        let mut pid_files: HashMap<u32, HashSet<String>> = HashMap::new();

        for (pid, kernel_path) in handles {
            if let Some(matched) = matcher.matches(&kernel_path) {
                pid_files.entry(pid).or_default().insert(matched);
            }
        }

        // Resolve process info.
        let mut results = Vec::new();
        for (pid, files) in pid_files {
            let name = process_name(pid).unwrap_or_else(|| "<unknown>".into());
            let user = process_user(pid).unwrap_or_else(|| "<unknown>".into());
            results.push(ProcessResult {
                name,
                pid,
                user,
                files: files.into_iter().collect(),
            });
        }
        results.sort_by_key(|r| r.pid);
        results
    }

    /// Convert a user-visible path to its NT kernel name by opening the file.
    fn user_path_to_kernel(path: &str) -> Option<String> {
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, OPEN_EXISTING,
        };

        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let handle = CreateFileW(
                wide.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            );
            if handle == INVALID_HANDLE_VALUE {
                return None;
            }
            let name = kernel_name_from_handle(handle as _);
            CloseHandle(handle);
            name
        }
    }

    fn kernel_name_from_handle(handle: *mut core::ffi::c_void) -> Option<String> {
        let nt_query_object = load_nt_query_object()?;
        let mut buf = vec![0u8; 1024];
        let mut ret_len: u32 = 0;
        unsafe {
            let status = nt_query_object(
                handle,
                OBJECT_NAME_INFORMATION,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut ret_len,
            );
            if status != STATUS_SUCCESS {
                return None;
            }
            let us = &*(buf.as_ptr() as *const UnicodeString);
            if us.buffer.is_null() || us.length == 0 {
                return None;
            }
            let slice = std::slice::from_raw_parts(us.buffer, (us.length / 2) as usize);
            Some(String::from_utf16_lossy(slice))
        }
    }

    fn enumerate_handles() -> Vec<(u32, String)> {
        let nt_query_sys = match load_nt_query_system_information() {
            Some(f) => f,
            None => return Vec::new(),
        };
        let nt_query_obj = match load_nt_query_object() {
            Some(f) => f,
            None => return Vec::new(),
        };

        let mut buf = vec![0u8; DEFAULT_BUF];
        let mut ret_len: u32 = 0;

        // Grow buffer until it fits.
        loop {
            let status = unsafe {
                nt_query_sys(
                    SYSTEM_EXTENDED_HANDLE_INFORMATION,
                    buf.as_mut_ptr(),
                    buf.len() as u32,
                    &mut ret_len,
                )
            };
            if status == STATUS_INFO_LENGTH_MISMATCH && buf.len() < MAX_BUF {
                buf.resize(buf.len() * 2, 0);
                continue;
            }
            if status != STATUS_SUCCESS {
                return Vec::new();
            }
            break;
        }

        let info = unsafe { &*(buf.as_ptr() as *const SystemHandleInformationEx) };
        let entries_ptr = unsafe {
            (buf.as_ptr() as *const SystemHandleInformationEx).add(1) as *const SystemHandleEntryEx
        };

        let mut results = Vec::new();
        let my_pid = unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() };

        for i in 0..info.number_of_handles {
            let entry = unsafe { &*entries_ptr.add(i) };
            let pid = entry.unique_process_id as u32;

            // Skip our own process to avoid deadlocks.
            if pid == my_pid || pid == 0 {
                continue;
            }

            // Try to duplicate the handle into our process.
            let dup = duplicate_handle(pid, entry.handle_value);
            let dup = match dup {
                Some(h) => h,
                None => continue,
            };

            // Check if it's a File handle.
            if is_file_handle(dup, &nt_query_obj) {
                if let Some(name) = kernel_name_from_handle(dup) {
                    if !name.is_empty() {
                        results.push((pid, name));
                    }
                }
            }

            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(dup as _);
            }
        }

        results
    }

    fn duplicate_handle(pid: u32, handle_value: usize) -> Option<*mut core::ffi::c_void> {
        use windows_sys::Win32::Foundation::{CloseHandle, DuplicateHandle, DUPLICATE_SAME_ACCESS};
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_DUP_HANDLE};

        unsafe {
            let proc = OpenProcess(PROCESS_DUP_HANDLE, 0, pid);
            if proc.is_null() {
                return None;
            }
            let mut local_handle: *mut core::ffi::c_void = std::ptr::null_mut();
            let ok = DuplicateHandle(
                proc,
                handle_value as _,
                windows_sys::Win32::System::Threading::GetCurrentProcess(),
                &mut local_handle as *mut _ as _,
                0,
                0,
                DUPLICATE_SAME_ACCESS,
            );
            CloseHandle(proc);
            if ok == 0 {
                None
            } else {
                Some(local_handle)
            }
        }
    }

    fn is_file_handle(handle: *mut core::ffi::c_void, nt_query_obj: &NtQueryObjectFn) -> bool {
        let mut buf = vec![0u8; 1024];
        let mut ret_len: u32 = 0;
        unsafe {
            let status = nt_query_obj(
                handle,
                OBJECT_TYPE_INFORMATION,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut ret_len,
            );
            if status != STATUS_SUCCESS {
                return false;
            }
            let info = &*(buf.as_ptr() as *const ObjectTypeInformationData);
            if info.type_name.buffer.is_null() || info.type_name.length == 0 {
                return false;
            }
            let slice =
                std::slice::from_raw_parts(info.type_name.buffer, (info.type_name.length / 2) as usize);
            let name = String::from_utf16_lossy(slice);
            name == "File"
        }
    }

    fn process_name(pid: u32) -> Option<String> {
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        };
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap.is_null() {
                return None;
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            if Process32FirstW(snap, &mut entry) != 0 {
                loop {
                    if entry.th32ProcessID == pid {
                        let len = entry
                            .szExeFile
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(entry.szExeFile.len());
                        let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
                        windows_sys::Win32::Foundation::CloseHandle(snap);
                        return Some(name);
                    }
                    if Process32NextW(snap, &mut entry) == 0 {
                        break;
                    }
                }
            }
            windows_sys::Win32::Foundation::CloseHandle(snap);
            None
        }
    }

    fn process_user(pid: u32) -> Option<String> {
        use windows_sys::Win32::Security::{
            GetTokenInformation, LookupAccountSidW, TokenUser, TOKEN_QUERY, TOKEN_USER,
        };
        use windows_sys::Win32::System::Threading::OpenProcess;

        unsafe {
            let proc = OpenProcess(
                windows_sys::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            );
            if proc.is_null() {
                return None;
            }

            let mut token: *mut core::ffi::c_void = std::ptr::null_mut();
            let ok = windows_sys::Win32::System::Threading::OpenProcessToken(
                proc,
                TOKEN_QUERY,
                &mut token as *mut _ as _,
            );
            windows_sys::Win32::Foundation::CloseHandle(proc);
            if ok == 0 {
                return None;
            }

            let mut buf = vec![0u8; 256];
            let mut ret_len: u32 = 0;
            let ok = GetTokenInformation(
                token as _,
                TokenUser,
                buf.as_mut_ptr() as _,
                buf.len() as u32,
                &mut ret_len,
            );
            if ok == 0 {
                windows_sys::Win32::Foundation::CloseHandle(token as _);
                return None;
            }

            let token_user = &*(buf.as_ptr() as *const TOKEN_USER);
            let sid = token_user.User.Sid;

            let mut name_buf = [0u16; 256];
            let mut domain_buf = [0u16; 256];
            let mut name_len: u32 = 256;
            let mut domain_len: u32 = 256;
            let mut sid_use: i32 = 0;
            let ok = LookupAccountSidW(
                std::ptr::null(),
                sid,
                name_buf.as_mut_ptr(),
                &mut name_len,
                domain_buf.as_mut_ptr(),
                &mut domain_len,
                &mut sid_use,
            );
            windows_sys::Win32::Foundation::CloseHandle(token as _);
            if ok == 0 {
                return None;
            }

            let domain = String::from_utf16_lossy(&domain_buf[..domain_len as usize]);
            let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            Some(format!("{domain}\\{name}"))
        }
    }

    fn is_directory(path: &str) -> bool {
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileAttributesW, FILE_ATTRIBUTE_DIRECTORY, INVALID_FILE_ATTRIBUTES,
        };
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let attrs = GetFileAttributesW(wide.as_ptr());
            attrs != INVALID_FILE_ATTRIBUTES && (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0
        }
    }

    // -----------------------------------------------------------------------
    // ntdll function loading
    // -----------------------------------------------------------------------

    fn load_nt_query_system_information() -> Option<NtQuerySystemInformationFn> {
        use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
        unsafe {
            let name: Vec<u16> = "ntdll.dll\0".encode_utf16().collect();
            let module = LoadLibraryW(name.as_ptr());
            if module.is_null() {
                return None;
            }
            let proc =
                GetProcAddress(module, b"NtQuerySystemInformation\0".as_ptr() as _);
            proc.map(|p| std::mem::transmute(p))
        }
    }

    fn load_nt_query_object() -> Option<NtQueryObjectFn> {
        use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
        unsafe {
            let name: Vec<u16> = "ntdll.dll\0".encode_utf16().collect();
            let module = LoadLibraryW(name.as_ptr());
            if module.is_null() {
                return None;
            }
            let proc = GetProcAddress(module, b"NtQueryObject\0".as_ptr() as _);
            proc.map(|p| std::mem::transmute(p))
        }
    }
}
