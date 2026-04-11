//! PowerAccent core library.
//!
//! Provides accent character maps, a key state machine for accent selection,
//! and settings management. This crate contains no platform-specific code;
//! the keyboard hook and SendInput calls live in the `poweraccent-kbd` binary.

pub mod accent_map;
pub mod settings;
pub mod state_machine;
