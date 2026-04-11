//! MeasureTool Module Interface — Rust implementation
//!
//! This is the PowerToys module DLL for MeasureTool (Screen Ruler).
//! It implements the `PowerToyModule` trait from `powertoys-module-ffi`
//! and manages launching/stopping the MeasureTool executable.
//!
//! Settings key: "Measure Tool"
//! GPO key: "EnableMeasureTool"

use powertoys_module_ffi::*;
use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

mod settings;
mod wide;

use settings::ModuleSettings;

pub struct MeasureToolModule {
    enabled: AtomicBool,
    settings: ModuleSettings,
    child_process: Option<u32>,
}

impl MeasureToolModule {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            settings: ModuleSettings::default(),
            child_process: None,
        }
    }

    fn launch_process(&mut self) {
        use windows_sys::Win32::System::Threading::GetCurrentProcessId;

        let pid = unsafe { GetCurrentProcessId() };
        let exe_path = self.find_exe();
        let args = format!("\"{}\" --wait-pid {}", exe_path, pid);

        match self.create_process(&args) {
            Ok(child_pid) => {
                self.child_process = Some(child_pid);
                eprintln!("[MeasureTool] Launched: {}", args);
            }
            Err(e) => {
                eprintln!("[MeasureTool] Failed to launch: {}", e);
            }
        }
    }

    fn find_exe(&self) -> String {
        std::env::var("POWERTOYS_MEASURETOOL_EXE")
            .unwrap_or_else(|_| "PowerToys.MeasureTool.exe".to_string())
    }

    fn create_process(&self, cmd: &str) -> Result<u32, String> {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            CreateProcessW, CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, PROCESS_INFORMATION,
            STARTUPINFOW,
        };

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
                0,
                CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW,
                std::ptr::null(),
                std::ptr::null(),
                &si,
                &mut pi,
            )
        };

        if result == 0 {
            return Err(format!(
                "CreateProcessW failed (error: {})",
                unsafe { windows_sys::Win32::Foundation::GetLastError() }
            ));
        }

        let pid = pi.dwProcessId;
        unsafe {
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
        }

        Ok(pid)
    }

    fn stop_process(&mut self) {
        if let Some(pid) = self.child_process.take() {
            use windows_sys::Win32::Foundation::CloseHandle;
            use windows_sys::Win32::System::Threading::{
                OpenProcess, TerminateProcess, WaitForSingleObject,
            };

            unsafe {
                let handle = OpenProcess(0x0001 | 0x00100000, 0, pid);
                if !handle.is_null() {
                    TerminateProcess(handle, 0);
                    WaitForSingleObject(handle, 1000);
                    CloseHandle(handle);
                }
            }
            eprintln!("[MeasureTool] Stopped (PID: {})", pid);
        }
    }
}

impl PowerToyModule for MeasureToolModule {
    fn get_name(&self) -> *const u16 {
        wide::measure_tool_wide_ptr()
    }

    fn get_key(&self) -> *const u16 {
        wide::measure_tool_wide_ptr()
    }

    fn enable(&mut self) {
        if !self.enabled.load(Ordering::SeqCst) {
            self.enabled.store(true, Ordering::SeqCst);
            self.launch_process();
        }
    }

    fn disable(&mut self) {
        if self.enabled.load(Ordering::SeqCst) {
            self.stop_process();
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
        if let Some(new_settings) = ModuleSettings::from_json(&s) {
            self.settings = new_settings;
        }
    }

    fn destroy(&mut self) {
        self.disable();
    }

    fn gpo_policy_enabled_configuration(&self) -> GpoRuleConfigured {
        check_gpo_for_measuretool()
    }
}

fn check_gpo_for_measuretool() -> GpoRuleConfigured {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD,
    };

    let subkey: Vec<u16> = "SOFTWARE\\Policies\\PowerToys"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let value_name: Vec<u16> = "EnableMeasureTool"
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

powertoys_module_ffi::register_module!(MeasureToolModule::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creates() {
        let module = MeasureToolModule::new();
        assert!(!module.is_enabled());
        assert!(module.child_process.is_none());
    }

    #[test]
    fn test_module_name_and_key() {
        let module = MeasureToolModule::new();
        let name = wide::wide_to_string(module.get_name());
        let key = wide::wide_to_string(module.get_key());
        assert_eq!(name, "Measure Tool");
        assert_eq!(key, "Measure Tool");
    }

    #[test]
    fn test_module_enable_disable_state() {
        let mut module = MeasureToolModule::new();
        assert!(!module.is_enabled());
        module.enabled.store(true, Ordering::SeqCst);
        assert!(module.is_enabled());
        module.disable();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_module_config_roundtrip() {
        let module = MeasureToolModule::new();
        let mut size: c_int = 0;
        module.get_config(std::ptr::null_mut(), &mut size);
        assert!(size > 0);

        let mut buffer = vec![0u16; size as usize];
        let mut buf_size = size;
        let ok = module.get_config(buffer.as_mut_ptr(), &mut buf_size);
        assert!(ok);

        let json_str = wide::wide_to_string(buffer.as_ptr());
        assert!(json_str.contains("Measure Tool"));
    }

    #[test]
    fn test_module_gpo_default() {
        let module = MeasureToolModule::new();
        let gpo = module.gpo_policy_enabled_configuration();
        assert_eq!(gpo, GpoRuleConfigured::NotConfigured);
    }

    #[test]
    fn test_set_null_config_no_crash() {
        let mut module = MeasureToolModule::new();
        module.set_config(std::ptr::null());
    }

    #[test]
    fn test_destroy_disables() {
        let mut module = MeasureToolModule::new();
        module.enabled.store(true, Ordering::SeqCst);
        module.destroy();
        assert!(!module.is_enabled());
    }

    #[test]
    fn test_register_macro_exports() {
        let table_ptr = rust_module_create();
        assert!(!table_ptr.is_null());

        unsafe {
            let table = &*table_ptr;
            assert!(!table.context.is_null());

            let name = (table.get_name)(table.context);
            let name_str = wide::wide_to_string(name);
            assert_eq!(name_str, "Measure Tool");

            (table.destroy)(table.context);
            let _ = Box::from_raw(table_ptr);
        }
    }
}
