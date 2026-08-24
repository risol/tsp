//! TSP v2 native host binary entry point (PoC 1, slice 1 of 5+).
//!
//! See `tsp-v2-plan.md` §70 (PoC 1) and `tsp-v2-specification.md` for the
//! contract this binary will eventually implement. v1's `src/main.ts`
//! remains the default working server; this binary is the side-by-side v2
//! host.
//!
//! Slice 1 scope (this file): print a boot banner that proves the crate
//! builds, links, and runs inside the Bun workspace. No HTTP listener, no
//! router, no JSC. Slice 2 adds the listener; Slice 3 the route matcher;
//! Slice 4 the JSC + transpile deps; Slice 5 the GET-execute-Hello path.

fn main() {
    // Plan §27: product name is `tsp`; this binary is the PoC build
    // (`tspserver_v2`) and the long-term CLI is what the user shells as
    // `tsp dev` / `tsp serve`. Keep the banner short and machine-greppable
    // so the slice-1 verify command can `grep TSPv2PoC1` against it.
    println!("TSPv2PoC1: bun_runtime_tsp crate booted (slice 1 of plan §70 PoC 1)");
    println!("TSPv2PoC1: plan = tsp-v2-plan.md, spec = tsp-v2-specification.md");
    println!("TSPv2PoC1: no listener yet -- slice 2 adds HTTP");
}
