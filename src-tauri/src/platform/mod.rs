#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::{is_laptop, replace_file_durably, WinBackend as NativeBackend};

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
pub use stub::{is_laptop, StubBackend as NativeBackend};

#[cfg(not(windows))]
pub fn replace_file_durably(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
}

/// Name shown for this computer when its input has no name yet.
pub fn computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "本机".into())
}
