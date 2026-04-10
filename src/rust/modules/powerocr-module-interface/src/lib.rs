use powertoys_module_ffi::*;
use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

static MODULE_WIDE: std::sync::LazyLock<&'static [u16]> = std::sync::LazyLock::new(|| {
    Box::leak(to_wide("TextExtractor").into_boxed_slice())
});

pub struct Module {
    enabled: AtomicBool,
    process_handle: Option<*mut std::ffi::c_void>,
    terminate_event: *mut std::ffi::c_void,
}
unsafe impl Send for Module {}

impl Module {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            process_handle: None,
            terminate_event: create_event("Local\\TerminatePowerOCREvent-08e5de9d-15df-4ea8-8840-487c13435a67"),
        }
    }

    fn launch_process(&mut self) {
        use windows_sys::Win32::UI::Shell::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let pid = unsafe { windows_sys::Win32::System::Threading::GetCurrentProcessId() };
        let exe = to_wide("PowerToys.PowerOCR.exe");
        let params = to_wide(&pid.to_string());
        unsafe {
            let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
            sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            sei.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI;
            sei.lpFile = exe.as_ptr();
            sei.nShow = SW_SHOWNORMAL;
            sei.lpParameters = params.as_ptr();
            if ShellExecuteExW(&mut sei) != 0 { self.process_handle = Some(sei.hProcess); }
        }
    }
}

impl PowerToyModule for Module {
    fn get_name(&self) -> *const u16 { MODULE_WIDE.as_ptr() }
    fn get_key(&self) -> *const u16 { MODULE_WIDE.as_ptr() }
    fn enable(&mut self) {
        self.enabled.store(true, Ordering::SeqCst);
        self.launch_process();
    }
    fn disable(&mut self) {
        self.enabled.store(false, Ordering::SeqCst);
        if !self.terminate_event.is_null() {
            unsafe { windows_sys::Win32::System::Threading::SetEvent(self.terminate_event); }
        }
        if let Some(h) = self.process_handle.take() {
            unsafe {
                windows_sys::Win32::System::Threading::WaitForSingleObject(h, 1500);
                windows_sys::Win32::System::Threading::TerminateProcess(h, 0);
                windows_sys::Win32::Foundation::CloseHandle(h);
            }
        }
    }
    fn is_enabled(&self) -> bool { self.enabled.load(Ordering::SeqCst) }
    fn get_config(&self, _buf: *mut u16, sz: *mut c_int) -> bool {
        if !sz.is_null() { unsafe { *sz = 0; } }
        false
    }
    fn set_config(&mut self, _config: *const u16) {}
    fn destroy(&mut self) { self.disable(); }
    fn gpo_policy_enabled_configuration(&self) -> GpoRuleConfigured {
        check_gpo("EnableTextExtractor")
    }
}

fn to_wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

fn create_event(name: &str) -> *mut std::ffi::c_void {
    let wide = to_wide(name);
    unsafe {
        let mut sa: windows_sys::Win32::Security::SECURITY_ATTRIBUTES = std::mem::zeroed();
        sa.nLength = std::mem::size_of::<windows_sys::Win32::Security::SECURITY_ATTRIBUTES>() as u32;
        windows_sys::Win32::System::Threading::CreateEventW(&sa, 0, 0, wide.as_ptr())
    }
}

fn check_gpo(value_name: &str) -> GpoRuleConfigured {
    use windows_sys::Win32::System::Registry::*;
    let subkey = to_wide("SOFTWARE\\Policies\\PowerToys");
    let val = to_wide(value_name);
    unsafe {
        let mut hkey = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return GpoRuleConfigured::NotConfigured;
        }
        let mut data: u32 = 0;
        let mut sz = 4u32;
        let mut dt: u32 = 0;
        let r = RegQueryValueExW(hkey, val.as_ptr(), std::ptr::null(), &mut dt, &mut data as *mut u32 as *mut u8, &mut sz);
        RegCloseKey(hkey);
        if r != 0 || dt != REG_DWORD { return GpoRuleConfigured::NotConfigured; }
        match data { 1 => GpoRuleConfigured::Enabled, 0 => GpoRuleConfigured::Disabled, _ => GpoRuleConfigured::NotConfigured }
    }
}

powertoys_module_ffi::register_module!(Module::new);
