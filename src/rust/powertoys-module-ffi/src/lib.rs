//! PowerToys Module FFI Bridge
//!
//! Provides the `PowerToyModule` trait that Rust modules implement,
//! and the `extern "C"` function exports that a thin C++ adapter calls.
//!
//! # Architecture
//!
//! ```text
//! Runner (C++) → LoadLibrary → C++ adapter DLL
//!   → PowertoyModuleIface vtable → extern "C" rust_* functions
//!   → PowerToyModule trait impl (Rust)
//! ```

pub mod types;
pub mod vtable;
pub mod macros;

pub use types::*;
pub use vtable::*;
