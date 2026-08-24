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
use std::path::PathBuf;
use std::process::Command;

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

/// Execute the page's `method` handler. Reads the .tsp source
/// (already prepared in slice 5 as `page::PageSource`), transforms
/// it via `jsx::tsx_to_js`, writes the result to a temp `.js` file,
/// spawns `bun run <tempfile>`, and returns the captured stdout.
pub fn execute(
    bun: &BunRuntime,
    source: &str,
    method: HttpMethod,
    ctx_json: Option<&str>,
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
        // debug) can read it.
        cmd.env("TSP_CONTEXT_JSON", json);
    }
    let output = cmd.output().map_err(JscError::Spawn)?;

    // Best-effort cleanup. Don't propagate a cleanup failure -- the
    // request has already succeeded or failed on its own merits.
    let _ = fs::remove_file(&tempfile);

    if !output.status.success() {
        // Cap stderr tail at 1 KiB so a JS error with a giant stack
        // does not blow the 500 page into megabytes. Collect into a
        // Vec<u8> first (so `from_utf8_lossy` takes a `&[u8]`), then
        // reverse to restore the chronological order we truncated.
        let mut tail_buf: Vec<u8> = output
            .stderr
            .iter()
            .rev()
            .take(1024)
            .copied()
            .collect();
        tail_buf.reverse();
        let tail = String::from_utf8_lossy(&tail_buf).into_owned();
        return Err(JscError::BunFailed {
            code: output.status.code(),
            stderr_tail: tail,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
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
}