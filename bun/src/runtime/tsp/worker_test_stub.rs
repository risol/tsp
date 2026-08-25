//! Protocol-only worker used by Worker Manager integration tests.
//!
//! This binary exercises the master process boundary without initializing
//! Bun. The real embedded worker is covered by the v2.4 smoke test.

#[path = "worker/protocol.rs"]
#[allow(unreachable_pub, dead_code)]
mod protocol;

use protocol::{ExecuteResponse, Message, ProtocolError};
use std::io::{Read, Write};

fn main() -> std::process::ExitCode {
    if std::env::args().any(|argument| argument == "--tsp-worker") {
        return run();
    }
    std::process::ExitCode::from(2)
}

fn run() -> std::process::ExitCode {
    #[cfg(unix)]
    {
        let Some(path) = std::env::var_os("TSP_WORKER_SOCKET") else {
            return std::process::ExitCode::from(2);
        };
        let Ok(stream) = std::os::unix::net::UnixStream::connect(path) else {
            return std::process::ExitCode::from(2);
        };
        return serve(stream);
    }
    #[cfg(not(unix))]
    {
        let Ok(endpoint) = std::env::var("TSP_WORKER_SOCKET") else {
            return std::process::ExitCode::from(2);
        };
        let Ok(stream) = std::net::TcpStream::connect(endpoint) else {
            return std::process::ExitCode::from(2);
        };
        return serve(stream);
    }
}

fn serve<S>(mut stream: S) -> std::process::ExitCode
where
    S: Read + Write,
{
    if !matches!(Message::read_from(&mut stream), Ok(Message::Hello)) {
        return std::process::ExitCode::from(2);
    }
    if (Message::Ready {
        worker_id: std::process::id() as u64,
    })
    .write_to(&mut stream)
    .is_err()
    {
        return std::process::ExitCode::from(2);
    }

    loop {
        let message = match Message::read_from(&mut stream) {
            Ok(message) => message,
            Err(ProtocolError::Truncated) => return std::process::ExitCode::SUCCESS,
            Err(_) => return std::process::ExitCode::from(2),
        };
        match message {
            Message::Execute { id, request } => {
                let script = String::from_utf8_lossy(&request.script);
                if request.path.contains("crash") || script.contains("__TSP_TEST_CRASH__") {
                    std::process::exit(17);
                }
                if request.path.contains("sleep") || script.contains("__TSP_TEST_SLEEP__") {
                    std::thread::sleep(std::time::Duration::from_millis(250));
                }
                let body = if request.script.is_empty() {
                    request.path.into_bytes()
                } else {
                    request.script
                };
                if (Message::Response {
                    id,
                    response: ExecuteResponse {
                        status: 200,
                        headers: Vec::new(),
                        body,
                    },
                })
                .write_to(&mut stream)
                .is_err()
                {
                    return std::process::ExitCode::from(2);
                }
            }
            Message::Heartbeat { id } => {
                if (Message::Heartbeat { id }).write_to(&mut stream).is_err() {
                    return std::process::ExitCode::from(2);
                }
            }
            Message::Cancel { .. } => {}
            Message::Shutdown => return std::process::ExitCode::SUCCESS,
            _ => return std::process::ExitCode::from(2),
        }
    }
}
