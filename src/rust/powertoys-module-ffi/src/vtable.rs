//! The PowerToyModule trait and the extern "C" vtable bridge.
//!
//! Rust modules implement `PowerToyModule`. The `register_module!` macro
//! generates the `extern "C"` functions that the C++ adapter calls.

use crate::types::*;
use std::ffi::c_int;

/// The trait every Rust PowerToys module implements.
///
/// Method signatures mirror `PowertoyModuleIface` from the C++ runner.
/// Wide-string returns must be null-terminated UTF-16 with `'static` lifetime
/// (typically via `once_cell` or leaked allocations).
pub trait PowerToyModule: Send {
    /// Display name (e.g., L"Awake"). Must return a stable pointer.
    fn get_name(&self) -> *const u16;

    /// Unique key used for settings and module identification.
    fn get_key(&self) -> *const u16;

    /// Activate the module.
    fn enable(&mut self);

    /// Deactivate the module.
    fn disable(&mut self);

    /// Whether the module is currently active.
    fn is_enabled(&self) -> bool;

    /// Serialize current config into the provided wide-string buffer.
    /// Returns `true` if the buffer was large enough.
    fn get_config(&self, buffer: *mut u16, buffer_size: *mut c_int) -> bool;

    /// Apply a new JSON config (wide string, null-terminated).
    fn set_config(&mut self, config: *const u16);

    /// Handle a custom action from the settings UI.
    fn call_custom_action(&mut self, _action: *const u16) {}

    /// Clean up. Called before the DLL is unloaded.
    fn destroy(&mut self);

    /// Fill the buffer with registered hotkeys. Returns number of hotkeys.
    fn get_hotkeys(&self, _buffer: *mut Hotkey, _buffer_size: usize) -> usize {
        0
    }

    /// Handle a hotkey press. Returns true if handled.
    fn on_hotkey(&mut self, _hotkey_id: usize) -> bool {
        false
    }

    /// Whether this module is enabled by default.
    fn is_enabled_by_default(&self) -> bool {
        true
    }

    /// Whether this module tracks Win key press (for ShortcutGuide-style activation).
    fn keep_track_of_pressed_win_key(&self) -> bool {
        false
    }

    /// Milliseconds the Win key must be held before triggering. 0 = don't track.
    fn milliseconds_win_key_must_be_pressed(&self) -> u32 {
        0
    }

    /// Return the custom hotkey for this module, or None for default behavior.
    fn get_hotkey_ex(&self) -> Option<HotkeyEx> {
        None
    }

    /// Called when the extended hotkey triggers (e.g., long Win press for ShortcutGuide).
    fn on_hotkey_ex(&mut self) {}

    /// Check GPO policy for this module.
    fn gpo_policy_enabled_configuration(&self) -> GpoRuleConfigured {
        GpoRuleConfigured::NotConfigured
    }
}

// ── Extern "C" trampolines ─────────────────────────────────────────────────
// These are the functions the C++ adapter calls. Each one recovers the
// `Box<dyn PowerToyModule>` from the opaque context pointer.

/// # Safety
/// `ctx` must be a valid pointer to a `Box<dyn PowerToyModule>` created by
/// the module's `powertoy_create_v2` or equivalent.
#[inline]
unsafe fn as_module<'a>(ctx: *mut std::ffi::c_void) -> &'a mut Box<dyn PowerToyModule> {
    unsafe { &mut *(ctx as *mut Box<dyn PowerToyModule>) }
}

pub unsafe extern "C" fn ffi_get_name(ctx: *mut std::ffi::c_void) -> *const u16 {
    unsafe { as_module(ctx).get_name() }
}

pub unsafe extern "C" fn ffi_get_key(ctx: *mut std::ffi::c_void) -> *const u16 {
    unsafe { as_module(ctx).get_key() }
}

pub unsafe extern "C" fn ffi_enable(ctx: *mut std::ffi::c_void) {
    unsafe { as_module(ctx).enable() }
}

pub unsafe extern "C" fn ffi_disable(ctx: *mut std::ffi::c_void) {
    unsafe { as_module(ctx).disable() }
}

pub unsafe extern "C" fn ffi_is_enabled(ctx: *mut std::ffi::c_void) -> bool {
    unsafe { as_module(ctx).is_enabled() }
}

pub unsafe extern "C" fn ffi_get_config(
    ctx: *mut std::ffi::c_void,
    buffer: *mut u16,
    buffer_size: *mut c_int,
) -> bool {
    unsafe { as_module(ctx).get_config(buffer, buffer_size) }
}

pub unsafe extern "C" fn ffi_set_config(ctx: *mut std::ffi::c_void, config: *const u16) {
    unsafe { as_module(ctx).set_config(config) }
}

pub unsafe extern "C" fn ffi_call_custom_action(ctx: *mut std::ffi::c_void, action: *const u16) {
    unsafe { as_module(ctx).call_custom_action(action) }
}

pub unsafe extern "C" fn ffi_destroy(ctx: *mut std::ffi::c_void) {
    unsafe {
        let module = &mut *(ctx as *mut Box<dyn PowerToyModule>);
        module.destroy();
        // Drop the Box to free memory
        let _ = Box::from_raw(ctx as *mut Box<dyn PowerToyModule>);
    }
}

pub unsafe extern "C" fn ffi_get_hotkeys(
    ctx: *mut std::ffi::c_void,
    buffer: *mut Hotkey,
    buffer_size: usize,
) -> usize {
    unsafe { as_module(ctx).get_hotkeys(buffer, buffer_size) }
}

pub unsafe extern "C" fn ffi_on_hotkey(ctx: *mut std::ffi::c_void, hotkey_id: usize) -> bool {
    unsafe { as_module(ctx).on_hotkey(hotkey_id) }
}

pub unsafe extern "C" fn ffi_gpo_policy(
    ctx: *mut std::ffi::c_void,
) -> GpoRuleConfigured {
    unsafe { as_module(ctx).gpo_policy_enabled_configuration() }
}

pub unsafe extern "C" fn ffi_is_enabled_by_default(ctx: *mut std::ffi::c_void) -> bool {
    unsafe { as_module(ctx).is_enabled_by_default() }
}

pub unsafe extern "C" fn ffi_keep_track_of_pressed_win_key(ctx: *mut std::ffi::c_void) -> bool {
    unsafe { as_module(ctx).keep_track_of_pressed_win_key() }
}

pub unsafe extern "C" fn ffi_milliseconds_win_key_must_be_pressed(ctx: *mut std::ffi::c_void) -> u32 {
    unsafe { as_module(ctx).milliseconds_win_key_must_be_pressed() }
}

pub unsafe extern "C" fn ffi_on_hotkey_ex(ctx: *mut std::ffi::c_void) {
    unsafe { as_module(ctx).on_hotkey_ex() }
}

/// Returns true if a custom hotkey is set, writing it to `out`. False = use default.
pub unsafe extern "C" fn ffi_get_hotkey_ex(ctx: *mut std::ffi::c_void, out: *mut HotkeyEx) -> bool {
    let module = unsafe { as_module(ctx) };
    match module.get_hotkey_ex() {
        Some(hk) => {
            if !out.is_null() { unsafe { *out = hk; } }
            true
        }
        None => false,
    }
}

/// Build a `ModuleFunctionTable` from a boxed module.
/// The returned table owns the module via the context pointer.
pub fn build_function_table(module: Box<dyn PowerToyModule>) -> ModuleFunctionTable {
    let boxed: Box<Box<dyn PowerToyModule>> = Box::new(module);
    let ctx = Box::into_raw(boxed) as *mut std::ffi::c_void;

    ModuleFunctionTable {
        context: ctx,
        get_name: ffi_get_name,
        get_key: ffi_get_key,
        enable: ffi_enable,
        disable: ffi_disable,
        is_enabled: ffi_is_enabled,
        get_config: ffi_get_config,
        set_config: ffi_set_config,
        call_custom_action: ffi_call_custom_action,
        destroy: ffi_destroy,
        get_hotkeys: ffi_get_hotkeys,
        on_hotkey: ffi_on_hotkey,
        is_enabled_by_default: ffi_is_enabled_by_default,
        keep_track_of_pressed_win_key: ffi_keep_track_of_pressed_win_key,
        milliseconds_win_key_must_be_pressed: ffi_milliseconds_win_key_must_be_pressed,
        on_hotkey_ex: ffi_on_hotkey_ex,
        get_hotkey_ex: ffi_get_hotkey_ex,
        gpo_policy_enabled_configuration: ffi_gpo_policy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Wide string helper: encode a &str to null-terminated UTF-16 and leak it
    fn wide_str(s: &str) -> *const u16 {
        let v: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
        let ptr = v.as_ptr();
        std::mem::forget(v);
        ptr
    }

    /// Wrapper to make *const u16 Send-safe for test purposes.
    /// These are leaked static allocations that live for the process lifetime.
    #[derive(Clone, Copy)]
    struct SendPtr(*const u16);
    unsafe impl Send for SendPtr {}

    struct TestModule {
        name: SendPtr,
        key: SendPtr,
        enabled: bool,
        destroyed: Arc<AtomicBool>,
    }

    impl PowerToyModule for TestModule {
        fn get_name(&self) -> *const u16 { self.name.0 }
        fn get_key(&self) -> *const u16 { self.key.0 }
        fn enable(&mut self) { self.enabled = true; }
        fn disable(&mut self) { self.enabled = false; }
        fn is_enabled(&self) -> bool { self.enabled }
        fn get_config(&self, _buf: *mut u16, _sz: *mut c_int) -> bool { false }
        fn set_config(&mut self, _config: *const u16) {}
        fn destroy(&mut self) { self.destroyed.store(true, Ordering::SeqCst); }
    }

    #[test]
    fn test_module_lifecycle_via_ffi() {
        let destroyed = Arc::new(AtomicBool::new(false));
        let module = TestModule {
            name: SendPtr(wide_str("TestModule")),
            key: SendPtr(wide_str("test")),
            enabled: false,
            destroyed: destroyed.clone(),
        };

        let table = build_function_table(Box::new(module));

        unsafe {
            // Initially disabled
            assert!(!(table.is_enabled)(table.context));

            // Enable
            (table.enable)(table.context);
            assert!((table.is_enabled)(table.context));

            // Disable
            (table.disable)(table.context);
            assert!(!(table.is_enabled)(table.context));

            // Name and key should be non-null
            let name = (table.get_name)(table.context);
            assert!(!name.is_null());
            let key = (table.get_key)(table.context);
            assert!(!key.is_null());

            // GPO default
            let gpo = (table.gpo_policy_enabled_configuration)(table.context);
            assert_eq!(gpo, GpoRuleConfigured::NotConfigured);

            // Hotkeys default
            let count = (table.get_hotkeys)(table.context, std::ptr::null_mut(), 0);
            assert_eq!(count, 0);

            // Destroy
            assert!(!destroyed.load(Ordering::SeqCst));
            (table.destroy)(table.context);
            assert!(destroyed.load(Ordering::SeqCst));
        }
    }

    #[test]
    fn test_on_hotkey_default_returns_false() {
        let module = TestModule {
            name: SendPtr(wide_str("Test")),
            key: SendPtr(wide_str("test")),
            enabled: false,
            destroyed: Arc::new(AtomicBool::new(false)),
        };
        let table = build_function_table(Box::new(module));
        unsafe {
            assert!(!(table.on_hotkey)(table.context, 0));
            assert!(!(table.on_hotkey)(table.context, 42));
            (table.destroy)(table.context);
        }
    }
}
