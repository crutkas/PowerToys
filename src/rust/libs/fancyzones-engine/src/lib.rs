//! FancyZones Engine - core runtime.

pub mod work_area;
pub mod drag_handler;
pub mod snap;

#[cfg(windows)]
pub mod overlay;
#[cfg(windows)]
pub mod engine;

pub use fancyzones_core::data::LayoutData;
pub use fancyzones_core::layout::Layout;
pub use fancyzones_core::rect::{Point, Rect};
pub use fancyzones_core::settings::Settings;
pub use fancyzones_core::zone::{ZoneIndex, ZoneIndexSet};