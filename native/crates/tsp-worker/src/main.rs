use std::io::{self, BufReader, BufWriter};

use tsp_core::{WORKER_PROTOCOL_VERSION, WorkerCommand, WorkerEvent};
use tsp_jsc::{Engine, NativeBackend};
use tsp_runtime::worker::{RouteExecutor, WorkerError, WorkerExecutor};

use tsp_worker::{json_frame, kind, parse_json, read_frame, write_frame};

fn send_event(writer: &mut BufWriter<impl io::Write>, event: &WorkerEvent) -> io::Result<()> {
    let frame = json_frame(kind::EVENT, event)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_frame(writer, &frame)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());
    send_event(
        &mut writer,
        &WorkerEvent::Ready {
            version: WORKER_PROTOCOL_VERSION,
        },
    )?;

    let mut executor: Option<RouteExecutor<Engine<NativeBackend>>> = None;
    while let Some(frame) = read_frame(&mut reader)? {
        if frame.kind != kind::COMMAND {
            send_event(
                &mut writer,
                &WorkerEvent::Error {
                    request_id: None,
                    message: "unexpected worker frame kind".into(),
                },
            )?;
            continue;
        }
        let command: WorkerCommand = match parse_json(&frame) {
            Ok(command) => command,
            Err(error) => {
                send_event(
                    &mut writer,
                    &WorkerEvent::Error {
                        request_id: None,
                        message: format!("invalid worker command: {error}"),
                    },
                )?;
                continue;
            }
        };
        match command {
            WorkerCommand::Hello { version } if version == WORKER_PROTOCOL_VERSION => {
                send_event(
                    &mut writer,
                    &WorkerEvent::Ready {
                        version: WORKER_PROTOCOL_VERSION,
                    },
                )?;
            }
            WorkerCommand::Hello { version } => {
                send_event(
                    &mut writer,
                    &WorkerEvent::Error {
                        request_id: None,
                        message: format!("unsupported worker protocol version {version}"),
                    },
                )?;
            }
            WorkerCommand::LoadGeneration {
                generation,
                bundle,
                filename,
            } => {
                let loaded = NativeBackend::new()
                    .map_err(|error| WorkerError::Execution(error.to_string()))
                    .and_then(|backend| {
                        RouteExecutor::new(Engine::new(backend), &bundle, &filename)
                    });
                match loaded {
                    Ok(loaded) => {
                        executor = Some(loaded);
                        send_event(&mut writer, &WorkerEvent::GenerationReady { generation })?;
                    }
                    Err(error) => send_event(
                        &mut writer,
                        &WorkerEvent::Error {
                            request_id: None,
                            message: error.to_string(),
                        },
                    )?,
                }
            }
            WorkerCommand::Execute {
                request,
                route,
                params,
            } => {
                let request_id = Some(request.request_id.clone());
                let event = match executor.as_mut() {
                    Some(executor) => match executor.execute(*request, route, params) {
                        Ok(response) => WorkerEvent::Result(response),
                        Err(error) => WorkerEvent::Error {
                            request_id,
                            message: error.to_string(),
                        },
                    },
                    None => WorkerEvent::Error {
                        request_id,
                        message: "no generation is loaded".into(),
                    },
                };
                send_event(&mut writer, &event)?;
            }
            WorkerCommand::Cancel { request_id } => {
                send_event(
                    &mut writer,
                    &WorkerEvent::Error {
                        request_id: Some(request_id),
                        message: "request cancellation is not active".into(),
                    },
                )?;
            }
            WorkerCommand::Ping => send_event(&mut writer, &WorkerEvent::Pong)?,
            WorkerCommand::Shutdown => {
                send_event(&mut writer, &WorkerEvent::Exiting)?;
                break;
            }
        }
    }
    Ok(())
}
