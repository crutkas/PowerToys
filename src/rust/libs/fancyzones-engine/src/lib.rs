//! FancyZones Engine — runtime snapping engine.

pub mod work_area;
pub mod drag_handler;
pub mod snap;

#[cfg(windows)]
pub mod overlay;
#[cfg(windows)]
pub mod engine;
