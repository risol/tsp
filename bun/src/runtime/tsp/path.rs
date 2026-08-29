//! Path helpers for TSP's native runtime.
//!
//! `std::fs::canonicalize` uses the `realpath(path, NULL)` form on POSIX. In
//! this binary that returned libc-owned buffer is later released through
//! Bun's mimalloc-backed Rust global allocator. Pass a caller-owned buffer to
//! `realpath` instead, so no libc-owned allocation crosses the boundary.

use std::io;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
pub(crate) fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    use std::ffi::{CStr, CString, OsString};
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let input = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "path contains an interior NUL",
        )
    })?;
    let mut buffer = vec![0u8; libc::PATH_MAX as usize];
    let resolved = unsafe { libc::realpath(input.as_ptr(), buffer.as_mut_ptr().cast()) };
    if resolved.is_null() {
        return Err(io::Error::last_os_error());
    }

    let resolved = unsafe { CStr::from_ptr(resolved).to_bytes() };

    Ok(PathBuf::from(OsString::from_vec(resolved.to_vec())))
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    std::fs::canonicalize(path)
}
