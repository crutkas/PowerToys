//! FileLocksmith core library.
//!
//! Provides path normalisation / matching logic, handle-info data types, and
//! output formatting.  Platform-specific enumeration (NtQuerySystemInformation)
//! lives in the `filelocksmith` binary.

pub mod handle_info;
pub mod path;
pub mod process;
