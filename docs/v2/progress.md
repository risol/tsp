# TSP v2 — progress log

Tracks the side-by-side v2 refactor driven by `tsp-v2-plan.md` (75 sections,
12 phases) and `tsp-v2-specification.md` (normative).

- **Strategy (locked with Sol 2026-08-24):** side-by-side. v1
  `src/main.ts` keeps working as the default binary. v2 native host lives
  in `bun/src/runtime/tsp/` and ships as `tspserver_v2`. No v1 code
  removal in v2 slices until each v2 capability has parity.
- **First slice (locked):** PoC 1 from plan §70 — the 7-step vertical slice
  (Rust HTTP -> `routes/index.tsp` -> TSX transpile -> JSC instantiate ->
  find `GET` -> call `GET(ctx)` -> return Response / `<h1>Hello</h1>`).
  Nothing else from the 75-section plan enters until PoC 1 lands.
- **Out of PoC 1 scope (per plan §70):** session, DB, redis, fragments,
  watcher, multi-worker, full JSX attrs, config hot reload.

## Slice ledger

Each slice has its own commit, a `verify` line that says what command
proves it works, and a `next` line that says what the next slice will
build on top. Don't add a new slice until the previous one's `verify`
is green.

### Slice 1 — workspace + boot stub (in progress)

- **Why:** prove the v2 crate slot exists, builds inside the Bun
  workspace, and produces a runnable binary. No HTTP, no JSC — the goal
  is "I can shell to `tspserver_v2` and see the banner."
- **What landed:**
  - `bun/Cargo.toml` workspace adds `"src/runtime/tsp"`
  - `bun/src/runtime/tsp/Cargo.toml` (no external deps yet)
  - `bun/src/runtime/tsp/lib.rs` (library shell, plan §26 module list as
    doc comment)
  - `bun/src/runtime/tsp/bin/tspserver_v2.rs` (prints boot banner)
- **Verify:** `cargo build -p bun_runtime_tsp` succeeds; running the
  produced `tspserver_v2(.exe)` prints the three-line banner.
- **Next:** Slice 2 adds the HTTP listener on port 3000 with a stub
  404 response. No routing yet.

### Slices 2..N (planned, not started)

- **Slice 2 — HTTP listener.** uWS-based listener on port 3000. Returns
  a hand-written 404 body for every request. No router, no JSC.
- **Slice 3 — Route scanner + matcher.** Read `routes/` directory, build
  a linear matcher (PoC 1: only `/` via `routes/index.tsp`). Return a
  stub "route matched but no JSC yet" body for `/`; 404 elsewhere.
- **Slice 4 — JSC + transpiler deps.** Add `bun_jsc` + `bun_transpiler`
  to the crate. First `cargo build` here is heavy (Bun workspace
  compile, 20–30 min on cold cache). Run it in the background while
  writing Slice 5 code.
- **Slice 5 — Hello vertical.** Read `routes/index.tsp`, transpile
  TSX, evaluate in JSC, find `GET` export, call it with an empty
  context, convert result to a 200 `text/html` response.
  **Verify:** `curl http://localhost:3000/` returns
  `<h1>Hello from TSP v2</h1>`.
- **Slice 6 — Ledger close + sync with Sol.** Mark PoC 1 done in this
  log, commit, then ask Sol which phase of plan §61 to enter next
  (Phase 0 docs freeze / Phase 1 native skeleton widening / direct
  Phase 4 module graph / etc.).

## Open decisions deferred past PoC 1

- Crate layout choice: `src/runtime/tsp/` (Bun-fork style, plan §26
  option A) vs. top-level `crates/tsp-v2-host/` (option B). Currently on
  option A because Bun-fork style is what the plan's risk mitigations
  (Risk 1) recommend; switching is mostly mechanical.
- Whether `tspserver_v2` binary stays as the long-term product name or
  is renamed to `tsp` / `tspserver` once v1 is retired.
- Whether `routes/` lives at the repo root or under a v2-specific
  subdir (e.g. `v2/routes/`) to avoid stepping on a future v1 routes
  dir at the root.
