//! Shared Win32 utilities for PowerToys Rust modules.
//!
//! Provides common helpers used across multiple PowerToys modules:
//! - Wide string conversions (UTF-8 ↔ UTF-16)
//! - Named event creation and signaling
//! - Process management (open, wait, terminate)
//! - Monitor enumeration and DPI queries
//! - PowerToys settings path resolution and JSON loading
//! - Window enumeration helpers
//! - Singleton mutex management

pub mod string;
pub mod settings;
pub mod rect;

#[cfg(windows)]
pub mod event;
#[cfg(windows)]
pub mod process;
#[cfg(windows)]
pub mod monitor;
#[cfg(windows)]
pub mod window;
#[cfg(windows)]
pub mod mutex;
