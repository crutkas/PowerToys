//! Awake Module Interface — Rust implementation
//!
//! This is the PowerToys module DLL for Awake. It implements the
//! `PowerToyModule` trait from `powertoys-module-ffi` and manages
//! launching/stopping the Awake executable.
//!
//! Architecture matches the existing C++ `AwakeModuleInterface`:
//! - `enable()` spawns `awake.exe --use-pt-config --pid <runner_pid>`
//! - `disable()` signals the named exit event `POWERTOYS_AWAKE_EXIT_EVENT`
//! - Settings are JSON serialized/deserialized

use powertoys_module_ffi::*;
use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

mod settings;
mod wide;

use settings::AwakeSettings;

pub struct AwakeModule {
    enabled: AtomicBool,
    settings: AwakeSettings,
    child_process: Option<u32>, // PID of awake.exe
}

impl AwakeModule {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            settings: AwakeSettings::default(),
            child_process: None,
        }
    }

    /// Launch the awake executable.
    /// Translates PowerToys settings into crutkas/awake CLI flags:
    ///   --wait-pid <PID>    Exit when runner dies
    ///   --display           Keep display on (if setting enabled)
    ///   --idle              Keep system awake (default)
    ///   --timeout <secs>    Timed mode (if hours/minutes > 0)
    fn launch_awake(&mut self) {
        use windows_sys::Win32::System::Threading::GetCurrentProcessId;

        let pid = unsafe { GetCurrentProcessId() };
        let exe_path = self.find_awake_exe();

        // Build CLI args from settings
        let mut args = format!("\"{}\" --wait-pid {}", exe_path, pid);

        // Display flag
        if self.settings.properties.awake_keep_display_on.value {
            args.push_str(" --display");
        }

        // Always keep idle sleep prevented
        args.push_str(" --idle");

        // Timed mode: convert hours+minutes to seconds for --timeout
        let mode = self.settings.properties.awake_mode.value;
        if mode == 2 {
            // Timed mode
            let hours = self.settings.properties.awake_hours.value;
            let minutes = self.settings.properties.awake_minutes.value;
            let total_secs = hours * 3600 + minutes * 60;
            if total_secs > 0 {
                args.push_str(&format!(" --timeout {}", total_secs));
            }
        }

        match self.create_process(&args) {
            Ok(child_pid) => {
                self.child_process = Some(child_pid);
                eprintln!("[Awake] Launched: {}", args);
            }
            Err(e) => {
                eprintln!("[Awake] Failed to launch: {}", e);
            }
        }
    }

    fn find_awake_exe(&self) -> String {
        std::env::var("POWERTOYS_AWAKE_EXE")
            .unwrap_or_else(|_| "PowerToys.Awake.exe".to_string())
    }

    fn create_process(&self, cmd: &str) -> Result<u32, String> {
        use windows_sys::Win32::System::Threading::{
            CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW,
            CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW,
        };
        use windows_sys::Win32::Foundation::CloseHandle;

        let cmd_wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();

        let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let result = unsafe {
            CreateProcessW(
                std::ptr::null(),
                cmd_wide.as_ptr() as *mut u16,
                std::ptr::null(),
                std::ptr::null(),
                0, // bInheritHandles = FALSE
                CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW,
                std::ptr::null(),
                std::ptr::null(),
                &si,
                &mut pi,
            )
        };

        if result == 0 {
            return Err(format!("CreateProcessW failed (error: {})", unsafe {
                windows_sys::Win32::Foundation::GetLastError()
            }));
        }

        let pid = pi.dwProcessId;
        unsafe {
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
        }

        Ok(pid)
    }

    /// Stop the running awake.exe process.
    /// The Rust awake binary doesn't use named events — it exits when
    /// the watched PID dies or on process termination.
    fn stop_awake(&mut self) {
        if let Some(pid) = self.child_process.take() {
            use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, WaitForSingleObject};
            use windows_sys::Win32::Foundation::CloseHandle;

            unsafe {
                let handle = OpenProcess(0x0001 | 0x00100000, 0, pid); // PROCESS_TERMINATE | SYNCHRONIZE
                if !handle.is_null() {
                    TerminateProcess(handle, 0);
                    WaitForSingleObject(handle, 1000);
                    CloseHandle(handle);
                }
            }
            eprintln!("[Awake] Stopped awake.exe (PID: {})", pid);
        }
    }
}

impl PowerToyModule for AwakeModule {
    fn get_name(&self) -> *const u16 {
        wide::awake_wide_ptr()
    }

    fn get_key(&self) -> *const u16 {
        wide::awake_wide_ptr()
    }

    fn enable(&mut self) {
        if !self.enabled.load(Ordering::SeqCst) {
            self.enabled.store(true, Ordering::SeqCst);
            self.launch_awake();
        }
    }

    fn disable(&mut self) {
        if self.enabled.load(Ordering::SeqCst) {
            self.stop_awake();
            self.enabled.store(false, Ordering::SeqCst);
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    fn get_config(&self, buffer: *mut u16, buffer_size: *mut c_int) -> bool {
        let json = self.settings.to_json();
        let wide: Vec<u16> = json.encode_utf16().chain(std::iter::once(0)).collect();
        let needed = wide.len() as c_int;

        if buffer.is_null() || buffer_size.is_null() {
            if !buffer_size.is_null() {
                unsafe { *buffer_size = needed };
            }
            return false;
        }

        let available = unsafe { *buffer_size };
        if available < needed {
            unsafe { *buffer_size = needed };
            return false;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), buffer, wide.len());
            *buffer_size = needed;
        }
        true
    }

    fn set_config(&mut self, config: *const u16) {
        if config.is_null() {
            return;
        }
        let s = wide::wide_to_string(config);
        if let Some(new_settings) = AwakeSettings::from_json(&s) {
            let was_enabled = self.enabled.load(Ordering::SeqCst);
            let settings_changed =
                new_settings.properties.awake_mode.value != self.settings.properties.awake_mode.value
                || new_settings.properties.awake_keep_display_on.value != self.settings.properties.awake_keep_display_on.value
                || new_settings.properties.awake_hours.value != self.settings.properties.awake_hours.value
                || new_settings.properties.awake_minutes.value != self.settings.properties.awake_minutes.value;

            self.settings = new_settings;

            // Restart awake.exe with new flags if settings changed while enabled
            if was_enabled && settings_changed {
                eprintln!("[Awake] Settings changed, restarting with new flags");
                self.stop_awake();
                // Brief pause for the old process to exit
                unsafe { windows_sys::Win32::System::Threading::Sleep(200); }
                self.launch_awake();
            }
        }
    }

    fn destroy(&mut self) {
        self.disable();
    }

    fn gpo_policy_enabled_configuration(&self) -> GpoRuleConfigured {
        // Check registry: HKLM\SOFTWARE\Policies\PowerToys\Awake\Enabled
        check_gpo_for_awake()
    }
}

fn check_gpo_for_awake() -> GpoRuleConfigured {
    use windows_sys::Win32::System::Registry::{
        RegOpenKeyExW, RegQueryValueExW, RegCloseKey, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD,
    };

    let subkey: Vec<u16> = "SOFTWARE\\Policies\\PowerToys"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let value_name: Vec<u16> = "EnableAwake"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey = std::ptr::null_mut();
        let res = RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey);
        if res != 0 {
            return GpoRuleConfigured::NotConfigured;
        }

        let mut data: u32 = 0;
        let mut data_size = std::mem::size_of::<u32>() as u32;
        let mut data_type: u32 = 0;
        let res = RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null(),
            &mut data_type,
            &mut data as *mut u32 as *mut u8,
            &mut data_size,
        );
        RegCloseKey(hkey);

        if res != 0 || data_type != REG_DWORD {
            return GpoRuleConfigured::NotConfigured;
        }

        match data {
            1 => GpoRuleConfigured::Enabled,
            0 => GpoRuleConfigured::Disabled,
            _ => GpoRuleConfigured::NotConfigured,
        }
    }
}

// Register this module — generates rust_module_create / rust_module_destroy exports
powertoys_module_ffi::register_module!(AwakeModule::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_awake_module_creates() {
        let module = AwakeModule::new();
        assert!(!module.is_enabled());
        assert!(module.child_process.is_none());
    }

    #[test]
    fn test_awake_name_and_key() {
        let module = AwakeModule::new();
        let name = wide::wide_to_string(module.get_name());
        let key = wide::wide_to_string(module.get_key());
        assert_eq!(name, "Awake");
        assert_eq!(key, "Awake");
    }

    #[test]
    fn test_awake_enable_disable_state() {
        let mut module = AwakeModule::new();
        assert!(!module.is_enabled());

        // Enable — will fail to launch awake.exe (not present) but state should change
        module.enabled.store(true, Ordering::SeqCst);
        assert!(module.is_enabled());

        module.disable();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_awake_config_roundtrip() {
        let mut module = AwakeModule::new();

        // Get config into a buffer
        let mut size: c_int = 0;
        module.get_config(std::ptr::null_mut(), &mut size);
        assert!(size > 0);

        let mut buffer = vec![0u16; size as usize];
        let mut buf_size = size;
        let ok = module.get_config(buffer.as_mut_ptr(), &mut buf_size);
        assert!(ok);

        // Parse it back
        let json_str = wide::wide_to_string(buffer.as_ptr());
        assert!(json_str.contains("Awake"));

        // Set config back
        module.set_config(buffer.as_ptr());
    }

    #[test]
    fn test_awake_gpo_default() {
        // On a machine without GPO configured, should be NotConfigured
        let module = AwakeModule::new();
        let gpo = module.gpo_policy_enabled_configuration();
        assert_eq!(gpo, GpoRuleConfigured::NotConfigured);
    }

    #[test]
    fn test_awake_set_null_config_no_crash() {
        let mut module = AwakeModule::new();
        module.set_config(std::ptr::null());
        // Should not crash
    }

    #[test]
    fn test_awake_destroy_disables() {
        let mut module = AwakeModule::new();
        module.enabled.store(true, Ordering::SeqCst);
        module.destroy();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_register_macro_exports() {
        // Verify the macro-generated functions exist and work
        let table_ptr = rust_module_create();
        assert!(!table_ptr.is_null());

        unsafe {
            let table = &*table_ptr;
            assert!(!table.context.is_null());

            // Verify it's a real Awake module
            let name = (table.get_name)(table.context);
            let name_str = wide::wide_to_string(name);
            assert_eq!(name_str, "Awake");

            // Clean up — destroy context then free table
            (table.destroy)(table.context);
            let _ = Box::from_raw(table_ptr);
        }
    }
}
