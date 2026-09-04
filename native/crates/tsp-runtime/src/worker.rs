//! TSP-owned worker pool and route execution orchestration.
//!
//! A worker owns an injected JavaScript runtime and is never used from
//! another thread. The host sends complete request jobs over bounded
//! channels; HTTP socket threads never hold JavaScript values or locks.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};

use crate::{Request, Response, RouteSpec};
use serde_json::json;
use tsp_js::JsRuntime;

pub trait WorkerExecutor: 'static {
    fn execute(
        &mut self,
        request: Request,
        route: RouteSpec,
        params: HashMap<String, String>,
    ) -> Result<Response, WorkerError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerError {
    QueueClosed,
    Execution(String),
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueClosed => formatter.write_str("worker queue is closed"),
            Self::Execution(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for WorkerError {}

const RUNTIME_PRELUDE: &str = include_str!("../../../runtime-js/src/bootstrap.js");

/// Executes a compiled route bundle through an injected JavaScript runtime.
/// The host runtime does not know whether the implementation is JSC, a test
/// double, or a future engine adapter.
pub struct RouteExecutor<J> {
    engine: J,
}

impl<J: JsRuntime> RouteExecutor<J> {
    pub fn new(mut engine: J, bundle: &str, filename: &str) -> Result<Self, WorkerError> {
        engine
            .evaluate(RUNTIME_PRELUDE, "tsp-runtime.js")
            .map_err(|error| WorkerError::Execution(error.to_string()))?;
        engine
            .evaluate(bundle, filename)
            .map_err(|error| WorkerError::Execution(error.to_string()))?;
        Ok(Self { engine })
    }

    fn dispatch_source(
        request: &Request,
        route: &RouteSpec,
        params: &HashMap<String, String>,
    ) -> Result<String, WorkerError> {
        let context = json!({
            "method": request.method,
            "target": request.target,
            "params": params,
            "request": {
                "method": request.method,
                "headers": request.headers.iter().cloned().collect::<HashMap<_, _>>(),
                "body": String::from_utf8_lossy(request.body.as_bytes()),
            },
        });
        let context = serde_json::to_string(&context).map_err(|error| {
            WorkerError::Execution(format!("context serialization failed: {error}"))
        })?;
        Ok(format!(
            r#"(function() {{
  globalThis.__tsp_pending = true;
  globalThis.__tsp_result = undefined;
  globalThis.__tsp_error = undefined;
  try {{
    const route = globalThis.__tsp_routes[{route}];
    const handler = route && (route[{method}] || route.ANY);
    const normalize = (value, context) => {{
      const response = value instanceof Response ? value : new Response(value);
      for (const cookie of context.__tsp_set_cookies || []) response.headers.push(["set-cookie", cookie]);
      return response;
    }};
    if (!handler) {{
      globalThis.__tsp_result = new Response("Method Not Allowed", {{ status: 405 }});
      globalThis.__tsp_pending = false;
    }} else {{
      const context = globalThis.__tsp_make_context(JSON.parse({context}));
      const value = handler(context);
      if (value && typeof value.then === "function") {{
        value.then((resolved) => {{ globalThis.__tsp_result = normalize(resolved, context); globalThis.__tsp_pending = false; }},
          (error) => {{ globalThis.__tsp_error = String(error); globalThis.__tsp_pending = false; }});
      }} else {{
        globalThis.__tsp_result = normalize(value, context);
        globalThis.__tsp_pending = false;
      }}
    }}
  }} catch (error) {{
    globalThis.__tsp_error = String(error && error.stack || error);
    globalThis.__tsp_pending = false;
  }}
  return "scheduled";
}})()"#,
            route = serde_json::to_string(&route.path).unwrap_or_else(|_| "\"/\"".into()),
            method = serde_json::to_string(&request.method).unwrap_or_else(|_| "\"GET\"".into()),
            context = serde_json::to_string(&context).unwrap_or_else(|_| "\"{}\"".into()),
        ))
    }
}

impl<J: JsRuntime + 'static> WorkerExecutor for RouteExecutor<J> {
    fn execute(
        &mut self,
        request: Request,
        route: RouteSpec,
        params: HashMap<String, String>,
    ) -> Result<Response, WorkerError> {
        let source = Self::dispatch_source(&request, &route, &params)?;
        self.engine
            .evaluate(&source, "tsp-request.js")
            .map_err(|error| WorkerError::Execution(error.to_string()))?;
        self.engine
            .drain_microtasks()
            .map_err(|error| WorkerError::Execution(error.to_string()))?;
        let state = self
            .engine
            .evaluate(
                "JSON.stringify({pending: !!globalThis.__tsp_pending, error: globalThis.__tsp_error || null, result: globalThis.__tsp_result || null})",
                "tsp-response.js",
            )
            .map_err(|error| WorkerError::Execution(error.to_string()))?;
        let state: serde_json::Value = serde_json::from_str(&state).map_err(|error| {
            WorkerError::Execution(format!("response JSON is invalid: {error}"))
        })?;
        if state["pending"].as_bool().unwrap_or(false) {
            return Err(WorkerError::Execution(
                "async handler did not settle at the microtask checkpoint".into(),
            ));
        }
        if let Some(error) = state["error"].as_str() {
            return Err(WorkerError::Execution(error.to_owned()));
        }
        serde_json::from_value(state["result"].clone())
            .map_err(|error| WorkerError::Execution(format!("route response is invalid: {error}")))
    }
}

struct Job {
    request: Request,
    route: RouteSpec,
    params: HashMap<String, String>,
    reply: SyncSender<Result<Response, WorkerError>>,
}

enum Command {
    Execute(Job),
    Shutdown,
}

pub struct WorkerPool {
    senders: Vec<SyncSender<Command>>,
    next_worker: AtomicUsize,
    handles: Mutex<Vec<JoinHandle<()>>>,
}

impl WorkerPool {
    pub fn new<E, F>(count: usize, factory: F) -> Result<Self, WorkerError>
    where
        E: WorkerExecutor,
        F: Fn(usize) -> E + Send + Sync + 'static,
    {
        Self::try_new(count, move |worker_id| Ok(factory(worker_id)))
    }

    pub fn try_new<E, F>(count: usize, factory: F) -> Result<Self, WorkerError>
    where
        E: WorkerExecutor,
        F: Fn(usize) -> Result<E, WorkerError> + Send + Sync + 'static,
    {
        if count == 0 {
            return Err(WorkerError::Execution(
                "worker count must be greater than zero".into(),
            ));
        }

        const WORKER_QUEUE_CAPACITY: usize = 64;
        let factory = std::sync::Arc::new(factory);
        let mut senders: Vec<SyncSender<Command>> = Vec::with_capacity(count);
        let mut handles: Vec<JoinHandle<()>> = Vec::with_capacity(count);
        for worker_id in 0..count {
            let (sender, receiver) = mpsc::sync_channel(WORKER_QUEUE_CAPACITY);
            let factory = std::sync::Arc::clone(&factory);
            let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
            let handle = thread::Builder::new()
                .name(format!("tsp-worker-{worker_id}"))
                .spawn(move || match factory(worker_id) {
                    Ok(executor) => {
                        let _ = ready_sender.send(Ok(()));
                        worker_loop(executor, receiver);
                    }
                    Err(error) => {
                        let _ = ready_sender.send(Err(error));
                    }
                })
                .map_err(|error| WorkerError::Execution(format!("worker spawn failed: {error}")))?;
            match ready_receiver.recv() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    let _ = sender.send(Command::Shutdown);
                    let _ = handle.join();
                    for previous in &senders {
                        let _ = previous.send(Command::Shutdown);
                    }
                    for previous in handles {
                        let _ = previous.join();
                    }
                    return Err(error);
                }
                Err(_) => {
                    let _ = sender.send(Command::Shutdown);
                    let _ = handle.join();
                    for previous in &senders {
                        let _ = previous.send(Command::Shutdown);
                    }
                    for previous in handles {
                        let _ = previous.join();
                    }
                    return Err(WorkerError::Execution(
                        "worker exited during initialization".into(),
                    ));
                }
            }
            senders.push(sender);
            handles.push(handle);
        }

        Ok(Self {
            senders,
            next_worker: AtomicUsize::new(0),
            handles: Mutex::new(handles),
        })
    }

    pub fn dispatch(
        &self,
        request: Request,
        route: &RouteSpec,
        params: HashMap<String, String>,
    ) -> Result<Response, WorkerError> {
        let worker_index = self.next_worker.fetch_add(1, Ordering::Relaxed) % self.senders.len();
        let (reply, response) = mpsc::sync_channel(1);
        self.senders[worker_index]
            .send(Command::Execute(Job {
                request,
                route: route.clone(),
                params,
                reply,
            }))
            .map_err(|_| WorkerError::QueueClosed)?;
        response.recv().map_err(|_| WorkerError::QueueClosed)?
    }
}

fn worker_loop<E: WorkerExecutor>(mut executor: E, receiver: Receiver<Command>) {
    while let Ok(command) = receiver.recv() {
        match command {
            Command::Execute(job) => {
                let result = executor.execute(job.request, job.route, job.params);
                let _ = job.reply.send(result);
            }
            Command::Shutdown => break,
        }
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        for sender in &self.senders {
            let _ = sender.send(Command::Shutdown);
        }
        if let Ok(handles) = self.handles.get_mut() {
            for handle in handles.drain(..) {
                let _ = handle.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoExecutor {
        worker_id: usize,
    }

    impl WorkerExecutor for EchoExecutor {
        fn execute(
            &mut self,
            request: Request,
            route: RouteSpec,
            params: HashMap<String, String>,
        ) -> Result<Response, WorkerError> {
            Ok(Response::new(
                200,
                format!(
                    "worker={} {} {} {:?}",
                    self.worker_id, request.method, route.path, params
                )
                .into_bytes(),
            ))
        }
    }

    fn route() -> RouteSpec {
        RouteSpec {
            path: "/users/:id".into(),
            source: "users/[id].tsp".into(),
            output: "users/[id].js".into(),
            methods: vec!["GET".into()],
            parameters: vec!["id".into()],
        }
    }

    #[test]
    fn jobs_are_executed_by_owned_worker_threads() {
        let pool = WorkerPool::new(2, |worker_id| EchoExecutor { worker_id }).unwrap();
        let mut params = HashMap::new();
        params.insert("id".into(), "42".into());
        let response = pool
            .dispatch(
                Request {
                    version: crate::PROTOCOL_VERSION,
                    request_id: "test-1".into(),
                    method: "GET".into(),
                    target: "/users/42".into(),
                    http_version: "HTTP/1.1".into(),
                    headers: Vec::new(),
                    body: crate::BodyEnvelope::Empty,
                },
                &route(),
                params,
            )
            .unwrap();
        assert_eq!(response.status, 200);
        assert!(
            String::from_utf8(response.body.into_bytes())
                .unwrap()
                .contains("/users/:id")
        );
    }

    #[test]
    fn zero_workers_are_rejected_before_threads_are_created() {
        let result = WorkerPool::new(0, |_| EchoExecutor { worker_id: 0 });
        assert!(matches!(
            result,
            Err(WorkerError::Execution(message)) if message == "worker count must be greater than zero"
        ));
    }

    #[test]
    fn worker_initialization_errors_are_returned_before_pool_creation() {
        let result = WorkerPool::try_new::<EchoExecutor, _>(1, |_| {
            Err(WorkerError::Execution("bundle failed to load".into()))
        });
        assert!(matches!(
            result,
            Err(WorkerError::Execution(message)) if message == "bundle failed to load"
        ));
    }
}
