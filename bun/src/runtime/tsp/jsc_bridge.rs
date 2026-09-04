//! JSC execution bridge for TSP PoC 1 slice 6.
//!
//! See `tsp-plan.md` sect.25.3 ("JSC 是执行引擎"). The production bridge
//! uses the Bun-linked worker pool: each worker owns one JSC VM and evaluates
//! the generated wrapper through the native transpiler and request protocol.
//! The packaged TSP executable is self-contained and does not resolve a
//! second Bun binary from PATH.
//!
//! The production bridge uses the Bun-linked embedded worker pool. Each
//! worker owns one JSC VM and evaluates the generated wrapper through the
//! native transpiler and request protocol; no external JavaScript runtime is
//! required.
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::jsx;
use crate::router::HttpMethod;
use crate::worker::application::ApplicationRegistry;
use crate::worker::manager::ManagerError;
use crate::worker::pool::{PoolError, WorkerPool};
use crate::worker::protocol::ExecuteRequest;

/// embedded-worker self-spawn runtime handle. The master holds the pool; each pool
/// slot owns a self-spawned `tspserver[.exe]` worker process (see
/// `worker/manager.rs` and `worker/pool.rs`). The `bin` field is the
/// path the master itself was launched from — workers reuse the same
/// executable and dispatch on `--tsp-worker`.
#[derive(Debug)]
pub struct BunRuntime {
    pub bin: PathBuf,
    pub embedded_pool: Option<Arc<WorkerPool>>,
}

static EXECUTION_GENERATION: AtomicU64 = AtomicU64::new(1);
static LAST_WORKER_RESTART_GENERATION: AtomicU64 = AtomicU64::new(0);
static WORKER_RESTART_LOCK: Mutex<()> = Mutex::new(());

pub fn bump_execution_generation() -> u64 {
    EXECUTION_GENERATION
        .fetch_add(1, Ordering::AcqRel)
        .saturating_add(1)
}

fn execution_generation() -> u64 {
    EXECUTION_GENERATION.load(Ordering::Acquire)
}

fn restart_workers_after_reload(pool: &WorkerPool) -> Result<(), JscError> {
    let generation = execution_generation();
    if LAST_WORKER_RESTART_GENERATION.load(Ordering::Acquire) == generation {
        return Ok(());
    }
    let _guard = WORKER_RESTART_LOCK.lock().map_err(|e| {
        JscError::Spawn(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    })?;
    if LAST_WORKER_RESTART_GENERATION.load(Ordering::Acquire) != generation {
        pool.restart_all().map_err(map_pool_error)?;
        LAST_WORKER_RESTART_GENERATION.store(generation, Ordering::Release);
    }
    Ok(())
}


/// One-shot cancellation shared by the host connection and the worker
/// watchdog. A cancelled request must never write a response.
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
    /// Starting or communicating with the worker failed (permission, etc.).
    Spawn(io::Error),
    /// bun exited non-zero. We surface the stderr tail (truncated to
    /// 1 KiB) so a JS error from `pages/index.tsp` shows up in the
    /// 500 page without overflowing the response body.
    BunFailed {
        code: Option<i32>,
        stderr_tail: String,
    },
    /// The client disconnected while the page was executing.
    Cancelled,
    /// The request deadline expired. The stderr tail preserves the
    /// page-side abort evidence when the handler cooperated.
    TimedOut { stderr_tail: String },
    /// A worker completed without producing a response envelope.
    EmptyStdout,
    /// I/O error from materializing a worker request.
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
                write!(
                    f,
                    "bun.exe not found at {} (set TSP_BUN_BIN to override)",
                    tried.display()
                )
            }
            Self::Spawn(e) => write!(f, "spawn bun failed: {e}"),
            Self::BunFailed { code, stderr_tail } => {
                let code = code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "<signal>".into());
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
    /// the failure phase (jsx transform / worker / empty response).
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
            Self::Spawn(_) => "embedded worker failed",
            Self::BunFailed { .. } => "embedded worker execution failed",
            Self::Cancelled => "request cancelled",
            Self::TimedOut { .. } => "request timed out",
            Self::EmptyStdout => "bun produced no stdout",
            Self::WriteTemp(_) => "writing bun temp file failed",
            Self::Jsx(_) => "jsx transform error",
        }
    }
}



/// Execute the page's `method` handler. Reads the .tsp source, transforms it
/// through TSP's TSX pipeline, and sends the generated wrapper to an embedded
/// Bun worker over the native protocol.
///
/// `timeout_ms` is the request timeout (spec sect.13.7).
/// `0` disables the watchdog. The worker pool owns the hard deadline and
/// recycles a worker when a route does not finish.
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
    execute_inner(
        bun,
        source,
        method,
        ctx_json,
        timeout_ms,
        cancellation,
        None,
        None,
        None,
        None,
    )
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
        None,
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
        None,
        None,
    )
}

/// Execute a page through the Master -> Worker IPC path while explicitly
/// carrying the original HTTP headers and body in the request frame.
pub fn execute_from_path_with_request(
    bun: &BunRuntime,
    source: &str,
    method: HttpMethod,
    ctx_json: Option<&str>,
    timeout_ms: u64,
    cancellation: &CancellationToken,
    source_path: &Path,
    headers: &[(String, String)],
    body: &[u8],
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
        Some(headers),
        Some(body),
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
    request_headers: Option<&[(String, String)]>,
    request_body: Option<&[u8]>,
) -> Result<String, JscError> {
    // Every request goes through the WorkerPool backed by self-spawned
    // `tspserver[.exe]` workers. The wrapper runs inside the worker's embedded
    // Bun VM and returns through the master-to-worker IPC channel (see
    // `worker/manager.rs::WorkerManager::spawn`).
    let graph_temp = match source_dir {
        Some(dir) if cfg!(windows) => {
            let (source, temp) = jsx::prepare_cli_module_graph(source, dir, execution_generation())
                .map_err(JscError::Jsx)?;
            (source, Some(temp))
        }
        Some(dir) => (
            jsx::rewrite_local_imports_for_generation(source, dir, execution_generation())
                .map_err(JscError::Jsx)?,
            None,
        ),
        None => (source.to_string(), None),
    };
    let source = graph_temp.0;
    let js_body = jsx::tsx_to_js(&source).map_err(JscError::Jsx)?;
    let mut wrapped = jsx::wrap_for_embedded_worker(&js_body, method.as_str(), ctx_json);
    if let Some(path) = source_path {
        let source_url = format!(
            "tsp://{}?generation={}",
            path.to_string_lossy().replace('\\', "/"),
            execution_generation()
        );
        wrapped.push_str(&format!("
//# sourceURL={}
", source_url));
    }
    let script = wrapped.into_bytes();

    let application_name = std::env::var("TSP_APPLICATION_NAME")
        .unwrap_or_else(|_| "main".into());
    let registered_application = ApplicationRegistry::global().get(&application_name);
    let registered_pool = registered_application
        .as_ref()
        .map(|application| application.workers().pool());
    let pool = registered_pool
        .or(bun.embedded_pool.as_ref())
        .ok_or_else(|| JscError::Spawn(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no embedded worker pool available; the host must start one before serving requests",
        )))?;
    restart_workers_after_reload(pool)?;
    let request = ExecuteRequest {
        application: application_name,
        method: method.as_str().to_string(),
        path: source_path
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default(),
        deadline_ms: request_deadline_ms(timeout_ms),
        headers: request_headers.unwrap_or(&[]).to_vec(),
        body: request_body.unwrap_or(&[]).to_vec(),
        script,
        context_json: ctx_json.unwrap_or_default().to_string(),
    };
    let result = pool.execute(request, timeout_ms).map_err(map_pool_error)?;
    if let Some(temp) = graph_temp.1 {
        let _ = std::fs::remove_dir_all(temp);
    }
    if cancellation.is_cancelled() {
        return Err(JscError::Cancelled);
    }
    let envelope = String::from_utf8(result.body)
        .map_err(|_| JscError::Spawn(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "embedded worker response was not UTF-8",
        )))?;
    Ok(format!("__TSP_OUT_V1__
{envelope}"))
}

fn request_deadline_ms(timeout_ms: u64) -> u64 {
    if timeout_ms == 0 {
        return 0;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|now| now.as_millis().checked_add(u128::from(timeout_ms)))
        .and_then(|deadline| u64::try_from(deadline).ok())
        .unwrap_or(u64::MAX)
}

fn map_pool_error(error: PoolError) -> JscError {
    match error {
        PoolError::Manager(manager) => match manager {
            ManagerError::Io(io) => JscError::Spawn(io),
            ManagerError::Protocol(_) | ManagerError::ResourceIsolation(_) => {
                JscError::Spawn(std::io::Error::other(manager.to_string()))
            }
            ManagerError::WorkerNotReady
            | ManagerError::WorkerExited
            | ManagerError::UnsupportedPlatform => JscError::Spawn(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                manager.to_string(),
            )),
            ManagerError::WorkerTimeout => JscError::TimedOut {
                stderr_tail: "embedded worker deadline expired; worker was restarted".into(),
            },
        },
        PoolError::Backpressure => JscError::Spawn(std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "embedded worker pool is full",
        )),
        PoolError::NoWorkers => JscError::Spawn(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "embedded worker pool has no live workers",
        )),
        PoolError::Poisoned => JscError::Spawn(std::io::Error::new(
            std::io::ErrorKind::Other,
            "worker pool mutex poisoned",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsc_error_codes_are_stable() {
        // Pin the embedded-worker self-spawn-only error code table. A refactor must
        // not silently renumber the prefix (e.g. accidentally reusing
        // `TSP3009` for JSX).
        let pairs: &[(JscError, &str)] = &[
            (JscError::BunNotFound { tried: PathBuf::from("x") }, "TSP3010"),
            (JscError::Cancelled, "TSP3015"),
            (JscError::TimedOut { stderr_tail: String::new() }, "TSP3009"),
            (JscError::EmptyStdout, "TSP3013"),
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
}
