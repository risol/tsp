//! JSC execution bridge for TSP v2 PoC 1 slice 6.
//!
//! See `tsp-v2-plan.md` sect.25.3 ("JSC 是执行引擎"). Slice 6
//! intentionally does not pull in `bun_runtime` (cold compile 20-40
//! min) nor wire `bun_jsc` directly (no standalone embeddable VM
//! per slice 4's discovery). Instead, the host spawns the project's
//! vendored `bun.exe` (1.4.0+) as a subprocess and asks it to
//! evaluate a slice-6-prepared `.js` file. That keeps the
//! "JavaScriptCore is the execution engine" promise while staying
//! small enough to ship this session.
//!
//! The actual `bun_runtime` integration (in-process JSC VM, native
//! module loader, the `tsp:*` builtins) lands in slice 7+ when we
//! have the time budget for the heavy cold compile.
use std::fs;
use std::io;
use std::io::Read as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::jsx;
use crate::router::HttpMethod;

/// Where `bun.exe` lives. Resolved once at boot via
/// [`resolve_bun_bin`]; the host can override via `TSP_BUN_BIN`.
#[derive(Debug, Clone)]
pub struct BunRuntime {
    pub bin: PathBuf,
}

#[derive(Debug)]
pub enum JscError {
    /// `bun.exe` not found at the resolved path. The operator must
    /// set `TSP_BUN_BIN` or run `bun install` to populate
    /// `.bun-bootstrap/node_modules/bun/bin/bun.exe`.
    BunNotFound { tried: PathBuf },
    /// Spawning the bun process failed (permission, etc.).
    Spawn(io::Error),
    /// bun exited non-zero. We surface the stderr tail (truncated to
    /// 1 KiB) so a JS error from `routes/index.tsp` shows up in the
    /// 500 page without overflowing the response body.
    BunFailed { code: Option<i32>, stderr_tail: String },
    /// bun ran but produced no stdout. Either the page's `GET()`
    /// returned `undefined` or it threw after writing to stderr.
    EmptyStdout,
    /// I/O error from writing the prepared JS to a temp file.
    WriteTemp(io::Error),
    /// The TSX -> JS transform rejected the page source. This is
    /// the only `JscError` a `.tsp` author can fix by editing their
    /// own code (the rest are infra).
    Jsx(jsx::JsxError),
}

impl std::fmt::Display for JscError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BunNotFound { tried } => {
                write!(f, "bun.exe not found at {} (set TSP_BUN_BIN to override)", tried.display())
            }
            Self::Spawn(e) => write!(f, "spawn bun failed: {e}"),
            Self::BunFailed { code, stderr_tail } => {
                let code = code.map(|c| c.to_string()).unwrap_or_else(|| "<signal>".into());
                write!(f, "bun exited {code}: {stderr_tail}")
            }
            Self::EmptyStdout => write!(f, "bun produced no stdout (page returned undefined?)"),
            Self::WriteTemp(e) => write!(f, "write temp .js failed: {e}"),
            Self::Jsx(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for JscError {}

impl JscError {
    /// The TSP-NNNN code for this JSC bridge failure
    /// (spec sect.6.3 / slice 16h). The host threads
    /// this into the 500 body so the dev can grep for
    /// the failure phase (jsx transform / subprocess
    /// / empty stdout).
    pub fn code(&self) -> &'static str {
        match self {
            Self::BunNotFound { .. } => "TSP3010",
            Self::Spawn(_) => "TSP3011",
            Self::BunFailed { .. } => "TSP3012",
            Self::EmptyStdout => "TSP3013",
            Self::WriteTemp(_) => "TSP3014",
            Self::Jsx(_) => "TSP3002",
        }
    }

    /// Short description for the `[TSP-NNNN] <desc>`
    /// line. Kept here (not in host.rs) so the bridge
    /// layer owns the wording for its own failures.
    pub fn describe(&self) -> &'static str {
        match self {
            Self::BunNotFound { .. } => "bun binary not found",
            Self::Spawn(_) => "bun subprocess spawn failed",
            Self::BunFailed { .. } => "bun subprocess exited non-zero",
            Self::EmptyStdout => "bun produced no stdout",
            Self::WriteTemp(_) => "writing bun temp file failed",
            Self::Jsx(_) => "jsx transform error",
        }
    }
}

/// Resolve the bun binary. `TSP_BUN_BIN` wins, else the vendored
/// `.bun-bootstrap/node_modules/bun/bin/bun.exe` relative to the
/// current working directory.
pub fn resolve_bun_bin() -> Result<PathBuf, JscError> {
    if let Ok(s) = std::env::var("TSP_BUN_BIN") {
        let p = PathBuf::from(s);
        if p.is_file() {
            return Ok(p);
        }
        return Err(JscError::BunNotFound { tried: p });
    }
    let p = PathBuf::from(".bun-bootstrap/node_modules/bun/bin/bun.exe");
    if p.is_file() {
        return Ok(p);
    }
    Err(JscError::BunNotFound { tried: p })
}

/// The single-byte marker the host writes to the bun
/// subprocess's stdin to fire `ctx.signal.abort()`
/// (spec sect.13.7). The wrap preamble's stdin
/// listener calls `__tspAbortCtrl.abort()` on the
/// first byte it sees, so any token works; we use
/// `A` for "abort" plus a newline so the listener
/// can detect a complete line on line-buffered
/// streams.
pub const ABORT_MARKER: &[u8] = b"A\n";

/// Execute the page's `method` handler. Reads the .tsp source
/// (already prepared in slice 5 as `page::PageSource`), transforms
/// it via `jsx::tsx_to_js`, writes the result to a temp `.js` file,
/// spawns `bun run <tempfile>`, and returns the captured stdout.
///
/// `timeout_ms` is the request timeout (spec sect.13.7).
/// `0` disables the watchdog. When the watchdog fires
/// (a) the host writes the abort marker to the bun
/// subprocess's stdin, which the wrap preamble's
/// listener turns into `__tspAbortCtrl.abort()`, and
/// (b) the host kills the subprocess after a short
/// grace period so a runaway page cannot hold the
/// worker thread indefinitely.
pub fn execute(
    bun: &BunRuntime,
    source: &str,
    method: HttpMethod,
    ctx_json: Option<&str>,
    timeout_ms: u64,
) -> Result<String, JscError> {
    let js_body = jsx::tsx_to_js(source).map_err(JscError::Jsx)?;
    let wrapped = jsx::wrap_for_bun_cli(&js_body, method.as_str(), ctx_json);

    // Use a per-call temp file. On Windows `std::env::temp_dir()` is
    // `%TEMP%`; the unique suffix avoids collisions under concurrent
    // requests.
    let mut tempfile = std::env::temp_dir();
    let suffix = format!(
        "tsp-v2-slice6-{}-{}.js",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    tempfile.push(suffix);
    fs::write(&tempfile, &wrapped).map_err(JscError::WriteTemp)?;

    let mut cmd = Command::new(&bun.bin);
    cmd.arg("run").arg(&tempfile);
    if let Some(json) = ctx_json {
        // The env var is the side-channel the JS side reads in
        // the wrap preamble. We embed the same JSON as a
        // literal in the JS too (so a page that does not use
        // the env var directly still gets the Context); the
        // env var is here for completeness so JS code that
        // wants the raw JSON (e.g. for streaming, or for
        // debug) can read it. Slice 16d strips the request
        // body from the env form -- env blocks on Windows are
        // capped at ~32 KiB while bodies can reach the 1 MiB
        // default limit; the body always rides inside the
        // embedded literal.
        cmd.env("TSP_CONTEXT_JSON", crate::host::ctx_json_for_env(json));
    }
    // The wrap preamble listens for `A\n` on stdin to
    // fire the abort controller; pipe the stdin end so
    // we can write to it from the watchdog thread.
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(JscError::Spawn)?;
    let pid = child.id();
    let stdin_handle = child.stdin.take();

    // Slice 16i: per-request timeout watchdog. The
    // thread sleeps for `timeout_ms` (or skips entirely
    // when 0), then writes the abort marker to the
    // child's stdin. The grace period gives the page a
    // chance to throw AbortError cleanly; if the child
    // is still alive afterwards, the watchdog kills
    // it so the host thread's `try_wait` loop exits.
    let timed_out = Arc::new(AtomicBool::new(false));
    let watchdog = if timeout_ms > 0 {
        let timed_out = Arc::clone(&timed_out);
        let mut stdin = stdin_handle;
        Some(thread::spawn(move || {
            thread::sleep(Duration::from_millis(timeout_ms));
            timed_out.store(true, Ordering::SeqCst);
            if let Some(ref mut s) = stdin {
                let _ = s.write_all(ABORT_MARKER);
                let _ = s.flush();
            }
            // Grace so the abort propagates through the
            // wrap preamble's stdin listener into
            // `__tspAbortCtrl.abort()` and any await in
            // the page's handler has a chance to throw /
            // resolve cleanly. 1000 ms is enough for an
            // in-process event-loop tick plus the page's
            // own `addEventListener('abort', ...)` work
            // and an async response write; we hard-kill
            // after the grace so the host thread cannot
            // stall on a runaway page that ignores the
            // signal.
            thread::sleep(Duration::from_millis(1000));
        }))
    } else {
        // No timeout: drop the stdin handle so the
        // child can never block waiting for input.
        drop(stdin_handle);
        None
    };

    // Use `try_wait` polling rather than
    // `wait_with_output` (which locks the child handle
    // for the duration of the wait). With a 50ms poll
    // interval the latency overhead is negligible
    // compared to the bun subprocess cost, and the
    // watchdog can still `kill` the process if the
    // grace period expires.
    let mut exited = false;
    let mut final_status: Option<std::process::ExitStatus> = None;
    while !exited {
        match child.try_wait().map_err(JscError::Spawn)? {
            Some(status) => {
                final_status = Some(status);
                exited = true;
            }
            None => {
                if timed_out.load(Ordering::SeqCst) {
                    // Grace expired; hard-kill the child so
                    // the loop exits. Best effort -- the
                    // kill may fail on Windows if the
                    // process already exited but the
                    // `try_wait` above missed it.
                    eprintln!("TSPv2PoC1: timeout grace expired, killing bun subprocess pid={:?}", child.id());
                    let kill_result = child.kill();
                    eprintln!("TSPv2PoC1: child.kill() result = {:?}", kill_result);
                    // After kill, the child should be
                    // reaped quickly; `wait` is blocking
                    // but bounded by the OS. If wait
                    // stalls, the host thread stalls --
                    // not ideal but the alternative is a
                    // runaway page that holds the worker
                    // thread forever, which is worse.
                    match child.wait() {
                        Ok(status) => {
                            final_status = Some(status);
                        }
                        Err(e) => {
                            eprintln!("TSPv2PoC1: child.wait() after kill failed: {e}");
                            // Synthesize a status so the
                            // 500 body still surfaces a
                            // meaningful code.
                            final_status = None;
                        }
                    }
                    exited = true;
                } else {
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
    // Drain stdout / stderr. After the child has
    // exited, `take_stdout` / `take_stderr` returns
    // the remaining bytes (Bun's stdout is line-buffered
    // so any `__TSP_OUT_V1__\n` line emitted before the
    // abort has been flushed).
    let stdout = child.stdout.take().map(|mut s| {
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf);
        buf
    }).unwrap_or_default();
    let stderr = child.stderr.take().map(|mut s| {
        let mut buf = Vec::new();
        let _ = s.read_to_end(&mut buf);
        buf
    }).unwrap_or_default();
    let _ = pid; // child id reserved for future diagnostics

    // Best-effort cleanup. Don't propagate a cleanup failure -- the
    // request has already succeeded or failed on its own merits.
    if let Some(h) = watchdog {
        let _ = h.join();
    }
    let _ = fs::remove_file(&tempfile);

    if timed_out.load(Ordering::SeqCst) {
        // Timeout is a hard failure -- a page that ignores
        // the abort signal is a programming error the dev
        // must see. The 500 body should tell them.
        eprintln!("TSPv2PoC1: request timed out after {timeout_ms}ms");
        return Err(JscError::BunFailed {
            code: None,
            stderr_tail: format!(
                "request timed out after {timeout_ms}ms; abort marker fired but page did not stop in time"
            ),
        });
    }

    if !final_status.as_ref().unwrap().success() {
        // Cap stderr tail at 1 KiB so a JS error with a giant stack
        // does not blow the 500 page into megabytes. Collect into a
        // Vec<u8> first (so `from_utf8_lossy` takes a `&[u8]`), then
        // reverse to restore the chronological order we truncated.
        let mut tail_buf: Vec<u8> = stderr.iter().rev().take(1024).copied().collect();
        tail_buf.reverse();
        let tail = String::from_utf8_lossy(&tail_buf).into_owned();
        return Err(JscError::BunFailed {
            code: final_status.as_ref().unwrap().code(),
            stderr_tail: tail,
        });
    }

    let stdout = String::from_utf8_lossy(&stdout).into_owned();
    if stdout.is_empty() {
        return Err(JscError::EmptyStdout);
    }
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_bun_bin_reports_missing_path() {
        // `resolve_bun_bin` reads TSP_BUN_BIN first, then falls back
        // to the vendored path. On most developer machines the
        // vendored binary is present (the common case during
        // slice-6 development), so this test is a no-op there. When
        // the binary really is missing the error must be
        // `BunNotFound` so the operator gets an actionable message.
        if let Ok(p) = resolve_bun_bin() {
            eprintln!(
                "note: bun resolved to {} in this environment; skipping missing-path check",
                p.display()
            );
            return;
        }
        match resolve_bun_bin() {
            Err(JscError::BunNotFound { .. }) => {}
            Err(other) => panic!("expected BunNotFound, got {other:?}"),
            Ok(_) => unreachable!("just checked Err above"),
        }
    }

    #[test]
    fn jsc_error_codes_are_stable() {
        // Slice 16h: the host formats 500 bodies with the
        // JSC bridge's own code + description. Pin the
        // code table here so a refactor cannot silently
        // renumber the prefix (e.g. turning `TSP3002`
        // for JSX into `TSP3009`).
        let pairs: &[(JscError, &str)] = &[
            (JscError::BunNotFound { tried: PathBuf::from("x") }, "TSP3010"),
            (
                JscError::BunFailed { code: Some(1), stderr_tail: String::new() },
                "TSP3012",
            ),
            (JscError::EmptyStdout, "TSP3013"),
            (
                JscError::WriteTemp(std::io::Error::new(std::io::ErrorKind::Other, "x")),
                "TSP3014",
            ),
            (
                JscError::Jsx(jsx::JsxError::UnsupportedShape { line: 1, reason: "x" }),
                "TSP3002",
            ),
        ];
        for (err, want) in pairs {
            assert_eq!(err.code(), *want, "err = {err:?}");
            assert!(!err.describe().is_empty(), "describe empty for {err:?}");
        }
    }

    #[test]
    fn abort_marker_is_a_single_line() {
        // Slice 16i: the host writes `ABORT_MARKER` to the
        // bun subprocess's stdin when the per-request
        // timeout fires. The wrap preamble's listener
        // reads whatever is buffered and calls
        // `__tspAbortCtrl.abort()`. Pin the marker shape
        // here so a future refactor cannot silently
        // change the wire form (e.g. multi-byte tokens
        // that would race with the listener's first-byte
        // check).
        assert_eq!(ABORT_MARKER, b"A\n");
        assert_eq!(ABORT_MARKER.len(), 2);
    }
}