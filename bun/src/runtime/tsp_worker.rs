//! Embedded Bun worker entry point for the TSP embedded-worker process architecture.
//!
//! This module is compiled into the Bun worker executable. It is intentionally
//! not part of the Rust master crate's dependency graph: the master must not
//! link Bun or initialize a JavaScript runtime.

#[path = "tsp/worker/protocol.rs"]
#[allow(unreachable_pub, dead_code)]
mod protocol;

use protocol::{ExecuteResponse, Message, ProtocolError};
#[cfg(not(windows))]
use crate::jsc::JSValue;

#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(not(unix))]
use std::net::TcpStream;
use std::io::{Read, Write};

#[cfg(not(windows))]
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
    fn Bun__JSC__disableEphemeralScriptCaches();
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
        if let Some(path) = std::env::var_os("TSP_WORKER_STARTUP_TRACE_FILE") {
            use std::fs::OpenOptions;

            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(file, "{} {stage}", std::process::id());
            }
        }
    }
}

/// Run one embedded Bun VM and serve requests until the master shuts down.
///
/// The master sends a generated wrapper source in `ExecuteRequest.script`.
/// The worker materializes it for diagnostics and relative resolution, then
/// uses Bun's Rust-side transpiler before evaluating a fresh plain-script
/// closure in the long-lived VM. Requests with local dynamic imports use an
/// async closure; the native worker protocol remains the only process boundary.
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
    #[cfg(not(windows))]
    startup_trace("vm-init:begin");
    #[cfg(not(windows))]
    let mut vm = match EmbeddedVm::initialize() {
        Ok(vm) => vm,
        Err(error) => {
            eprintln!("TSP worker: embedded Bun initialization failed: {error}");
            return 2;
        }
    };
    #[cfg(not(windows))]
    startup_trace("vm-init:end");
    #[cfg(windows)]
    startup_trace("windows-cli-supervisor:ready");

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
                #[cfg(windows)]
                let result = execute_windows_cli(&request);
                #[cfg(not(windows))]
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

/// Execute a generated route through the packaged Bun CLI on Windows.
///
/// The long-lived worker remains the protocol, timeout, and recycling
/// boundary. A request child is the same single-file executable without the
/// `--tsp-worker` dispatch flag, so Bun initializes its standard process-main
/// event loop before running async route code. This avoids the hosted-runner
/// JSC microtask crash at the custom embedding boundary without introducing a
/// second runtime binary.
#[cfg(windows)]
fn execute_windows_cli(request: &protocol::ExecuteRequest) -> Result<String, String> {
    use std::process::{Command, Stdio};

    let path = std::env::temp_dir().join(format!(
        "tsp-worker-request-{}-{}.tsx",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| format!("worker clock failed: {error}"))?
            .as_nanos()
    ));
    std::fs::write(&path, &request.script)
        .map_err(|error| format!("failed to materialize Windows route module: {error}"))?;
    let executable = std::env::current_exe()
        .map_err(|error| format!("failed to locate packaged Bun executable: {error}"))?;
    let output = Command::new(executable)
        .arg("run")
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let _ = std::fs::remove_file(&path);
    let output = output.map_err(|error| format!("failed to start packaged Bun CLI: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "packaged Bun CLI exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| "packaged Bun CLI response was not UTF-8".to_string())?;
    let marker = "__TSP_OUT_V1__\n";
    let envelope = stdout
        .split_once(marker)
        .map(|(_, value)| value.trim_end())
        .ok_or_else(|| "packaged Bun CLI produced no TSP response envelope".to_string())?;
    Ok(envelope.to_string())
}

#[cfg(not(windows))]
struct EmbeddedVm {
    vm: &'static mut crate::jsc::VirtualMachineRef,
    _log: &'static mut bun_ast::Log,
    global_require_ready: bool,
    // `VirtualMachine::set_main` stores a borrowed slice. Normal Bun entry
    // paths live in process-lifetime argv storage; TSP paths arrive in an IPC
    // request and would otherwise dangle after that request is dropped.
    entry_path: Vec<u8>,
}

#[cfg(not(windows))]
impl EmbeddedVm {
    fn initialize() -> Result<Self, String> {
        startup_trace("jsc-initialize:begin");
        crate::jsc::initialize(false);
        startup_trace("jsc-initialize:end");
        // This process evaluates a fresh generated top-level program for every
        // request. Disable JSC's VM-scoped script caches before the first VM is
        // created; retaining request-specific programs is unsafe for a
        // long-lived embedded worker on Windows.
        startup_trace("jsc-ephemeral-caches:disable:begin");
        unsafe {
            Bun__JSC__disableEphemeralScriptCaches();
        }
        startup_trace("jsc-ephemeral-caches:disable:end");
        // The generated wrapper is request-specific and includes a fresh source
        // URL, so Bun's on-disk runtime transpiler cache cannot hit. Disable it in
        // the worker as well; otherwise every request creates a never-reused
        // `.pile` entry in the user's global cache.
        crate::jsc::runtime_transpiler_cache::IS_DISABLED
            .store(true, std::sync::atomic::Ordering::Relaxed);
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
        // The wrapper is evaluated as a fresh program, so the filename is
        // used for diagnostics and relative `require()` resolution rather
        // than as a module-cache identity.
        let path = std::env::temp_dir().join(format!(
            "tsp-embedded-worker-{}.tsx",
            std::process::id()
        ));
        std::fs::write(&path, &request.script)
            .map_err(|error| format!("failed to materialize request script: {error}"))?;
        let result = self.execute_embedded_script(&request.script, &path);
        let _ = std::fs::remove_file(path);
        result
    }

    /// Evaluate one request wrapper as a synchronous JavaScript program inside
    /// the long-lived VM. The wrapper is transpiled through Bun's native
    /// Rust-side printer, then evaluated as a plain IIFE. This avoids creating
    /// a synthetic ESM `bun:main` module and its persistent module-evaluation
    /// promise for every request.
    fn execute_embedded_script(
        &mut self,
        script: &[u8],
        path: &std::path::Path,
    ) -> Result<String, String> {
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
        startup_trace("request:transpile:begin");
        let transpiled = crate::jsc_hooks::transpile_embedded_source(
            std::ptr::from_ref(self.vm.global()).cast_mut(),
            path.as_bytes(),
            script,
        )
        .map_err(|error| format!("embedded TSP wrapper transpilation failed: {error}"))?;
        startup_trace("request:transpile:end");
        let wrapper_source = if transpiled
            .windows(b"await import(".len())
            .any(|window| window == b"await import(")
        {
            format!(
                "(async () => {{\n{}\n}})();",
                String::from_utf8_lossy(&transpiled)
            )
        } else {
            format!(
                "(() => {{\n{}\n}})();",
                String::from_utf8_lossy(&transpiled)
            )
        };
        let source = format!(
            "globalThis.__tspImportMeta__ = {{ require: typeof require === 'function' ? require : globalThis.require, url: {:?}, path: {:?} }};\n{}",
            path, path, wrapper_source
        );
        let filename = path.as_bytes();
        let mut exception = JSValue::UNDEFINED;
        startup_trace("request:evaluate:begin");
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
        startup_trace("request:evaluate:end");
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
        if !self.global_require_ready {
            let cwd = std::path::Path::new(path)
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            let cwd = cwd
                .to_str()
                .ok_or_else(|| "embedded worker entry directory is not UTF-8".to_string())?;
            // SAFETY: the global object is live for the VM lifetime and the
            // UTF-8 cwd slice remains valid for this synchronous FFI call.
            unsafe {
                Bun__REPL__setupGlobalRequire(self.vm.global(), cwd.as_ptr(), cwd.len());
            }
            self.global_require_ready = true;
        }
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
        // already-resolved promise and the new wrapper never runs.
        startup_trace("request:clear-entry:begin");
        self.vm
            .clear_entry_point()
            .map_err(|error| format!("{error:?}"))?;
        startup_trace("request:clear-entry:end");
        // Keep the previous backing allocation valid until
        // `clear_entry_point` has completed its last read of `vm.main`.
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
