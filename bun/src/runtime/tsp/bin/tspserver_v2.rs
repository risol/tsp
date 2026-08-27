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
use bun_runtime_tsp::jsc_bridge::BunRuntime;
use bun_runtime_tsp::module_graph::ModuleGraph;
use bun_runtime_tsp::router::RouteTable;
use bun_runtime_tsp::services::load_counter_services_from_config;
use bun_runtime_tsp::services::SESSION_STORE_CAP_DEFAULT;
use bun_runtime_tsp::services::ServiceRegistry;
use bun_runtime_tsp::session_backend::{MemoryBackend, RedisBackend, SessionBackend};
use bun_runtime_tsp::typings;
use bun_runtime_tsp::watcher::{self, WatchConfig};
use bun_runtime_tsp::worker::pool::WorkerPool;
use bun_runtime_tsp::worker::lifecycle::RecyclePolicy;
use bun_runtime_tsp::worker::sandbox::ResourceLimits;
use bun_runtime_tsp::worker::application::{Application, ApplicationRegistry, WorkerGroup};

pub fn run() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("check") => run_check(),
        Some("routes") => run_routes(),
        Some("graph") => run_graph(),
        Some("typings") => run_typings(),
        Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        _ => serve_main(),
    }
}

#[allow(dead_code)]
fn main() -> ExitCode {
    run()
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

    // v2.4 self-spawn only: the master always embeds a WorkerPool that
    // self-spawns the same `tspserver_v2[.exe]` (resolved via
    // `resolve_worker_bin()` → `current_exe()`) with `--tsp-worker`.
    // See `worker/manager.rs` for the spawn loop and `tsp_worker::run()`
    // for the worker-side dispatch.
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
    let bun: &'static BunRuntime = leak_bun(BunRuntime {
        bin: worker_binary,
        embedded_pool: Some(pool),
    });
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
    // Slice 22 prototype: load config-driven custom services
    // (plan §17.5 / §21). The host reads a JSON file pointed
    // at by `TSP_CONFIG` (default: `tsp.config.json`) and
    // registers any `services.<name>` entries it declares.
    // Currently only `kind: "counter"` is supported; a typo'd
    // kind is a hard error so a misconfigured deploy fails
    // fast at boot rather than at the first request.
    //
    // We build the registry in a local first so the
    // `register` calls go through the typed API (no `unsafe`).
    // The final registry is `Box::leak`ed into a `&'static`
    // so every connection thread shares the same instance;
    // it is never owned by a page generation, so reloads do
    // not tear services down (plan sect.61 Phase 8
    // acceptance).
    let mut registry_builder = ServiceRegistry::with_backends(session_backend);
    let config_path =
        std::env::var("TSP_CONFIG").unwrap_or_else(|_| "tsp.config.json".to_string());
    let mut custom_labels: Vec<String> = Vec::new();
    if std::path::Path::new(&config_path).is_file() {
        let text = std::fs::read_to_string(&config_path)
            .unwrap_or_else(|e| panic!("TSPv2PoC1: read {config_path}: {e}"));
        let custom = load_counter_services_from_config(&text)
            .unwrap_or_else(|e| panic!("TSPv2PoC1: parse {config_path}: {e}"));
        for svc in custom {
            custom_labels.push(svc.name().to_string());
            registry_builder.register(svc);
        }
        eprintln!(
            "TSPv2PoC1: custom services from {config_path}: {}",
            custom_labels.join(", ")
        );
    } else {
        eprintln!(
            "TSPv2PoC1: no config at {config_path} (set TSP_CONFIG to enable \
             custom services)"
        );
    }
    let services: &'static ServiceRegistry = Box::leak(Box::new(registry_builder));
    eprintln!(
        "TSPv2PoC1: services registered: {}",
        services
            .iter_names()
            .map(|n| n.to_string())
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
        "TSP v2 commands:\n  tspserver_v2              run the native HTTP server\n  tspserver_v2 check       validate routes and local imports\n  tspserver_v2 routes      list filesystem routes and exports\n  tspserver_v2 graph       print the resolved module graph\n  tspserver_v2 typings     write tsp:* TypeScript declaration files\n\nEnvironment:\n  TSP_ROUTES_DIR            route root (default: routes)\n  TSP_PUBLIC_DIR            public asset root (default: public)\n  TSP_PORT                  HTTP port (default: 3000)\n  TSP_WORKER_COUNT          embedded self-spawned worker processes (default: 1)\n  TSP_WORKER_MAX_IN_FLIGHT  max concurrent requests per worker (default: 2*count)\n  TSP_WORKER_MAX_REQUESTS   recycle each worker after N requests\n  TSP_WORKER_MAX_AGE_MS     recycle each worker after this many ms\n  TSP_WORKER_MAX_MEMORY_BYTES  recycle each worker after RSS reaches this\n  TSP_INVALIDATION_FILE     shared cross-worker invalidation log\n  TSP_MAX_BODY_BYTES         per-request body size cap; requests with\n                            Content-Length over this are rejected with\n                            413 Payload Too Large (default: 1 MiB)\n  TSP_CGROUP_ROOT           explicit Linux cgroup v2 parent directory\n  TSP_WORKER_MEMORY_MAX / TSP_WORKER_CPU_MAX / TSP_WORKER_PIDS_MAX  cgroup limits\n  TSP_REDIS_URL             optional Redis URL for the session backend\n  TSP_CONFIG                JSON file declaring config-driven custom\n                            services (default: tsp.config.json);\n                            supports `kind: counter` with `initial`\n  TSP_APPLICATION_NAME      application name registered in the registry (default: main)"
    );
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()?
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
}

// v2.4 self-spawn only: the master executable is the worker executable.
// The same `tspserver_v2[.exe]` runs the embedded Bun VM when invoked
// with `--tsp-worker` (see `worker/manager.rs` and `bun_bin/lib.rs`'s
// `tsp_worker::requested()` dispatch). There is intentionally no
// fallback to a separate `bun(.exe)` runtime, no `TSP_WORKER_BIN` env
// override, and no host-sibling lookup — production deployments ship a
// single self-contained executable.
fn resolve_worker_bin() -> Result<PathBuf, String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("cannot resolve worker executable: {error}"))?;
    if !exe.is_file() {
        return Err(format!(
            "worker executable does not exist: {}",
            exe.display()
        ));
    }
    Ok(exe)
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

/// Phase 11 tooling (plan §11): write the three
/// `tsp:*` declaration files (`tsp-server.d.ts`,
/// `tsp-html.d.ts`, `tsp-runtime.d.ts`) into the
/// user-supplied output directory (default `.tsp-types`).
///
/// Usage:
///   tspserver_v2 typings                  # writes to ./.tsp-types
///   tspserver_v2 typings <DIR>            # writes to <DIR>
///   tspserver_v2 typings --out <DIR>      # same
///
/// The hand-rolled content lives in `bun/src/runtime/tsp/typings.rs`
/// (loaded via `include_str!` from `tsp-types/` at the repo
/// root). A drift between the runtime surface and the
/// typings is pinned by the unit test in `typings.rs`
/// (asserts the public exports are still wired correctly)
/// and by the e2e in `start_order.rs`
/// (`tspserver_v2_typings_emits_three_dts_files`).
fn run_typings() -> ExitCode {
    // Parse the second positional arg (after "typings") as
    // the output dir. We accept both `<DIR>` and
    // `--out <DIR>` for ergonomics. The flag-form matches
    // other dev tools; the bare-positional-form matches
    // `tsc --outDir`.
    let raw_args: Vec<String> = std::env::args().skip(2).collect();
    let mut out_dir: Option<String> = None;
    let mut i = 0;
    while i < raw_args.len() {
        let arg = &raw_args[i];
        if arg == "--out" {
            if i + 1 >= raw_args.len() {
                eprintln!("tsp typings: --out requires a directory argument");
                return ExitCode::from(2);
            }
            out_dir = Some(raw_args[i + 1].clone());
            i += 2;
        } else if arg == "--help" || arg == "-h" {
            println!(
                "Usage: tspserver_v2 typings [--out <DIR>]\n\n\
                 Writes the three `tsp:*` TypeScript declaration files\n\
                 (tsp-server.d.ts, tsp-html.d.ts, tsp-runtime.d.ts) into\n\
                 <DIR> (default: .tsp-types). Add <DIR> to your\n\
                 `tsconfig.json` `include` list to enable type-checking\n\
                 of `import {{ ... }} from \"tsp:*\"` declarations."
            );
            return ExitCode::SUCCESS;
        } else if out_dir.is_none() {
            out_dir = Some(arg.clone());
            i += 1;
        } else {
            eprintln!("tsp typings: unexpected argument `{arg}`");
            return ExitCode::from(2);
        }
    }
    let out_dir = out_dir.unwrap_or_else(|| ".tsp-types".to_string());
    let out_path = PathBuf::from(&out_dir);

    if let Err(error) = std::fs::create_dir_all(&out_path) {
        eprintln!("tsp typings: cannot create {out_dir}: {error}");
        return ExitCode::from(2);
    }

    let files: [(&str, &str); 3] = [
        ("tsp-server.d.ts", typings::tsp_server_dts()),
        ("tsp-html.d.ts", typings::tsp_html_dts()),
        ("tsp-runtime.d.ts", typings::tsp_runtime_dts()),
    ];
    for (name, content) in &files {
        let target = out_path.join(name);
        if let Err(error) = std::fs::write(&target, content) {
            eprintln!(
                "tsp typings: cannot write {}: {error}",
                target.display()
            );
            return ExitCode::from(2);
        }
        println!("wrote {}", target.display());
    }
    println!(
        "tsp typings: {} file(s) written to {}",
        files.len(),
        out_path.display()
    );
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
