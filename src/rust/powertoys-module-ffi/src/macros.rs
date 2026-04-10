//! Macros for registering PowerToys modules.
//!
//! The `register_module!` macro generates the `extern "C"` export functions
//! that the C++ adapter DLL (or future V2 runner) looks for.

/// Register a PowerToys module by generating the required DLL exports.
///
/// Usage:
/// ```ignore
/// register_module!(AwakeModule::new);
/// ```
///
/// This generates:
/// - `rust_module_create() -> *mut ModuleFunctionTable` — called from C++ adapter
/// - `rust_module_destroy(*mut ModuleFunctionTable)` — cleanup
#[macro_export]
macro_rules! register_module {
    ($constructor:expr) => {
        /// Creates the module and returns a function table pointer.
        /// Called by the C++ adapter's `powertoy_create()`.
        #[unsafe(no_mangle)]
        pub extern "C" fn rust_module_create() -> *mut $crate::ModuleFunctionTable {
            let module = $constructor();
            let table = $crate::build_function_table(Box::new(module));
            Box::into_raw(Box::new(table))
        }

        /// Destroys the function table. Called during DLL unload.
        /// # Safety
        /// `table` must be a valid pointer from `rust_module_create`.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn rust_module_destroy(table: *mut $crate::ModuleFunctionTable) {
            if !table.is_null() {
                unsafe {
                    // First destroy the module via the destroy function pointer
                    let t = &*table;
                    (t.destroy)(t.context);
                    // Then free the table itself — but context was already freed by destroy,
                    // so just drop the table box
                    // NOTE: destroy already freed context, so we just free the table wrapper
                    // Actually, ffi_destroy already drops the module. We just need to drop the table.
                    // Re-take ownership of the Box<ModuleFunctionTable> to drop it.
                    let _ = Box::from_raw(table);
                }
            }
        }
    };
}
