//! Windows keyboard hook and SendInput integration.

use std::sync::Mutex;

use poweraccent_core::settings::Settings;
use poweraccent_core::state_machine::{AccentStateMachine, Action, TriggerKey};
use windows_sys::Win32::Foundation::{CloseHandle, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VK_LEFT, VK_RIGHT, VK_SPACE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
};

/// `SYNCHRONIZE` access right (0x0010_0000).
const SYNCHRONIZE: u32 = 0x0010_0000;

static STATE: Mutex<Option<AccentStateMachine>> = Mutex::new(None);
static mut HOOK_HANDLE: HHOOK = std::ptr::null_mut();

/// Entry point — installs the hook, monitors the parent process, and runs
/// the message loop until the parent exits.
pub fn run() {
    // Initialise the state machine with default settings.
    {
        let mut guard = STATE.lock().unwrap();
        *guard = Some(AccentStateMachine::new(Settings::default()));
    }

    // Install the low-level keyboard hook.
    unsafe {
        HOOK_HANDLE = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(low_level_keyboard_proc),
            GetModuleHandleW(std::ptr::null()),
            0,
        );
        if HOOK_HANDLE.is_null() {
            eprintln!("Failed to install keyboard hook");
            return;
        }
    }

    // Spawn a thread that watches the parent process and exits when it dies.
    let parent_pid = parent_pid();
    if let Some(ppid) = parent_pid {
        std::thread::spawn(move || {
            wait_for_parent(ppid);
            std::process::exit(0);
        });
    }

    // Run the Windows message loop (required for the hook to work).
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            DispatchMessageW(&msg);
        }
        UnhookWindowsHookEx(HOOK_HANDLE);
    }
}

/// The low-level keyboard hook callback.
unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let kb = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        let vk = kb.vkCode;
        let time = kb.time as u64;

        let shift_held = {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                GetAsyncKeyState, VK_LSHIFT, VK_RSHIFT,
            };
            unsafe {
                GetAsyncKeyState(VK_LSHIFT as i32) < 0
                    || GetAsyncKeyState(VK_RSHIFT as i32) < 0
            }
        };

        if let Ok(mut guard) = STATE.lock() {
            if let Some(sm) = guard.as_mut() {
                let result = match wparam as u32 {
                    WM_KEYDOWN => sm.on_key_down(vk, time, shift_held),
                    WM_KEYUP => sm.on_key_up(vk, time),
                    _ => {
                        return unsafe {
                            CallNextHookEx(HOOK_HANDLE, code, wparam, lparam)
                        };
                    }
                };

                // Process actions.
                for action in &result.actions {
                    match action {
                        Action::EmitAccent { ch } => send_unicode_char(*ch),
                        Action::EmitTrigger { trigger } => send_trigger_key(*trigger),
                        _ => {}
                    }
                }

                if result.suppress {
                    return 1; // swallow the key
                }
            }
        }
    }
    unsafe { CallNextHookEx(HOOK_HANDLE, code, wparam, lparam) }
}

// ---------------------------------------------------------------------------
// SendInput helpers
// ---------------------------------------------------------------------------

fn send_unicode_char(ch: char) {
    let mut utf16_buf = [0u16; 2];
    let encoded = ch.encode_utf16(&mut utf16_buf);
    for unit in &*encoded {
        let inputs: [INPUT; 2] = [
            make_unicode_input(*unit, 0),
            make_unicode_input(*unit, KEYEVENTF_KEYUP),
        ];
        unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            );
        }
    }
}

fn make_unicode_input(scan: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: scan,
                dwFlags: KEYEVENTF_UNICODE | flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_trigger_key(trigger: TriggerKey) {
    let vk = match trigger {
        TriggerKey::Space => VK_SPACE,
        TriggerKey::Left => VK_LEFT,
        TriggerKey::Right => VK_RIGHT,
    };
    let inputs: [INPUT; 2] = [make_vk_input(vk, 0), make_vk_input(vk, KEYEVENTF_KEYUP)];
    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
    }
}

fn make_vk_input(vk: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

// ---------------------------------------------------------------------------
// Parent PID monitoring
// ---------------------------------------------------------------------------

fn parent_pid() -> Option<u32> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap.is_null() {
            return None;
        }
        let mut entry: PROCESSENTRY32 = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
        let my_pid = GetCurrentProcessId();
        if Process32First(snap, &mut entry) != 0 {
            loop {
                if entry.th32ProcessID == my_pid {
                    let ppid = entry.th32ParentProcessID;
                    CloseHandle(snap);
                    return Some(ppid);
                }
                if Process32Next(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
        None
    }
}

fn wait_for_parent(pid: u32) {
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};
    unsafe {
        let handle = OpenProcess(SYNCHRONIZE, 0, pid);
        if !handle.is_null() {
            WaitForSingleObject(handle, u32::MAX);
            CloseHandle(handle);
        }
    }
}
