//! Embedded Bun worker entry point for the TSP v2.4 process architecture.
//!
//! This module is compiled into the Bun worker executable. It is intentionally
//! not part of the Rust master crate's dependency graph: the master must not
//! link Bun or initialize a JavaScript runtime.

#[path = "tsp/worker/protocol.rs"]
#[allow(unreachable_pub, dead_code)]
mod protocol;

use protocol::{ExecuteResponse, Message, ProtocolError};
use crate::jsc::JSValue;

#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(not(unix))]
use std::net::TcpStream;
use std::io::{Read, Write};

unsafe extern "C" {
    fn Bun__REPL__setupGlobalRequire(
        global_object: *const crate::jsc::JSGlobalObject,
        cwd_ptr: *const u8,
        cwd_len: usize,
    );
    fn Bun__REPL__evaluate(
        global_object: *const crate::jsc::JSGlobalObject,
        source_ptr: *const u8,
        source_len: usize,
        filename_ptr: *const u8,
        filename_len: usize,
        exception: *mut JSValue,
    ) -> JSValue;
}

pub fn requested() -> bool {
    bun_core::argv()
        .iter()
        .any(|argument| argument == b"--tsp-worker")
}

#[inline]
fn startup_trace(stage: &str) {
    if std::env::var_os("TSP_WORKER_STARTUP_TRACE").is_some() {
        eprintln!("TSP worker startup: {stage}");
    }
}

/// Run one embedded Bun VM and serve requests until the master shuts down.
///
/// The first implementation accepts the generated wrapper path in
/// `ExecuteRequest.path`. The next migration step will move wrapper creation
/// and route loading behind this same boundary; the VM lifecycle is already
/// correct here: one initialization per worker process and no Bun child.
pub fn run() -> i32 {
    #[cfg(unix)]
    { return run_unix(); }
    #[cfg(not(unix))]
    { return run_tcp(); }
}

#[cfg(unix)]
fn run_unix() -> i32 {
    let socket_path = match std::env::var_os("TSP_WORKER_SOCKET") {
        Some(path) => path,
        None => {
            eprintln!("TSP worker: TSP_WORKER_SOCKET is required");
            return 2;
        }
    };
    let stream = match UnixStream::connect(socket_path) {
        Ok(stream) => stream,
        Err(error) => {
            eprintln!("TSP worker: failed to connect to master: {error}");
            return 2;
        }
    };
    serve_stream(stream)
}

#[cfg(not(unix))]
fn run_tcp() -> i32 {
    startup_trace("tcp-connect:begin");
    let endpoint = match std::env::var("TSP_WORKER_SOCKET") {
        Ok(endpoint) => endpoint,
        Err(_) => {
            eprintln!("TSP worker: TSP_WORKER_SOCKET is required");
            return 2;
        }
    };
    let stream = match TcpStream::connect(endpoint) {
        Ok(stream) => stream,
        Err(error) => {
            eprintln!("TSP worker: failed to connect to master: {error}");
            return 2;
        }
    };
    startup_trace("tcp-connect:end");
    serve_stream(stream)
}

fn serve_stream<S>(mut stream: S) -> i32
where
    S: Read + Write,
{
    startup_trace("vm-init:begin");
    let mut vm = match EmbeddedVm::initialize() {
        Ok(vm) => vm,
        Err(error) => {
            eprintln!("TSP worker: embedded Bun initialization failed: {error}");
            return 2;
        }
    };
    startup_trace("vm-init:end");

    startup_trace("handshake:read-hello");
    match Message::read_from(&mut stream) {
        Ok(Message::Hello) => {}
        Ok(_) => {
            eprintln!("TSP worker: master handshake message was not HELLO");
            return 2;
        }
        Err(error) => {
            eprintln!("TSP worker: failed to read HELLO: {error}");
            return 2;
        }
    }
    startup_trace("handshake:hello-received");
    if let Err(error) = (Message::Ready {
        worker_id: std::process::id() as u64,
    })
    .write_to(&mut stream)
    {
        eprintln!("TSP worker: failed to send READY: {error}");
        return 2;
    }
    startup_trace("handshake:ready-sent");

    loop {
        let message = match Message::read_from(&mut stream) {
            Ok(message) => message,
            Err(ProtocolError::Truncated) => {
                return 0;
            }
            Err(error) => {
                eprintln!("TSP worker: protocol read failed: {error}");
                return 2;
            }
        };
        match message {
            Message::Execute { id, request } => {
                let result = vm.execute_request(&request);
                let response = match result {
                    Ok(body) => Message::Response {
                        id,
                        response: ExecuteResponse {
                            status: 200,
                            headers: Vec::new(),
                            body: body.into_bytes(),
                        },
                    },
                    Err(error) => Message::Error {
                        id,
                        code: "TSP_WORKER_EXEC".into(),
                        message: error,
                    },
                };
                if let Err(error) = response.write_to(&mut stream) {
                    eprintln!("TSP worker: failed to send response: {error}");
                    return 2;
                }
            }
            Message::Heartbeat { id } => {
                if let Err(error) = (Message::Heartbeat { id }).write_to(&mut stream) {
                    eprintln!("TSP worker: failed to send heartbeat: {error}");
                    return 2;
                }
            }
            Message::Cancel { .. } => {
                // The native VM cancellation hook is wired in the next slice.
                // The master still owns the hard deadline and may recycle us.
            }
            Message::Shutdown => return 0,
            Message::Hello | Message::Ready { .. } | Message::Response { .. } | Message::Error { .. } => {
                eprintln!("TSP worker: unexpected message from master");
                return 2;
            }
        }
    }
}

struct EmbeddedVm {
    vm: &'static mut crate::jsc::VirtualMachineRef,
    _log: &'static mut bun_ast::Log,
    global_require_ready: bool,
    // `VirtualMachine::set_main` stores a borrowed slice. Normal Bun entry
    // paths live in process-lifetime argv storage; TSP paths arrive in an IPC
    // request and would otherwise dangle after that request is dropped.
    entry_path: Vec<u8>,
}

impl EmbeddedVm {
    fn initialize() -> Result<Self, String> {
        startup_trace("jsc-initialize:begin");
        crate::jsc::initialize(false);
        startup_trace("jsc-initialize:end");
        bun_ast::initialize_store();
        startup_trace("ast-store:end");
        let log = Box::leak(Box::new(bun_ast::Log::init()));
        startup_trace("log-init:end");
        let transform_options = bun_options_types::schema::api::TransformOptions {
            jsx: Some(bun_options_types::schema::api::Jsx {
                factory: b"React.createElement".as_slice().into(),
                fragment: b"React.Fragment".as_slice().into(),
                runtime: bun_options_types::schema::api::JsxRuntime::Classic,
                ..Default::default()
            }),
            ..Default::default()
        };
        let options = crate::jsc::VirtualMachineInitOptions {
            log: Some(std::ptr::NonNull::from(&mut *log)),
            transform_options,
            // The TSP child owns the only JSC VM in its operating-system
            // process. It is not Bun's in-process WebWorker, so its VM role
            // is process-relative main rather than WebWorker or Auxiliary.
            role: crate::jsc::virtual_machine::VmRole::ProcessMain,
            startup_trace: Some(startup_trace),
            ..Default::default()
        };
        startup_trace("virtual-machine-init:begin");
        let ptr = crate::jsc::virtual_machine::VirtualMachine::init(options)
            .map_err(|error| format!("{error:?}"))?;
        startup_trace("virtual-machine-init:end");
        let vm = unsafe { &mut *ptr };

        // `VirtualMachine::init` constructs the low-level VM and Transpiler,
        // but every module-loading Bun entry point performs this second phase
        // before evaluating user code. In particular, `configure_defines`
        // materializes the define/env tables used by transpilation and
        // `load_extra_env_and_source_code_printer` installs per-thread runtime
        // state used by module loading and error reporting. Skipping this
        // phase leaves a VM that can complete Hello/Ready yet crash during its
        // first module evaluation on Windows.
        startup_trace("runtime-config:begin");
        {
            let transpiler = &mut vm.transpiler;
            transpiler.resolver.env_loader = std::ptr::NonNull::new(transpiler.env);
            transpiler.options.env.behavior =
                bun_options_types::schema::api::DotEnvBehavior::LoadAllWithoutInlining;
            transpiler.configure_defines().map_err(|error| {
                format!("failed to configure embedded worker defines: {error:?}")
            })?;
        }
        startup_trace("runtime-config:defines:end");
        bun_http::async_http::load_env(
            vm.log_mut()
                .ok_or_else(|| "embedded worker log is unavailable".to_string())?,
            vm.env_loader(),
        );
        vm.load_extra_env_and_source_code_printer();
        // Match the process-main CLI boot path. `VmRole::ProcessMain` sets
        // the per-VM flag during construction; set the thread-local marker as
        // well so native/JSC helpers observe the same main-VM state while the
        // embedded worker is evaluating modules.
        vm.is_main_thread = true;
        crate::jsc::virtual_machine::VirtualMachine::set_is_main_thread_vm(true);
        startup_trace("runtime-config:end");

        Ok(Self {
            vm,
            _log: log,
            global_require_ready: false,
            entry_path: Vec::new(),
        })
    }

    fn execute_request(
        &mut self,
        request: &protocol::ExecuteRequest,
    ) -> Result<String, String> {
        if request.script.is_empty() {
            return self.execute_path(&request.path);
        }
        let path = std::env::temp_dir().join(format!(
            "tsp-embedded-worker-{}-{}.cts",
            std::process::id(),
            NEXT_SCRIPT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::write(&path, &request.script)
            .map_err(|error| format!("failed to materialize request script: {error}"))?;
        let result = self.execute_embedded_path(&path);
        let _ = std::fs::remove_file(path);
        result
    }

    /// Load one request wrapper through CommonJS and evaluate the module
    /// synchronously. The old path imported every wrapper from synthetic
    /// `bun:main`, which created an ESM async-module resume microtask even for
    /// a synchronous route. On Windows that first JSC checkpoint is where the
    /// source-built worker faults. CommonJS evaluation keeps the route and
    /// wrapper in one module scope and returns before any module-resume job is
    /// needed.
    fn execute_embedded_path(&mut self, path: &std::path::Path) -> Result<String, String> {
        startup_trace("request:api-lock:begin");
        let _api_lock = self.vm.global().vm().get_api_lock();
        startup_trace("request:api-lock:acquired");

        if !self.global_require_ready {
            let cwd = path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            let cwd = cwd
                .to_str()
                .ok_or_else(|| "embedded worker temp directory is not UTF-8".to_string())?;
            // SAFETY: the global object is live for the VM lifetime and the
            // UTF-8 cwd slice remains valid for the duration of the FFI call.
            unsafe {
                Bun__REPL__setupGlobalRequire(self.vm.global(), cwd.as_ptr(), cwd.len());
            }
            self.global_require_ready = true;
        }

        let path = path
            .to_str()
            .ok_or_else(|| "embedded worker request path is not UTF-8".to_string())?;
        let source = format!("require({});", js_string_literal(path));
        let filename = b"[tsp-embedded-worker]";
        let mut exception = JSValue::UNDEFINED;
        // SAFETY: all pointers refer to live byte slices for the duration of
        // the synchronous call; `global` is the VM's live opaque handle and
        // `exception` is a stack out-parameter understood by the C++ shim.
        unsafe {
            let _ = Bun__REPL__evaluate(
                self.vm.global(),
                source.as_ptr(),
                source.len(),
                filename.as_ptr(),
                filename.len(),
                &raw mut exception,
            );
        }
        if !exception.is_undefined() && !exception.is_null() {
            let detail = self
                .read_global_string("_error")?
                .unwrap_or_else(|| "unknown JavaScript exception".to_string());
            return Err(format!("embedded TSP wrapper evaluation failed: {detail}"));
        }

        if let Some(value) = self.read_global_string("__tspEmbeddedResponse")? {
            startup_trace("request:response-ready");
            return Ok(value);
        }
        if let Some(error) = self.read_global_string("__tspEmbeddedError")? {
            startup_trace("request:error-ready");
            return Err(error);
        }

        // Async handlers and Response.text() still use ordinary Promise jobs.
        // They are not ESM module-resume jobs, so drive the normal event loop
        // only when the synchronous wrapper did not publish a result.
        startup_trace("load-entry:wait:begin");
        for _ in 0..10_000 {
            if let Some(value) = self.read_global_string("__tspEmbeddedResponse")? {
                startup_trace("request:response-ready");
                return Ok(value);
            }
            if let Some(error) = self.read_global_string("__tspEmbeddedError")? {
                startup_trace("request:error-ready");
                return Err(error);
            }
            self.vm.event_loop_mut().tick();
            self.vm.auto_tick();
            std::thread::yield_now();
        }
        Err("embedded TSP wrapper did not produce a response".into())
    }

    fn execute_path(&mut self, path: &str) -> Result<String, String> {
        // JSC requires its API lock to be held while loading modules and
        // driving the event loop. This is especially strict on Windows,
        // where the embedded worker uses TCP for the master connection. Keep
        // the guard for the whole worker lifetime, matching Bun CLI's
        // `Run::start` path instead of entering/exiting through the callback
        // trampoline for every request.
        startup_trace("request:api-lock:begin");
        let _api_lock = self.vm.global().vm().get_api_lock();
        startup_trace("request:api-lock:acquired");
        let result = self.execute_path_with_api_lock(path);
        startup_trace("request:api-lock:end");
        result
    }

    fn execute_path_with_api_lock(&mut self, path: &str) -> Result<String, String> {
        let path = std::path::Path::new(path);
        let path = path
            .to_str()
            .ok_or_else(|| "worker request path is not valid UTF-8".to_string())?;
        // Per-request cache bust. `load_entry_point` evaluates the
        // synthetic `bun:main` module and stores the resolved promise
        // in the JS module registry under that hardcoded name. Without
        // this drop the second call returns the first request's
        // already-resolved promise, the new wrap preamble never runs,
        // and every URL aliases to whichever .tsp the first request hit
        // (see docs/v2/bugs/0001). `clear_entry_point` removes the
        // `bun:main` entry only; shared modules (e.g. `tsp:server`
        // shims) keep their cache hits across requests.
        startup_trace("request:clear-entry:begin");
        self.vm
            .clear_entry_point()
            .map_err(|error| format!("{error:?}"))?;
        startup_trace("request:clear-entry:end");
        // Keep the previous backing allocation valid until
        // `clear_entry_point` has completed its last read of `vm.main`.
        // Replacing this buffer first could reallocate for a longer path and
        // leave that cleanup call observing the old dangling slice.
        self.entry_path.clear();
        self.entry_path.extend_from_slice(path.as_bytes());
        startup_trace("request:load-entry:begin");
        let entry_promise = {
            let (vm, entry_path) = (&mut *self.vm, self.entry_path.as_slice());
            vm.reload_entry_point(entry_path)
                .map_err(|error| format!("{error:?}"))?
        };
        startup_trace("request:load-entry:reload-end");

        // A synchronous embedded wrapper can publish its response while the
        // synthetic bun:main module is evaluated. Read that result before
        // entering the first JSC microtask checkpoint; only asynchronous
        // handlers or Response body reads need the normal promise wait.
        if let Some(value) = self.read_global_string("__tspEmbeddedResponse")? {
            startup_trace("request:response-ready");
            return Ok(value);
        }
        if let Some(error) = self.read_global_string("__tspEmbeddedError")? {
            startup_trace("request:error-ready");
            return Err(error);
        }

        startup_trace("load-entry:wait:begin");
        let _ = self
            .vm
            .wait_for_promise(crate::jsc::AnyPromise::Internal(entry_promise));
        startup_trace("load-entry:wait:end");
        startup_trace("request:load-entry:end");

        for _ in 0..10_000 {
            if let Some(value) = self.read_global_string("__tspEmbeddedResponse")? {
                startup_trace("request:response-ready");
                return Ok(value);
            }
            if let Some(error) = self.read_global_string("__tspEmbeddedError")? {
                startup_trace("request:error-ready");
                return Err(error);
            }
            self.vm.event_loop_mut().tick();
            self.vm.auto_tick();
            std::thread::yield_now();
        }
        Err("embedded TSP wrapper did not produce a response".into())
    }

    fn read_global_string(&mut self, name: &str) -> Result<Option<String>, String> {
        let global = self.vm.global();
        let value = global
            .to_js_value()
            .get(global, name)
            .map_err(|error| format!("{error:?}"))?;
        let Some(value) = value else { return Ok(None) };
        if value.is_undefined_or_null() {
            return Ok(None);
        }
        let value = value
            .to_slice(global)
            .map_err(|error| format!("{error:?}"))?;
        Ok(Some(String::from_utf8_lossy(value.slice()).into_owned()))
    }
}

fn js_string_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

static NEXT_SCRIPT_ID: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);
