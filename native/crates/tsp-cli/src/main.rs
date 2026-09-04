use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tsp_core::{CompiledManifest, ErrorEnvelope, Request, Response, RouteTable};
use tsp_http::{Server, ServerLimits};
use tsp_runtime::{GenerationRegistry, ProcessWorkerManager, WorkerError};

#[derive(Debug)]
struct Options {
    manifest: PathBuf,
    bundle: Option<PathBuf>,
    listen: String,
    workers: usize,
    request_timeout: Duration,
    worker: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse(env::args().skip(1))?;
    let manifest_path = fs::canonicalize(&options.manifest)?;
    let manifest: CompiledManifest =
        CompiledManifest::from_json(&fs::read_to_string(&manifest_path)?)?;
    let bundle_path = options.bundle.unwrap_or_else(|| {
        manifest_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("bundle.js")
    });
    let bundle = fs::read_to_string(&bundle_path)?;
    let table = Arc::new(RouteTable::from_manifest(&manifest)?);
    let registry = Arc::new(GenerationRegistry::new());
    let generation = registry.publish(bundle, bundle_path.display().to_string())?;
    let worker_path = options.worker.unwrap_or_else(default_worker_path);
    let workers = Arc::new(ProcessWorkerManager::with_timeout(
        worker_path,
        options.workers,
        options.request_timeout,
    )?);
    workers.load_generation(generation.id, &generation.bundle, &generation.filename)?;
    let handler = move |request: Request| {
        let pathname = request
            .target
            .split_once('?')
            .map_or(request.target.as_str(), |(path, _)| path);
        let Some(matched) = table.lookup(pathname) else {
            return Response::new(404, "Not Found");
        };
        if matched
            .route
            .methods
            .iter()
            .all(|method| method != &request.method && method != "ANY")
        {
            return Response::new(405, "Method Not Allowed");
        }
        let mut request = request;
        if registry.pin(&mut request).is_none() {
            return Response::new(503, "no application generation is loaded");
        }
        match workers.dispatch(request, matched.route, matched.params) {
            Ok(response) => response,
            Err(WorkerError::QueueClosed) => error_response(503, "worker queue closed", "TSP3001"),
            Err(WorkerError::Timeout) => {
                error_response(504, "request execution timed out", "TSP3002")
            }
            Err(WorkerError::Execution(_)) => {
                error_response(500, "internal server error", "TSP3000")
            }
        }
    };
    let server = Server::bind(&options.listen, ServerLimits::default())?;
    println!("TSP_LISTENING {}", server.local_addr()?);
    std::io::stdout().flush()?;
    server.run(handler)?;
    Ok(())
}

fn error_response(status: u16, message: &str, code: &str) -> Response {
    let mut response = Response::new(status, message);
    response.error = Some(ErrorEnvelope {
        code: code.into(),
        kind: "RuntimeError".into(),
        message: message.into(),
    });
    response
}

fn default_worker_path() -> PathBuf {
    let mut path = env::current_exe().unwrap_or_else(|_| PathBuf::from("tsp-cli"));
    path.set_file_name(if cfg!(windows) {
        "tsp-worker.exe"
    } else {
        "tsp-worker"
    });
    path
}

impl Options {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut manifest = None;
        let mut bundle = None;
        let mut listen = "127.0.0.1:3000".to_owned();
        let mut workers = 1;
        let mut request_timeout = Duration::from_secs(35);
        let mut worker = None;
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--manifest" => {
                    manifest = Some(PathBuf::from(args.next().ok_or("--manifest needs a path")?))
                }
                "--bundle" => {
                    bundle = Some(PathBuf::from(args.next().ok_or("--bundle needs a path")?))
                }
                "--listen" => listen = args.next().ok_or("--listen needs an address")?,
                "--workers" => {
                    workers = args
                        .next()
                        .ok_or("--workers needs a number")?
                        .parse()
                        .map_err(|_| "--workers must be a positive number")?
                }
                "--request-timeout-ms" => {
                    let milliseconds = args
                        .next()
                        .ok_or("--request-timeout-ms needs a number")?
                        .parse::<u64>()
                        .map_err(|_| "--request-timeout-ms must be a positive number")?;
                    if milliseconds == 0 {
                        return Err("--request-timeout-ms must be a positive number".into());
                    }
                    request_timeout = Duration::from_millis(milliseconds);
                }
                "--worker" => {
                    worker = Some(PathBuf::from(args.next().ok_or("--worker needs a path")?))
                }
                "--help" | "-h" => return Err(Self::usage()),
                other => return Err(format!("unknown argument: {other}\n{}", Self::usage())),
            }
        }
        Ok(Self {
            manifest: manifest.ok_or_else(Self::usage)?,
            bundle,
            listen,
            workers,
            request_timeout,
            worker,
        })
    }

    fn usage() -> String {
        "usage: tsp-cli --manifest DIR/manifest.json [--bundle DIR/bundle.js] [--listen HOST:PORT] [--workers N] [--request-timeout-ms N] [--worker PATH]".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use super::Options;

    #[test]
    fn parses_required_manifest_and_runtime_defaults() {
        let options = Options::parse(
            ["--manifest", "manifest.json"]
                .into_iter()
                .map(String::from),
        )
        .unwrap();
        assert_eq!(options.manifest, Path::new("manifest.json"));
        assert_eq!(options.listen, "127.0.0.1:3000");
        assert_eq!(options.workers, 1);
        assert_eq!(options.request_timeout, Duration::from_secs(35));
        assert!(options.bundle.is_none());
    }

    #[test]
    fn rejects_unknown_arguments() {
        let error = Options::parse(["--nope"].into_iter().map(String::from)).unwrap_err();
        assert!(error.contains("unknown argument"));
    }

    #[test]
    fn parses_request_timeout() {
        let options = Options::parse(
            ["--manifest", "manifest.json", "--request-timeout-ms", "250"]
                .into_iter()
                .map(String::from),
        )
        .unwrap();
        assert_eq!(options.request_timeout, Duration::from_millis(250));
    }
}
