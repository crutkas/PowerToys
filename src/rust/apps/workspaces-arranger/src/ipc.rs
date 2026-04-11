//! Named-pipe IPC for communicating between launcher and arranger.
//!
//! Pipe names match the C++ IPCHelperStrings.h:
//!   - WindowArranger listens on: `\\.\pipe\powertoys_workspaces_window_arranger_`
//!   - WindowArranger sends to:  `\\.\pipe\powertoys_workspaces_launcher_arranger_`

use std::thread;

use powertoys_win32::string::to_wide;
use windows_sys::Win32::Foundation::{
    CloseHandle, INVALID_HANDLE_VALUE, GENERIC_READ, GENERIC_WRITE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL,
};

// Named pipe APIs — declare directly to avoid feature fragmentation issues
unsafe extern "system" {
    fn CreateNamedPipeW(
        name: *const u16,
        open_mode: u32,
        pipe_mode: u32,
        max_instances: u32,
        out_buffer_size: u32,
        in_buffer_size: u32,
        default_timeout: u32,
        security_attributes: *const std::ffi::c_void,
    ) -> windows_sys::Win32::Foundation::HANDLE;
    fn ConnectNamedPipe(
        pipe: windows_sys::Win32::Foundation::HANDLE,
        overlapped: *mut std::ffi::c_void,
    ) -> i32;
    fn ReadFile(
        file: windows_sys::Win32::Foundation::HANDLE,
        buffer: *mut std::ffi::c_void,
        number_of_bytes_to_read: u32,
        number_of_bytes_read: *mut u32,
        overlapped: *mut std::ffi::c_void,
    ) -> i32;
    fn WriteFile(
        file: windows_sys::Win32::Foundation::HANDLE,
        buffer: *const std::ffi::c_void,
        number_of_bytes_to_write: u32,
        number_of_bytes_written: *mut u32,
        overlapped: *mut std::ffi::c_void,
    ) -> i32;
}

const PIPE_ACCESS_DUPLEX: u32 = 0x00000003;
const PIPE_TYPE_MESSAGE: u32 = 0x00000004;
const PIPE_READMODE_MESSAGE: u32 = 0x00000002;
const PIPE_WAIT: u32 = 0x00000000;

pub const WINDOW_ARRANGER_PIPE: &str = r"\\.\pipe\powertoys_workspaces_window_arranger_";
pub const LAUNCHER_ARRANGER_PIPE: &str = r"\\.\pipe\powertoys_workspaces_launcher_arranger_";

const BUFFER_SIZE: u32 = 4096;

/// IPC helper for two-way named pipe communication.
pub struct IpcHelper {
    _listen_thread: Option<thread::JoinHandle<()>>,
    send_pipe_name: String,
}

impl IpcHelper {
    /// Create an IPC helper that listens on `listen_pipe_name` and sends to `send_pipe_name`.
    /// The `callback` is invoked on a background thread for each incoming message.
    pub fn new<F>(listen_pipe_name: &str, send_pipe_name: &str, callback: F) -> Self
    where
        F: Fn(String) + Send + 'static,
    {
        let listen_name = listen_pipe_name.to_string();
        let handle = thread::spawn(move || {
            listen_loop(&listen_name, callback);
        });

        IpcHelper {
            _listen_thread: Some(handle),
            send_pipe_name: send_pipe_name.to_string(),
        }
    }

    /// Send a message to the paired pipe.
    pub fn send(&self, message: &str) {
        send_message(&self.send_pipe_name, message);
    }
}

/// Listen loop: creates a named pipe server and accepts messages.
fn listen_loop<F>(pipe_name: &str, callback: F)
where
    F: Fn(String) + Send + 'static,
{
    let wide_name = to_wide(pipe_name);
    loop {
        let pipe = unsafe {
            CreateNamedPipeW(
                wide_name.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                1,    // max instances
                BUFFER_SIZE,
                BUFFER_SIZE,
                0,    // default timeout
                std::ptr::null(),
            )
        };
        if pipe == INVALID_HANDLE_VALUE {
            eprintln!("IPC: failed to create pipe {pipe_name}");
            return;
        }

        // Wait for a client to connect
        let connected = unsafe { ConnectNamedPipe(pipe, std::ptr::null_mut()) };
        if connected == 0 {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            if err != 535 {
                // ERROR_PIPE_CONNECTED = 535
                unsafe { CloseHandle(pipe); }
                continue;
            }
        }

        // Read messages
        let mut buf = vec![0u8; BUFFER_SIZE as usize];
        loop {
            let mut bytes_read: u32 = 0;
            let ok = unsafe {
                ReadFile(
                    pipe,
                    buf.as_mut_ptr() as *mut _,
                    buf.len() as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 || bytes_read == 0 {
                break;
            }
            if let Ok(msg) = String::from_utf8(buf[..bytes_read as usize].to_vec()) {
                callback(msg);
            }
        }

        unsafe { CloseHandle(pipe); }
        // Exit after first connection for simplicity (arranger is short-lived)
        break;
    }
}

/// Send a message to a named pipe server.
fn send_message(pipe_name: &str, message: &str) {
    let wide_name = to_wide(pipe_name);
    let pipe = unsafe {
        CreateFileW(
            wide_name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    if pipe == INVALID_HANDLE_VALUE {
        eprintln!("IPC: failed to connect to pipe {pipe_name}");
        return;
    }

    let bytes = message.as_bytes();
    let mut written: u32 = 0;
    unsafe {
        WriteFile(
            pipe,
            bytes.as_ptr() as *const _,
            bytes.len() as u32,
            &mut written,
            std::ptr::null_mut(),
        );
        CloseHandle(pipe);
    }
}
