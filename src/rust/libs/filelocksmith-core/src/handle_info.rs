use serde::{Deserialize, Serialize};

/// Information about a single open handle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleInfo {
    /// The process ID that owns the handle.
    pub pid: u32,
    /// The raw handle value.
    pub handle_value: u64,
    /// The object type name (e.g. `"File"`).
    pub type_name: String,
    /// The kernel-space file path (e.g. `\Device\HarddiskVolume2\…`).
    pub kernel_path: String,
}

/// Result describing a process that holds open files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    /// Process executable name (e.g. `"notepad.exe"`).
    pub name: String,
    /// Process ID.
    pub pid: u32,
    /// User name under which the process runs.
    pub user: String,
    /// List of files the process has open that match the query.
    pub files: Vec<String>,
}
