//! Phase 8 slice 16j: ServiceRegistry infrastructure.
//!
//! This module is the host-side home for TSP services (spec
//! sect.17, plan sect.61 Phase 8). It is deliberately small:
//! a registry that owns runtime-scoped service instances, a
//! `Service` trait describing how a service exposes itself to
//! pages, and the first built-in (`logger`).
//!
//! Lifetime model (spec sect.17.1):
//! - Runtime-scoped services are registered once at host boot
//!   and live for the whole process. They are NOT owned by a
//!   page generation, so a generation reload (watcher swap)
//!   or an old-generation release never tears them down --
//!   the "reload 页面后 service 不重建 / old generation
//!   release 不关闭 service" acceptance in plan sect.61.
//! - Request-scoped services are created per request and
//!   dropped with it; 16j surfaces the snapshot slot in the
//!   registry API but the host has no built-in request-scoped
//!   service yet.
//!
//! Wire model (spec sect.17.3): each request's Context carries
//! a *snapshot* of `ctx.services` (name -> JSON descriptor).
//! The wrap preamble hydrates the descriptor into an adapter
//! object. The JS wrapper object is recreated on every request
//! (each request is a fresh bun subprocess), so application
//! code MUST NOT rely on wrapper identity across requests.
//!
//! Back-channel: state-bearing host services cannot be called
//! synchronously while the page runs in a throwaway subprocess.
//! 16j defines the log-line back-channel: `ctx.services.X.info
//! (...)` calls buffer into the envelope's `service_logs` field;
//! the host flushes those lines into the owning service after
//! the envelope returns. Persistent JS adapter realms / session
//! stores build on this registry in later Phase 8 slices.

use std::any::Any;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::session_backend::SessionBackend;

/// Built-in runtime-scoped logger service name (spec sect.21
/// logger surface; plan sect.61 Phase 8 `logger service`).
pub const BUILTIN_LOGGER: &str = "logger";
/// Built-in runtime-scoped session service name (spec
/// sect.16, plan sect.61 Phase 8 `memory session`).
pub const BUILTIN_SESSION: &str = "session";
/// Built-in runtime-scoped time service name (16m).
/// The page reads `ctx.services.time.iso` /
/// `ctx.services.time.uptime_ms`; the host captures a
/// fresh snapshot per request so the values are never
/// stale even when the generation cache bypasses.
pub const BUILTIN_TIME: &str = "time";

/// Maximum number of live sessions kept in the in-memory
/// store (spec sect.16.2: the store MUST survive page
/// reloads, so it accumulates; 16k caps the cap to keep
/// the dev host bounded). Replaces the oldest entry on
/// overflow. Redis-backed session later raises / removes
/// this cap.
pub const SESSION_STORE_CAP_DEFAULT: usize = 10_000;

/// Where a service lives, per spec sect.17.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceScope {
    /// Survives page generation replacement; owned by the host.
    Runtime,
    /// Must not escape the request that created it.
    Request,
}

impl ServiceScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceScope::Runtime => "runtime",
            ServiceScope::Request => "request",
        }
    }
}

/// A log line a page emitted through `ctx.services.<name>`.
/// Carried from the bun subprocess to the host in the
/// envelope's `service_logs` field (wire shape:
/// `{svc, level, message}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// Service name the page called (matches a registry name).
    pub service: String,
    /// Log level: `debug` / `info` / `warn` / `error`.
    pub level: String,
    /// Joined message the page passed.
    pub message: String,
}

/// A host-owned service. Implementors are Send + Sync so the
/// registry can share them across connection threads.
///
/// `as_any` enables downcasting a registered service to its
/// concrete type (e.g. the log-line flush path needs to find
/// the `LoggerService` under a name). Every implementor
/// returns `self`.
pub trait Service: Send + Sync + Any {
    fn name(&self) -> &str;
    fn scope(&self) -> ServiceScope;
    /// Whether this service's visible state changes between
    /// requests. When any registered runtime service reports
    /// `true`, the host bypasses the generation cache for
    /// every render (a page may read live service state, so
    /// cached output would replay a stale snapshot). The
    /// logger's `total_lines` counter is the canonical case.
    /// Defaults to `false` for stateless services.
    fn is_request_varying(&self) -> bool {
        false
    }
    /// Full JSON object (WITH braces) the wrap preamble
    /// hydrates into `ctx.services.<name>`. The `kind` key
    /// selects the JS adapter (`logger` -> log methods;
    /// anything else surfaces read-only).
    fn describe_json(&self) -> String;
    fn as_any(&self) -> &dyn Any;
}

/// Host-owned registry of runtime-scoped services. Created at
/// host boot (`ServiceRegistry::with_defaults`), `Box::leak`ed
/// into a `&'static` like the PageRegistry, and shared by every
/// connection thread. Never owned by a generation.
pub struct ServiceRegistry {
    runtime: BTreeMap<String, Arc<dyn Service>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        ServiceRegistry {
            runtime: BTreeMap::new(),
        }
    }

    /// Built-in service set. 16j ships the logger (in-memory
    /// sink; file / rotation backends are a later slice).
    /// 16k ships the in-memory session store; 16l
    /// re-uses the same default for the dev / single-host
    /// path. Production boots that want a Redis store
    /// use [`with_backends`] instead.
    pub fn with_defaults() -> Self {
        let mut reg = ServiceRegistry::new();
        reg.register(Arc::new(LoggerService::new()));
        reg.register(Arc::new(SessionService::new(SESSION_STORE_CAP_DEFAULT)));
        reg.register(Arc::new(TimeService::new()));
        reg
    }

    /// 16l: build a registry with an explicit session
    /// backend. The logger is the same default sink; the
    /// session service is the only thing that varies.
    /// The bin uses this with `MemoryBackend` /
    /// `RedisBackend` (driven by `TSP_REDIS_URL`).
    pub fn with_backends(session_backend: Arc<dyn SessionBackend>) -> Self {
        let mut reg = ServiceRegistry::new();
        reg.register(Arc::new(LoggerService::new()));
        reg.register(Arc::new(SessionService::with_backend(session_backend)));
        reg.register(Arc::new(TimeService::new()));
        reg
    }

    /// Register a runtime-scoped service. Duplicate names are
    /// replaced (last registration wins) -- registration only
    /// happens at host boot, so this is a boot-time contract,
    /// not a runtime race.
    pub fn register(&mut self, svc: Arc<dyn Service>) {
        let name = svc.name().to_string();
        self.runtime.insert(name, svc);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Service>> {
        self.runtime.get(name).cloned()
    }

    /// Any registered runtime service reports
    /// `is_request_varying()`. See the trait doc for why the
    /// host needs this.
    pub fn any_request_varying(&self) -> bool {
        self.runtime.values().any(|s| s.is_request_varying())
    }

    /// Per-request snapshot in `(name, full JSON object)` form.
    /// Runtime services come first, sorted by name (BTreeMap),
    /// then request-scoped descriptors appended in caller
    /// order. The host embeds this into the wire Context
    /// (`ctx.services`).
    pub fn snapshot(&self, request_scoped: &[(String, String)]) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .runtime
            .values()
            .map(|s| (s.name().to_string(), s.describe_json()))
            .collect();
        out.extend(request_scoped.iter().cloned());
        out
    }

    /// Flush envelope `service_logs` into the owning services.
    /// Returns how many lines were accepted. Lines for unknown
    /// services or non-log services are dropped with a host
    /// diagnostic (the dev sees the typo, the response is
    /// unaffected).
    pub fn flush_log_lines(&self, lines: &[LogLine]) -> usize {
        let mut forwarded = 0usize;
        for line in lines {
            match self.get(&line.service) {
                Some(svc) => match svc.as_any().downcast_ref::<LoggerService>() {
                    Some(logger) => {
                        logger.write_line(&line.level, &line.message);
                        forwarded += 1;
                    }
                    None => eprintln!(
                        "TSPv2PoC1: service '{}' is not a log sink; dropping line",
                        line.service
                    ),
                },
                None => eprintln!(
                    "TSPv2PoC1: log line for unknown service '{}' dropped",
                    line.service
                ),
            }
        }
        forwarded
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        ServiceRegistry::new()
    }
}

/// Built-in `logger` service (plan sect.61 Phase 8). Host-owned
/// and runtime-scoped: the sink (an in-memory ring buffer in
/// 16j; file / rotation backends later) and the `total_lines`
/// counter survive every page generation reload.
///
/// Pages reach it via `ctx.services.logger`:
/// `info/warn/error/debug(...)` buffer a line into the
/// envelope; the host flushes it here after the page returns
/// (see `ServiceRegistry::flush_log_lines`).
pub struct LoggerService {
    sink: Mutex<Vec<String>>,
    total_lines: AtomicU64,
    max_buffered: usize,
}

impl LoggerService {
    pub fn new() -> Self {
        LoggerService {
            sink: Mutex::new(Vec::new()),
            total_lines: AtomicU64::new(0),
            max_buffered: 1000,
        }
    }

    /// Total lines accepted since host boot (never reset by
    /// generation reloads). Exposed to pages via the
    /// descriptor snapshot so an E2E can observe state
    /// continuity across requests / reloads.
    pub fn total_lines(&self) -> u64 {
        self.total_lines.load(Ordering::Relaxed)
    }

    /// The most recent buffered lines, oldest first. Capped at
    /// `max_buffered`.
    pub fn recent_lines(&self, n: usize) -> Vec<String> {
        let sink = self.sink.lock().unwrap();
        if sink.len() <= n {
            sink.clone()
        } else {
            sink[sink.len() - n..].to_vec()
        }
    }

    /// Accept one line from the envelope back-channel.
    pub fn write_line(&self, level: &str, message: &str) {
        let line = format!("[{level}] {message}");
        let mut sink = self.sink.lock().unwrap();
        sink.push(line);
        while sink.len() > self.max_buffered {
            sink.remove(0);
        }
        self.total_lines.fetch_add(1, Ordering::Relaxed);
    }
}

impl Default for LoggerService {
    fn default() -> Self {
        LoggerService::new()
    }
}

impl Service for LoggerService {
    fn name(&self) -> &str {
        BUILTIN_LOGGER
    }

    fn scope(&self) -> ServiceScope {
        ServiceScope::Runtime
    }

    fn is_request_varying(&self) -> bool {
        // `total_lines` (visible in the descriptor) changes
        // after every flush, so a page reading it produces
        // request-varying output -- the generation cache must
        // not replay a stale snapshot.
        true
    }

    fn describe_json(&self) -> String {
        format!(
            "{{\"kind\":\"logger\",\"scope\":\"{}\",\"total_lines\":{}}}",
            self.scope().as_str(),
            self.total_lines()
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// =====================================================================
// Slice 16k: in-memory session store (spec sect.16)
// =====================================================================

/// A single session value (spec sect.16.1: JSON-compatible
/// only). Functions / Symbols / native handles / DOM-like
/// nodes / Context objects / cyclic objects MUST NOT be
/// accepted; the JS hydration path rejects those shapes
/// before pushing a write into the envelope (the wrap
/// preamble coerces unknown shapes to a string tag and the
/// host drops the line with a diagnostic).
#[derive(Debug, Clone, PartialEq)]
pub enum SessionValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<SessionValue>),
    Object(Vec<(String, SessionValue)>),
}

impl SessionValue {
    /// Serialise back to a JSON string the wrap preamble
    /// (or the host's own wire encoder) can hand to
    /// `JSON.parse`. Hand-rolled to keep the slice-16
    /// "no new dep" discipline.
    pub fn to_json(&self, out: &mut String) {
        use std::fmt::Write as _;
        match self {
            SessionValue::Null => out.push_str("null"),
            SessionValue::Bool(true) => out.push_str("true"),
            SessionValue::Bool(false) => out.push_str("false"),
            SessionValue::Number(n) => {
                let _ = write!(out, "{n}");
            }
            SessionValue::String(s) => json_string_session_value(out, s),
            SessionValue::Array(items) => {
                out.push('[');
                let mut first = true;
                for item in items {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    item.to_json(out);
                }
                out.push(']');
            }
            SessionValue::Object(entries) => {
                out.push('{');
                let mut first = true;
                for (k, v) in entries {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    json_string_session_value(out, k);
                    out.push(':');
                    v.to_json(out);
                }
                out.push('}');
            }
        }
    }
}

fn json_string_session_value(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// One row in the session store: the session id (used as
/// the `tsp_sid` cookie value) and the page's key/value
/// bag. The id is opaque; we mint a 32-byte hex string
/// (16 random bytes) so an attacker cannot guess a valid
/// sid. Spec sect.16.2 (survive reload) is satisfied by
/// the fact that `SessionService` is a runtime-scoped
/// service -- generation replacement never touches the
/// underlying `Mutex<HashMap<...>>`.
#[derive(Debug, Clone)]
pub struct SessionData {
    pub id: String,
    pub data: BTreeMap<String, SessionValue>,
}

/// A write a page applied during one request. Mirrors the
/// `ctx.session.{set,delete,clear,regenerate,destroy}` calls:
/// - `Set(name, value)`: insert a JSON-compatible value.
/// - `Delete(name)`: drop one key.
/// - `Clear`: drop the whole data map (keeps the id so the
///   Set-Cookie stays valid until the response is written;
///   the page may follow up with a destroy).
/// - `Regenerate`: spec sect.16.3 -- replace the id while
///   keeping data.
/// - `Destroy`: spec sect.16.4 -- drop the entire session
///   and the cookie.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionWrite {
    Set(String, SessionValue),
    Delete(String),
    Clear,
    Regenerate,
    Destroy,
}

/// A request-scoped view of the current session the wrap
/// preamble hydrates into `ctx.session`. The host hands it
/// to the renderer; the renderer never sees the store.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionView {
    pub id: String,
    pub data: BTreeMap<String, SessionValue>,
}

impl SessionView {
    /// Wire form embedded into `Context.session`:
    /// `{"id":"<sid>","data":{...}}`. Null when there is
    /// no current session (the page created + destroyed in
    /// the same request, or a first request with no cookie).
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(32 + self.data.len() * 16);
        out.push_str("{\"id\":");
        json_string_session_value(&mut out, &self.id);
        out.push_str(",\"data\":{");
        let mut first = true;
        for (k, v) in &self.data {
            if !first {
                out.push(',');
            }
            first = false;
            json_string_session_value(&mut out, k);
            out.push(':');
            v.to_json(&mut out);
        }
        out.push_str("}}");
        out
    }
}

/// Host-owned, runtime-scoped, in-memory session store.
/// Created at boot, `Box::leak`'d into the registry, shared
/// by every connection thread. Never owned by a page
/// generation (spec sect.16.2).
/// Host-owned, runtime-scoped session service (spec
/// sect.16, plan sect.61 Phase 8). 16k shipped the
/// in-memory store inline; 16l factored the storage
/// into the [`crate::session_backend::SessionBackend`]
/// trait and plugged in two implementations: an
/// in-memory `MemoryBackend` and a hand-rolled RESP
/// `RedisBackend`. `SessionService` is now a thin
/// facade that owns the backend handle and exposes
/// the same 16k API to the host (so handle_connection
/// / `ctx.session` flow is unchanged).
pub struct SessionService {
    backend: Arc<dyn SessionBackend>,
}

impl SessionService {
    /// 16l factory: wire any backend implementation.
    /// The bin uses this with `MemoryBackend` by
    /// default and `RedisBackend` when `TSP_REDIS_URL`
    /// is set.
    pub fn with_backend(backend: Arc<dyn SessionBackend>) -> Self {
        SessionService { backend }
    }

    /// 16k convenience: build a service backed by an
    /// in-memory store with the given cap. Kept so
    /// existing tests (`SessionService::new(N)`) and
    /// the default boot path keep working without
    /// any bin changes.
    pub fn new(cap: usize) -> Self {
        use crate::session_backend::MemoryBackend;
        Self::with_backend(Arc::new(MemoryBackend::new(cap)))
    }

    /// Backend identifier (`"memory"` / `"redis"`)
    /// surfaced in the `ctx.services.session`
    /// descriptor snapshot so the dev can confirm
    /// which backend served a given request.
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    /// Look up an existing session by sid. Returns `None`
    /// when the sid is unknown or has been destroyed --
    /// the host then mints a new one (this matches spec
    /// sect.16.4: after destroy the session is no longer
    /// usable as an authenticated persistent session).
    pub fn lookup(&self, sid: &str) -> Option<SessionView> {
        self.backend.lookup(sid)
    }

    /// Mint a fresh session id and insert an empty row.
    /// The id is a 32-char hex string of a counter-derived
    /// 16-byte block (predictable but unique; production
    /// uses a CSPRNG -- 16k keeps the test surface small).
    pub fn create(&self) -> SessionView {
        self.backend.create()
    }

    /// Apply a list of writes from one request to the
    /// session identified by `current_sid` (the sid the
    /// host embedded in the wire Context for this request).
    /// Returns the new id (same as `current_sid` unless
    /// Regenerate minted a replacement, or Destroy / Clear
    /// + Destroy cleared the row -- in which case the
    /// returned id is the empty string and the host
    /// expunges the Set-Cookie).
    ///
    /// Writes for an unknown sid (e.g. a page called
    /// `set` after destroy) are dropped with a host
    /// diagnostic; we never resurrect a destroyed session
    /// because the spec is explicit (16.4).
    pub fn apply_writes(&self, current_sid: &str, writes: &[SessionWrite]) -> String {
        if writes.is_empty() {
            return current_sid.to_string();
        }
        self.backend.apply_writes(current_sid, writes)
    }

    /// How many sessions the store currently holds. Used
    /// by tests + dev diagnostics; not exposed to pages.
    pub fn len(&self) -> usize {
        self.backend.len()
    }
}

impl Service for SessionService {
    fn name(&self) -> &str {
        BUILTIN_SESSION
    }

    fn scope(&self) -> ServiceScope {
        ServiceScope::Runtime
    }

    fn is_request_varying(&self) -> bool {
        // `ctx.session` is per-request: the id rotates on
        // the first request / regenerate, and the data
        // mutates inside a request. Even when nothing
        // changes, a future page may read live session
        // state, so the generation cache must never
        // replay a stale snapshot.
        true
    }

    fn describe_json(&self) -> String {
        // The runtime-scoped services snapshot does NOT
        // carry the current request's session data (the
        // host hands the SessionView to the renderer
        // through a dedicated `Context.session` field,
        // not through `ctx.services.session` -- see spec
        // sect.13 / 16). 16k keeps the snapshot payload
        // minimal: just a hint that the service exists
        // and how many sessions live in the store right
        // now. Pages reach the live view through
        // `ctx.session` only.
        format!(
            "{{\"kind\":\"session\",\"scope\":\"{}\",\"backend\":\"{}\",\"live\":{}}}",
            self.scope().as_str(),
            self.backend.name(),
            self.len()
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// =====================================================================
// Slice 16m: built-in `time` service (call-capable read-only snapshot)
// =====================================================================

/// Built-in `time` service (16m). Runtime-scoped (the
/// host captures the snapshot per request) but
/// `is_request_varying()` returns `true` so the
/// generation cache bypasses every render -- a page
/// that reads `ctx.services.time.iso` produces
/// request-dependent output.
///
/// This is the first service that exposes a real
/// *read* surface to the page (as opposed to the
/// 16j logger, which is fire-and-forget). The host
/// captures `iso` / `epoch_ms` / `uptime_ms` once per
/// request and the wrap preamble freezes the result
/// into `ctx.services.time` via `Object.freeze`, so
/// the page sees a normal JS object and CANNOT call
/// back into the host (spec 17.3: no wrapper identity
/// across requests). A real persistent JS adapter
/// realm with RPC stays a 16n+ follow-up; 16m is the
/// snapshot shape that any such realm will round-trip
/// through.
pub struct TimeService {
    started: std::time::Instant,
}

impl TimeService {
    pub fn new() -> Self {
        TimeService {
            started: std::time::Instant::now(),
        }
    }

    pub fn snapshot_now(&self) -> TimeSnapshot {
        let now = std::time::SystemTime::now();
        let epoch_ms = now
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let iso = format_iso8601_utc(now);
        let uptime_ms = self.started.elapsed().as_millis() as i64;
        TimeSnapshot {
            iso,
            epoch_ms,
            uptime_ms,
        }
    }
}

impl Default for TimeService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSnapshot {
    pub iso: String,
    pub epoch_ms: i64,
    pub uptime_ms: i64,
}

impl Service for TimeService {
    fn name(&self) -> &str {
        BUILTIN_TIME
    }

    fn scope(&self) -> ServiceScope {
        ServiceScope::Runtime
    }

    fn is_request_varying(&self) -> bool {
        true
    }

    fn describe_json(&self) -> String {
        let s = self.snapshot_now();
        format!(
            "{{\"kind\":\"time\",\"scope\":\"{}\",\"iso\":{},\"epoch_ms\":{},\"uptime_ms\":{}}}",
            self.scope().as_str(),
            json_string_field(&s.iso),
            s.epoch_ms,
            s.uptime_ms,
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn json_string_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn format_iso8601_utc(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let ms = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_millis())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let time_secs = (secs % 86_400) as u32;
    let hour = time_secs / 3600;
    let minute = (time_secs % 3600) / 60;
    let second = time_secs % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    use std::fmt::Write as _;
    let mut out = String::with_capacity(24);
    let _ = write!(
        &mut out,
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y, m, d, hour, minute, second, ms
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyService {
        name: String,
        scope: ServiceScope,
        varying: bool,
    }

    impl DummyService {
        fn runtime(name: &str) -> Self {
            DummyService {
                name: name.to_string(),
                scope: ServiceScope::Runtime,
                varying: false,
            }
        }
    }

    impl Service for DummyService {
        fn name(&self) -> &str {
            &self.name
        }
        fn scope(&self) -> ServiceScope {
            self.scope
        }
        fn is_request_varying(&self) -> bool {
            self.varying
        }
        fn describe_json(&self) -> String {
            format!(
                "{{\"kind\":\"dummy\",\"scope\":\"{}\"}}",
                self.scope.as_str()
            )
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn scopes_have_stable_wire_names() {
        assert_eq!(ServiceScope::Runtime.as_str(), "runtime");
        assert_eq!(ServiceScope::Request.as_str(), "request");
    }

    #[test]
    fn with_defaults_registers_logger() {
        let reg = ServiceRegistry::with_defaults();
        let logger = reg.get(BUILTIN_LOGGER).expect("logger registered");
        assert_eq!(logger.name(), BUILTIN_LOGGER);
        assert_eq!(logger.scope(), ServiceScope::Runtime);
        assert!(reg.get("nope").is_none());
    }

    #[test]
    fn register_duplicate_replaces() {
        let mut reg = ServiceRegistry::new();
        reg.register(Arc::new(DummyService::runtime("dup")));
        reg.register(Arc::new(DummyService::runtime("dup")));
        // Last registration wins; a boot-time-only contract.
        assert!(reg.get("dup").is_some());
        assert_eq!(reg.snapshot(&[]).len(), 1);
    }

    #[test]
    fn snapshot_lists_runtime_then_request_scoped() {
        let reg = ServiceRegistry::with_defaults();
        let snap = reg.snapshot(&[("req-x".to_string(), "{\"kind\":\"x\"}".to_string())]);
        // 16j shipped only the logger; 16k added the
        // in-memory session service; 16m added the
        // request-varying `time` service. Default set
        // is now 3 runtime + 1 request-scoped.
        assert_eq!(snap.len(), 4);
        // Runtime first, sorted by name (BTreeMap);
        // request-scoped appended.
        assert_eq!(snap[0].0, BUILTIN_LOGGER);
        assert_eq!(snap[1].0, BUILTIN_SESSION);
        assert_eq!(snap[2].0, BUILTIN_TIME);
        assert_eq!(snap[3].0, "req-x");
        assert!(
            snap[0].1.contains("\"kind\":\"logger\""),
            "got: {}",
            snap[0].1
        );
        assert!(
            snap[0].1.contains("\"scope\":\"runtime\""),
            "got: {}",
            snap[0].1
        );
        assert!(
            snap[0].1.contains("\"total_lines\":0"),
            "got: {}",
            snap[0].1
        );
        assert!(
            snap[1].1.contains("\"kind\":\"session\""),
            "got: {}",
            snap[1].1
        );
        assert!(
            snap[2].1.contains("\"kind\":\"time\""),
            "got: {}",
            snap[2].1
        );
        assert_eq!(snap[3].1, "{\"kind\":\"x\"}");
    }
    #[test]
    fn dummy_service_defaults_to_not_request_varying() {
        let mut reg = ServiceRegistry::new();
        reg.register(Arc::new(DummyService::runtime("plain")));
        assert!(!reg.any_request_varying());
        // A varying runtime service flips the flag.
        reg.register(Arc::new(DummyService {
            name: "live".to_string(),
            scope: ServiceScope::Runtime,
            varying: true,
        }));
        assert!(reg.any_request_varying());
    }

    #[test]
    fn logger_describes_and_counts() {
        let logger = LoggerService::new();
        assert_eq!(logger.total_lines(), 0);
        let desc = logger.describe_json();
        assert!(desc.contains("\"kind\":\"logger\""), "got: {desc}");
        assert!(desc.contains("\"total_lines\":0"), "got: {desc}");

        logger.write_line("info", "hello");
        logger.write_line("error", "boom");
        assert_eq!(logger.total_lines(), 2);
        assert!(logger.describe_json().contains("\"total_lines\":2"));
        assert_eq!(
            logger.recent_lines(10),
            vec!["[info] hello".to_string(), "[error] boom".to_string()]
        );
    }

    #[test]
    fn logger_buffer_is_capped() {
        let logger = LoggerService::new();
        for i in 0..1100 {
            logger.write_line("debug", &format!("line-{i}"));
        }
        assert_eq!(logger.total_lines(), 1100);
        // Only the newest `max_buffered` lines stay resident.
        assert_eq!(logger.recent_lines(10_000).len(), 1000);
        assert_eq!(
            logger.recent_lines(3),
            vec![
                "[debug] line-1097",
                "[debug] line-1098",
                "[debug] line-1099"
            ]
        );
    }

    #[test]
    fn flush_log_lines_routes_to_logger() {
        let reg = ServiceRegistry::with_defaults();
        let lines = vec![
            LogLine {
                service: BUILTIN_LOGGER.to_string(),
                level: "info".to_string(),
                message: "a".to_string(),
            },
            LogLine {
                service: BUILTIN_LOGGER.to_string(),
                level: "warn".to_string(),
                message: "b".to_string(),
            },
            // Unknown service -> dropped, not forwarded.
            LogLine {
                service: "ghost".to_string(),
                level: "info".to_string(),
                message: "c".to_string(),
            },
        ];
        let forwarded = reg.flush_log_lines(&lines);
        assert_eq!(forwarded, 2);
        let logger = reg.get(BUILTIN_LOGGER).unwrap();
        let logger = logger.as_any().downcast_ref::<LoggerService>().unwrap();
        assert_eq!(logger.total_lines(), 2);
        assert_eq!(
            logger.recent_lines(10),
            vec!["[info] a".to_string(), "[warn] b".to_string()]
        );
    }

    #[test]
    fn flush_log_lines_non_logger_service_dropped() {
        let mut reg = ServiceRegistry::new();
        reg.register(Arc::new(DummyService::runtime("plain")));
        let lines = vec![LogLine {
            service: "plain".to_string(),
            level: "info".to_string(),
            message: "x".to_string(),
        }];
        assert_eq!(reg.flush_log_lines(&lines), 0);
    }

    #[test]
    fn service_survives_generation_drop() {
        // Plan sect.61 acceptance: "old generation release
        // 不关闭 service". The registry is host-owned and a
        // generation never references it; dropping every
        // generation must leave the service fully usable.
        let reg = ServiceRegistry::with_defaults();
        {
            // A PageRegistry stands in for whatever a
            // generation release touches; it is independent by
            // construction (the slice 12 state machine lives in
            // generation.rs and never sees services).
            let _generation_owned = crate::generation::PageRegistry::new();
        }
        let logger = reg.get(BUILTIN_LOGGER).unwrap();
        let logger = logger.as_any().downcast_ref::<LoggerService>().unwrap();
        logger.write_line("info", "still-alive");
        assert_eq!(logger.total_lines(), 1);
        assert!(reg.snapshot(&[])[0].1.contains("\"total_lines\":1"));
    }

    // ===================== Slice 16m: time service tests =====================

    #[test]
    fn time_service_is_registered_in_with_defaults() {
        let reg = ServiceRegistry::with_defaults();
        let t = reg.get(BUILTIN_TIME).expect("time registered");
        assert_eq!(t.name(), BUILTIN_TIME);
        assert_eq!(t.scope(), ServiceScope::Runtime);
        assert!(t.is_request_varying());
    }

    #[test]
    fn time_service_descriptor_carries_iso_epoch_and_uptime() {
        let svc = TimeService::new();
        let desc = svc.describe_json();
        assert!(desc.starts_with("{"), "got: {desc}");
        assert!(desc.contains("\"kind\":\"time\""), "got: {desc}");
        assert!(desc.contains("\"scope\":\"runtime\""), "got: {desc}");
        assert!(desc.contains("\"iso\":\""), "got: {desc}");
        assert!(desc.contains("\"epoch_ms\":"), "got: {desc}");
        assert!(desc.contains("\"uptime_ms\":"), "got: {desc}");
    }

    #[test]
    fn time_service_snapshot_advances_uptime() {
        let svc = TimeService::new();
        let s1 = svc.snapshot_now();
        std::thread::sleep(std::time::Duration::from_millis(15));
        let s2 = svc.snapshot_now();
        assert!(s2.uptime_ms >= s1.uptime_ms);
        assert!(s2.epoch_ms >= s1.epoch_ms);
    }

    #[test]
    fn time_service_iso_matches_system_time() {
        let svc = TimeService::new();
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let snap = svc.snapshot_now();
        let after = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        assert!(snap.epoch_ms >= before);
        assert!(snap.epoch_ms <= after + 5);
        assert_eq!(snap.iso.len(), 24);
        assert!(snap.iso.ends_with("Z"));
        assert!(snap.iso.contains("T"));
    }

    #[test]
    fn format_iso8601_utc_handles_known_epoch() {
        let t = std::time::UNIX_EPOCH;
        assert_eq!(format_iso8601_utc(t), "1970-01-01T00:00:00.000Z");
        let t = std::time::UNIX_EPOCH + std::time::Duration::from_millis(1);
        assert_eq!(format_iso8601_utc(t), "1970-01-01T00:00:00.001Z");
        let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1);
        assert_eq!(format_iso8601_utc(t), "1970-01-01T00:00:01.000Z");
    }

    // ===================== Slice 16k: session tests =====================

    fn sv(s: &str) -> SessionValue {
        SessionValue::String(s.to_string())
    }
    fn nv(n: f64) -> SessionValue {
        SessionValue::Number(n)
    }

    #[test]
    fn session_value_serialises_to_json() {
        // Wire form is part of the slice 16k contract; the
        // wrap preamble (and any future host-side encoder)
        // parse it via `JSON.parse`.
        let mut data: BTreeMap<String, SessionValue> = BTreeMap::new();
        data.insert("name".to_string(), sv("alice"));
        data.insert("n".to_string(), nv(7.0));
        data.insert("flag".to_string(), SessionValue::Bool(true));
        let view = SessionView {
            id: "deadbeef".to_string(),
            data,
        };
        let s = view.to_json();
        assert!(s.contains("\"id\":\"deadbeef\""), "got: {s}");
        assert!(s.contains("\"name\":\"alice\""), "got: {s}");
        assert!(s.contains("\"n\":7"), "got: {s}");
        assert!(s.contains("\"flag\":true"), "got: {s}");
    }

    #[test]
    fn session_create_lookup_apply_writes_round_trip() {
        let svc = SessionService::new(8);
        let view = svc.create();
        let sid = view.id.clone();
        // After create the row is empty.
        let looked = svc.lookup(&sid).unwrap();
        assert!(looked.data.is_empty());
        assert_eq!(looked.id, sid);

        // Apply set + delete + another set.
        let writes = vec![
            SessionWrite::Set("a".to_string(), sv("1")),
            SessionWrite::Set("b".to_string(), nv(2.0)),
            SessionWrite::Delete("a".to_string()),
        ];
        let next = svc.apply_writes(&sid, &writes);
        assert_eq!(next, sid);
        let view = svc.lookup(&sid).unwrap();
        assert!(!view.data.contains_key("a"));
        assert_eq!(view.data.get("b").unwrap(), &nv(2.0));
    }

    #[test]
    fn session_regenerate_keeps_data_and_swaps_id() {
        let svc = SessionService::new(8);
        let view = svc.create();
        let old_sid = view.id.clone();
        svc.apply_writes(&old_sid, &[SessionWrite::Set("k".to_string(), sv("v"))]);

        let new_sid = svc.apply_writes(&old_sid, &[SessionWrite::Regenerate]);
        assert_ne!(old_sid, new_sid);
        // Old id is gone (regenerate replaces the row).
        assert!(svc.lookup(&old_sid).is_none());
        // Data survived the regenerate (spec 16.3).
        let v = svc.lookup(&new_sid).unwrap();
        assert_eq!(v.data.get("k").unwrap(), &sv("v"));
        assert_eq!(v.id, new_sid);
    }

    #[test]
    fn session_destroy_removes_row_and_clears_returned_sid() {
        let svc = SessionService::new(8);
        let view = svc.create();
        let sid = view.id;
        let next = svc.apply_writes(&sid, &[SessionWrite::Destroy]);
        assert!(next.is_empty());
        assert!(svc.lookup(&sid).is_none());
        // Writes after destroy are dropped (we never
        // resurrect; spec 16.4).
        let next = svc.apply_writes(&sid, &[SessionWrite::Set("k".to_string(), sv("v"))]);
        assert_eq!(next, sid);
        assert!(svc.lookup(&sid).is_none());
    }

    #[test]
    fn session_clear_keeps_id_drops_data() {
        let svc = SessionService::new(8);
        let view = svc.create();
        let sid = view.id;
        svc.apply_writes(&sid, &[SessionWrite::Set("k".to_string(), sv("v"))]);
        let next = svc.apply_writes(&sid, &[SessionWrite::Clear]);
        assert_eq!(next, sid);
        let v = svc.lookup(&sid).unwrap();
        assert!(v.data.is_empty());
    }

    #[test]
    fn session_cap_evicts_oldest() {
        // Fill the cap, then create one more -- the FIFO
        // order is the eviction order.
        let svc = SessionService::new(3);
        let a = svc.create().id;
        let b = svc.create().id;
        let c = svc.create().id;
        assert_eq!(svc.len(), 3);
        let d = svc.create().id;
        assert_eq!(svc.len(), 3);
        assert!(svc.lookup(&a).is_none());
        assert!(svc.lookup(&b).is_some());
        assert!(svc.lookup(&c).is_some());
        assert!(svc.lookup(&d).is_some());
    }

    #[test]
    fn session_service_is_registered_in_with_defaults() {
        let reg = ServiceRegistry::with_defaults();
        let s = reg.get(BUILTIN_SESSION).expect("session registered");
        assert_eq!(s.name(), BUILTIN_SESSION);
        assert_eq!(s.scope(), ServiceScope::Runtime);
        assert!(s.is_request_varying());
    }

    #[test]
    fn session_survives_generation_drop() {
        // Plan sect.61 acceptance: "reload 页面后 session
        // 不丢". The store is host-owned; a generation
        // release never touches the HashMap.
        let svc = SessionService::new(8);
        let view = svc.create();
        let sid = view.id.clone();
        svc.apply_writes(&sid, &[SessionWrite::Set("k".to_string(), sv("v"))]);
        // A PageRegistry stands in for whatever a generation
        // release touches; the two structures are
        // independent by construction.
        let _pr = crate::generation::PageRegistry::new();
        let v = svc.lookup(&sid).unwrap();
        assert_eq!(v.data.get("k").unwrap(), &sv("v"));
    }
}
