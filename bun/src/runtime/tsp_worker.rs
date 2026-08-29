//! Embedded Bun worker entry point for the TSP v2.4 process architecture.
//!
//! This module is compiled into the Bun worker executable. It is intentionally
//! not part of the Rust master crate's dependency graph: the master must not
//! link Bun or initialize a JavaScript runtime.

#[path = "tsp/worker/protocol.rs"]
#[allow(unreachable_pub, dead_code)]
mod protocol;

use protocol::{ExecuteResponse, Message, ProtocolError};

#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(not(unix))]
use std::net::TcpStream;
use std::io::{Read, Write};

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
            // This is the process's OS entry thread, but it is not Bun's
            // main-thread VM. The TSP worker is an isolated child process
            // with no WebWorker/WorkerMessagingProxy owner. Marking it as a
            // main VM publishes it in MAIN_THREAD_VM and selects the
            // main-global path in Zig__GlobalObject__create, which is an
            // invalid model for this standalone worker.
            is_main_thread: false,
            ..Default::default()
        };
        startup_trace("virtual-machine-init:begin");
        let ptr = crate::jsc::virtual_machine::VirtualMachine::init(options)
            .map_err(|error| format!("{error:?}"))?;
        startup_trace("virtual-machine-init:end");
        let vm = unsafe { &mut *ptr };
        Ok(Self { vm, _log: log })
    }

    fn execute_request(
        &mut self,
        request: &protocol::ExecuteRequest,
    ) -> Result<String, String> {
        if request.script.is_empty() {
            return self.execute_path(&request.path);
        }
        let path = std::env::temp_dir().join(format!(
            "tsp-embedded-worker-{}-{}.tsx",
            std::process::id(),
            NEXT_SCRIPT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::write(&path, &request.script)
            .map_err(|error| format!("failed to materialize request script: {error}"))?;
        let result = self.execute_path(&path.to_string_lossy());
        let _ = std::fs::remove_file(path);
        result
    }

    fn execute_path(&mut self, path: &str) -> Result<String, String> {
        // JSC requires its API lock to be held while loading modules and
        // driving the event loop. This is especially strict on Windows,
        // where the embedded worker uses TCP for the master connection.
        let this = self as *mut Self;
        unsafe {
            (*this)
                .vm
                .run_with_api_lock(|| (*this).execute_path_with_api_lock(path))
        }
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
        self.vm
            .clear_entry_point()
            .map_err(|error| format!("{error:?}"))?;
        let _ = self
            .vm
            .load_entry_point(path.as_bytes())
            .map_err(|error| format!("{error:?}"))?;

        for _ in 0..10_000 {
            if let Some(value) = self.read_global_string("__tspEmbeddedResponse")? {
                return Ok(value);
            }
            if let Some(error) = self.read_global_string("__tspEmbeddedError")? {
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

static NEXT_SCRIPT_ID: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);
