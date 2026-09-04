//! TSP's JavaScriptCore boundary.
//!
//! This crate deliberately does not expose host event-loop, transpiler, or
//! module-loader types. A VM is owned by exactly one thread and all operations
//! must happen on that owner thread. The native FFI contract is kept in
//! `include/tsp_jsc.h`; the default backend is a test backend so the ownership
//! and protocol tests run without a WebKit build.

use std::fmt;
use std::thread::{self, ThreadId};

#[cfg(feature = "native-ffi")]
#[global_allocator]
static TSP_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod ffi {
    use std::ffi::c_void;

    /// Opaque native VM. The allocation and destruction domain belong to the
    /// JSC binding implementation, never to Rust's allocator.
    #[repr(C)]
    pub struct TspJscVm {
        _private: [u8; 0],
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default)]
    pub struct TspJscBuffer {
        pub ptr: *const u8,
        pub len: usize,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default)]
    pub struct TspJscResult {
        pub ok: u8,
        pub value: TspJscBuffer,
        pub error: TspJscBuffer,
    }

    #[cfg(feature = "native-ffi")]
    unsafe extern "C" {
        pub fn tsp_jsc_initialize() -> i32;
        pub fn tsp_jsc_vm_create() -> *mut TspJscVm;
        pub fn tsp_jsc_vm_destroy(vm: *mut TspJscVm);
        pub fn tsp_jsc_evaluate(
            vm: *mut TspJscVm,
            source: TspJscBuffer,
            filename: TspJscBuffer,
        ) -> TspJscResult;
        pub fn tsp_jsc_call_json(
            vm: *mut TspJscVm,
            function: TspJscBuffer,
            argument_json: TspJscBuffer,
        ) -> TspJscResult;
        pub fn tsp_jsc_drain_microtasks(vm: *mut TspJscVm) -> i32;
        pub fn tsp_jsc_buffer_free(buffer: TspJscBuffer);
    }

    /// Marker used by the safe owner wrapper to make the non-Send contract
    /// explicit even though the opaque pointer itself has no Rust fields.
    #[allow(dead_code)]
    pub(crate) type NativeHandle = *mut c_void;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    WrongThread { owner: ThreadId, current: ThreadId },
    Initialization(String),
    Evaluation(String),
    Microtasks(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongThread { owner, current } => {
                write!(
                    formatter,
                    "JSC VM belongs to {owner:?}, called from {current:?}"
                )
            }
            Self::Initialization(message) => {
                write!(formatter, "JSC initialization failed: {message}")
            }
            Self::Evaluation(message) => write!(formatter, "JSC evaluation failed: {message}"),
            Self::Microtasks(message) => write!(formatter, "JSC microtask drain failed: {message}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptValue(pub String);

/// A backend is intentionally smaller than a full application runtime. It is the
/// only surface the TSP runtime needs from a JavaScript engine in phase one.
pub trait Backend {
    fn evaluate(&mut self, source: &str, filename: &str) -> Result<ScriptValue, Error>;
    fn call_json(&mut self, function: &str, argument: &str) -> Result<ScriptValue, Error>;
    fn drain_microtasks(&mut self) -> Result<(), Error>;
}

#[cfg(feature = "native-ffi")]
pub struct NativeBackend {
    vm: std::ptr::NonNull<ffi::TspJscVm>,
    owner: ThreadId,
}

#[cfg(feature = "native-ffi")]
impl NativeBackend {
    /// Create a VM through the TSP JSC ABI. The native side owns the VM and
    /// every buffer returned by it; Rust copies buffers before freeing them.
    pub fn new() -> Result<Self, Error> {
        let status = unsafe { ffi::tsp_jsc_initialize() };
        if status != 0 {
            return Err(Error::Initialization(format!("status {status}")));
        }
        let vm = unsafe { ffi::tsp_jsc_vm_create() };
        let vm = std::ptr::NonNull::new(vm)
            .ok_or_else(|| Error::Initialization("VM allocation returned null".into()))?;
        Ok(Self {
            vm,
            owner: thread::current().id(),
        })
    }

    fn copy_buffer(buffer: ffi::TspJscBuffer) -> Result<String, Error> {
        if buffer.ptr.is_null() && buffer.len != 0 {
            return Err(Error::Evaluation("native buffer has a null pointer".into()));
        }
        let owned = if buffer.len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(buffer.ptr, buffer.len) }.to_vec()
        };
        unsafe { ffi::tsp_jsc_buffer_free(buffer) };
        String::from_utf8(owned).map_err(|_| Error::Evaluation("native buffer is not UTF-8".into()))
    }
}

#[cfg(feature = "native-ffi")]
impl Backend for NativeBackend {
    fn evaluate(&mut self, source: &str, filename: &str) -> Result<ScriptValue, Error> {
        let source_buffer = ffi::TspJscBuffer {
            ptr: source.as_ptr(),
            len: source.len(),
        };
        let filename_buffer = ffi::TspJscBuffer {
            ptr: filename.as_ptr(),
            len: filename.len(),
        };
        let result =
            unsafe { ffi::tsp_jsc_evaluate(self.vm.as_ptr(), source_buffer, filename_buffer) };
        if result.ok == 0 {
            return Err(Error::Evaluation(Self::copy_buffer(result.error)?));
        }
        Ok(ScriptValue(Self::copy_buffer(result.value)?))
    }

    fn call_json(&mut self, function: &str, argument: &str) -> Result<ScriptValue, Error> {
        let function_buffer = ffi::TspJscBuffer {
            ptr: function.as_ptr(),
            len: function.len(),
        };
        let argument_buffer = ffi::TspJscBuffer {
            ptr: argument.as_ptr(),
            len: argument.len(),
        };
        let result =
            unsafe { ffi::tsp_jsc_call_json(self.vm.as_ptr(), function_buffer, argument_buffer) };
        if result.ok == 0 {
            return Err(Error::Evaluation(Self::copy_buffer(result.error)?));
        }
        Ok(ScriptValue(Self::copy_buffer(result.value)?))
    }

    fn drain_microtasks(&mut self) -> Result<(), Error> {
        let status = unsafe { ffi::tsp_jsc_drain_microtasks(self.vm.as_ptr()) };
        if status == 0 {
            Ok(())
        } else {
            Err(Error::Microtasks(format!("status {status}")))
        }
    }
}

#[cfg(feature = "native-ffi")]
impl Drop for NativeBackend {
    fn drop(&mut self) {
        // Native JSC objects are thread-affine. If an owner is accidentally
        // dropped elsewhere, leaking is safer than invoking JSC on a foreign
        // thread; the runtime reports this during debug shutdown tests.
        if thread::current().id() == self.owner {
            unsafe { ffi::tsp_jsc_vm_destroy(self.vm.as_ptr()) };
        }
    }
}

/// Owner-thread wrapper around a JavaScriptCore backend.
pub struct Engine<B> {
    owner: ThreadId,
    backend: B,
}

impl<B: Backend> Engine<B> {
    pub fn new(backend: B) -> Self {
        Self {
            owner: thread::current().id(),
            backend,
        }
    }

    pub fn owner(&self) -> ThreadId {
        self.owner
    }

    pub fn evaluate(&mut self, source: &str, filename: &str) -> Result<ScriptValue, Error> {
        self.assert_owner()?;
        self.backend.evaluate(source, filename)
    }

    pub fn drain_microtasks(&mut self) -> Result<(), Error> {
        self.assert_owner()?;
        self.backend.drain_microtasks()
    }

    pub fn call_json(&mut self, function: &str, argument: &str) -> Result<ScriptValue, Error> {
        self.assert_owner()?;
        self.backend.call_json(function, argument)
    }

    fn assert_owner(&self) -> Result<(), Error> {
        let current = thread::current().id();
        if current == self.owner {
            Ok(())
        } else {
            Err(Error::WrongThread {
                owner: self.owner,
                current,
            })
        }
    }
}

impl<B: Backend> tsp_js::JsRuntime for Engine<B> {
    type Error = Error;

    fn evaluate(&mut self, source: &str, filename: &str) -> Result<String, Self::Error> {
        Engine::evaluate(self, source, filename).map(|value| value.0)
    }

    fn call_json(&mut self, function: &str, argument: &str) -> Result<String, Self::Error> {
        Engine::call_json(self, function, argument).map(|value| value.0)
    }

    fn drain_microtasks(&mut self) -> Result<(), Self::Error> {
        Engine::drain_microtasks(self)
    }
}

#[cfg(feature = "mock")]
#[derive(Debug, Default)]
pub struct MockBackend {
    pub evaluations: Vec<(String, String)>,
    pub pending_microtasks: usize,
    pub drained_microtasks: usize,
}

#[cfg(feature = "mock")]
impl Backend for MockBackend {
    fn evaluate(&mut self, source: &str, filename: &str) -> Result<ScriptValue, Error> {
        if source.contains("throw new Error") {
            return Err(Error::Evaluation(format!("exception in {filename}")));
        }
        self.evaluations
            .push((source.to_owned(), filename.to_owned()));
        if source.contains("queueMicrotask") || source.contains("Promise.resolve") {
            self.pending_microtasks += 1;
        }
        Ok(ScriptValue("undefined".to_owned()))
    }

    fn call_json(&mut self, function: &str, argument: &str) -> Result<ScriptValue, Error> {
        self.evaluations
            .push((format!("call:{function}:{argument}"), String::new()));
        Ok(ScriptValue("undefined".to_owned()))
    }

    fn drain_microtasks(&mut self) -> Result<(), Error> {
        self.drained_microtasks += self.pending_microtasks;
        self.pending_microtasks = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_evaluates_and_drains_on_the_owner_thread() {
        let mut engine = Engine::new(MockBackend::default());
        assert_eq!(
            engine.evaluate("Promise.resolve(1)", "route.tsp"),
            Ok(ScriptValue("undefined".into()))
        );
        engine.drain_microtasks().unwrap();
        assert_eq!(engine.backend.drained_microtasks, 1);
    }

    #[test]
    fn engine_rejects_calls_from_another_thread() {
        let engine = Engine::new(MockBackend::default());
        let owner = engine.owner();
        let result = thread::spawn(move || {
            let mut engine = engine;
            engine.evaluate("1", "thread.tsp")
        })
        .join()
        .unwrap();
        assert!(matches!(result, Err(Error::WrongThread { owner: actual, .. }) if actual == owner));
    }

    #[test]
    fn evaluation_errors_do_not_cross_the_backend_boundary() {
        let mut engine = Engine::new(MockBackend::default());
        let result = engine.evaluate("throw new Error('boom')", "broken.tsp");
        assert!(
            matches!(result, Err(Error::Evaluation(message)) if message.contains("broken.tsp"))
        );
    }

    #[cfg(feature = "native-ffi")]
    #[test]
    fn empty_native_buffers_are_safe_to_copy_and_release() {
        assert_eq!(
            NativeBackend::copy_buffer(ffi::TspJscBuffer::default()).unwrap(),
            ""
        );
    }

    #[cfg(feature = "native-ffi")]
    #[test]
    fn native_jsc_smoke_runs_when_a_webkit_root_is_configured() {
        if std::env::var_os("TSP_JSC_SDK_ROOT").is_none() {
            return;
        }
        let mut engine = Engine::new(NativeBackend::new().unwrap());
        assert_eq!(engine.evaluate("1 + 1", "native-smoke.js").unwrap().0, "2");
        engine
            .evaluate(
                "globalThis.__tsp_test_call = (value) => JSON.stringify({value: value.value})",
                "native-smoke.js",
            )
            .unwrap();
        assert_eq!(
            engine
                .call_json("__tsp_test_call", r#"{"value":"ok"}"#)
                .unwrap()
                .0,
            r#"{"value":"ok"}"#
        );
        engine
            .evaluate("Promise.resolve('ready')", "native-smoke.js")
            .unwrap();
        engine.drain_microtasks().unwrap();
    }
}
