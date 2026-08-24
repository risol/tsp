//! TSP v2 native host binary entry point (PoC 1, slice 6 of 6).
//!
//! See `tsp-v2-plan.md` sect.70 (PoC 1) and `tsp-v2-specification.md` for
//! the contract this binary will eventually implement. v1's `src/main.ts`
//! remains the default working server; this binary is the side-by-side
//! v2 host.
//!
//! Slice 6 closes the PoC 1 vertical slice by spawning the vendored
//! `bun.exe` to evaluate matched `.tsp` pages. Verify is `curl /`
//! returns the rendered HTML from `routes/index.tsp`.

use std::path::PathBuf;
use std::process::ExitCode;

use bun_runtime_tsp::host;
use bun_runtime_tsp::jsc_bridge::{self, BunRuntime};
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
    let routes: &'static RouteTable = match RouteTable::scan(&routes_dir) {
        Ok(table) => leak_table(table),
        Err(e) => {
            eprintln!("TSPv2PoC1: {e}");
            return ExitCode::from(2);
        }
    };
    eprintln!("TSPv2PoC1: loaded {} route(s)", routes.len());

    let bun: &'static BunRuntime = match jsc_bridge::resolve_bun_bin() {
        Ok(p) => leak_bun(BunRuntime { bin: p }),
        Err(e) => {
            eprintln!("TSPv2PoC1: {e}");
            return ExitCode::from(2);
        }
    };
    eprintln!("TSPv2PoC1: bun = {}", bun.bin.display());

    if let Err(e) = host::serve("0.0.0.0", port, routes, bun) {
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

fn leak_table(table: RouteTable) -> &'static RouteTable {
    Box::leak(Box::new(table))
}

fn leak_bun(b: BunRuntime) -> &'static BunRuntime {
    Box::leak(Box::new(b))
}