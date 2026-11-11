#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::{is_laptop, WinBackend as NativeBackend};

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
pub use stub::{is_laptop, StubBackend as NativeBackend};

/// Name shown for this computer when its input has no name yet.
pub fn computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "本机".into())
}
