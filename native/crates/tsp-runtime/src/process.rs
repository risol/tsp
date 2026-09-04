//! Master-side process worker manager.
//!
//! The manager owns only pipes and serialized protocol values. A JSC VM never
//! crosses this boundary. Failed children are replaced after the last known
//! generation is loaded into the replacement.

use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use crate::{Request, Response, RouteSpec, WorkerError};
use tsp_core::{GenerationId, WORKER_PROTOCOL_VERSION, WorkerCommand, WorkerEvent};

const MAX_FRAME_BYTES: u32 = 16 * 1024 * 1024;
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(35);

struct ProcessWorker {
    child: Child,
    input: BufWriter<ChildStdin>,
    output: Option<BufReader<ChildStdout>>,
    generation: Option<(GenerationId, String, String)>,
}

impl ProcessWorker {
    fn spawn(executable: &Path) -> Result<Self, WorkerError> {
        let mut child = Command::new(executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| WorkerError::Execution(format!("worker spawn failed: {error}")))?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| WorkerError::Execution("worker stdin was not piped".into()))?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| WorkerError::Execution("worker stdout was not piped".into()))?;
        let mut worker = Self {
            child,
            input: BufWriter::new(input),
            output: Some(BufReader::new(output)),
            generation: None,
        };
        match worker.receive()? {
            WorkerEvent::Ready { version } if version == WORKER_PROTOCOL_VERSION => {}
            event => {
                return Err(WorkerError::Execution(format!(
                    "worker did not announce READY: {event:?}"
                )));
            }
        }
        worker.send(WorkerCommand::Hello {
            version: WORKER_PROTOCOL_VERSION,
        })?;
        match worker.receive()? {
            WorkerEvent::Ready { version } if version == WORKER_PROTOCOL_VERSION => Ok(worker),
            event => Err(WorkerError::Execution(format!(
                "worker rejected HELLO: {event:?}"
            ))),
        }
    }

    fn send(&mut self, command: WorkerCommand) -> Result<(), WorkerError> {
        let payload = serde_json::to_vec(&command).map_err(|error| {
            WorkerError::Execution(format!("worker command encoding failed: {error}"))
        })?;
        let length = payload
            .len()
            .checked_add(1)
            .filter(|length| *length <= MAX_FRAME_BYTES as usize)
            .ok_or_else(|| WorkerError::Execution("worker command is too large".into()))?;
        self.input
            .write_all(&(length as u32).to_be_bytes())
            .and_then(|_| self.input.write_all(&[1]))
            .and_then(|_| self.input.write_all(&payload))
            .and_then(|_| self.input.flush())
            .map_err(|error| {
                WorkerError::Execution(format!("worker command write failed: {error}"))
            })
    }

    fn receive(&mut self) -> Result<WorkerEvent, WorkerError> {
        let mut output = self
            .output
            .take()
            .ok_or_else(|| WorkerError::Execution("worker output is unavailable".into()))?;
        let result = receive_from(&mut output);
        self.output = Some(output);
        result
    }

    fn receive_timeout(&mut self, timeout: Duration) -> Result<WorkerEvent, WorkerError> {
        let mut output = self
            .output
            .take()
            .ok_or_else(|| WorkerError::Execution("worker output is unavailable".into()))?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let reader = std::thread::spawn(move || {
            let result = receive_from(&mut output);
            let _ = sender.send((result, output));
        });
        match receiver.recv_timeout(timeout) {
            Ok((result, output)) => {
                self.output = Some(output);
                let _ = reader.join();
                result
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = self.child.kill();
                let _ = self.child.wait();
                let _ = reader.join();
                self.output = None;
                Err(WorkerError::Timeout)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = reader.join();
                self.output = None;
                Err(WorkerError::Execution(
                    "worker response reader stopped".into(),
                ))
            }
        }
    }

    fn execute(
        &mut self,
        request: Request,
        route: RouteSpec,
        params: HashMap<String, String>,
        timeout: Duration,
    ) -> Result<Response, WorkerError> {
        let request_id = request.request_id.clone();
        self.send(WorkerCommand::Execute {
            request: Box::new(request),
            route,
            params,
        })?;
        match self.receive_timeout(timeout)? {
            WorkerEvent::Result(response) => Ok(response),
            WorkerEvent::Error { message, .. } if message == "request execution timed out" => {
                Err(WorkerError::Timeout)
            }
            WorkerEvent::Error { message, .. } => Err(WorkerError::Execution(message)),
            event => Err(WorkerError::Execution(format!(
                "worker did not return result for {request_id}: {event:?}"
            ))),
        }
    }

    fn is_alive(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }
}

fn receive_from(output: &mut BufReader<ChildStdout>) -> Result<WorkerEvent, WorkerError> {
    let mut length_bytes = [0; 4];
    output
        .read_exact(&mut length_bytes)
        .map_err(|error| WorkerError::Execution(format!("worker response read failed: {error}")))?;
    let length = u32::from_be_bytes(length_bytes);
    if !(1..=MAX_FRAME_BYTES).contains(&length) {
        return Err(WorkerError::Execution(
            "worker response length is invalid".into(),
        ));
    }
    let mut bytes = vec![0; length as usize];
    output
        .read_exact(&mut bytes)
        .map_err(|error| WorkerError::Execution(format!("worker response read failed: {error}")))?;
    if bytes[0] != 2 {
        return Err(WorkerError::Execution(
            "worker response kind is invalid".into(),
        ));
    }
    serde_json::from_slice(&bytes[1..])
        .map_err(|error| WorkerError::Execution(format!("worker response is invalid: {error}")))
}

impl ProcessWorker {
    fn load_generation(
        &mut self,
        generation: GenerationId,
        bundle: &str,
        filename: &str,
    ) -> Result<(), WorkerError> {
        self.send(WorkerCommand::LoadGeneration {
            generation,
            bundle: bundle.to_owned(),
            filename: filename.to_owned(),
        })?;
        match self.receive()? {
            WorkerEvent::GenerationReady { generation: actual } if actual == generation => {
                self.generation = Some((generation, bundle.to_owned(), filename.to_owned()));
                Ok(())
            }
            WorkerEvent::Error { message, .. } => Err(WorkerError::Execution(message)),
            event => Err(WorkerError::Execution(format!(
                "worker generation load failed: {event:?}"
            ))),
        }
    }
}

impl Drop for ProcessWorker {
    fn drop(&mut self) {
        let _ = self.send(WorkerCommand::Shutdown);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct ProcessWorkerManager {
    executable: PathBuf,
    workers: Vec<Mutex<ProcessWorker>>,
    loads: Vec<AtomicUsize>,
    generation: Mutex<Option<(GenerationId, String, String)>>,
    request_timeout: Duration,
}

impl ProcessWorkerManager {
    pub fn new(executable: impl AsRef<Path>, count: usize) -> Result<Self, WorkerError> {
        Self::with_timeout(executable, count, DEFAULT_REQUEST_TIMEOUT)
    }

    pub fn with_timeout(
        executable: impl AsRef<Path>,
        count: usize,
        request_timeout: Duration,
    ) -> Result<Self, WorkerError> {
        if count == 0 {
            return Err(WorkerError::Execution(
                "worker count must be greater than zero".into(),
            ));
        }
        let executable = executable.as_ref().to_owned();
        let mut workers = Vec::with_capacity(count);
        for _ in 0..count {
            workers.push(Mutex::new(ProcessWorker::spawn(&executable)?));
        }
        Ok(Self {
            executable,
            workers,
            loads: (0..count).map(|_| AtomicUsize::new(0)).collect(),
            generation: Mutex::new(None),
            request_timeout,
        })
    }

    pub fn load_generation(
        &self,
        generation: GenerationId,
        bundle: &str,
        filename: &str,
    ) -> Result<(), WorkerError> {
        for worker in &self.workers {
            worker
                .lock()
                .map_err(|_| WorkerError::Execution("worker lock poisoned".into()))?
                .load_generation(generation, bundle, filename)?;
        }
        *self
            .generation
            .lock()
            .map_err(|_| WorkerError::Execution("generation lock poisoned".into()))? =
            Some((generation, bundle.to_owned(), filename.to_owned()));
        Ok(())
    }

    pub fn dispatch(
        &self,
        request: Request,
        route: &RouteSpec,
        params: HashMap<String, String>,
    ) -> Result<Response, WorkerError> {
        let worker_index = self
            .loads
            .iter()
            .enumerate()
            .min_by_key(|(_, load)| load.load(Ordering::Acquire))
            .map(|(index, _)| index)
            .expect("worker list is non-empty");
        let _load = LoadGuard(&self.loads[worker_index]);
        let mut worker = self.workers[worker_index]
            .lock()
            .map_err(|_| WorkerError::Execution("worker lock poisoned".into()))?;
        if !worker.is_alive() {
            self.replace_worker(&mut worker)?;
        }
        match worker.execute(request, route.clone(), params, self.request_timeout) {
            Ok(response) => Ok(response),
            Err(error) if !worker.is_alive() => {
                let _ = self.replace_worker(&mut worker);
                Err(error)
            }
            Err(error) => Err(error),
        }
    }

    fn replace_worker(&self, worker: &mut ProcessWorker) -> Result<(), WorkerError> {
        let generation = self
            .generation
            .lock()
            .map_err(|_| WorkerError::Execution("generation lock poisoned".into()))?
            .clone();
        *worker = ProcessWorker::spawn(&self.executable)?;
        if let Some((id, bundle, filename)) = generation {
            worker.load_generation(id, &bundle, &filename)?;
        }
        Ok(())
    }
}

struct LoadGuard<'a>(&'a AtomicUsize);

impl Drop for LoadGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_limit_is_bounded() {
        assert_eq!(MAX_FRAME_BYTES, 16 * 1024 * 1024);
    }
}
