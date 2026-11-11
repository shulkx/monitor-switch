use crate::model::{MonitorInfo, MonitorKey};

/// Everything the app needs from a platform's DDC/CI implementation.
/// All calls happen on the MonitorService worker thread.
pub trait DisplayBackend: Send {
    /// Enumerate monitors again. Handles from the previous call may be released.
    fn list(&mut self) -> Vec<MonitorInfo>;
    /// Current value of VCP 0x60 (Input Select).
    fn read_input(&mut self, key: &MonitorKey) -> Result<u16, String>;
    fn set_input(&mut self, key: &MonitorKey, value: u16) -> Result<(), String>;
    /// The raw capabilities string.
    fn capabilities(&mut self, key: &MonitorKey) -> Result<String, String>;
}
