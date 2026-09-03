//! Cross-platform process information for process-model tests.
//!
//! Process-model verification needs to read fields the protocol
//! intentionally does not carry: parent PID, the actual executable the
//! process started from, and the full command line. The master speaks
//! only the versioned TSPW protocol, so this helper is read-only and
//! lives outside the protocol surface.
//!
//! The platform split is intentional:
//!
//! - Linux uses `/proc/<pid>/{stat,exe,cmdline}`, the same surface
//!   `kill -0` uses and which the supported Linux target ships.
//! - Windows has no `/proc`; process model assertions that need the
//!   parent's exe path or argv are satisfied by having the *master*
//!   pass its own exe path through an environment variable before
//!   spawning the worker, and the worker cross-checks
//!   `std::env::current_exe()`. This module therefore only exposes
//!   `current()` (which still works on Windows because of
//!   `GetModuleFileNameW` semantics exposed by `current_exe()`).
//!
//! Both platforms intentionally expose the same [`ProcessInfo`] shape
//! so a test can assert against `info.exe_path` / `info.ppid` /
//! `info.argv` without `#[cfg]` noise at the call site. On Windows,
//! `ppid` and `argv` are best-effort: `ppid` may be `0` if the
//! underlying query is unavailable, and `argv` is `Vec::new()` because
//! reading the target's command line requires walking the PEB.

use std::path::PathBuf;

/// Snapshot of the fields a process-model test can assert against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    /// Parent PID. On Windows this may be `0` if the query is
    /// unavailable; Unix always returns a real value.
    pub ppid: u32,
    pub exe_path: PathBuf,
    /// Command-line argv. Unix reads `/proc/<pid>/cmdline`; on Windows
    /// this is `Vec::new()` because the PEB walk is intentionally
    /// out of scope (use `TSP_MASTER_ARGV` env passing instead).
    pub argv: Vec<String>,
}

/// Return the [`ProcessInfo`] of the calling process.
pub fn current() -> std::io::Result<ProcessInfo> {
    collect(std::process::id())
}

/// Read the [`ProcessInfo`] for an arbitrary live PID.
///
/// Returns [`std::io::ErrorKind::NotFound`] if the PID has already
/// exited or never existed. Permission errors surface with the
/// platform's default kind.
///
/// On non-Linux platforms, this is a no-op for arbitrary PIDs (returns
/// `PermissionDenied`); tests that need cross-process inspection should
/// arrange for the inspected process to publish its info to a file.
pub fn collect(pid: u32) -> std::io::Result<ProcessInfo> {
    #[cfg(target_os = "linux")]
    {
        unix::collect(pid)
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Non-Linux platforms: only "self" is fully supported. For an arbitrary
        // PID we return PermissionDenied so tests that try to inspect
        // a peer process are told to use the file-based path instead
        // of silently getting a half-populated record.
        if pid == std::process::id() {
            self_info_non_linux()
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "process_inspector::collect(arbitrary pid) is Linux-only; \
                 use the file-based process_info.json path on non-Linux platforms",
            ))
        }
    }
}

/// True if a live process exists for `pid`.
///
/// Unlike [`collect`], this does not surface permission errors: a
/// process owned by another user is treated as "alive" if the
/// existence check succeeds, because process-model tests only need
/// to know whether the worker is gone (the system already reaped
/// it) or still around.
pub fn is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // Safety: `kill(pid, 0)` performs the existence / permission
        // check without sending a signal. Returns 0 on success, -1
        // with `EPERM` (process exists but owned by another user) or
        // `ESRCH` (no such process).
        let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if result == 0 {
            true
        } else {
            matches!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EPERM)
            )
        }
    }
    #[cfg(not(unix))]
    {
        // On Windows, the master process can only directly query
        // itself. Reaping a peer is observable through the
        // protocol (the peer's stream closes) or the test
        // framework, not through this helper.
        pid == std::process::id()
    }
}

#[cfg(target_os = "linux")]
mod unix {
    use super::ProcessInfo;
    use std::io;

    pub(super) fn collect(pid: u32) -> io::Result<ProcessInfo> {
        let stat_path = format!("/proc/{pid}/stat");
        let stat = std::fs::read_to_string(&stat_path).map_err(|e| map_proc_error(pid, e))?;
        let ppid = parse_ppid_from_stat(&stat)?;
        let exe_path = std::fs::read_link(format!("/proc/{pid}/exe"))
            .map_err(|e| map_proc_error(pid, e))?;
        let argv = read_cmdline(pid)?;
        Ok(ProcessInfo {
            pid,
            ppid,
            exe_path,
            argv,
        })
    }

    /// `/proc/<pid>/stat` line 1: `pid (comm) state ppid pgrp ...`.
    ///
    /// `comm` may contain spaces and parentheses (a process can
    /// rename itself to anything), so split on the *last* `)` to
    /// find the end of the comm field. Field index 3 in the
    /// resulting list (0-based after the comm pair) is the parent
    /// PID.
    pub(super) fn parse_ppid_from_stat(stat: &str) -> io::Result<u32> {
        let comm_end = stat.rfind(')').ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "missing comm close paren")
        })?;
        let after_comm = &stat[comm_end + 1..];
        let mut fields = after_comm.split_whitespace();
        // field 0 = state, field 1 = ppid
        let _state = fields
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing state field"))?;
        let ppid_field = fields
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing ppid field"))?;
        ppid_field.parse::<u32>().map_err(|error| {
            io::Error::new(io::ErrorKind::InvalidData, format!("bad ppid: {error}"))
        })
    }

    fn read_cmdline(pid: u32) -> io::Result<Vec<String>> {
        let raw = std::fs::read(format!("/proc/{pid}/cmdline")).map_err(|e| map_proc_error(pid, e))?;
        if raw.is_empty() {
            return Ok(Vec::new());
        }
        // cmdline is a sequence of NUL-terminated UTF-8 (or
        // locale-encoded) strings; the final byte may also be NUL.
        // Split and decode each non-empty piece, replacing invalid
        // bytes rather than failing because some kernels append
        // locale-encoded arguments.
        let mut argv = Vec::new();
        for piece in raw.split(|byte| *byte == 0) {
            if piece.is_empty() {
                continue;
            }
            argv.push(String::from_utf8_lossy(piece).into_owned());
        }
        Ok(argv)
    }

    fn map_proc_error(pid: u32, error: io::Error) -> io::Error {
        if error.kind() == io::ErrorKind::NotFound {
            // /proc returns NotFound both for "no such file" and
            // "process exited between the lookup and the read";
            // keep the kind so callers can distinguish from generic
            // I/O errors but attach the pid for diagnostics.
            io::Error::new(io::ErrorKind::NotFound, format!("process {pid} not found"))
        } else {
            error
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn self_info_non_linux() -> std::io::Result<ProcessInfo> {
    let pid = std::process::id();
    let exe_path = std::env::current_exe()?;
    #[cfg(unix)]
    let ppid = unsafe { libc::getppid() as u32 };
    #[cfg(not(unix))]
    let ppid = 0;
    Ok(ProcessInfo {
        pid,
        ppid,
        exe_path,
        argv: std::env::args().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_reports_self_pid() {
        let info = current().expect("self inspection should succeed");
        assert_eq!(info.pid, std::process::id());
        assert!(!info.exe_path.as_os_str().is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parse_ppid_handles_comm_with_parentheses() {
        // A renamed self (e.g. setproctitle-style "foo)bar") must
        // not break the field split; we just exercise the parser
        // against a synthesised line.
        let line = "1234 (foo)bar) S 9999 1234 1234 0 -1 4194304 100 0 0 0 0 0 0 0 20 0 1 0 1234567 1000000 100";
        let ppid = unix::parse_ppid_from_stat(line).expect("parse should succeed");
        assert_eq!(ppid, 9999);
    }
}
