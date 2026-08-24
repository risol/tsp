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

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use bun_runtime_tsp::generation::{PageRef, PageRegistry};
use bun_runtime_tsp::host;
use bun_runtime_tsp::jsc_bridge::{self, BunRuntime};
use bun_runtime_tsp::module_graph::ModuleGraph;
use bun_runtime_tsp::router::RouteTable;
use bun_runtime_tsp::watcher::{self, WatchConfig};

fn main() -> ExitCode {
    let port = match host::resolve_port() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("TSPv2PoC1: {e}");
            return ExitCode::from(2);
        }
    };

    let routes_dir = resolve_routes_dir();
    eprintln!("TSPv2PoC1: scanning routes from {}", routes_dir.display());
    let routes: &'static RouteTable = match RouteTable::scan(&routes_dir) {
        Ok(table) => leak_table(table),
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
    let registry: &'static PageRegistry = match build_registry(routes) {
        Ok(r) => leak_registry(r),
        Err(e) => {
            eprintln!("TSPv2PoC1: {e}");
            return ExitCode::from(2);
        }
    };

    let bun: &'static BunRuntime = match jsc_bridge::resolve_bun_bin() {
        Ok(p) => leak_bun(BunRuntime { bin: p }),
        Err(e) => {
            eprintln!("TSPv2PoC1: {e}");
            return ExitCode::from(2);
        }
    };
    eprintln!("TSPv2PoC1: bun = {}", bun.bin.display());

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
    let watch_config = WatchConfig {
        routes_root: routes_dir.clone(),
        poll_ms: watcher::DEFAULT_POLL_MS,
    };
    let watcher_handle = watcher::spawn(watch_config, graph, registry_arc);
    eprintln!(
        "TSPv2PoC1: watcher polling {} every {}ms",
        routes_dir.display(),
        watcher::DEFAULT_POLL_MS
    );

    if let Err(e) = host::serve("0.0.0.0", port, routes, registry, bun) {
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

/// Walk the RouteTable and create one PageRef per HTTP method
/// the source file actually exports. We read the source once
/// at boot (instead of per request) so the registry's
/// `state` is meaningful from the first call.
fn build_registry(routes: &RouteTable) -> Result<PageRegistry, String> {
    let registry = PageRegistry::new();
    for route in routes.iter() {
        let source = match bun_runtime_tsp::page::prepare(route) {
            Ok(s) => s,
            Err(e) => return Err(format!("prepare {}: {e}", route.source.display())),
        };
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

fn leak_table(table: RouteTable) -> &'static RouteTable {
    Box::leak(Box::new(table))
}
fn leak_registry(r: PageRegistry) -> &'static PageRegistry {
    Box::leak(Box::new(r))
}
fn leak_bun(b: BunRuntime) -> &'static BunRuntime {
    Box::leak(Box::new(b))
}