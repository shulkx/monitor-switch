use serde::{Deserialize, Serialize};

/// Stable identifier of a monitor on this computer, derived from its device path.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MonitorKey(pub String);

/// How this computer is physically connected to a monitor, as reported by the OS.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionKind {
    Hdmi,
    DisplayPort,
    /// DisplayPort carried over USB4 / Thunderbolt.
    UsbTunnel,
    Other,
}

/// A monitor the backend can talk to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorInfo {
    pub key: MonitorKey,
    pub label: String,
    pub connection: ConnectionKind,
}
