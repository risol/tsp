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

/// Built-in runtime-scoped logger service name (spec sect.21
/// logger surface; plan sect.61 Phase 8 `logger service`).
pub const BUILTIN_LOGGER: &str = "logger";

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
    pub fn with_defaults() -> Self {
        let mut reg = ServiceRegistry::new();
        reg.register(Arc::new(LoggerService::new()));
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
        assert_eq!(snap.len(), 2);
        // Runtime first, sorted by name; request-scoped appended.
        assert_eq!(snap[0].0, BUILTIN_LOGGER);
        assert_eq!(snap[1].0, "req-x");
        assert!(snap[0].1.contains("\"kind\":\"logger\""), "got: {}", snap[0].1);
        assert!(snap[0].1.contains("\"scope\":\"runtime\""), "got: {}", snap[0].1);
        assert!(snap[0].1.contains("\"total_lines\":0"), "got: {}", snap[0].1);
        assert_eq!(snap[1].1, "{\"kind\":\"x\"}");
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
        assert_eq!(logger.recent_lines(3), vec!["[debug] line-1097", "[debug] line-1098", "[debug] line-1099"]);
    }

    #[test]
    fn flush_log_lines_routes_to_logger() {
        let reg = ServiceRegistry::with_defaults();
        let lines = vec![
            LogLine { service: BUILTIN_LOGGER.to_string(), level: "info".to_string(), message: "a".to_string() },
            LogLine { service: BUILTIN_LOGGER.to_string(), level: "warn".to_string(), message: "b".to_string() },
            // Unknown service -> dropped, not forwarded.
            LogLine { service: "ghost".to_string(), level: "info".to_string(), message: "c".to_string() },
        ];
        let forwarded = reg.flush_log_lines(&lines);
        assert_eq!(forwarded, 2);
        let logger = reg.get(BUILTIN_LOGGER).unwrap();
        let logger = logger.as_any().downcast_ref::<LoggerService>().unwrap();
        assert_eq!(logger.total_lines(), 2);
        assert_eq!(logger.recent_lines(10), vec!["[info] a".to_string(), "[warn] b".to_string()]);
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
}