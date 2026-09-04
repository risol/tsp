use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tsp_http::{Request, Response, Server, ServerLimits};
use tsp_runtime::worker::{NativeRouteExecutor, WorkerError};
use tsp_runtime::{CompiledManifest, RouteTable, WorkerPool};

#[derive(Debug)]
struct Options {
    manifest: PathBuf,
    bundle: Option<PathBuf>,
    listen: String,
    workers: usize,
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
    let bundle = Arc::new(fs::read_to_string(&bundle_path)?);
    let table = Arc::new(RouteTable::from_manifest(&manifest)?);
    let bundle_name = bundle_path.display().to_string();
    let pool = Arc::new(WorkerPool::try_new(options.workers, move |_| {
        NativeRouteExecutor::new(&bundle, &bundle_name)
    })?);
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
        match pool.dispatch(request, matched.route, matched.params) {
            Ok(response) => response,
            Err(WorkerError::QueueClosed) => Response::new(503, "worker queue closed"),
            Err(WorkerError::Execution(error)) => Response::new(500, error),
        }
    };
    let server = Server::bind(&options.listen, ServerLimits::default())?;
    println!("TSP_LISTENING {}", server.local_addr()?);
    std::io::stdout().flush()?;
    server.run(handler)?;
    Ok(())
}

impl Options {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut manifest = None;
        let mut bundle = None;
        let mut listen = "127.0.0.1:3000".to_owned();
        let mut workers = 1;
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
                "--help" | "-h" => return Err(Self::usage()),
                other => return Err(format!("unknown argument: {other}\n{}", Self::usage())),
            }
        }
        Ok(Self {
            manifest: manifest.ok_or_else(Self::usage)?,
            bundle,
            listen,
            workers,
        })
    }

    fn usage() -> String {
        "usage: tsp-cli --manifest DIR/manifest.json [--bundle DIR/bundle.js] [--listen HOST:PORT] [--workers N]".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

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
        assert!(options.bundle.is_none());
    }

    #[test]
    fn rejects_unknown_arguments() {
        let error = Options::parse(["--nope"].into_iter().map(String::from)).unwrap_err();
        assert!(error.contains("unknown argument"));
    }
}
