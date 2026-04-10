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

const EXIT_EVENT_NAME: &str = "POWERTOYS_AWAKE_EXIT_EVENT";

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
    fn launch_awake(&mut self) {
        use windows_sys::Win32::System::Threading::GetCurrentProcessId;

        let pid = unsafe { GetCurrentProcessId() };

        // Build command: awake.exe --use-pt-config --pid <runner_pid>
        let exe_path = self.find_awake_exe();
        let cmd = format!("\"{}\" --use-pt-config --pid {}", exe_path, pid);

        match self.create_process(&cmd) {
            Ok(child_pid) => {
                self.child_process = Some(child_pid);
                eprintln!("[Awake] Launched awake.exe (PID: {})", child_pid);
            }
            Err(e) => {
                eprintln!("[Awake] Failed to launch awake.exe: {}", e);
            }
        }
    }

    fn find_awake_exe(&self) -> String {
        // In production, this resolves relative to the module DLL path.
        // For now, use a placeholder that can be overridden.
        std::env::var("POWERTOYS_AWAKE_EXE")
            .unwrap_or_else(|_| "PowerToys.Awake.exe".to_string())
    }

    fn create_process(&self, cmd: &str) -> Result<u32, String> {
        use windows_sys::Win32::System::Threading::{
            CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW,
            CREATE_NEW_PROCESS_GROUP, CREATE_UNICODE_ENVIRONMENT,
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
                CREATE_NEW_PROCESS_GROUP | CREATE_UNICODE_ENVIRONMENT,
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

    /// Signal the named exit event to tell awake.exe to stop.
    fn signal_exit_event(&self) {
        use windows_sys::Win32::System::Threading::{OpenEventW, SetEvent};
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::EVENT_MODIFY_STATE;

        let event_name: Vec<u16> = EXIT_EVENT_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let handle = OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr());
            if !handle.is_null() {
                SetEvent(handle);
                CloseHandle(handle);
                eprintln!("[Awake] Signaled exit event");
            } else {
                eprintln!("[Awake] Exit event not found (awake.exe may not be running)");
            }
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
            self.signal_exit_event();
            self.enabled.store(false, Ordering::SeqCst);
            self.child_process = None;
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
        if let Some(settings) = AwakeSettings::from_json(&s) {
            self.settings = settings;
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
