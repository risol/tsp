//! TSP v2 native host binary entry point (PoC 1, slice 3 of 5+).
//!
//! See `tsp-v2-plan.md` sect.70 (PoC 1) and `tsp-v2-specification.md` for
//! the contract this binary will eventually implement. v1's `src/main.ts`
//! remains the default working server; this binary is the side-by-side
//! v2 host.
//!
//! Slice 3 scope (this file): scan the `routes/` directory, build the
//! route table, hand it to `host::serve`. Slice 4 adds the JSC +
//! transpile deps; slice 5 executes `routes/index.tsp` and returns the
//! rendered HTML.

use std::path::PathBuf;
use std::process::ExitCode;

use bun_runtime_tsp::host;
use bun_runtime_tsp::router::RouteTable;

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
    let routes: &'static RouteTable = match scan_routes(&routes_dir) {
        Ok(table) => leak_table(table),
        Err(e) => {
            eprintln!("TSPv2PoC1: {e}");
            return ExitCode::from(2);
        }
    };
    eprintln!("TSPv2PoC1: loaded {} route(s)", routes.len());

    if let Err(e) = host::serve("0.0.0.0", port, routes) {
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

fn scan_routes(dir: &std::path::Path) -> Result<RouteTable, bun_runtime_tsp::router::RouterError> {
    RouteTable::scan(dir)
}

/// Leak the route table so the listener can hold a `&'static` reference
/// for the rest of the process. The table is built once at boot, never
/// mutated, and lives as long as the server -- `Box::leak` makes the
/// lifetime story compile-check trivially for slice 3. Future slices
/// move to an `Arc<RouteTable>` so the leak goes away.
fn leak_table(table: RouteTable) -> &'static RouteTable {
    Box::leak(Box::new(table))
}