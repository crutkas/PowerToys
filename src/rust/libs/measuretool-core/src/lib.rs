//! MeasureTool core logic — pure computation, no Win32 dependencies.
//!
//! Provides:
//! - **types**: MeasureMode, Settings, MeasurementResult, pixel buffer view
//! - **edge_detector**: Find color boundaries in pixel buffers
//! - **measurement**: Calculate measurement lines from cursor + edges
//! - **settings**: JSON settings (de)serialization

pub mod edge_detector;
pub mod measurement;
pub mod settings;
pub mod types;
