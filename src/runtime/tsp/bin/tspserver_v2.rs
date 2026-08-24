//! TSP v2 native host binary entry point (PoC 1, slice 2 of 5+).
//!
//! See `tsp-v2-plan.md` sect.70 (PoC 1) and `tsp-v2-specification.md` for
//! the contract this binary will eventually implement. v1's `src/main.ts`
//! remains the default working server; this binary is the side-by-side
//! v2 host.
//!
//! Slice 2 scope (this file): bind a stdlib TCP listener, accept
//! connections in a thread, hand-write a 404 response for every
//! request. No router, no JSC. Slice 3 adds the route scanner; slice 4
//! adds the JSC + transpile deps; slice 5 executes `routes/index.tsp`
//! and returns the rendered HTML.

use std::process::ExitCode;

use bun_runtime_tsp::host;

fn main() -> ExitCode {
    let port = match host::resolve_port() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("TSPv2PoC1: {e}");
            return ExitCode::from(2);
        }
    };
    if let Err(e) = host::serve("0.0.0.0", port) {
        eprintln!("TSPv2PoC1: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}