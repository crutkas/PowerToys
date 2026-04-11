// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

//! PowerToys LightSwitch service — automatic theme scheduling.
//!
//! This is a headless EXE that:
//! - Monitors system theme (light/dark mode) via registry
//! - Applies theme changes based on schedule (fixed hours, sunset/sunrise,
//!   follow Night Light)
//! - Watches for parent process exit (runner)
//! - Handles manual override via named event

mod theme;

use lightswitch_core::settings::{parse_settings_json, LightSwitchConfig};
use lightswitch_core::state::StateManager;
use std::ptr;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Threading::*;

// Access rights constants not re-exported by windows-sys.
const SYNCHRONIZE: u32 = 0x00100000;
const EVENT_MODIFY_STATE: u32 = 0x0002;

/// Named event for manual override from the module interface.
const MANUAL_OVERRIDE_EVENT: &str = "POWERTOYS_LIGHTSWITCH_MANUAL_OVERRIDE";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut parent_pid: u32 = 0;

    // Parse --pid argument.
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--pid" && i + 1 < args.len() {
            parent_pid = args[i + 1].parse().unwrap_or(0);
            i += 2;
        } else {
            i += 1;
        }
    }

    run_worker(parent_pid);
}

fn run_worker(parent_pid: u32) {
    let provider = theme::WinThemeProvider;
    let mut state_manager = StateManager::new();

    // Load initial settings.
    let mut config = load_settings_from_file();

    // Open parent process handle for monitoring.
    let h_parent = if parent_pid != 0 {
        unsafe { OpenProcess(SYNCHRONIZE, 0, parent_pid) }
    } else {
        ptr::null_mut()
    };

    // Create stop event.
    let h_stop = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };

    // Try to open the manual override named event.
    let event_name = wide(MANUAL_OVERRIDE_EVENT);
    let h_manual = unsafe {
        OpenEventW(
            SYNCHRONIZE | EVENT_MODIFY_STATE,
            0,
            event_name.as_ptr(),
        )
    };

    // Sync initial theme state.
    state_manager.sync_initial_theme_state(&config, &provider);

    // Main loop: wait for events or minute ticks.
    loop {
        let mut handles: Vec<HANDLE> = Vec::with_capacity(4);
        handles.push(h_stop);
        if !h_parent.is_null() {
            handles.push(h_parent);
        }
        if !h_manual.is_null() {
            handles.push(h_manual);
        }

        // Calculate ms to next minute boundary.
        let ms_to_next = ms_to_next_minute();

        let result = unsafe {
            WaitForMultipleObjects(
                handles.len() as u32,
                handles.as_ptr(),
                0,     // wait for ANY
                ms_to_next,
            )
        };

        if result == WAIT_TIMEOUT {
            // Minute tick.
            config = load_settings_from_file();
            state_manager.on_tick(&config, &provider);
            continue;
        }

        let index = result.wrapping_sub(WAIT_OBJECT_0) as usize;

        if index == 0 {
            // Stop event.
            break;
        }

        if !h_parent.is_null() && index == 1 {
            // Parent process exited.
            break;
        }

        let manual_index = if h_parent.is_null() { 1 } else { 2 };
        if !h_manual.is_null() && index == manual_index {
            state_manager.on_manual_override(&config, &provider);
            // Reset the event.
            unsafe {
                ResetEvent(h_manual);
            }
            continue;
        }
    }

    // Cleanup.
    if !h_manual.is_null() {
        unsafe { CloseHandle(h_manual); }
    }
    if !h_parent.is_null() {
        unsafe { CloseHandle(h_parent); }
    }
    unsafe { CloseHandle(h_stop); }
}

/// Load settings from the PowerToys settings file.
fn load_settings_from_file() -> LightSwitchConfig {
    // Build the settings file path: %LOCALAPPDATA%\Microsoft\PowerToys\LightSwitch\settings.json
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let settings_path = std::path::Path::new(&local_app_data)
        .join("Microsoft")
        .join("PowerToys")
        .join("LightSwitch")
        .join("settings.json");

    match std::fs::read_to_string(&settings_path) {
        Ok(content) => parse_settings_json(&content),
        Err(_) => LightSwitchConfig::default(),
    }
}

/// Calculate milliseconds until the next minute boundary.
fn ms_to_next_minute() -> u32 {
    unsafe {
        let mut st: windows_sys::Win32::Foundation::SYSTEMTIME = std::mem::zeroed();
        windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st);
        let ms = (60 - st.wSecond as u32) * 1000 - st.wMilliseconds as u32;
        if ms < 50 { 50 } else { ms }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
