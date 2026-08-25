//! TSP v2 native host binary entry point (PoC 1, slice 11b).
//!
//! See `tsp-v2-plan.md` sect.70 (PoC 1) and `tsp-v2-specification.md` for
//! the contract this binary will eventually implement. v1's `src/main.ts`
//! remains the default working server; this binary is the side-by-side
//! v2 host.
//!
//! Slice 10b boots a `PageRegistry`, walks the `routes/` directory
//! once to discover exported HTTP methods (via `page::prepare`),
//! registers one `PageSlot` per (route, method) pair, then
//! hands the registry to the listener.
//!
//! Slice 11b spawns the watcher thread (plan sect.22): changes under
//! the routes root mark all slots dirty, and the next request rebuilds
//! the page (lazy reload). The watcher handle is kept alive for the
//! lifetime of `serve`; its Drop impl stops + joins the thread.
//!
//! Slice 15a extends the watcher with hot-reload of the route table:
//! when a `.tsp` file is added or removed under the routes root, the
//! watcher diffs the freshly-scanned view against the live
//! `RouteTable` and `PageRegistry`, applying additions and removals
//! (spec sect.12 + sect.33.5). Both the host and the watcher hold
//! independent `Arc<RouteTable>` handles into the same backing
//! storage; the bin no longer needs to `Box::leak` the table.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use bun_runtime_tsp::generation::{PageRef, PageRegistry};
use bun_runtime_tsp::host;
use bun_runtime_tsp::jsc_bridge::{self, BunRuntime, PersistentBunWorker};
use bun_runtime_tsp::module_graph::ModuleGraph;
use bun_runtime_tsp::router::RouteTable;
use bun_runtime_tsp::services::SESSION_STORE_CAP_DEFAULT;
use bun_runtime_tsp::services::ServiceRegistry;
use bun_runtime_tsp::session_backend::{MemoryBackend, RedisBackend, SessionBackend};
use bun_runtime_tsp::watcher::{self, WatchConfig};
use bun_runtime_tsp::worker::pool::WorkerPool;
use bun_runtime_tsp::worker::lifecycle::RecyclePolicy;
use bun_runtime_tsp::worker::sandbox::ResourceLimits;
use bun_runtime_tsp::worker::application::{Application, ApplicationRegistry, WorkerGroup};

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("check") => run_check(),
        Some("routes") => run_routes(),
        Some("graph") => run_graph(),
        Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        _ => serve_main(),
    }
}

fn serve_main() -> ExitCode {
    let port = match host::resolve_port() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("TSPv2PoC1: {e}");
            return ExitCode::from(2);
        }
    };

    let routes_dir = resolve_routes_dir();
    eprintln!("TSPv2PoC1: scanning routes from {}", routes_dir.display());
    // Slice 15a: the RouteTable is now Arc<Mutex<...>> under
    // the hood so the watcher can add/remove routes while
    // requests are in flight. We keep the boot-time scan
    // result in an Arc; the host and the watcher each
    // receive a clone of that Arc (cheap, since the inner
    // Arc<Mutex<...>> is shared).
    let routes: Arc<RouteTable> = match RouteTable::scan(&routes_dir) {
        Ok(table) => Arc::new(table),
        Err(e) => {
            eprintln!("TSPv2PoC1: {e}");
            return ExitCode::from(2);
        }
    };
    eprintln!("TSPv2PoC1: loaded {} route(s)", routes.len());

    // Boot-time preparation: for each route, run the slice-5
    // static method detector so we know which methods each file
    // exports. Register one PageSlot per (route, method) pair
    // in the PageRegistry. The host then consults the registry
    // per request instead of re-detecting on every call.
    let registry: &'static PageRegistry = match build_registry(&routes) {
        Ok(r) => leak_registry(r),
        Err(e) => {
            eprintln!("TSPv2PoC1: {e}");
            return ExitCode::from(2);
        }
    };

    let embedded_worker_enabled = matches!(
        std::env::var("TSP_EMBEDDED_WORKER").as_deref(),
        Ok("1") | Ok("true")
    );
    let bun: &'static BunRuntime = if embedded_worker_enabled {
        let worker_binary = match resolve_worker_bin() {
            Ok(path) => path,
            Err(error) => {
                eprintln!("TSPv2PoC1: {error}");
                return ExitCode::from(2);
            }
        };
        let socket_dir = std::env::temp_dir().join(format!("tsp-v24-workers-{}", std::process::id()));
        if let Err(error) = std::fs::create_dir_all(&socket_dir) {
            eprintln!("TSPv2PoC1: cannot create worker socket directory: {error}");
            return ExitCode::from(2);
        }
        let worker_count = std::env::var("TSP_WORKER_COUNT")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1)
            .max(1);
        let max_in_flight = std::env::var("TSP_WORKER_MAX_IN_FLIGHT")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(worker_count * 2)
            .max(worker_count);
        let recycle_policy = RecyclePolicy {
            max_requests: env_u64("TSP_WORKER_MAX_REQUESTS"),
            max_age: env_u64("TSP_WORKER_MAX_AGE_MS").map(std::time::Duration::from_millis),
            max_memory_bytes: env_u64("TSP_WORKER_MAX_MEMORY_BYTES"),
        };
        let resource_limits = ResourceLimits {
            cgroup_root: std::env::var_os("TSP_CGROUP_ROOT").map(PathBuf::from),
            memory_max: env_u64("TSP_WORKER_MEMORY_MAX"),
            cpu_max: std::env::var("TSP_WORKER_CPU_MAX").ok(),
            pids_max: env_u64("TSP_WORKER_PIDS_MAX"),
        };
        let pool = WorkerPool::new(worker_binary.clone(), socket_dir, worker_count, max_in_flight)
            .with_recycle_policy(recycle_policy)
            .with_resource_limits(resource_limits);
        let pool = Arc::new(pool);
        let application_name = std::env::var("TSP_APPLICATION_NAME").unwrap_or_else(|_| "main".into());
        ApplicationRegistry::global().register(Application::new(
            application_name,
            WorkerGroup::new(Arc::clone(&pool)),
        ));
        if let Err(error) = pool.start() {
            eprintln!("TSPv2PoC1: embedded worker failed to start: {error}");
            return ExitCode::from(2);
        }
        eprintln!(
            "TSPv2PoC1: v2.4 embedded worker enabled ({})",
            worker_binary.display()
        );
        leak_bun(BunRuntime {
            bin: worker_binary,
            persistent_worker: None,
            embedded_worker: None,
            embedded_pool: Some(pool),
        })
    } else {
        match jsc_bridge::resolve_bun_bin() {
        Ok(p) => {
            let persistent_worker = match std::env::var("TSP_PERSISTENT_WORKER").as_deref() {
                Ok("1") | Ok("true") => {
                    match PersistentBunWorker::new(p.clone(), Some(routes_dir.clone())) {
                    Ok(worker) => {
                        eprintln!("TSPv2PoC1: persistent Bun worker enabled");
                        Some(Arc::new(worker))
                    }
                    Err(e) => {
                        eprintln!("TSPv2PoC1: persistent Bun worker failed to start: {e}");
                        return ExitCode::from(2);
                    }
                    }
                }
                _ => None,
            };
            leak_bun(BunRuntime { bin: p, persistent_worker, embedded_worker: None, embedded_pool: None })
        }
        Err(e) => {
            eprintln!("TSPv2PoC1: {e}");
            return ExitCode::from(2);
        }
        }
    };
    eprintln!("TSPv2PoC1: worker executable = {}", bun.bin.display());

    // Slice 16j (Phase 8): the host-owned ServiceRegistry.
    // `with_defaults` registers the runtime-scoped logger
    // (spec sect.17). It is Box::leak'ed like the PageRegistry
    // so every connection thread shares the same instance; it
    // is never owned by a page generation, so reloads do not
    // tear services down (plan sect.61 Phase 8 acceptance).
    //
    // Slice 16l: the session backend is selected by
    // `TSP_REDIS_URL`. When set, a hand-rolled RESP
    // `RedisBackend` carries every session; otherwise
    // the host falls back to the in-process `MemoryBackend`
    // (16k default). A missing / unreachable Redis is
    // NOT fatal -- the backend's `is_available()` flag
    // stays false, every `lookup` returns `None` (so the
    // host mints a fresh session), and the next command
    // self-heals once Redis is reachable again.
    let session_backend: Arc<dyn SessionBackend> = match std::env::var("TSP_REDIS_URL") {
        Ok(url) if !url.is_empty() => match RedisBackend::with_default_ttl(&url) {
            Ok(backend) => {
                let available = backend.is_available();
                let arc: Arc<dyn SessionBackend> = Arc::new(backend);
                eprintln!("TSPv2PoC1: session backend = redis (url={url}, available={available})");
                arc
            }
            Err(e) => {
                eprintln!("TSPv2PoC1: TSP_REDIS_URL parse failed ({e}); falling back to memory");
                Arc::new(MemoryBackend::new(SESSION_STORE_CAP_DEFAULT))
            }
        },
        _ => {
            eprintln!("TSPv2PoC1: session backend = memory (cap={SESSION_STORE_CAP_DEFAULT})");
            Arc::new(MemoryBackend::new(SESSION_STORE_CAP_DEFAULT))
        }
    };
    let services: &'static ServiceRegistry =
        Box::leak(Box::new(ServiceRegistry::with_backends(session_backend)));
    eprintln!(
        "TSPv2PoC1: services registered: {}",
        services
            .snapshot(&[])
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Slice 11b: build the module graph (frozen at boot; slice 12
    // lets the watcher pick up new routes without a restart) and
    // spawn the watcher thread. Any file change under the routes
    // root marks every registered slot dirty; the next request
    // triggers the rebuild. The handle lives as long as `serve` and
    // joins the thread on drop.
    let graph = Arc::new(match ModuleGraph::from_routes_dir(&routes_dir) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("TSPv2PoC1: build module graph: {e}");
            return ExitCode::from(2);
        }
    });
    let registry_arc = Arc::new(registry.clone());
    let routes_for_watcher = Arc::clone(&routes);
    let watch_config = WatchConfig {
        routes_root: routes_dir.clone(),
        poll_ms: watcher::DEFAULT_POLL_MS,
    };
    let watcher_handle = watcher::spawn(watch_config, graph, routes_for_watcher, registry_arc);
    eprintln!(
        "TSPv2PoC1: watcher polling {} every {}ms",
        routes_dir.display(),
        watcher::DEFAULT_POLL_MS
    );

    if let Err(e) = host::serve("0.0.0.0", port, routes, registry, bun, services) {
        // serve returns only on a fatal listener error; dropping
        // watcher_handle here stops + joins the watcher thread.
        drop(watcher_handle);
        eprintln!("TSPv2PoC1: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn resolve_routes_dir() -> PathBuf {
    match std::env::var("TSP_ROUTES_DIR") {
        Ok(s) => PathBuf::from(s),
        Err(_) => PathBuf::from("routes"),
    }
}

fn print_help() {
    println!(
        "TSP v2 commands:\n  tspserver_v2              run the native HTTP server\n  tspserver_v2 check       validate routes and local imports\n  tspserver_v2 routes      list filesystem routes and exports\n  tspserver_v2 graph       print the resolved module graph\n\nEnvironment:\n  TSP_ROUTES_DIR       route root (default: routes)\n  TSP_PUBLIC_DIR       public asset root (default: public)\n  TSP_PORT             HTTP port (default: 3000)\n  TSP_BUN_BIN          legacy one-shot Bun runtime path\n  TSP_PERSISTENT_WORKER legacy external Bun worker\n  TSP_EMBEDDED_WORKER  enable v2.4 Rust worker with embedded Bun\n  TSP_WORKER_BIN       Bun executable containing the embedded worker\n  TSP_INVALIDATION_FILE shared cross-worker invalidation log\n  TSP_WORKER_MAX_REQUESTS recycle after N requests\n  TSP_WORKER_MAX_AGE_MS recycle after this many milliseconds\n  TSP_WORKER_MAX_MEMORY_BYTES recycle after RSS reaches this limit\n  TSP_CGROUP_ROOT explicit Linux cgroup v2 parent directory\n  TSP_WORKER_MEMORY_MAX / TSP_WORKER_CPU_MAX / TSP_WORKER_PIDS_MAX cgroup limits"
    );
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()?
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
}

fn resolve_worker_bin() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("TSP_WORKER_BIN") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!("TSP_WORKER_BIN does not point to a file: {}", path.display()));
    }
    let executable_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from));
    let filename = if cfg!(windows) { "bun-debug.exe" } else { "bun-debug" };
    if let Some(dir) = executable_dir {
        let candidate = dir.join(filename);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err("embedded worker executable not found; set TSP_WORKER_BIN".into())
}

fn run_routes() -> ExitCode {
    let root = resolve_routes_dir();
    let table = match RouteTable::scan(&root) {
        Ok(table) => table,
        Err(error) => {
            eprintln!("tsp routes: {error}");
            return ExitCode::from(2);
        }
    };
    for route in table.iter() {
        match bun_runtime_tsp::page::prepare(&route) {
            Ok(page) => println!(
                "{}\t{}\t{}",
                route.path,
                route.source.display(),
                page.methods.iter().map(|m| m.as_str()).collect::<Vec<_>>().join(",")
            ),
            Err(error) => println!("{}\t{}\tERROR: {error}", route.path, route.source.display()),
        }
    }
    ExitCode::SUCCESS
}

fn run_check() -> ExitCode {
    let root = resolve_routes_dir();
    let table = match RouteTable::scan(&root) {
        Ok(table) => table,
        Err(error) => {
            eprintln!("tsp check: {error}");
            return ExitCode::from(2);
        }
    };
    let mut failed = false;
    for route in table.iter() {
        match bun_runtime_tsp::page::prepare(&route) {
            Ok(page) => println!(
                "OK {} [{}]",
                route.path,
                page.methods.iter().map(|m| m.as_str()).collect::<Vec<_>>().join(",")
            ),
            Err(error) => {
                failed = true;
                eprintln!("ERROR {}: {error}", route.source.display());
            }
        }
    }
    match ModuleGraph::from_routes_dir(&root) {
        Ok(graph) => println!("OK module graph: {} module(s)", graph.len()),
        Err(error) => {
            failed = true;
            eprintln!("ERROR module graph: {error}");
        }
    }
    if failed { ExitCode::from(1) } else { ExitCode::SUCCESS }
}

fn run_graph() -> ExitCode {
    let root = resolve_routes_dir();
    let graph = match ModuleGraph::from_routes_dir(&root) {
        Ok(graph) => graph,
        Err(error) => {
            eprintln!("tsp graph: {error}");
            return ExitCode::from(2);
        }
    };
    let mut nodes = graph.nodes();
    nodes.sort_by(|a, b| a.path.cmp(&b.path));
    for node in nodes {
        let imports = node
            .imports
            .iter()
            .map(|id| id.as_path().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(",");
        println!("{}\timports=[{}]", node.path.display(), imports);
    }
    ExitCode::SUCCESS
}

/// Walk the RouteTable and create one PageRef per HTTP method
/// the source file actually exports. We read the source once
/// at boot (instead of per request) so the registry's
/// `state` is meaningful from the first call.
fn build_registry(routes: &RouteTable) -> Result<PageRegistry, String> {
    let registry = PageRegistry::new();
    for route in routes.iter() {
        let source = match bun_runtime_tsp::page::prepare(&route) {
            Ok(s) => s,
            Err(e) => return Err(format!("prepare {}: {e}", route.source.display())),
        };
        // RouteTable::scan cannot depend on page parsing without creating a
        // module cycle, so it initially carries the broad REAL method set.
        // Align the live route metadata with the actual exports before the
        // server starts; the watcher applies the same normalization for
        // newly added or changed route files.
        let mut normalized_route = route.clone();
        normalized_route.methods = source.methods.clone();
        routes.replace_by_path(normalized_route);
        for method in &source.methods {
            let page_ref = PageRef {
                route: route.path.clone(),
                method: *method,
            };
            registry.register(
                page_ref,
                bun_runtime_tsp::module_graph::ModuleId::from_path(&route.source),
            );
        }
    }
    Ok(registry)
}

fn leak_registry(r: PageRegistry) -> &'static PageRegistry {
    Box::leak(Box::new(r))
}
fn leak_bun(b: BunRuntime) -> &'static BunRuntime {
    Box::leak(Box::new(b))
}
