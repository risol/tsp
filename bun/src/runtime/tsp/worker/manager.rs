//! Master-side Worker Manager for the embedded-worker process boundary.
//!
//! This manager starts a declared worker executable. It deliberately has no
//! Bun path, Bun environment, or JSC dependency: only the worker executable
//! is allowed to contain and initialize Bun.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use super::protocol::{ExecuteRequest, ExecuteResponse, ProtocolError};
use super::lifecycle::{self, RecyclePolicy};
use super::sandbox::ResourceLimits;
use super::sandbox::CgroupHandle;
use super::protocol::Message;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub enum ManagerError {
    Io(std::io::Error),
    Protocol(ProtocolError),
    WorkerNotReady,
    WorkerExited,
    WorkerTimeout,
    ResourceIsolation(String),
    UnsupportedPlatform,
}

impl std::fmt::Display for ManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "worker manager I/O failed: {error}"),
            Self::Protocol(error) => write!(f, "worker manager protocol failed: {error}"),
            Self::WorkerNotReady => write!(f, "worker did not become ready"),
            Self::WorkerExited => write!(f, "worker exited before responding"),
            Self::WorkerTimeout => write!(f, "worker did not respond before the deadline"),
            Self::ResourceIsolation(error) => write!(f, "worker resource isolation failed: {error}"),
            Self::UnsupportedPlatform => {
                write!(f, "worker IPC transport is not available on this platform")
            }
        }
    }
}

impl std::error::Error for ManagerError {}

impl From<std::io::Error> for ManagerError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ProtocolError> for ManagerError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

#[cfg(unix)]
type WorkerStream = std::os::unix::net::UnixStream;

#[cfg(not(unix))]
type WorkerStream = std::net::TcpStream;

struct WorkerHandle {
    child: std::process::Child,
    stream: WorkerStream,
    started_at: Instant,
    completed_requests: u64,
    _cgroup: Option<CgroupHandle>,
}

pub struct WorkerManager {
    worker_binary: PathBuf,
    socket_path: PathBuf,
    next_request_id: AtomicU64,
    worker: Option<WorkerHandle>,
    resource_limits: ResourceLimits,
}

impl std::fmt::Debug for WorkerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerManager")
            .field("worker_binary", &self.worker_binary)
            .field("socket_path", &self.socket_path)
            .field("next_request_id", &self.next_request_id.load(Ordering::Relaxed))
            .field("running", &self.worker.is_some())
            .field("resource_limits", &self.resource_limits)
            .finish()
    }
}

impl WorkerManager {
    pub fn new(worker_binary: PathBuf, socket_path: PathBuf) -> Self {
        Self {
            worker_binary,
            socket_path,
            next_request_id: AtomicU64::new(1),
            worker: None,
            resource_limits: ResourceLimits::disabled(),
        }
    }

    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_limits = limits;
        self
    }

    pub fn set_resource_limits(&mut self, limits: ResourceLimits) {
        self.resource_limits = limits;
    }

    pub fn is_running(&self) -> bool {
        self.worker.is_some()
    }

    /// Start one worker and complete the protocol handshake.
    ///
    /// The executable receives only the socket endpoint. In particular, this
    /// method never resolves a Bun executable or starts a Bun child process.
    pub fn start_worker(&mut self) -> Result<(), ManagerError> {
        if self.worker.is_some() {
            return Ok(());
        }
        self.start_worker_inner()
    }

    #[cfg(unix)]
    fn start_worker_inner(&mut self) -> Result<(), ManagerError> {
        use std::fs;
        use std::os::unix::net::UnixListener;
        use std::process::Stdio;

        let _ = fs::remove_file(&self.socket_path);
        let listener = UnixListener::bind(&self.socket_path)?;
        listener.set_nonblocking(true)?;
        let mut child = std::process::Command::new(&self.worker_binary)
            .arg("--tsp-worker")
            .env("TSP_WORKER_SOCKET", &self.socket_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()?;

        let deadline = Instant::now() + STARTUP_TIMEOUT;
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(false)?;
                    break stream;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        let _ = fs::remove_file(&self.socket_path);
                        return Err(ManagerError::WorkerNotReady);
                    }
                    if let Some(status) = child.try_wait()? {
                        let _ = fs::remove_file(&self.socket_path);
                        return Err(if status.success() {
                            ManagerError::WorkerNotReady
                        } else {
                            ManagerError::WorkerExited
                        });
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_file(&self.socket_path);
                    return Err(ManagerError::Io(error));
                }
            }
        };
        let cgroup = match CgroupHandle::attach(child.id(), &self.resource_limits) {
            Ok(cgroup) => cgroup,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_file(&self.socket_path);
                return Err(ManagerError::ResourceIsolation(error.to_string()));
            }
        };
        let mut handle = WorkerHandle {
            child,
            stream,
            started_at: Instant::now(),
            completed_requests: 0,
            _cgroup: cgroup,
        };
        if let Err(error) = Message::Hello.write_to(&mut handle.stream) {
            let _ = handle.child.kill();
            let _ = handle.child.wait();
            let _ = fs::remove_file(&self.socket_path);
            return Err(error.into());
        }
        let ready = match Message::read_from(&mut handle.stream) {
            Ok(ready) => ready,
            Err(error) => {
                let _ = handle.child.kill();
                let _ = handle.child.wait();
                let _ = fs::remove_file(&self.socket_path);
                return Err(error.into());
            }
        };
        if !matches!(ready, Message::Ready { .. }) {
            let _ = handle.child.kill();
            let _ = handle.child.wait();
            let _ = fs::remove_file(&self.socket_path);
            return Err(ManagerError::WorkerNotReady);
        }
        self.worker = Some(handle);
        Ok(())
    }

    #[cfg(not(unix))]
    fn start_worker_inner(&mut self) -> Result<(), ManagerError> {
            use std::net::TcpListener;
            use std::process::Stdio;

            let listener = TcpListener::bind("127.0.0.1:0")?;
            listener.set_nonblocking(true)?;
            let endpoint = listener.local_addr()?.to_string();
            let mut child = std::process::Command::new(&self.worker_binary)
                .arg("--tsp-worker")
                .env("TSP_WORKER_SOCKET", endpoint)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()?;
            let deadline = Instant::now() + STARTUP_TIMEOUT;
            let stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(false)?;
                        break stream;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if Instant::now() >= deadline {
                            let _ = child.kill();
                            let _ = child.wait();
                            return Err(ManagerError::WorkerNotReady);
                        }
                        if let Some(status) = child.try_wait()? {
                            return Err(if status.success() {
                                ManagerError::WorkerNotReady
                            } else {
                                ManagerError::WorkerExited
                            });
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(ManagerError::Io(error));
                    }
                }
            };
            let cgroup = match CgroupHandle::attach(child.id(), &self.resource_limits) {
                Ok(cgroup) => cgroup,
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ManagerError::ResourceIsolation(error.to_string()));
                }
            };
            let mut handle = WorkerHandle {
                child,
                stream,
                started_at: Instant::now(),
                completed_requests: 0,
                _cgroup: cgroup,
            };
            if let Err(error) = Message::Hello.write_to(&mut handle.stream) {
                let _ = handle.child.kill();
                let _ = handle.child.wait();
                return Err(error.into());
            }
            let ready = match Message::read_from(&mut handle.stream) {
                Ok(ready) => ready,
                Err(error) => {
                    let _ = handle.child.kill();
                    let _ = handle.child.wait();
                    return Err(error.into());
                }
            };
            if !matches!(ready, Message::Ready { .. }) {
                let _ = handle.child.kill();
                let _ = handle.child.wait();
                return Err(ManagerError::WorkerNotReady);
            }
            self.worker = Some(handle);
            Ok(())
    }

    pub fn execute(&mut self, request: ExecuteRequest) -> Result<ExecuteResponse, ManagerError> {
        if self.worker.is_none() {
            self.start_worker()?;
        }
        // Clone once so the retry-after-restart path can re-send the same
        // request against the replacement worker. The clone cost is
        // bounded by the request size (string + script bytes) and only
        // paid once per call; the alternative -- restructuring
        // `execute_inner` to take a reference -- would ripple through
        // every protocol::Message::Execute write site.
        let first = self.execute_inner(request.clone());
        if first.as_ref().is_err_and(|error| Self::should_restart(error))
            && self.restart_worker().is_ok()
        {
            // The worker died between requests (e.g. SIGKILL landed on
            // the previous process). The replacement is alive and ready;
            // re-issue the original request so the caller sees a
            // transparent recovery rather than the underlying
            // BrokenPipe / Io error.
            return self.execute_inner(request);
        }
        first
    }

    /// Execute with a hard response deadline. A timed-out worker is never
    /// reused: the manager gives it a short cancellation window, then
    /// replaces the whole process so a stuck VM cannot poison later requests.
    pub fn execute_with_timeout(
        &mut self,
        request: ExecuteRequest,
        timeout_ms: u64,
    ) -> Result<ExecuteResponse, ManagerError> {
        let timeout_ms = effective_timeout_ms(request.deadline_ms, timeout_ms);
        if request.deadline_ms != 0 && timeout_ms == 0 {
            let _ = self.restart_worker();
            return Err(ManagerError::WorkerTimeout);
        }
        if timeout_ms == 0 {
            return self.execute(request);
        }
        if self.worker.is_none() {
            self.start_worker()?;
        }

        // A protocol disconnect means the embedded worker died while handling
        // this request. Restarting the worker without re-sending the request
        // turns a recoverable worker crash into a user-visible 500 response.
        // Keep one copy so the request can be retried against the replacement.
        let can_retry = Self::can_retry_request(&request);
        let retry_request = request.clone();
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let result = self.execute_with_timeout_once(request, timeout_ms);
        if result.is_ok() || matches!(&result, Err(ManagerError::WorkerTimeout)) {
            return result;
        }
        if can_retry
            && result.as_ref().is_err_and(|error| Self::should_restart(error))
            && self.restart_worker().is_ok()
        {
            let remaining_timeout_ms = deadline
                .saturating_duration_since(Instant::now())
                .as_millis() as u64;
            if remaining_timeout_ms == 0 {
                return Err(ManagerError::WorkerTimeout);
            }
            let retry_result =
                self.execute_with_timeout_once(retry_request, remaining_timeout_ms);
            if retry_result
                .as_ref()
                .is_err_and(|error| Self::should_restart(error))
            {
                let _ = self.restart_worker();
            }
            return retry_result;
        }
        result
    }

    fn execute_with_timeout_once(
        &mut self,
        request: ExecuteRequest,
        timeout_ms: u64,
    ) -> Result<ExecuteResponse, ManagerError> {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let result = {
            let handle = self.worker.as_mut().ok_or(ManagerError::WorkerExited)?;
            handle.stream.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;
            let result = (|| {
                Message::Execute { id, request }.write_to(&mut handle.stream)?;
                match Message::read_from(&mut handle.stream)? {
                    Message::Response { id: response_id, response } if response_id == id => {
                        handle.completed_requests = handle.completed_requests.saturating_add(1);
                        Ok(response)
                    }
                    Message::Error { id: error_id, code, message } if error_id == id => {
                        Err(ManagerError::Protocol(ProtocolError::RemoteError(
                            format!("{code}: {message}"),
                        )))
                    }
                    Message::Shutdown => Err(ManagerError::WorkerExited),
                    _ => Err(ManagerError::Protocol(ProtocolError::InvalidField(
                        "unexpected worker response",
                    ))),
                }
            })();
            let _ = handle.stream.set_read_timeout(None);
            result
        };
        if matches!(
            &result,
            Err(ManagerError::Protocol(ProtocolError::Io(error)))
                if matches!(error.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock)
        ) {
            if let Some(handle) = self.worker.as_mut() {
                let _ = Message::Cancel { id }.write_to(&mut handle.stream);
            }
            std::thread::sleep(Duration::from_millis(100));
            let _ = self.restart_worker();
            return Err(ManagerError::WorkerTimeout);
        }
        result
    }

    pub fn health_check(&mut self) -> Result<(), ManagerError> {
        if self.worker.is_none() {
                self.start_worker()?;
        }
            let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
            let result = {
                let handle = self.worker.as_mut().ok_or(ManagerError::WorkerExited)?;
                let result = (|| {
                    handle.stream.set_read_timeout(Some(HEALTH_CHECK_TIMEOUT))?;
                    Message::Heartbeat { id }.write_to(&mut handle.stream)?;
                    match Message::read_from(&mut handle.stream)? {
                        Message::Heartbeat { id: response_id } if response_id == id => Ok(()),
                        _ => Err(ManagerError::Protocol(ProtocolError::InvalidField(
                            "unexpected heartbeat response",
                        ))),
                    }
                })();
                let _ = handle.stream.set_read_timeout(None);
                result
            };
            if result.as_ref().is_err_and(|error| Self::should_restart(error)) {
                let _ = self.restart_worker();
            }
        result
    }

    pub fn restart_worker(&mut self) -> Result<(), ManagerError> {
        self.stop_worker()?;
        self.start_worker()
    }

    fn execute_inner(&mut self, request: ExecuteRequest) -> Result<ExecuteResponse, ManagerError> {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let handle = self.worker.as_mut().ok_or(ManagerError::WorkerExited)?;
        let result = (|| {
            Message::Execute { id, request }.write_to(&mut handle.stream)?;
            match Message::read_from(&mut handle.stream)? {
                Message::Response { id: response_id, response } if response_id == id => {
                    handle.completed_requests = handle.completed_requests.saturating_add(1);
                    Ok(response)
                }
                Message::Error { id: error_id, code, message } if error_id == id => {
                    Err(ManagerError::Protocol(ProtocolError::RemoteError(
                        format!("{code}: {message}"),
                    )))
                }
                Message::Shutdown => Err(ManagerError::WorkerExited),
                _ => Err(ManagerError::Protocol(ProtocolError::InvalidField(
                    "unexpected worker response",
                ))),
            }
        })();
        result
    }

    pub fn cancel(&mut self, id: u64) -> Result<(), ManagerError> {
        let handle = self.worker.as_mut().ok_or(ManagerError::WorkerExited)?;
        Message::Cancel { id }.write_to(&mut handle.stream)?;
        Ok(())
    }

    pub fn stop_worker(&mut self) -> Result<(), ManagerError> {
        if let Some(mut handle) = self.worker.take() {
            let _ = Message::Shutdown.write_to(&mut handle.stream);
            let _ = handle.child.kill();
            let _ = handle.child.wait();
            let _ = std::fs::remove_file(&self.socket_path);
        }
        Ok(())
    }

    fn should_restart(error: &ManagerError) -> bool {
        !matches!(
            error,
            ManagerError::UnsupportedPlatform
                | ManagerError::Protocol(ProtocolError::RemoteError(_))
        )
    }

    // A disconnected response does not tell us whether the worker applied a
    // request's side effects before it died. Only automatically retry methods
    // whose HTTP semantics are safe when the caller did not provide an
    // application-level idempotency key.
    fn can_retry_request(request: &ExecuteRequest) -> bool {
        matches!(
            request.method.as_str(),
            "GET" | "HEAD" | "OPTIONS" | "TRACE"
        )
    }

    pub fn stats(&mut self) -> Option<(Duration, u64, u32)> {
        let handle = self.worker.as_mut()?;
        Some((handle.started_at.elapsed(), handle.completed_requests, handle.child.id()))
    }

    pub fn should_recycle(&mut self, policy: &RecyclePolicy) -> bool {
        let Some((uptime, completed_requests, pid)) = self.stats() else {
            return false;
        };
        lifecycle::should_recycle(
            policy,
            uptime,
            completed_requests,
            lifecycle::resident_memory_bytes(pid),
        )
    }

}

fn effective_timeout_ms(deadline_ms: u64, configured_timeout_ms: u64) -> u64 {
    if deadline_ms == 0 {
        return configured_timeout_ms;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(u64::MAX);
    let remaining = deadline_ms.saturating_sub(now);
    if configured_timeout_ms == 0 {
        remaining
    } else {
        remaining.min(configured_timeout_ms)
    }
}

impl Drop for WorkerManager {
    fn drop(&mut self) {
        let _ = self.stop_worker();
    }
}
