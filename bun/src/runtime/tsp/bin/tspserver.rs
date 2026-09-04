//! TSP native host binary entry point (PoC 1, slice 11b).
//!
//! See `tsp-plan.md` sect.70 and `tsp-specification.md` for the
//! current application contract implemented by this binary.
//!
//! Slice 10b boots a `PageRegistry`, walks the `pages/` directory
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
use bun_runtime_tsp::router::{HttpMethod, RouteTable};
use bun_runtime_tsp::services::SESSION_STORE_CAP_DEFAULT;
use bun_runtime_tsp::services::ServiceRegistry;
use bun_runtime_tsp::services::{RuntimeConfig, load_config_services, load_runtime_config};
use bun_runtime_tsp::session_backend::{MemoryBackend, RedisBackend, SessionBackend};
use bun_runtime_tsp::typings;
use bun_runtime_tsp::watcher::{self, WatchConfig};
use bun_runtime_tsp::worker::application::{Application, ApplicationRegistry, WorkerGroup};
use bun_runtime_tsp::worker::lifecycle::RecyclePolicy;
use bun_runtime_tsp::worker::manager::ImageLimits;
use bun_runtime_tsp::worker::pool::WorkerPool;
use bun_runtime_tsp::worker::sandbox::ResourceLimits;

/// Version string baked into the binary by `build.rs`.
/// Release builds derive it from the current `v*` Git tag;
/// `TSP_VERSION` can override that value in CI or source archives.
/// The slice 11b PoC 1 contract is "the host binary prints its
/// version and exits".
const VERSION: &str = concat!("tspserver ", env!("TSP_VERSION"));

pub fn run() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("check") => run_check(),
        Some("routes") => run_routes(),
        Some("graph") => run_graph(),
        Some("typings") => run_typings(),
        Some("--version") | Some("-V") => {
            println!("{VERSION}");
            ExitCode::SUCCESS
        }
        Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("--config") | Some("-c") => serve_main(),
        Some(value) if value.starts_with("--config=") => serve_main(),
        _ => serve_main(),
    }
}

#[allow(dead_code)]
fn main() -> ExitCode {
    run()
}

fn serve_main() -> ExitCode {
    let config_path = match resolve_config_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("TSP: {error}");
            return ExitCode::from(2);
        }
    };
    let config_text = if config_path.is_file() {
        Some(
            std::fs::read_to_string(&config_path)
                .unwrap_or_else(|e| panic!("TSP: read {}: {e}", config_path.display())),
        )
    } else {
        None
    };
    let file_config = match config_text.as_deref().map(load_runtime_config).transpose() {
        Ok(value) => value.unwrap_or_default(),
        Err(error) => {
            eprintln!("TSP: parse {}: {error}", config_path.display());
            return ExitCode::from(2);
        }
    };

    let port = match std::env::var("TSP_PORT") {
        Ok(value) => match value.parse::<u16>() {
            Ok(port) => port,
            Err(_) => {
                eprintln!("TSP: TSP_PORT is not a u16: {value:?}");
                return ExitCode::from(2);
            }
        },
        Err(_) => file_config.port,
    };

    let routes_dir = resolve_routes_dir_for_config(&file_config, &config_path);
    eprintln!("TSP: scanning pages from {}", routes_dir.display());
    // Slice 15a: the RouteTable is now Arc<Mutex<...>> under
    // the hood so the watcher can add/remove routes while
    // requests are in flight. We keep the boot-time scan
    // result in an Arc; the host and the watcher each
    // receive a clone of that Arc (cheap, since the inner
    // Arc<Mutex<...>> is shared).
    let routes: Arc<RouteTable> = match RouteTable::scan(&routes_dir) {
        Ok(table) => Arc::new(table),
        Err(e) => {
            eprintln!("TSP: {e}");
            return ExitCode::from(2);
        }
    };
    eprintln!("TSP: loaded {} route(s)", routes.len());

    // Boot-time preparation: for each route, run the slice-5
    // static method detector so we know which methods each file
    // exports. Register one PageSlot per (route, method) pair
    // in the PageRegistry. The host then consults the registry
    // per request instead of re-detecting on every call.
    let registry: &'static PageRegistry = match build_registry(&routes) {
        Ok(r) => leak_registry(r),
        Err(e) => {
            eprintln!("TSP: {e}");
            return ExitCode::from(2);
        }
    };

    // embedded-worker self-spawn only: the master always embeds a WorkerPool that
    // self-spawns the same `tspserver[.exe]` (resolved via
    // `resolve_worker_bin()` -> `current_exe()`) with `--tsp-worker`.
    // See `worker/manager.rs` for the spawn loop and `tsp_worker::run()`
    // for the worker-side dispatch.
    let worker_binary = match resolve_worker_bin() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("TSP: {error}");
            return ExitCode::from(2);
        }
    };
    let socket_dir = std::env::temp_dir().join(format!("tspserver-workers-{}", std::process::id()));
    if let Err(error) = std::fs::create_dir_all(&socket_dir) {
        eprintln!("TSP: cannot create worker socket directory: {error}");
        return ExitCode::from(2);
    }
    let worker_count = env_usize("TSP_WORKER_COUNT")
        .unwrap_or(file_config.worker_count)
        .max(1);
    let max_in_flight = std::env::var("TSP_WORKER_MAX_IN_FLIGHT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(file_config.worker_max_in_flight)
        .max(worker_count);
    let recycle_policy = RecyclePolicy {
        max_requests: env_u64("TSP_WORKER_MAX_REQUESTS").or(file_config.worker_max_requests),
        max_age: env_u64("TSP_WORKER_MAX_AGE_MS")
            .or(file_config.worker_max_age_ms)
            .map(std::time::Duration::from_millis),
        max_memory_bytes: env_u64("TSP_WORKER_MAX_MEMORY_BYTES")
            .or(file_config.worker_max_memory_bytes),
    };
    let resource_limits = ResourceLimits {
        cgroup_root: std::env::var_os("TSP_CGROUP_ROOT").map(PathBuf::from),
        memory_max: env_u64("TSP_WORKER_MEMORY_MAX"),
        cpu_max: std::env::var("TSP_WORKER_CPU_MAX").ok(),
        pids_max: env_u64("TSP_WORKER_PIDS_MAX"),
    };
    let image_limits = ImageLimits {
        max_input_bytes: env_u64("TSP_IMAGE_MAX_INPUT_BYTES")
            .unwrap_or(file_config.image_max_input_bytes),
        max_pixels: env_u64("TSP_IMAGE_MAX_PIXELS").unwrap_or(file_config.image_max_pixels),
        max_concurrent_tasks: env_usize("TSP_IMAGE_MAX_CONCURRENT_TASKS")
            .unwrap_or(file_config.image_max_concurrent_tasks)
            .max(1),
    };
    eprintln!(
        "TSP: image limits = input {} bytes, {} pixels, {} concurrent task(s) per worker",
        image_limits.max_input_bytes, image_limits.max_pixels, image_limits.max_concurrent_tasks
    );
    let pool = WorkerPool::new(
        worker_binary.clone(),
        socket_dir,
        worker_count,
        max_in_flight,
    )
    .with_recycle_policy(recycle_policy)
    .with_resource_limits(resource_limits)
    .with_image_limits(image_limits);
    let pool = Arc::new(pool);
    let application_name = std::env::var("TSP_APPLICATION_NAME")
        .unwrap_or_else(|_| file_config.application_name.clone());
    ApplicationRegistry::global().register(Application::new(
        application_name,
        WorkerGroup::new(Arc::clone(&pool)),
    ));
    if let Err(error) = pool.start() {
        eprintln!("TSP: embedded worker failed to start: {error}");
        return ExitCode::from(2);
    }
    eprintln!("TSP: embedded worker enabled ({})", worker_binary.display());
    let bun: &'static BunRuntime = leak_bun(BunRuntime {
        bin: worker_binary,
        embedded_pool: Some(pool),
    });
    eprintln!("TSP: worker executable = {}", bun.bin.display());

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
    let session_backend: Arc<dyn SessionBackend> = match std::env::var("TSP_REDIS_URL")
        .ok()
        .or(file_config.redis_url.clone())
    {
        Some(url) if !url.is_empty() => match RedisBackend::with_default_ttl(&url) {
            Ok(backend) => {
                let available = backend.is_available();
                let arc: Arc<dyn SessionBackend> = Arc::new(backend);
                eprintln!("TSP: session backend = redis (url={url}, available={available})");
                arc
            }
            Err(e) => {
                eprintln!("TSP: TSP_REDIS_URL parse failed ({e}); falling back to memory");
                Arc::new(MemoryBackend::new(SESSION_STORE_CAP_DEFAULT))
            }
        },
        _ => {
            eprintln!("TSP: session backend = memory (cap={SESSION_STORE_CAP_DEFAULT})");
            Arc::new(MemoryBackend::new(SESSION_STORE_CAP_DEFAULT))
        }
    };
    // Slice 22 prototype: load config-driven custom services
    // (plan §17.5 / §21). The host reads a JSON file pointed
    // at by `--config`, `TSP_CONFIG`, or the default
    // `tsp.config.json` in the current application root. The
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
    let mut custom_labels: Vec<String> = Vec::new();
    if let Some(text) = config_text.as_deref() {
        let custom = load_config_services(text)
            .unwrap_or_else(|e| panic!("TSP: parse {}: {e}", config_path.display()));
        // §22.3: route the config-declared services through
        // `apply_config_snapshot` so the registry's
        // `config_decls` set is populated. A future reload
        // uses that set to drop ONLY the config-driven
        // services (the built-in `logger` / `session` /
        // `time` survive a reload that does not mention
        // them).
        registry_builder.apply_config_snapshot(custom.clone());
        for svc in custom {
            custom_labels.push(svc.name().to_string());
        }
        eprintln!(
            "TSP: custom services from {}: {}",
            config_path.display(),
            custom_labels.join(", ")
        );
    } else {
        eprintln!(
            "TSP: no config at {} (create the default file or use --config <PATH>)",
            config_path.display()
        );
    }
    // §22.3: the registry is wrapped in an `RwLock` so the
    // config-reload watcher can swap config-driven services
    // (counter / kv / feature_flag) without a master
    // restart, while the request path takes a read lock for
    // each snapshot. Built-in services (logger, session,
    // time) are never replaced by the config watcher; the
    // `apply_config_snapshot` method only touches names that
    // appear in the freshly-parsed config snapshot.
    let services: &'static std::sync::RwLock<ServiceRegistry> =
        Box::leak(Box::new(std::sync::RwLock::new(registry_builder)));
    eprintln!(
        "TSP: services registered: {}",
        services
            .read()
            .expect("services read lock at boot")
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
            eprintln!("TSP: build module graph: {e}");
            return ExitCode::from(2);
        }
    });
    let registry_arc = Arc::new(registry.clone());
    let routes_for_watcher = Arc::clone(&routes);
    // §22.3: the watcher's config-reload callback re-parses
    // the config file and applies the fresh snapshot to the
    // shared `services` registry. The callback takes a
    // WRITE lock for the duration of the apply; a request
    // thread holding a read lock would block the watcher
    // briefly, but the request path only holds the read
    // lock for a snapshot call (microseconds), so the
    // wait is invisible in practice. The callback is the
    // same one the host's boot path uses (`load_config_services`),
    // so a typo in the new config that the boot path would
    // have rejected is also rejected here -- the watcher
    // logs the error and the previous snapshot stays in
    // place (the registry is unchanged on Err).
    let on_config_reload: Arc<dyn Fn(&str) -> Result<String, String> + Send + Sync> = {
        let services_for_reload: &'static std::sync::RwLock<ServiceRegistry> = services;
        Arc::new(move |text: &str| {
            let fresh = load_config_services(text)?;
            let names: Vec<String> = fresh.iter().map(|s| s.name().to_string()).collect();
            {
                let mut guard = services_for_reload
                    .write()
                    .expect("services write lock from config reload");
                guard.apply_config_snapshot(fresh);
            }
            Ok(format!(
                "applied {} service(s): {}",
                names.len(),
                names.join(", ")
            ))
        })
    };
    let watch_config = WatchConfig {
        routes_root: routes_dir.clone(),
        poll_ms: watcher::DEFAULT_POLL_MS,
        // §22.3: wire the config-file watcher. The host
        // already read the file at boot; the watcher's
        // initial hash snapshot is built from the same
        // on-disk content so a no-op edit at boot does
        // not fire a reload. If the file is missing at
        // boot, the watcher's first poll that sees the
        // file fires the callback.
        config_path: config_path.is_file().then(|| config_path.clone()),
        on_config_reload: Some(on_config_reload),
    };
    let watcher_handle = watcher::spawn(watch_config, graph, routes_for_watcher, registry_arc);
    eprintln!(
        "TSP: watcher polling {} every {}ms",
        routes_dir.display(),
        watcher::DEFAULT_POLL_MS
    );
    if config_path != PathBuf::from("tsp.config.json") || config_path.is_file() {
        eprintln!(
            "TSP: config hot-reload watching {} (poll interval = watcher poll)",
            config_path.display()
        );
    }

    let public_root =
        host::resolve_public_root_with_config(file_config.public_dir, config_path.parent());
    let public_prefix = file_config.public_prefix.clone();
    eprintln!(
        "TSP: public directory = {} (prefix = {})",
        public_root.as_deref().map_or_else(
            || "(disabled)".to_string(),
            |path| path.display().to_string()
        ),
        public_prefix.as_deref().unwrap_or("/")
    );
    let request_settings = host::RequestSettings {
        max_body_bytes: env_usize("TSP_MAX_BODY_BYTES").unwrap_or(file_config.max_body_bytes),
        timeout_ms: env_u64("TSP_TIMEOUT_MS").unwrap_or(file_config.timeout_ms),
        development: std::env::var("TSP_DEVELOPMENT")
            .map(|value| value == "1")
            .unwrap_or(file_config.development),
    };
    if let Err(e) = host::serve_with_public_root_and_settings_and_prefix(
        &file_config.host,
        port,
        routes,
        registry,
        bun,
        services,
        request_settings,
        public_root,
        public_prefix,
    ) {
        // serve returns only on a fatal listener error; dropping
        // watcher_handle here stops + joins the watcher thread.
        drop(watcher_handle);
        eprintln!("TSP: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn resolve_routes_dir_for_config(config: &RuntimeConfig, config_path: &PathBuf) -> PathBuf {
    let path = std::env::var_os("TSP_ROUTES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| config.routes_dir.clone());
    if path.is_absolute() {
        path
    } else {
        config_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.join(&path))
            .unwrap_or(path)
    }
}

fn resolve_routes_dir() -> PathBuf {
    std::env::var_os("TSP_ROUTES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("pages"))
}

fn resolve_config_path() -> Result<PathBuf, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut config_from_flag: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if argument == "--config" || argument == "-c" {
            if config_from_flag.is_some() {
                return Err("--config may be specified only once".to_string());
            }
            index += 1;
            let path = args.get(index).ok_or_else(|| {
                "--config requires a path (for example: --config tsp.config.json)".to_string()
            })?;
            if path.is_empty() {
                return Err("--config requires a non-empty path".to_string());
            }
            config_from_flag = Some(PathBuf::from(path));
        } else if let Some(path) = argument.strip_prefix("--config=") {
            if config_from_flag.is_some() {
                return Err("--config may be specified only once".to_string());
            }
            if path.is_empty() {
                return Err("--config requires a non-empty path".to_string());
            }
            config_from_flag = Some(PathBuf::from(path));
        } else {
            return Err(format!(
                "unexpected argument `{argument}`; use `--config <PATH>` only with the server"
            ));
        }
        index += 1;
    }

    if let Some(path) = config_from_flag {
        return Ok(path);
    }
    if let Some(path) = std::env::var_os("TSP_CONFIG").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(PathBuf::from("tsp.config.json"))
}

fn print_help() {
    println!(
        "Server option: --config, -c <PATH> (precedence: flag, TSP_CONFIG, ./tsp.config.json)"
    );
    println!(
        "TSP commands:\n  tspserver              run the native HTTP server\n  tspserver check       validate routes and local imports\n  tspserver routes      list filesystem routes and exports\n  tspserver graph       print the resolved module graph\n  tspserver typings     write tsp:* TypeScript declaration files\n  tspserver --version   print the version and exit\n  tspserver --help      print this help and exit\n\nEnvironment:\n  TSP_ROUTES_DIR            page source root (default: pages)\n  TSP_PUBLIC_DIR            public asset root (default: public)\n  TSP_PORT                  HTTP port (default: 3000)\n  TSP_TIMEOUT_MS            per-request timeout in ms; 0 disables the\n                            watchdog (default: 30000). The per-page\n                            `config.timeoutMs` overrides this per\n                            request (spec section 7 current contract PageConfig)\n  TSP_DEVELOPMENT           set to 1 for dev mode: page-throw 500\n                            responses render as self-contained HTML\n                            error pages (name + message + stack) instead\n                            of the prod JSON body (default: 0 / prod)\n  TSP_WORKER_COUNT          embedded self-spawned worker processes (default: 1)\n  TSP_WORKER_MAX_IN_FLIGHT  max concurrent requests per worker (default: 2*count)\n  TSP_WORKER_MAX_REQUESTS   recycle each worker after N requests\n  TSP_WORKER_MAX_AGE_MS     recycle each worker after this many ms\n  TSP_WORKER_MAX_MEMORY_BYTES  recycle each worker when RSS reaches this\n  TSP_INVALIDATION_FILE     shared cross-worker invalidation log\n  TSP_MAX_BODY_BYTES         per-request body size cap; requests with\n                            Content-Length over this are rejected with\n                            413 Payload Too Large (default: 1 MiB)\n  TSP_CGROUP_ROOT           explicit Linux cgroup v2 parent directory\n  TSP_WORKER_MEMORY_MAX / TSP_WORKER_CPU_MAX / TSP_WORKER_PIDS_MAX  cgroup limits\n  TSP_REDIS_URL             optional Redis URL for the session backend\n  TSP_CONFIG                JSON file declaring config-driven custom\n                            services (default: tsp.config.json);\n                            supports `kind: counter` with `initial`\n  TSP_APPLICATION_NAME      application name registered in the registry (default: main)"
    );
    println!(
        "Image environment overrides:\n  TSP_IMAGE_MAX_INPUT_BYTES       encoded input cap (default: 256 MiB)\n  TSP_IMAGE_MAX_PIXELS            decoded width × height cap (default: 268402689)\n  TSP_IMAGE_MAX_CONCURRENT_TASKS  in-flight image pipelines per worker (default: 4)"
    );
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()?
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()?
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
}

// embedded-worker self-spawn only: the master executable is the worker executable.
// The same `tspserver[.exe]` runs the embedded Bun VM when invoked
// with `--tsp-worker` (see `worker/manager.rs` and `bun_bin/lib.rs`'s
// `tsp_worker::requested()` dispatch). There is intentionally no
// fallback to a separate `bun(.exe)` runtime, no `TSP_WORKER_BIN` env
// override, and no host-sibling lookup ' production deployments ship a
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
    // Parse optional flag. Default output is
    // tab-separated (`<path>\t<source>\t<methods>`)
    // for human / shell consumption; `--json` emits
    // a JSON array for tooling / CI.
    let raw_args: Vec<String> = std::env::args().skip(2).collect();
    let mut json = false;
    let mut i = 0;
    while i < raw_args.len() {
        let arg = &raw_args[i];
        if arg == "--json" {
            json = true;
            i += 1;
        } else if arg == "--help" || arg == "-h" {
            println!(
                "Usage: tspserver routes [--json]\n\n\
                 Prints one line per route in the pages/ directory:\n\
                   <path>\\t<source>\\t<methods>\n\
                 --json    emit a JSON array of {{path, source, methods}} instead."
            );
            return ExitCode::SUCCESS;
        } else {
            eprintln!("tsp routes: unexpected argument `{arg}`");
            return ExitCode::from(2);
        }
    }

    let root = resolve_routes_dir();
    let table = match RouteTable::scan(&root) {
        Ok(table) => table,
        Err(error) => {
            eprintln!("tsp routes: {error}");
            return ExitCode::from(2);
        }
    };
    if json {
        // Hand-rolled JSON (no serde dep). Each entry
        // is `{"path": "...", "source": "...", "methods":
        // [..]}`. Errors are surfaced via an extra
        // `"error": "..."` field on the same row.
        println!("[");
        let mut first = true;
        for route in table.iter() {
            match bun_runtime_tsp::page::prepare(&route) {
                Ok(page) => {
                    if !first {
                        println!(",");
                    }
                    first = false;
                    let methods: Vec<&str> = page.methods.iter().map(|m| m.as_str()).collect();
                    print!(
                        "  {{\"path\":{},\"source\":{},\"methods\":[{}]}}",
                        json_string(&route.path),
                        json_string(&route.source.display().to_string()),
                        methods
                            .iter()
                            .map(|m| json_string(m))
                            .collect::<Vec<_>>()
                            .join(",")
                    );
                }
                Err(error) => {
                    if !first {
                        println!(",");
                    }
                    first = false;
                    print!(
                        "  {{\"path\":{},\"source\":{},\"error\":{}}}",
                        json_string(&route.path),
                        json_string(&route.source.display().to_string()),
                        json_string(&error.to_string())
                    );
                }
            }
        }
        if !first {
            println!();
        }
        println!("]");
    } else {
        for route in table.iter() {
            match bun_runtime_tsp::page::prepare(&route) {
                Ok(page) => println!(
                    "{}\t{}\t{}",
                    route.path,
                    route.source.display(),
                    page.methods
                        .iter()
                        .map(|m| m.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                Err(error) => {
                    println!("{}\t{}\tERROR: {error}", route.path, route.source.display())
                }
            }
        }
    }
    ExitCode::SUCCESS
}

/// Hand-rolled JSON string escape. Wraps the input in
/// double quotes and escapes the JSON-required
/// characters (`"`, `\`, and control chars).
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn run_check() -> ExitCode {
    // Parse optional flags. The default check does
    // route-table scan + regex static-analysis; `--tsc`
    // additionally runs a real `tsc --noEmit` pass against
    // the routes + the tsp:* declaration files.
    let raw_args: Vec<String> = std::env::args().skip(2).collect();
    let mut tsc = false;
    let mut no_color = false;
    let mut i = 0;
    while i < raw_args.len() {
        let arg = &raw_args[i];
        if arg == "--tsc" {
            tsc = true;
            i += 1;
        } else if arg == "--no-color" {
            no_color = true;
            i += 1;
        } else if arg == "--help" || arg == "-h" {
            println!(
                "Usage: tspserver check [--tsc] [--no-color]\n\n\
                 Scans the pages/ directory and prints each route's\n\
                 static export set. Returns 1 if any route fails to\n\
                 parse, 0 otherwise. Also validates `PageConfig`\n\
                 fields (contract.md §11): a `config.methods` mismatch\n\
                 (declared set vs. actual exports) is reported as\n\
                 an ERROR and the check exits 1. Also reports any\n\
                 `export function NAME(...)` whose name is not a\n\
                 standard HTTP method handler as an unknown runtime\n\
                 export (spec §46); a clean run must have no such\n\
                 exports. Also reports any top-level `export default`\n\
                 as a `default export` violation (spec §46); a `.tsp`\n\
                 file's exports must be the named HTTP method handlers\n\
                 and (optionally) a `const config = {{ ... }}`.\n\n\
                 --tsc        additionally run `tsc --noEmit` against the\n\
                              routes (after rewriting `.tsp` to `.tsx`)\n\
                              and the bundled `tsp:*` declaration files.\n\
                              Returns 1 if tsc reports any error.\n\
                 --no-color   pass `--noColor` to tsc and strip any ANSI\n\
                              escape sequences from the bin's own output\n\
                              (the path-rewrite prefix). Useful when the\n\
                              bin's stdout is piped to a log file or a\n\
                              non-ANSI terminal."
            );
            return ExitCode::SUCCESS;
        } else {
            eprintln!("tsp check: unexpected argument `{arg}`");
            return ExitCode::from(2);
        }
    }

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
            Ok(page) => {
                // contract.md §11: when a page declares
                // `config.methods`, the static check
                // validates the declared set against
                // the actual exports. A mismatch is
                // a contract break: the page says it
                // serves one set, the user would
                // see another.
                if let Some(declared) = &page.config_methods {
                    // `HttpMethod` does not implement
                    // `Ord`, so a sorted comparison via
                    // `BTreeSet` is not available.
                    // Build a HashSet from each side and
                    // compare (the field is a `Vec`, not
                    // a `HashSet`, so we collect twice).
                    use std::collections::HashSet;
                    let actual: HashSet<HttpMethod> = page.methods.iter().copied().collect();
                    let want: HashSet<HttpMethod> = declared.iter().copied().collect();
                    if actual != want {
                        failed = true;
                        eprintln!(
                            "ERROR {}: config.methods mismatch -- declared {:?} but exports {:?}; \
                             the page must export exactly the methods listed in `config.methods`",
                            route.source.display(),
                            declared,
                            page.methods
                        );
                        // Still print the OK line so the
                        // operator can see the static
                        // scan result; the ERROR line
                        // above already set `failed`.
                    }
                }
                // Spec §46: "no unknown runtime
                // exports". The static check
                // reports any `export function
                // NAME(...)` whose name is not a
                // standard HTTP method handler.
                // The runtime still serves the
                // page (the unknown export is
                // silently ignored), but a clean
                // `check` run should make the
                // violation visible so the user
                // can move the helper to a
                // non-`.tsp` module. The full
                // spec §46 treatment (unknown
                // exports are a build-time
                // generation failure) lands with
                // the AST detector in a future
                // slice; for now this is a
                // quality-of-experience check.
                if !page.unknown_exports.is_empty() {
                    failed = true;
                    eprintln!(
                        "ERROR {}: unknown runtime export(s) {:?} \
                         (spec §46 \"no unknown runtime exports\"); \
                         only the standard HTTP method handlers are \
                         allowed as top-level `export function` calls",
                        route.source.display(),
                        page.unknown_exports,
                    );
                }
                // Spec §46: "no `default`
                // export". The page registry
                // reads only the named HTTP
                // method exports and the
                // `config` const; a default
                // export is silently ignored
                // at runtime. The check
                // surfaces it so the user gets
                // a clear message at check
                // time rather than a silent
                // no-op.
                if page.has_default_export {
                    failed = true;
                    eprintln!(
                        "ERROR {}: `export default` is not allowed \
                         (spec §46 \"no `default` export\"); \
                         a `.tsp` file's exports must be the named \
                         HTTP method handlers and (optionally) a \
                         `const config = {{ ... }}`",
                        route.source.display(),
                    );
                }
                println!(
                    "OK {} [{}]",
                    route.path,
                    page.methods
                        .iter()
                        .map(|m| m.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
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
    if tsc {
        match run_tsc_check(&root, no_color) {
            Ok(()) => println!("OK tsc: 0 error(s)"),
            Err(error) => {
                failed = true;
                eprintln!("ERROR tsc: {error}");
            }
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Phase 11 follow-up: real `tsc --noEmit` type-check pass
/// over the page source directory.
///
/// The pass:
///   1. Walks `routes_root` and copies every `.tsp` file to
///      a temp directory as `.tsx` (tsc treats .tsp as
///      unknown). `.ts` helper files (e.g. `pages/_db.ts`)
///      are copied verbatim.
///   2. Copies the three bundled `tsp:*` declaration files
///      into the temp dir's `tsp-types/` subdir.
///   3. Writes a `tsconfig.json` that maps the `tsp:*`
///      module names to those declarations and includes
///      `pages/**/*.tsx` + `tsp-types/**/*.d.ts`.
///   4. Locates the `tsc` binary (CWD `node_modules/.bin`
///      first, then PATH).
///   5. Invokes `tsc --noEmit --project <tsconfig>` and
///      forwards its stdout to the user's stdout verbatim.
///   6. Returns Ok if tsc exits 0, Err if tsc exits
///      non-zero or cannot be invoked.
///
/// The temp directory is removed on every return path
/// (success, parse error, tsc invocation failure).
///
/// The check is conservative: `strict: false` so the user
/// is not forced to handle `null`/`undefined` exactly; the
/// goal is to surface gross type mismatches (typos in
/// imported names, wrong argument shapes, missing
/// properties), not to enforce strictness the runtime
/// itself does not enforce.
fn run_tsc_check(routes_root: &std::path::Path, no_color: bool) -> Result<(), String> {
    use std::process::Command;

    // (1) Set up the temp dir.
    let temp = std::env::temp_dir().join(format!(
        "tsp-tsc-check-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("clock: {e}"))?
            .as_nanos()
    ));
    let temp_routes = temp.join("pages");
    let temp_types = temp.join("tsp-types");
    std::fs::create_dir_all(&temp_routes).map_err(|e| format!("mkdir temp routes: {e}"))?;
    std::fs::create_dir_all(&temp_types).map_err(|e| format!("mkdir temp tsp-types: {e}"))?;

    // RAII guard: remove the temp dir on every return path.
    // The closure cannot move `temp` into itself, so we
    // capture the path and use a flag-based cleanup.
    let cleanup_path = temp.clone();
    let cleanup = || {
        let _ = std::fs::remove_dir_all(&cleanup_path);
    };

    let result: Result<(), String> = (|| {
        // (2) Copy `.tsp` -> `.tsx` and `.ts` -> `.ts`
        // recursively, preserving directory structure.
        copy_routes_recursive(routes_root, &temp_routes)
            .map_err(|e| format!("copy routes: {e}"))?;

        // (3) Copy the three bundled declaration files.
        // The bin does not know where the user keeps them,
        // so we probe a small list of conventional
        // locations: CWD-relative `.tsp-types/`, then
        // CWD-relative `tsp-types/`, then the parent of
        // the routes root (e.g. `<project>/.tsp-types/`
        // when the user runs from `<project>/`). The
        // first dir that contains all three d.ts files
        // wins. If none does, error with a hint that
        // points at the typings subcommand.
        let mut tsp_types_src: Option<std::path::PathBuf> = None;
        for base in [
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            routes_root
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".")),
        ] {
            for sub in &[".tsp-types", "tsp-types"] {
                let candidate = base.join(sub);
                if ["tsp-server.d.ts", "tsp-html.d.ts", "tsp-runtime.d.ts"]
                    .iter()
                    .all(|name| candidate.join(name).is_file())
                {
                    tsp_types_src = Some(candidate);
                    break;
                }
            }
            if tsp_types_src.is_some() {
                break;
            }
        }
        let tsp_types_src = tsp_types_src.ok_or_else(|| {
            "cannot locate the tsp:* declaration files: probed \
             `.tsp-types/` and `tsp-types/` in the current \
             directory and in the parent of the routes root. \
             Run `tspserver typings --out .tsp-types` to \
             generate them."
                .to_string()
        })?;
        println!(
            "tsp check --tsc: using declaration files from {}",
            tsp_types_src.display()
        );
        for name in &["tsp-server.d.ts", "tsp-html.d.ts", "tsp-runtime.d.ts"] {
            let src = tsp_types_src.join(name);
            let dst = temp_types.join(name);
            std::fs::copy(&src, &dst).map_err(|e| format!("copy {src:?} -> {dst:?}: {e}"))?;
        }

        // (4) Write the tsconfig.
        let tsconfig = temp.join("tsconfig.json");
        std::fs::write(&tsconfig, TSC_TSCONFIG_JSON).map_err(|e| format!("write tsconfig: {e}"))?;

        // (5) Locate tsc. CWD-relative node_modules wins
        // over PATH (a project pinned to tsc 5.9 should
        // not be overridden by a system tsc 4.x).
        let tsc = locate_tsc_invocation().ok_or_else(|| {
            "cannot locate tsc binary: looked for \
             ./node_modules/.bin/tsc, ./node_modules/.bin/tsc.cmd, and \
             `tsc` on PATH. Install TypeScript (`bun add -d typescript`) \
             to enable this check."
                .to_string()
        })?;
        println!("tsp check --tsc: using {}", tsc.display);

        // (6) Run tsc. We use the working directory of the
        // user (so the user's node_modules / package.json
        // are visible to tsc for any third-party imports
        // the route makes), and pass --project pointing at
        // our temp tsconfig.
        let mut cmd = Command::new(&tsc.program);
        cmd.args(&tsc.prefix_args)
            .arg("--noEmit")
            .arg("--project")
            .arg(&tsconfig);
        if no_color {
            // tsc 5.x does not accept `--noColor` as a
            // CLI flag (TS5023: Unknown compiler option).
            // The standard escape is the `NO_COLOR`
            // environment variable (a no-color convention
            // honoured by most modern CLIs). We also set
            // `--pretty false` as a belt-and-braces for
            // tsc versions that ignore `NO_COLOR`.
            cmd.env("NO_COLOR", "1").arg("--pretty").arg("false");
        }
        let output = cmd
            .current_dir(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .output()
            .map_err(|e| format!("invoke {}: {e}", tsc.display))?;
        // tsc prints absolute paths in its diagnostics
        // (it sees the temp dir, not the routes root). For
        // the user, those paths are noise -- their source
        // lives at `<routes>/foo.tsp`, not at
        // `<some temp dir>/pages/foo.tsx`. Rewrite each
        // diagnostic's path prefix so the user can copy /
        // click straight to the original file.
        rewrite_tsc_paths(
            &String::from_utf8_lossy(&output.stdout),
            &temp_routes,
            routes_root,
        );
        rewrite_tsc_paths(
            &String::from_utf8_lossy(&output.stderr),
            &temp_routes,
            routes_root,
        );
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "tsc reported error(s); see the diagnostic output above \
                 (exit status: {})",
                output.status
            ))
        }
    })();

    cleanup();
    result
}

/// Rewrite `tsc` diagnostic paths from
/// `<temp>/pages/<rest>` to `<routes_root>/<rest>` so
/// the user sees a path that actually exists on their
/// disk. The rewrite is conservative: only paths that
/// start with the temp `pages/` prefix are touched;
/// anything else (e.g. a path inside `node_modules/`)
/// passes through unchanged. The output is written
/// directly to stdout / stderr (the function takes the
/// raw tsc output as input).
fn rewrite_tsc_paths(text: &str, temp_routes: &std::path::Path, routes_root: &std::path::Path) {
    // tsc on Windows emits paths with forward slashes
    // (`C:/...`), but the host's `PathBuf` round-trips
    // with backslashes. Build both candidate prefixes
    // so either representation matches.
    let temp_back = temp_routes.to_string_lossy().to_string();
    let temp_fwd = temp_back.replace('\\', "/");
    let routes_back = routes_root.to_string_lossy().to_string();
    let routes_fwd = routes_back.replace('\\', "/");
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let normalized_temp = lexical_absolute(temp_routes, &cwd);

    for line in text.lines() {
        // Strip ANSI escape sequences (CSI SGR + others)
        // from tsc's diagnostic output. tsc 5.x does not
        // emit color by default, but a user-supplied
        // tsconfig (or a future tsc release) might. The
        // regex-free strip below handles the common
        // `\x1b[...m` form (color) and `\x1b[...;<n>m` form
        // (parameterized color). The pattern matches:
        //   ESC [ (any chars not in @-\\[-~) (final byte)
        // per ECMA-48. We only need the m-finalizer
        // family; non-m finalizers are passed through
        // (they would be a bug in tsc anyway).
        let stripped = strip_ansi(line);

        // tsc's diagnostic format is:
        //   <path>(<line>,<col>): <message>
        // or a continuation line with no path. We need
        // to extract just the `<path>` portion (stopping
        // at the `(<line>,<col>)` opener) so the rewrite
        // doesn't smear the message into the path.
        let (path_end, prefix) = if let Some(idx) = stripped.find('(') {
            if let Some(comma_idx) = stripped[idx..].find(',') {
                let after_comma = idx + comma_idx + 1;
                if let Some(close_idx) = stripped[after_comma..].find(')') {
                    let col_part = &stripped[after_comma..after_comma + close_idx];
                    if col_part.chars().all(|c| c.is_ascii_digit()) {
                        // `<path>(<digits>,<digits>)` matches the tsc
                        // diagnostic shape; everything before
                        // the `(` is the path.
                        (idx, &stripped[..idx])
                    } else {
                        // `(` was not the diagnostic opener; treat
                        // the whole line as a path-less continuation
                        // (pass through unchanged).
                        (stripped.len(), &stripped[..])
                    }
                } else {
                    (stripped.len(), &stripped[..])
                }
            } else {
                (stripped.len(), &stripped[..])
            }
        } else {
            (stripped.len(), &stripped[..])
        };
        if path_end == stripped.len() {
            // Path-less line (continuation, header, etc.):
            // pass through unchanged.
            println!("{}", stripped);
            continue;
        }
        // `prefix` is the path. Try to rewrite it.
        let relative_rewrite = {
            let diagnostic_path = lexical_absolute(std::path::Path::new(prefix), &cwd);
            diagnostic_path
                .strip_prefix(&normalized_temp)
                .ok()
                .map(|relative| routes_root.join(relative).to_string_lossy().to_string())
        };
        let rewritten_path = if let Some(rewritten) = relative_rewrite {
            rewritten
        } else if prefix.starts_with(&temp_back) {
            let rel = prefix.strip_prefix(&temp_back).unwrap();
            let rel = rel.trim_start_matches('\\').trim_start_matches('/');
            let mut new_path = std::path::PathBuf::from(&routes_back);
            for part in rel.split(['\\', '/']) {
                if !part.is_empty() {
                    new_path.push(part);
                }
            }
            new_path.to_string_lossy().to_string()
        } else if prefix.starts_with(&temp_fwd) {
            let rel = prefix.strip_prefix(&temp_fwd).unwrap();
            let rel = rel.trim_start_matches('/');
            let mut new_path = std::path::PathBuf::from(&routes_fwd);
            for part in rel.split('/') {
                if !part.is_empty() {
                    new_path.push(part);
                }
            }
            new_path.to_string_lossy().to_string()
        } else {
            prefix.to_string()
        };
        // Re-assemble: rewritten path + the original
        // `(<line>,<col>): <message>` suffix.
        let suffix = &stripped[path_end..];
        println!("{}{}", rewritten_path, suffix);
    }
}

/// Resolve a path without touching the filesystem. TypeScript may print a
/// temp source path relative to the caller's working directory, while the
/// temp root supplied by the checker is absolute. A lexical normalization is
/// sufficient here and avoids allocator-sensitive canonicalization on Linux.
fn lexical_absolute(path: &std::path::Path, cwd: &std::path::Path) -> PathBuf {
    use std::path::Component;

    let source = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in source.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

/// Strip ANSI CSI escape sequences from a string. Handles
/// the common color / cursor forms tsc and bun emit. The
/// strip is conservative: any `\x1b[...` sequence that ends
/// in a non-`m` finalizer is left alone (it is a bug in
/// the emitter, not our problem).
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next(); // consume '['
            let mut seq = String::new();
            while let Some(&nc) = chars.peek() {
                if nc.is_ascii_alphabetic() {
                    chars.next();
                    seq.push(nc);
                    break;
                }
                seq.push(nc);
                chars.next();
            }
            if seq.ends_with('m') {
                // color / SGR -- drop the whole escape
                continue;
            } else {
                // unknown CSI; pass through verbatim
                out.push('\x1b');
                out.push('[');
                out.push_str(&seq);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Recursively copy `routes_root` into `dst_root`, renaming
/// `.tsp` files to `.tsx` (TypeScript's compiler does not
/// recognise the `.tsp` extension by default). All other
/// files (notably `.ts` helpers like `pages/_db.ts`) are
/// copied verbatim. The directory layout under
/// `dst_root` mirrors `routes_root` exactly.
fn copy_routes_recursive(
    src_root: &std::path::Path,
    dst_root: &std::path::Path,
) -> std::io::Result<()> {
    let entries = std::fs::read_dir(src_root)?;
    for entry in entries {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let src = entry.path();
        let dst = dst_root.join(&*name_str);
        if file_type.is_dir() {
            std::fs::create_dir_all(&dst)?;
            copy_routes_recursive(&src, &dst)?;
        } else if file_type.is_file() {
            if name_str.ends_with(".tsp") {
                // .tsp -> .tsx. The contents are the same;
                // tsc treats .tsx as TSX.
                let dst_tsx = dst_root.join(format!(
                    "{}.tsx",
                    name_str.strip_suffix(".tsp").unwrap_or(&name_str)
                ));
                std::fs::copy(&src, &dst_tsx)?;
            } else {
                // .ts, .d.ts, .json, etc. -- copy verbatim.
                // This is what makes the slice
                // transparent to user helpers like
                // `pages/_db.ts` (the SQL demo imports
                // from it).
                std::fs::copy(&src, &dst)?;
            }
        }
    }
    Ok(())
}

/// Find the `tsc` binary. Order:
///   1. `./node_modules/.bin/tsc.cmd` (Windows)
///   2. `./node_modules/.bin/tsc`     (POSIX)
///   3. `tsc` on PATH
struct TscInvocation {
    program: PathBuf,
    prefix_args: Vec<PathBuf>,
    display: String,
}

fn locate_tsc_invocation() -> Option<TscInvocation> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut roots = vec![cwd.clone()];
    if cwd.join("bun").is_dir() {
        roots.push(cwd.join("bun"));
    }
    if let Some(workspace) = std::env::var_os("GITHUB_WORKSPACE") {
        let workspace = PathBuf::from(workspace);
        roots.push(workspace.clone());
        roots.push(workspace.join("bun"));
    }
    for root in roots {
        let bin_dir = root.join("node_modules").join(".bin");
        let candidate_cmd = bin_dir.join("tsc.cmd");
        if candidate_cmd.is_file() {
            return Some(TscInvocation {
                display: candidate_cmd.display().to_string(),
                program: candidate_cmd,
                prefix_args: Vec::new(),
            });
        }
        let candidate = bin_dir.join("tsc");
        if candidate.is_file() {
            return Some(TscInvocation {
                display: candidate.display().to_string(),
                program: candidate,
                prefix_args: Vec::new(),
            });
        }
        // Bun's Windows install does not always materialize npm's `.bin`
        // command shims. Invoke TypeScript's JavaScript entry point through
        // the already-installed Bun executable in that case.
        let script = root
            .join("node_modules")
            .join("typescript")
            .join("bin")
            .join("tsc");
        if script.is_file() {
            return Some(TscInvocation {
                display: format!("bun {}", script.display()),
                program: PathBuf::from("bun"),
                prefix_args: vec![script],
            });
        }
    }
    // Fall back to PATH: `Command::new("tsc")` resolves
    // through the standard PATH search on both Windows
    // and POSIX.
    Some(TscInvocation {
        program: PathBuf::from("tsc"),
        prefix_args: Vec::new(),
        display: "tsc".to_string(),
    })
}

/// The tsconfig the tsc check writes. Conservative: it
/// only enables the language features a `.tsp` page uses
/// (JSX, ES modules) and explicitly relaxes `strict` so
/// existing pages that use `?? null` etc. are not flagged.
/// The `paths` table maps the three frozen module names
/// to the bundled declaration files; without it, tsc
/// would refuse to resolve `import "tsp:server"`.
///
/// `skipLibCheck: true` is required because `tsp-server.d.ts`
/// hand-rolls its `Context` interface; without skipping
/// lib check, tsc would complain that some interface
/// members are not strictly compatible with each other
/// across the file (the hand-rolled shape is the contract
/// per the typings e2e pin, not a bug to be fixed).
const TSC_TSCONFIG_JSON: &str = r#"{
  "compilerOptions": {
    "module": "esnext",
    "moduleResolution": "bundler",
    "target": "es2020",
    "jsx": "preserve",
    "noEmit": true,
    "skipLibCheck": true,
    "strict": false,
    "esModuleInterop": true,
    "paths": {
      "tsp:server": ["./tsp-types/tsp-server.d.ts"],
      "tsp:html": ["./tsp-types/tsp-html.d.ts"],
      "tsp:runtime": ["./tsp-types/tsp-runtime.d.ts"]
    }
  },
  "include": [
    "pages/**/*.tsx",
    "pages/**/*.ts",
    "tsp-types/**/*.d.ts"
  ]
}
"#;

fn run_graph() -> ExitCode {
    // Parse optional flag. Default output is
    // tab-separated (`<path>\timports=[<a>,<b>,...]`).
    // `--json` emits a stable JSON array for tooling /
    // CI consumption.
    let raw_args: Vec<String> = std::env::args().skip(2).collect();
    let mut json = false;
    let mut i = 0;
    while i < raw_args.len() {
        let arg = &raw_args[i];
        if arg == "--json" {
            json = true;
            i += 1;
        } else if arg == "--help" || arg == "-h" {
            println!(
                "Usage: tspserver graph [--json]\n\n\
                 Prints one line per module in the pages/ directory:\n\
                   <path>\\timports=[<a>,<b>,...]\n\
                 --json    emit a JSON array of {{path, imports}} instead."
            );
            return ExitCode::SUCCESS;
        } else {
            eprintln!("tsp graph: unexpected argument `{arg}`");
            return ExitCode::from(2);
        }
    }

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
    if json {
        println!("[");
        for (idx, node) in nodes.iter().enumerate() {
            let imports: Vec<String> = node
                .imports
                .iter()
                .map(|id| id.as_path().to_string_lossy().into_owned())
                .collect();
            let imports_json: Vec<String> = imports.iter().map(|i| json_string(i)).collect();
            let prefix = if idx == 0 { "  " } else { ",\n  " };
            print!(
                "{prefix}{{\"path\":{},\"imports\":[{}]}}",
                json_string(&node.path.to_string_lossy()),
                imports_json.join(",")
            );
        }
        if !nodes.is_empty() {
            println!();
        }
        println!("]");
    } else {
        for node in nodes {
            let imports = node
                .imports
                .iter()
                .map(|id| id.as_path().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(",");
            println!("{}\timports=[{}]", node.path.display(), imports);
        }
    }
    ExitCode::SUCCESS
}

/// Phase 11 tooling (plan §11): write the three
/// `tsp:*` declaration files (`tsp-server.d.ts`,
/// `tsp-html.d.ts`, `tsp-runtime.d.ts`) into the
/// user-supplied output directory (default `.tsp-types`).
///
/// Usage:
///   tspserver typings                  # writes to ./.tsp-types
///   tspserver typings <DIR>            # writes to <DIR>
///   tspserver typings --out <DIR>      # same
///
/// The hand-rolled content lives in `bun/src/runtime/tsp/typings.rs`
/// (loaded via `include_str!` from `tsp-types/` at the repo
/// root). A drift between the runtime surface and the
/// typings is pinned by the unit test in `typings.rs`
/// (asserts the public exports are still wired correctly)
/// and by the e2e in `start_order.rs`
/// (`tspserver_typings_emits_three_dts_files`).
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
                "Usage: tspserver typings [--out <DIR>]\n\n\
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
            eprintln!("tsp typings: cannot write {}: {error}", target.display());
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
