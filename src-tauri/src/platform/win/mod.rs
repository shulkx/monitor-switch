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

/// Width of a small icon (the tray's) at the system scaling: 16 px at 100 %, 24 px at 150 %.
pub fn small_icon_size() -> u32 {
    let size = unsafe { ffi::GetSystemMetricsForDpi(ffi::SM_CXSMICON, ffi::GetDpiForSystem()) };
    if size > 0 { size as u32 } else { 16 }
}

/// The pop-up holding the tray icon at (x, y), i.e. the overflow menu, as [left, top, right, bottom];
/// `None` when the icon sits on a taskbar. Call it right after the click, while the menu is open.
/// Works for any pop-up, so it doesn't depend on the class name Windows 10 or 11 gives the menu.
pub fn tray_icon_host(x: i32, y: i32) -> Option<[i32; 4]> {
    unsafe {
        let root = ffi::GetAncestor(ffi::WindowFromPoint(ffi::POINT { x, y }), ffi::GA_ROOT);
        if root == 0 {
            return None;
        }
        let mut class = [0u16; 64];
        let len = ffi::GetClassNameW(root, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
        let class = String::from_utf16_lossy(&class[..len]);
        if class == "Shell_TrayWnd" || class == "Shell_SecondaryTrayWnd" {
            return None;
        }
        let mut pid = 0;
        ffi::GetWindowThreadProcessId(root, &mut pid);
        if pid == ffi::GetCurrentProcessId() {
            return None;
        }
        let mut r = ffi::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if ffi::GetWindowRect(root, &mut r) == 0 {
            return None;
        }
        Some([r.left, r.top, r.right, r.bottom])
    }
}

/// Open a web page in the default browser.
pub fn open_url(url: &str) {
    let (op, file) = (ffi::to_wide("open"), ffi::to_wide(url));
    unsafe {
        ffi::ShellExecuteW(0, op.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), ffi::SW_SHOWNORMAL);
    }
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
