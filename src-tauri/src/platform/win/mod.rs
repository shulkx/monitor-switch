mod backend;
mod ffi;
mod topology;

use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

pub use backend::{is_laptop, WinBackend};

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
