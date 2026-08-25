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
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
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

/// One-shot cancellation shared by the host connection and the Bun
/// subprocess watchdog. A cancelled request must never write a response.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
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
    /// The client disconnected while the page was executing.
    Cancelled,
    /// The request deadline expired. The stderr tail preserves the
    /// page-side abort evidence when the handler cooperated.
    TimedOut { stderr_tail: String },
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
            Self::Cancelled => write!(f, "request cancelled after client disconnect"),
            Self::TimedOut { stderr_tail } => {
                write!(f, "request timed out: {stderr_tail}")
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
            Self::Cancelled => "TSP3015",
            Self::TimedOut { .. } => "TSP3009",
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
            Self::Cancelled => "request cancelled",
            Self::TimedOut { .. } => "request timed out",
            Self::EmptyStdout => "bun produced no stdout",
            Self::WriteTemp(_) => "writing bun temp file failed",
            Self::Jsx(_) => "jsx transform error",
        }
    }
}

/// Resolve the bundled Bun runtime. `TSP_BUN_BIN` is an explicit override;
/// production packages may place `bun.exe` beside `tspserver_v2.exe` or in a
/// `.tsp-runtime/` child directory. The development bootstrap path remains a
/// final fallback, so the target machine never needs a globally installed Bun
/// CLI.
pub fn resolve_bun_bin() -> Result<PathBuf, JscError> {
    let bun_name = if cfg!(windows) { "bun.exe" } else { "bun" };
    if let Ok(s) = std::env::var("TSP_BUN_BIN") {
        let p = PathBuf::from(s);
        if p.is_file() {
            return Ok(p);
        }
        return Err(JscError::BunNotFound { tried: p });
    }
    let mut candidates = Vec::new();
    if let Ok(runtime_dir) = std::env::var("TSP_RUNTIME_DIR") {
        candidates.push(PathBuf::from(runtime_dir).join(bun_name));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(bun_name));
            candidates.push(dir.join(format!(".tsp-runtime/{bun_name}")));
            candidates.push(dir.join(format!("runtime/{bun_name}")));
        }
    }
    candidates.push(PathBuf::from(format!(".tsp-runtime/{bun_name}")));
    candidates.push(PathBuf::from(format!(".bun-bootstrap/node_modules/bun/bin/{bun_name}")));
    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(JscError::BunNotFound {
        tried: PathBuf::from("bundled bun.exe (set TSP_BUN_BIN to override)"),
    })
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
/// Grace window after the watchdog writes ABORT_MARKER before the
/// host hard-kills the child. Long enough for the page to observe
/// ctx.signal and exit cleanly (a few ms in practice), short enough
/// that a runaway page cannot stall the worker thread indefinitely.
const GRACE_AFTER_MARKER: std::time::Duration = std::time::Duration::from_millis(1000);

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
/// `cancellation` is also triggered by a client disconnect;
/// that path uses the same marker and grace period but returns
/// `Cancelled` instead of a timeout error.
pub fn execute(
    bun: &BunRuntime,
    source: &str,
    method: HttpMethod,
    ctx_json: Option<&str>,
    timeout_ms: u64,
    cancellation: &CancellationToken,
) -> Result<String, JscError> {
    execute_inner(bun, source, method, ctx_json, timeout_ms, cancellation, None, None)
}

/// Execute a page while resolving its relative imports against the route's
/// source directory. The wrapper itself remains in the system temp directory,
/// but local dependencies are rewritten to absolute file URLs first.
pub fn execute_from_dir(
    bun: &BunRuntime,
    source: &str,
    method: HttpMethod,
    ctx_json: Option<&str>,
    timeout_ms: u64,
    cancellation: &CancellationToken,
    source_dir: &Path,
) -> Result<String, JscError> {
    execute_inner(
        bun,
        source,
        method,
        ctx_json,
        timeout_ms,
        cancellation,
        Some(source_dir),
        None,
    )
}

/// Execute a page while retaining its original source path in Bun stack
/// traces. The path is emitted as a `tsp://` source URL instead of an opaque
/// temporary `.tsx` filename.
pub fn execute_from_path(
    bun: &BunRuntime,
    source: &str,
    method: HttpMethod,
    ctx_json: Option<&str>,
    timeout_ms: u64,
    cancellation: &CancellationToken,
    source_path: &Path,
) -> Result<String, JscError> {
    let source_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
    execute_inner(
        bun,
        source,
        method,
        ctx_json,
        timeout_ms,
        cancellation,
        Some(source_dir),
        Some(source_path),
    )
}

fn execute_inner(
    bun: &BunRuntime,
    source: &str,
    method: HttpMethod,
    ctx_json: Option<&str>,
    timeout_ms: u64,
    cancellation: &CancellationToken,
    source_dir: Option<&Path>,
    source_path: Option<&Path>,
) -> Result<String, JscError> {
    let source = match source_dir {
        Some(dir) => jsx::rewrite_local_imports(source, dir).map_err(JscError::Jsx)?,
        None => source.to_string(),
    };
    let js_body = jsx::tsx_to_js(&source).map_err(JscError::Jsx)?;
    let mut wrapped = jsx::wrap_for_bun_cli(&js_body, method.as_str(), ctx_json);
    if let Some(path) = source_path {
        let source_url = format!("tsp://{}", path.to_string_lossy().replace('\\', "/"));
        wrapped.push_str(&format!("\n//# sourceURL={}\n", source_url));
    }

    // Use a per-call temp file. On Windows `std::env::temp_dir()` is
    // `%TEMP%`; the unique suffix avoids collisions under concurrent
    // requests.
    let mut tempfile = std::env::temp_dir();
    let suffix = format!(
        "tsp-v2-slice6-{}-{}.tsx",
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
    if let Some(dir) = source_dir {
        // The generated entry file lives in the system temp directory. Tell
        // Bun where application dependencies live and use the route directory
        // as the process cwd so package resolution does not depend on the
        // operator having installed dependencies next to `%TEMP%`.
        let mut node_paths = vec![dir.join("node_modules")];
        if let Some(parent) = dir.parent() {
            node_paths.push(parent.join("node_modules"));
        }
        if let Ok(cwd) = std::env::current_dir() {
            node_paths.push(cwd.join("node_modules"));
        }
        let separator = if cfg!(windows) { ";" } else { ":" };
        cmd.env(
            "NODE_PATH",
            node_paths
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(separator),
        );
        cmd.current_dir(dir);
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
    // 0 = running, 1 = child completed, 2 = timeout won, 3 = client
    // cancellation won. The CAS makes completion cancellation race-safe.
    const WATCHDOG_RUNNING: u8 = 0;
    const CHILD_COMPLETED: u8 = 1;
    const WATCHDOG_TIMED_OUT: u8 = 2;
    const WATCHDOG_CANCELLED: u8 = 3;
    let watchdog_state = Arc::new(AtomicU8::new(WATCHDOG_RUNNING));
    let watchdog = {
        let timed_out = Arc::clone(&timed_out);
        let watchdog_state = Arc::clone(&watchdog_state);
        let cancellation = cancellation.clone();
        let mut stdin = stdin_handle;
        Some(thread::spawn(move || {
            let deadline = (timeout_ms > 0)
                .then(|| std::time::Instant::now() + Duration::from_millis(timeout_ms));
            loop {
                if watchdog_state.load(Ordering::SeqCst) != WATCHDOG_RUNNING {
                    return;
                }
                let reason = if cancellation.is_cancelled() {
                    Some(WATCHDOG_CANCELLED)
                } else if deadline.is_some_and(|deadline| std::time::Instant::now() >= deadline) {
                    Some(WATCHDOG_TIMED_OUT)
                } else {
                    None
                };
                if let Some(reason) = reason {
                    if watchdog_state
                        .compare_exchange(
                            WATCHDOG_RUNNING,
                            reason,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        )
                        .is_ok()
                    {
                        if reason == WATCHDOG_TIMED_OUT {
                            timed_out.store(true, Ordering::SeqCst);
                        }
                        if let Some(ref mut s) = stdin {
                            let _ = s.write_all(ABORT_MARKER);
                            let _ = s.flush();
                        }
                    }
                    return;
                }
                let sleep_for = deadline
                    .map(|deadline| {
                        deadline
                            .saturating_duration_since(std::time::Instant::now())
                            .min(Duration::from_millis(20))
                    })
                    .unwrap_or(Duration::from_millis(20));
                thread::sleep(sleep_for);
            }
        }))
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
                let _ = watchdog_state.compare_exchange(
                    WATCHDOG_RUNNING,
                    CHILD_COMPLETED,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
                final_status = Some(status);
                exited = true;
            }
            None => {
                if timed_out.load(Ordering::SeqCst) || cancellation.is_cancelled() {
                    // The watchdog wrote ABORT_MARKER to the child stdin,
                    // firing ctx.signal inside the page. Give the page a
                    // grace window before hard-killing it. This covers
                    // both timeout and client-disconnect cancellation.
                    let grace_start = std::time::Instant::now();
                    let mut killed = false;
                    loop {
                        if let Some(status) = child.try_wait().map_err(JscError::Spawn)? {
                            final_status = Some(status);
                            exited = true;
                            break;
                        }
                        if grace_start.elapsed() >= GRACE_AFTER_MARKER {
                            // Grace expired; hard-kill so the loop exits.
                            // Best effort -- the kill may fail on Windows
                            // if the process exited but try_wait missed it.
                            killed = true;
                            eprintln!("TSPv2PoC1: timeout grace expired, killing bun subprocess pid={:?}", child.id());
                            let kill_result = child.kill();
                            eprintln!("TSPv2PoC1: child.kill() result = {:?}", kill_result);
                            let _ = child.wait();
                            final_status = None;
                            exited = true;
                            break;
                        }
                        thread::sleep(Duration::from_millis(20));
                    }
                    let _ = killed;
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
    // 16n' diagnosis: keep the generated tempfile for inspection
    // when TSP_KEEP_TEMP=1 (mirrors 16n tracing; not a hot path).
    if std::env::var("TSP_KEEP_TEMP").map(|v| v == "1").unwrap_or(false) {
        eprintln!("TSPv2PoC1: kept tempfile {}", tempfile.display());
    } else {
        let _ = fs::remove_file(&tempfile);
    }

    if cancellation.is_cancelled() {
        return Err(JscError::Cancelled);
    }

    if timed_out.load(Ordering::SeqCst) {
        // Timeout is a hard failure -- a page that ignores
        // the abort signal is a programming error the dev
        // must see. The 500 body should tell them.
        eprintln!("TSPv2PoC1: request timed out after {timeout_ms}ms");
        // Slice 16n': distinguish a page that honored `ctx.signal`
        // (exited cleanly after the abort) from one that had to be
        // hard-killed. When the child exited successfully we surface
        // its real stderr -- a cooperating page typically prints an
        // "aborted" marker there -- so the E2E/dev can confirm the
        // signal actually reached page code before the host declared
        // the timeout.
        let clean_abort = match &final_status {
            Some(st) => st.success(),
            None => false,
        };
        if clean_abort {
            let mut tail_buf: Vec<u8> = stderr.iter().rev().take(1024).copied().collect();
            tail_buf.reverse();
            let tail = String::from_utf8_lossy(&tail_buf).into_owned();
            return Err(JscError::TimedOut {
                stderr_tail: format!(
                    "request timed out after {timeout_ms}ms; page exited cleanly on ctx.signal (stderr follows)\n{tail}"
                ),
            });
        }
        return Err(JscError::TimedOut {
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
        // renumber the prefix (e.g. accidentally reusing
        // the timeout code `TSP3009` for JSX).
        let pairs: &[(JscError, &str)] = &[
            (JscError::BunNotFound { tried: PathBuf::from("x") }, "TSP3010"),
            (
                JscError::BunFailed { code: Some(1), stderr_tail: String::new() },
                "TSP3012",
            ),
            (JscError::TimedOut { stderr_tail: String::new() }, "TSP3009"),
            (JscError::Cancelled, "TSP3015"),
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

    #[test]
    fn normal_page_exits_before_timeout_watchdog() {
        let Ok(bin) = resolve_bun_bin() else {
            // The vendored Bun binary is optional for the library-only test
            // environment; the host's integration test supplies it.
            return;
        };
        let result = execute(
            &BunRuntime { bin },
            "export function GET(ctx) { return 'ok'; }\n",
        HttpMethod::Get,
        Some(r#"{"method":"GET","path":"/","query":"","params":{},"body_b64":"","headers":{}}"#),
        5_000,
        &CancellationToken::new(),
    );
        assert!(result.is_ok(), "normal page must not wait for timeout: {result:?}");
    }

    #[test]
    fn tsp_server_json_helper_executes_through_subprocess_bridge() {
        let Ok(bin) = resolve_bun_bin() else {
            return;
        };
        let result = execute(
            &BunRuntime { bin },
            r#"
import { type Context, json } from "tsp:server";
export async function GET(ctx: Context) {
  return json({ ok: true, method: ctx.method });
}
"#,
            HttpMethod::Get,
            Some(r#"{"method":"GET","path":"/","query":"","params":{},"body_b64":"","headers":{}}"#),
            5_000,
            &CancellationToken::new(),
        );
        let stdout = result.expect("tsp:server json helper should execute");
        assert!(stdout.contains("\"status\":200"), "stdout={stdout}");
        assert!(stdout.contains("\"content-type\",\"application/json; charset=utf-8\""), "stdout={stdout}");
        assert!(stdout.contains("\"ok\":true"), "stdout={stdout}");
        assert!(stdout.contains("\"method\":\"GET\""), "stdout={stdout}");
    }

    #[test]
    fn tsp_server_http_error_becomes_response_envelope() {
        let Ok(bin) = resolve_bun_bin() else {
            return;
        };
        let result = execute(
            &BunRuntime { bin },
            r#"
import { HttpError } from "tsp:server";
export function GET() {
  throw new HttpError(404, "missing");
}
"#,
            HttpMethod::Get,
            None,
            5_000,
            &CancellationToken::new(),
        );
        let stdout = result.expect("HttpError should become a response envelope");
        assert!(stdout.contains("\"status\":404"), "stdout={stdout}");
        assert!(stdout.contains("\"body\":\"missing\""), "stdout={stdout}");
    }

    #[test]
    fn nested_tsx_async_components_render_with_escaping() {
        let Ok(bin) = resolve_bun_bin() else {
            return;
        };
        let result = execute(
            &BunRuntime { bin },
            r#"
function Item({ value }: { value: string }) {
  return <li data-value={value}><span>{value}</span></li>;
}
export async function GET(ctx: Context) {
  return <div className="card"><Item value="<unsafe>" /><>{42}{null}<strong>ok</strong></></div>;
}
"#,
            HttpMethod::Get,
            Some(r#"{"method":"GET","path":"/","query":"","params":{},"body_b64":"","headers":{}}"#),
            5_000,
            &CancellationToken::new(),
        );
        let stdout = result.expect("nested TSX should render");
        assert!(stdout.contains("<div class=\"card\"><li data-value=\"&lt;unsafe&gt;\"><span>&lt;unsafe&gt;</span></li>42<strong>ok</strong></div>"), "stdout={stdout}");
    }

    #[test]
    fn named_fragment_export_renders_when_selected_by_context() {
        let Ok(bin) = resolve_bun_bin() else {
            return;
        };
        let result = execute(
            &BunRuntime { bin },
            r#"
import { type Context, fragment } from "tsp:server";
export const list = fragment(async (ctx: Context) => <ul><li>one</li><li>two</li></ul>);
"#,
            HttpMethod::Get,
            Some(r#"{"method":"GET","path":"/users","query":"","params":{},"body_b64":"","headers":{},"__tsp_fragment":"list"}"#),
            5_000,
            &CancellationToken::new(),
        );
        let stdout = result.expect("selected fragment should render");
        assert!(stdout.contains("<ul><li>one</li><li>two</li></ul>"), "stdout={stdout}");
    }

    #[test]
    fn relative_ts_dependency_executes_from_route_source_directory() {
        let Ok(bin) = resolve_bun_bin() else {
            return;
        };
        let dir = std::env::temp_dir().join(format!("tsp-v2-dep-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("shared.tsx"),
            "export function Shared() { return <em>from-dep</em>; }\n",
        )
        .unwrap();
        let result = execute_from_dir(
            &BunRuntime { bin },
            r#"
import { Shared } from "./shared";
export function GET() { return <p><Shared /></p>; }
"#,
            HttpMethod::Get,
            None,
            5_000,
            &CancellationToken::new(),
            &dir,
        );
        let _ = std::fs::remove_dir_all(&dir);
        let stdout = result.expect("relative TypeScript dependency should execute");
        assert!(stdout.contains("<p><em>from-dep</em></p>"), "stdout={stdout}");
    }

    #[test]
    fn package_dependency_resolves_from_route_source_directory() {
        let Ok(bin) = resolve_bun_bin() else {
            return;
        };
        let dir = std::env::temp_dir().join(format!("tsp-v2-package-dep-{}", std::process::id()));
        let package = dir.join("node_modules/tsp-v2-fixture");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("package.json"), r#"{"name":"tsp-v2-fixture","module":"index.ts"}"#).unwrap();
        std::fs::write(package.join("index.ts"), "export const answer = 42;\n").unwrap();
        let result = execute_from_dir(
            &BunRuntime { bin },
            r#"
import { answer } from "tsp-v2-fixture";
export function GET() { return <p>{answer}</p>; }
"#,
            HttpMethod::Get,
            None,
            5_000,
            &CancellationToken::new(),
            &dir,
        );
        let _ = std::fs::remove_dir_all(&dir);
        let stdout = result.expect("package dependency should resolve");
        assert!(stdout.contains("<p>42</p>"), "stdout={stdout}");
    }

    #[test]
    fn client_cancellation_stops_hanging_page() {
        let Ok(bin) = resolve_bun_bin() else {
            return;
        };
        let cancellation = CancellationToken::new();
        let cancel_later = cancellation.clone();
        let cancel_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(250));
            cancel_later.cancel();
        });
        let result = execute(
            &BunRuntime { bin },
            "export async function GET(ctx) { await new Promise(() => {}); return 'never'; }\n",
            HttpMethod::Get,
            Some(r#"{"method":"GET","path":"/slow","query":"","params":{},"body_b64":"","headers":{}}"#),
            5_000,
            &cancellation,
        );
        cancel_thread.join().expect("cancellation thread");
        assert!(matches!(result, Err(JscError::Cancelled)), "result={result:?}");
    }
}
