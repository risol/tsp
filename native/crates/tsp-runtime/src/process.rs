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

use crate::{Request, Response, RouteSpec, WorkerError};
use tsp_core::{GenerationId, WORKER_PROTOCOL_VERSION, WorkerCommand, WorkerEvent};

const MAX_FRAME_BYTES: u32 = 16 * 1024 * 1024;

struct ProcessWorker {
    child: Child,
    input: BufWriter<ChildStdin>,
    output: BufReader<ChildStdout>,
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
            output: BufReader::new(output),
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
        let mut length_bytes = [0; 4];
        self.output.read_exact(&mut length_bytes).map_err(|error| {
            WorkerError::Execution(format!("worker response read failed: {error}"))
        })?;
        let length = u32::from_be_bytes(length_bytes);
        if !(1..=MAX_FRAME_BYTES).contains(&length) {
            return Err(WorkerError::Execution(
                "worker response length is invalid".into(),
            ));
        }
        let mut bytes = vec![0; length as usize];
        self.output.read_exact(&mut bytes).map_err(|error| {
            WorkerError::Execution(format!("worker response read failed: {error}"))
        })?;
        if bytes[0] != 2 {
            return Err(WorkerError::Execution(
                "worker response kind is invalid".into(),
            ));
        }
        serde_json::from_slice(&bytes[1..])
            .map_err(|error| WorkerError::Execution(format!("worker response is invalid: {error}")))
    }

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

    fn execute(
        &mut self,
        request: Request,
        route: RouteSpec,
        params: HashMap<String, String>,
    ) -> Result<Response, WorkerError> {
        let request_id = request.request_id.clone();
        self.send(WorkerCommand::Execute {
            request,
            route,
            params,
        })?;
        match self.receive()? {
            WorkerEvent::Result(response) => Ok(response),
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
    next_worker: AtomicUsize,
    generation: Mutex<Option<(GenerationId, String, String)>>,
}

impl ProcessWorkerManager {
    pub fn new(executable: impl AsRef<Path>, count: usize) -> Result<Self, WorkerError> {
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
            next_worker: AtomicUsize::new(0),
            generation: Mutex::new(None),
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
        let worker_index = self.next_worker.fetch_add(1, Ordering::Relaxed) % self.workers.len();
        let mut worker = self.workers[worker_index]
            .lock()
            .map_err(|_| WorkerError::Execution("worker lock poisoned".into()))?;
        match worker.execute(request.clone(), route.clone(), params.clone()) {
            Ok(response) => Ok(response),
            Err(error) if !worker.is_alive() => {
                let generation = self
                    .generation
                    .lock()
                    .map_err(|_| WorkerError::Execution("generation lock poisoned".into()))?
                    .clone();
                *worker = ProcessWorker::spawn(&self.executable)?;
                if let Some((id, bundle, filename)) = generation {
                    worker.load_generation(id, &bundle, &filename)?;
                    worker.execute(request, route.clone(), params)
                } else {
                    Err(error)
                }
            }
            Err(error) => Err(error),
        }
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
