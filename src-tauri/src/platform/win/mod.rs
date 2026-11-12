mod backend;
mod ffi;
mod keyboard;
mod topology;

use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

pub use backend::{is_laptop, WinBackend};
pub use keyboard::{mask_modifier_release, modifiers_down, start_recording, stop_recording};

/// Show a window on top without taking keyboard focus. Showing it the normal way activates it,
/// and the program the user is typing in then never sees the hotkey's Ctrl/Alt being released.
/// Both are asynchronous: they are called from worker threads and must not wait for the UI thread.
pub fn show_without_focus(hwnd: isize) {
    let flags = ffi::SWP_NOMOVE | ffi::SWP_NOSIZE | ffi::SWP_NOACTIVATE | ffi::SWP_SHOWWINDOW | ffi::SWP_ASYNCWINDOWPOS;
    unsafe { ffi::SetWindowPos(hwnd, ffi::HWND_TOPMOST, 0, 0, 0, 0, flags) };
}

pub fn hide_window(hwnd: isize) {
    unsafe { ffi::ShowWindowAsync(hwnd, ffi::SW_HIDE) };
}

/// Rename `from` over `to` and return only once the change is on disk, so it survives
/// a system crash right afterwards (spec 6.4).
pub fn replace_file_durably(from: &Path, to: &Path) -> io::Result<()> {
    let wide = |p: &Path| -> Vec<u16> { p.as_os_str().encode_wide().chain(std::iter::once(0)).collect() };
    let (from, to) = (wide(from), wide(to));
    let flags = ffi::MOVEFILE_REPLACE_EXISTING | ffi::MOVEFILE_WRITE_THROUGH;
    if unsafe { ffi::MoveFileExW(from.as_ptr(), to.as_ptr(), flags) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
