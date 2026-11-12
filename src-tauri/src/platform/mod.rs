#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::{
    hide_window, is_laptop, mask_modifier_release, modifiers_down, replace_file_durably, show_without_focus,
    small_icon_size, start_recording, stop_recording, WinBackend as NativeBackend,
};

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
pub use stub::{is_laptop, StubBackend as NativeBackend};

/// Elsewhere the settings page records hotkeys from its own key events.
#[cfg(not(windows))]
pub fn start_recording(_on_combo: impl FnOnce(String) + Send + 'static) -> bool {
    false
}
#[cfg(not(windows))]
pub fn stop_recording() {}
#[cfg(not(windows))]
pub fn mask_modifier_release() {}
#[cfg(not(windows))]
pub fn modifiers_down() -> bool {
    false
}
#[cfg(not(windows))]
pub fn small_icon_size() -> u32 {
    32
}

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
