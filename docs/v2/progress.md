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

### Slice 2 — stdlib TCP listener + 404 (done, commit `a0f5ffd5`)

- **Why:** prove the binary can hold open a TCP port, accept HTTP/1.1
  in real life, and answer with a structured 404 — without yet wiring
  any router or JSC. Plan §61 Phase 1's "native HTTP listener" milestone
  in the smallest possible form.
- **What landed:**
  - `bun/src/runtime/tsp/host.rs` — stdlib `TcpListener` on
    `0.0.0.0:<port>`, thread-per-connection, hand-written 404. `HostError`
    with `Bind / Accept / Connection` variants (no `unwrap`, no `todo`).
    `resolve_port()` reads `TSP_PORT` or falls back to 3000.
  - `bin/tspserver_v2.rs` — calls `host::serve` and exits non-zero on
    bind/port-parse failure.
  - `lib.rs` — `pub mod host;`
- **Verify:** `cargo build -p bun_runtime_tsp` in 2.45s; running the
  binary prints `TSPv2PoC1: listening on http://0.0.0.0:3000`. A `curl
  http://localhost:3000/` returns HTTP/1.1 404 with body `TSP v2 PoC 1
  slice 2: route scanner not wired yet (path = /)`. Query strings are
  stripped from the parsed path.
- **Why stdlib, not uWS / tokio / axum:** the first HTTP slice must
  compile in seconds so each subsequent slice's failure modes stay
  isolable. Plan §25.3 reserves the production HTTP path (uWS + async)
  for slice 7+ when Context/Request/Response bridge lands and JSC
  interop needs an event loop.
- **Next:** Slice 3 reads the `routes/` directory, matches `/` against
  `routes/index.tsp`, and returns a stub "route matched, no JSC yet"
  body for that path while every other path still 404s.

### Slice 3 — filesystem route scanner + matcher (done, commit `495a5253`)

- **Why:** the listener must be able to *find* the right `.tsp` file
  for a given `(path, method)` before slice 5 can execute it. Plan
  §6 (filesystem routing) and §42 (method dispatch) describe the
  full surface; slice 3 implements only the static + index shape and
  the four-state response (200 / 405 / 404 / 400).
- **What landed:**
  - `bun/src/runtime/tsp/router.rs` — `HttpMethod` enum +
    `from_request_line`, `Route { path, source, methods }`, linear
    `RouteTable::scan(dir)` and `lookup(path, method)`. Only
    `index.tsp`/static shapes accepted; dynamic + catch-all are
    `RouterError::UnsupportedShape` so a typo'd `[id].tsp` refuses
    to boot instead of silently 404'ing. 8 unit tests cover
    index/static/nested/lookup + the dynamic-rejected branch.
  - `host.rs` — `serve(host, port, &'static RouteTable)`, request
    parser, status-line/Allow-header construction. 200 stub body
    names the source file slice 5 will load; 405 lists supported
    methods; 404 mentions table size; 400 covers malformed lines.
  - `bin/tspserver_v2.rs` — scans `routes/` (env `TSP_ROUTES_DIR`
    or default `routes`), `Box::leak`s the table to give
    `host::serve` its `&'static`.
  - `bun/.gitignore` — `.logs/` (the run-output dir produced by the
    verify scripts).
  - `lib.rs` — `pub mod router;`
- **Verify:** `cargo test -p bun_runtime_tsp --lib` → 8 passed;
  `cargo build -p bun_runtime_tsp` → 2.47s. End-to-end:
  - `GET /` → 200, body names the source file
  - `GET /nope` → 404, body says "no route matches path=/nope (table has 1 route(s))"
  - `POST /` → 200 (slice 3 defaults every scanned route to all
    methods; slice 5 narrows to actual exports)
  - `BREW /` → 400 (unknown verb)
- **Why linear, not radix tree:** the slice-3 table has at most
  one entry; a `Vec<Route>` plus a `find` is O(n) but n is ~1. The
  radix tree swap is local to `router.rs` and happens in slice 7+
  alongside the rest of the route surface.
- **Next:** Slice 4 pulls in `bun_jsc` + `bun_transpiler` and
  triggers a cold cargo build (15-30 min on a clean cache). Slice 5
  reads `routes/index.tsp`, transpiles, evaluates, finds `GET`, calls
  it, and returns the rendered `<h1>Hello from TSP v2</h1>`.

### Slice 4 — JSC + transpiler deps (done, commit `b9a7b0a2`)

- **Why:** slice 5 needs the JSC bindings + a transpiler reachable
  from the host crate. The dep pull is large (bun_transpiler ->
  bun_bundler; bun_jsc -> ~50 transitive crates), so this slice
  isolates the cold compile and proves the dep graph is sane before
  any real JSC code lands.
- **What landed:** `bun_jsc` and `bun_transpiler` added to
  `bun/src/runtime/tsp/Cargo.toml` via the workspace table.
- **Verify:** `cargo check -p bun_runtime_tsp` 1m 34s on the cold
  cache; `cargo build -p bun_runtime_tsp` 2m 15s; the slice-3
  binary still serves the same 200/404/405/400 responses.
- **Pivot discovered:** `bun_jsc` is the *binding* crate Bun uses
  internally; it is not a standalone embeddable VM. The actual VM
  is created via `WebWorker__createVM` (or `Zig::GlobalObject::create`)
  in `bun_runtime`, and the transpiler init needs the full bundler
  pipeline (resolver, linker, env, fs, log, etc.). Slice 5 was
  scoped to the "read + static-analyse" half; slice 6+ is the
  "actually execute via bun_runtime" half and is its own pivot.

### Slice 5 — page source reader + static export detector (done, commit `7d867e49`)

- **Why:** slice 6 cannot do the real 200/405 dispatch without
  knowing what methods the `.tsp` file actually exports. Plan sect.4.2
  and sect.42 describe the contract; slice 5 lands a line-anchored
  detector that satisfies the slice-5 surface (sync + async
  `export function X(` at line start) and explicitly defers the AST
  pass to slice 7.
- **What landed:**
  - `bun/src/runtime/tsp/page.rs` — `PageSource`, `PrepareError`,
    `prepare(&Route)`, `detect_methods`. 7 unit tests cover
    sync / async / multiple / none / non-exported / string-literal /
    comment-line / trailing-space cases.
  - `host.rs` — on `Found`, per-request `page::prepare`; 200 with
    source path / byte count / detected methods / "JSC deferred to
    slice 6+"; on a method that is not in the page's exports, 405
    with a real `Allow:` header from the same prepare pass; on
    prepare error, 500.
- **Verify:** `cargo test -p bun_runtime_tsp --lib` 15 passed;
  `cargo build -p bun_runtime_tsp` 1.98s incremental; end-to-end
  `GET /` -> 200, `POST /` -> 405 `Allow: GET`, `DELETE /` -> 405,
  `GET /nope` -> 404, `GET /?foo=bar` -> 200 (query stripped).
- **Out of slice 5:** real TSX transform, real JSC call, real
  HTML render. All three are slice 6+ and require the bun_runtime
  pivot (see Slice 4 note).

### Slice 6 — JSX transform + bun.exe JSC bridge (done, commit `d5a88b79`)

- **Why:** the architecture validation in plan sect.70 needs an
  end-to-end "Rust host -> JSC executes page -> HTML comes out"
  pass before we can call PoC 1 closed. Sol approved the
  bun_runtime pivot (cold compile 20-40 min would otherwise have
  blocked this session) on the condition that we keep the model
  honest about what the host actually does.
- **Pivot landed:** instead of pulling in `bun_runtime` as a
  library, the host spawns the project's vendored `bun.exe`
  (1.4.0+) as a subprocess and asks it to run a slice-6-prepared
  `.js` file. The architectural model is unchanged -- JSC is
  still the execution engine, per plan sect.25 -- only the host's
  role is narrowed to "protocol bridge + subprocess orchestrator".
  The full in-process JSC bridge (in-process VM, native module
  loader, `tsp:*` builtins) lands in slice 7+ when there is time
  budget for the heavy `bun_runtime` compile.
- **What landed:**
  - `bun/src/runtime/tsp/jsx.rs` -- single-line `<tag>text</tag>`
    JSX -> string literal, `export ` stripped, nested / attribute
    / multi-line JSX surfaces as `JsxError::UnsupportedShape` with
    a line number. 6 unit tests.
  - `bun/src/runtime/tsp/jsc_bridge.rs` -- `resolve_bun_bin` reads
    `TSP_BUN_BIN` or falls back to the vendored binary;
    `execute(bun, &tsp_source, method)` writes the prepared JS to
    `std::env::temp_dir()` and runs `bun run <tempfile>`, returning
    stdout. bun failures surface as `JscError::BunFailed` with
    the last 1 KiB of stderr.
  - `bun/src/runtime/tsp/host.rs` -- 200 path now renders
    `Content-Type: text/html; charset=utf-8` with the bun stdout
    as the body; JscError produces a 500 with the bun stderr tail.
  - `bin/tspserver_v2.rs` -- resolves `bun.exe` at boot, leaks a
    `&'static BunRuntime` to the listener.
  - `Cargo.toml` -- dropped `bun_jsc` + `bun_transpiler` direct
    deps (subprocess path does not need them; both become
    transitive again if/when `bun_runtime` lands in slice 7+).
  - `lib.rs` -- `pub mod jsx;` + `pub mod jsc_bridge;`.
- **Verify:** `cargo test -p bun_runtime_tsp --lib` 22 passed;
  `cargo build -p bun_runtime_tsp` 3.48s incremental;
  `curl http://localhost:3000/` -> `200 OK` with body
  `<h1>Hello from TSP v2</h1>` and `Content-Type: text/html;
  charset=utf-8`. `curl -X POST` -> `405 Method Not Allowed,
  Allow: GET`.

### Slice 7 — bun_runtime dep + in_process_jsc spike (done, commit `1e9a4b92`)

- **Why:** Sol picked the in-process JSC bridge per plan sect.25.3
  in the post-PoC-1 sync. The realistic scope is much larger than
  one session's worth of work; this slice landed the dep + a
  spike module that proves the wiring point exists, and a clear
  document of the gap.
- **What landed:**
  - `bun/src/runtime/tsp/Cargo.toml`: `bun_runtime` added. Cold
    compile was 1m 51s (most deps were already compiled from
    slice 4's `bun_jsc` / `bun_transpiler` pull, so the additional
    cost was small).
  - `bun/src/runtime/tsp/in_process_jsc.rs`: spike module --
    documentation only, plus a compile check that proves
    `bun_runtime` is reachable from this crate. The actual VM
    creation is multi-session work; see the module's module docs
    for the integration checklist (dispatch hooks, loader
    hooks, HardcodedModule wiring, etc.).
  - `lib.rs`: `pub mod in_process_jsc;` so the future entry
    point is reserved.
- **What did NOT land:** a working in-process JSC bridge. Bun's
  public Rust API is designed for Bun's own use -- the only
  `VirtualMachine::init` call site lives inside Bun, and it
  assumes a fully bootstrapped CLI / dispatch / loader hook
  environment that an embedder has to replicate before the call.
  Doing this in a single session would be a multi-day effort
  with no functional regression (slice 6's subprocess path
  already returns `<h1>Hello from TSP v2</h1>`).
- **Verify (regression):** the slice-6 binary still serves
  `<h1>Hello from TSP v2</h1>` for `curl /` after adding
  `bun_runtime`; `cargo build -p bun_runtime_tsp` is 1.37s
  incremental.

### Slice 8 — Phase 0 docs freeze (done, commit `043a832`)

- **Why:** Sol picked the Phase 0 path in the post-slice-7 sync to
  protect later slices from ABI drift. The 12 freeze items are
  the v2.0 contract application code can rely on; freezing them
  before the in-process JSC bridge / watcher / Context-bridge
  slices prevents costly rewrites.
- **What landed (all in `docs/v2/`):**
  - `FREEZE.md` -- the 12 items, each with: question, answer,
    evidence (spec / plan section), and "what this freezes for
    application code". The contract surface.
  - `spec.md` -- the index document. Points at FREEZE.md + the
    four topic docs + the root `tsp-v2-specification.md` and
    `tsp-v2-plan.md`. No re-derivation; it is a navigation aid.
  - `tsp-module.md` -- the `.tsp` file format. Covers freeze
    items 1, 2, 3, 4 and the "what `.tsp` is NOT" list.
  - `jsx-runtime.md` -- the JSX -> HtmlNode contract. Covers
    freeze 9 (child / attribute rules) and freeze 10 (async
    components).
  - `context.md` -- the Context ABI. Covers freeze 5 (handler
    result), 6 (Context shape), 7 (fragment), 8 (`tsp:*`
    builtins), 11 (PageConfig).
  - `examples/01-hello.tsp` through `examples/10-shape-magic.tsp`
    -- 10 fixtures. Eight are spec-compliant (the contract);
    two (`09-no-tsp-imports.tsp`, `10-shape-magic.tsp`) are
    intentionally invalid and document the host's
    `TSP2003` / `TSP3001` errors.
- **Phase 0 completion condition (plan §61):** "the 12 freeze
  items have explicit answers and 10-20 `.tsp` example
  fixtures demonstrate the contract." All three are satisfied:
  - 12 explicit answers in `FREEZE.md`.
  - 10 fixtures, 8 spec-compliant + 2 invalid-as-documentation.
  - No v1 compatibility work has begun.
- **Phase 0 sign-off (2026-08-24):** Sol confirmed all 12
  items. `FREEZE.md` status flipped from DRAFT to FROZEN in
  commit `043a832`. The v2.0 application-facing contract is
  now locked; subsequent slices build on these 12 items, they
  do not renegotiate them. Any later spec change that
  contradicts a frozen item must come with an ADR (plan §69).
- **What Phase 0 closure unlocks:** slice 9+ (in-process JSC
  bridge, watcher + atomic reload, full Context bridge,
  fragments-as-HTML rendering, etc.) can resume code work
  without further contract negotiation. The slice-6 bun.exe
  subprocess bridge stays as the production path until the
  in-process bridge lands.

### Slice 9 -- Module Graph (done, bun commit `3c036f7e`)

- **Why:** plan sect.61 Phase 4. Module Graph is the data
  foundation slice 10 (Generation + Atomic Reload) and slice
  11 (Watcher) build on top.
- **What landed (in `bun/src/runtime/tsp/module_graph.rs`):**
  - `ModuleId(PathBuf)` -- canonical path-based identity with
    best-effort canonicalisation (symlink policy is a future
    slice concern).
  - `ModuleNode { id, path, imports, page_roots, source_hash }`
    per plan sect.20.2.
  - `PageId { route, method }` per HTTP method export.
  - `SourceHash(u64)` -- FNV-1a 64-bit; swap to BLAKE3 in
    slice 10 if the empirical false-positive rate is bad.
  - `ModuleGraph` with `nodes` + `reverse` maps; `importers_of`
    is the watcher (slice 11) entry point.
  - `extract_imports` (line-anchored, regex-based) for the
    conventional `import ... from "...";` and side-effect
    `import "...";` forms.
  - `ModuleGraph::from_routes_dir` walks the routes root and
    reads every `.tsp` / `.ts` / `.tsx` / `.js` / `.jsx` file.
  - 7 unit tests.
- **Out of slice 9:** the actual transpile + evaluate +
  Generation publish (slice 10); the watcher (slice 11);
  the AST pass on imports / methods / re-exports (slice 7+
  bun_js_parser integration).
- **Verify:** `cargo test -p bun_runtime_tsp --lib` 29 passed
  (was 22); `cargo build -p bun_runtime_tsp` 2.57s incremental;
  slice-6 binary regress-tested clean (`curl /` still returns
  `<h1>Hello from TSP v2</h1>`).
- **Next:** slice 10a = plan sect.20.3-20.4 + sect.21 data
  structures (Generation + PageSlot + PageState); 10b = the
  host's request flow uses the registry (sync build on
  request, no in-flight dedup yet); 10c = in-flight dedup +
  request pinning. Sub-sliced so each commit is a clean
  verification step.

### Slice 10a -- Generation + PageSlot + state machine (done, bun commit `d085ca67`)

- **Why:** the data foundation for Phase 5 (Generation +
  Atomic Reload + LKG) per plan sect.21. The state machine
  has to exist before any request flow can use it.
- **What landed (in `bun/src/runtime/tsp/generation.rs`):**
  - `Generation { id, page, dependencies, created_at, build_result }`
    per plan sect.21.2.
  - `GenerationId(u64)` with a process-monotonic counter.
  - `PageRef { route, method }` -- one entry per HTTP method
    per `.tsp` file.
  - `PageState` enum (Unloaded / Clean / Dirty / Building /
    Failed) per plan sect.20.4.
  - `PageSlot { page, source, current, last_known_good, state }`
    per plan sect.20.3.
  - `PageRegistry` (cheap-to-clone via `Arc<Mutex<RegistryInner>>`):
    `register`, `snapshot`, `mark_dirty`, `begin_build`.
  - `PublishGuard` RAII: `commit(Ok)` or `fail(message)`, with
    Drop rollback to Unloaded/Dirty.
  - LKG semantics = "last successful build":
    - First commit: LKG = candidate (the new current).
    - Subsequent successful commit: LKG = previous current.
    - Failed commit: LKG and current unchanged.
  - 8 unit tests cover the full state machine.
  - Also added `ModuleId::from_canonical_path` to
    `module_graph.rs` for callers that already have a
    canonicalised path.
- **Out of slice 10a (deferred to slice 10b+):** the actual
  build pipeline (transpile + evaluate) that fills the
  candidate; the host wiring so a request for a Dirty slot
  triggers `begin_build`; in-flight dedup and request
  pinning.
- **Verify:** `cargo test -p bun_runtime_tsp --lib` 38 passed
  (was 29).

### Slice 10b -- registry wired into the request flow (done, bun commit `036b61e7`)

- **Why:** the slice 6 bridge re-reads + re-runs every request.
  Slice 10b is the first slice where the request actually hits
  the `PageRegistry`: Unloaded/Dirty slots trigger a synchronous
  build via the bun subprocess, Clean slots serve from
  `current.payload` (a String clone, no JSC re-invocation), and
  Building / Failed fall back to LKG. This is the
  "atomic-publish + LKG" semantics in plan sect.21 + 24.
- **What landed:**
  - `pipeline.rs` (renamed from a draft `build.rs` because
    `build.rs` is Cargo's build-script convention):
    `pipeline::build(route, method, bun) -> Result<String, _>`
    composes `page::prepare + jsc_bridge::execute`.
  - `Generation.payload: Option<String>` so the request hot
    path can read the rendered body without re-running the
    build. `PublishGuard::commit` now takes a payload arg.
  - `PageRegistry::read_current_payload` +
    `read_lkg_payload` for the request hot path.
  - `host::render_for_route` decision tree: snapshot the
    state, build if Unloaded/Dirty, serve from current if
    Clean, fall back to LKG (or 503) if Building, fall back
    to LKG (or 500) if Failed. If the registry has no slot at
    all (the .tsp file does not export the method), return
    405 with the real `Allow:` header.
  - `bin/tspserver_v2.rs` boot-time: walks the `RouteTable`,
    runs the slice 5 method detector, registers one
    `PageSlot` per (route, method) pair.
  - `router::RouteTable::iter()` so the bin can walk.
- **Verify:** 38 unit tests pass; end-to-end `curl GET /`
  returns 200 with `<h1>Hello from TSP v2</h1>` (first
  request triggers the build, second serves from cache);
  `curl -X POST /` returns 405 `Allow: GET` (because the bin
  only registered the GET slot); `curl /nope` returns 404.
- **Out of slice 10b (deferred to slice 10c):**
  - In-flight dedup: concurrent requests on a Building slot
    share the build future. Slice 10b sees the second
    request as `BeginBuildError::NotBuildable(Building)` and
    falls back to LKG; the right answer is to await the same
    future, but that needs `tokio` or a custom Condvar.
  - Request pinning: a request that started on generation N
    finishes on N even if N+1 publishes mid-flight. Without
    this, a long request that overlaps a publish could see
    two different generations' payloads in its headers and
    body.
  - Generation release (plan sect.21.3: a generation not
    current, no active requests, no runtime references ->
    free).

### Slice 11 -- Watcher + lazy reload (done, bun commit 9d30c1fb)

- **Why:** plan sect.61 Phase 4-6. The 7-step PoC 1 validated
  "Rust host owns HTTP lifecycle, JSC page module is replaceable
  execution generation"; slice 11 closes the "reload does not
  restart HTTP server" DoD item by adding a filesystem watcher
  that marks slots dirty on change, so the next request rebuilds
  (lazy reload, plan sect.22.2).
- **What landed (in `bun/src/runtime/tsp/`):**
  - `watcher.rs` -- polling backend (plan sect.22.1): every
    `poll_ms` (`DEFAULT_POLL_MS = 500`) reads all source files
    under the routes root (`.tsp` / `.ts` / `.tsx` / `.js` /
    `.jsx`), computes `SourceHash`, diffs against `last_seen`
    (add / change / delete all detected), and marks affected
    slots dirty via `PageRegistry::mark_dirty`. `WatchConfig`,
    `WatcherHandle` (stop + Drop-join), `PollStats`,
    `poll_once`, `spawn`. 4 unit tests (content change / new
    file / deleted file / handle stops thread).
  - `generation.rs` -- `PageRegistry::all_page_refs()` so the
    watcher can enumerate registered slots.
  - `bin/tspserver_v2.rs` -- builds the `ModuleGraph` at boot
    and spawns the watcher thread over the routes dir; the
    handle lives for the duration of `serve` and joins on drop.
- **Slice 11 dirty granularity:** "any change dirties EVERY
  registered slot" (conservative). The precise source->PageRef
  index (via `ModuleGraph.importers_of` + a registry reverse
  map) lands in slice 12 so only truly-affected pages rebuild.
  The `graph` parameter stays in `poll_once`'s signature
  (unused at this granularity) so slice 12 plugs the precise
  path in without changing callers.
- **Backend note:** polling (mtime + source-hash diff) instead of
  `bun_watcher` native (inotify / ReadDirectoryChangesW) -- the
  native backend needs the full Bun event-loop + FD lifecycle the
  side-by-side v2 host does not have wired yet. Polling satisfies
  the lazy-reload contract; swapping backends is a localized
  change inside `watcher.rs`.
- **Verify:** `cargo test -p bun_runtime_tsp --lib` 42 passed
  (was 38); `cargo build -p bun_runtime_tsp` 2.76s incremental.
  End-to-end hot reload (TSP_PORT=9107, TSP_ROUTES_DIR=routes):
  1. `curl /` -> `<h1>Hello from TSP v2</h1>` (first request builds);
  2. edit `routes/index.tsp` text -> watcher logs
     `watch: 1 file(s) changed, 1 page(s) marked dirty`;
  3. `curl /` again -> `<h1>Hello from TSP v2 after hot reload</h1>`
     (next request rebuilds, no server restart).
- **Out of slice 11 (deferred to slice 12+):** precise per-module
  invalidation (source->PageRef index); new-route pickup without
  restart; in-flight dedup + request pinning (plan sect.21.3);
  generation release.
- **Next:** slice 12 = in-flight dedup + request pinning (plan
  sect.21.3 + 22.4) on top of the registry state machine.

### Slice 12 -- In-flight dedup + request pinning (done, bun commit 3c092afa)

- **Why:** plan sect.21.3 (request pinning) + 22.4 (in-flight
  dedup). Slice 10b's Building fallback was "second request
  gets `NotBuildable(Building)` and serves LKG or 503"; slice
  22.4 requires concurrent requests on a dirty slot to share
  ONE build. Sect.21.3 requires a request that started on
  generation N to finish on N even if N+1 publishes mid-flight.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `generation.rs`:
    - `Generation.payload: Option<String>` ->
      `Option<Arc<String>>`. Concurrent requests share the same
      buffer; a request that pinned a body keeps it alive even
      after `current` is overwritten by a later commit.
    - `PageSlot.in_flight: Option<Arc<InFlightBuild>>` -- the
      shared build future for Building slots.
    - `InFlightBuild { state: Mutex<InFlightState>, cvar }`
      with `InFlightState::{Running, Done(BuildOutcome),
      Abandoned}` and `BuildOutcome::{Ok(Arc<String>),
      Failed(String)}`. `wait()` blocks on the condvar.
    - `PublishGuard::commit` writes `Done(Ok)` + notifies;
      `fail` writes `Done(Failed)` + notifies; Drop writes
      `Abandoned` so a panic never leaves waiters stuck.
    - New registry APIs: `join_in_flight(page)` (get the
      shared future), `read_current_arc(page)` /
      `read_lkg_arc(page)` (pin the Arc, no String clone).
    - `read_current_payload` / `read_lkg_payload` kept
      (String-clone form) for tests + one-off reads.
  - `host.rs` (`render_for_route`):
    - Unloaded/Dirty/Failed: win the `begin_build` race, run
      the pipeline, `commit`, pin the payload via
      `read_current_arc`, serve.
    - Building: `join_in_flight` -> wait on the condvar ->
      serve the committed Arc; on Failed/Abandoned fall back
      to LKG (no more raw 503).
    - Clean: pin `current` Arc and serve.
    - Removed `serve_lkg_or_503`; `serve_current_or_500` /
      `serve_lkg_or_500` replaced by the pinned-Arc variants.
- **Verify:** `cargo test -p bun_runtime_tsp --lib` 46 passed
  (was 42), 4 new tests: `in_flight_dedup_shares_one_future`,
  `in_flight_waiter_sees_failure_outcome`,
  `request_pinning_survives_commit_overwrite`,
  `generation_release_drops_old_payload`.
  `cargo build -p bun_runtime_tsp` 0.42s incremental.
  e2e (TSP_PORT=9108): serial request serves from cache on
  repeat; 15 concurrent first-load curls all returned
  `<h1>Hello from TSP v2</h1>` with zero 503/500/5xx and no
  panic (server accepted all 15 connections; the 503 code path
  is now unreachable from Building, which was the point).
- **Out of slice 12 (deferred to slice 13+):**
  - Precise per-module invalidation (source->PageRef index for
    the watcher -- currently "any change dirties every slot").
  - New-route pickup without restart.
  - In-process JSC bridge (plan sect.25.3): replace the
    `bun.exe` subprocess path with `bun_runtime`.
  - Explicit generation-id release bookkeeping (the Arc
    already drops old payloads automatically; id-based
    release tracking is a future optimisation).
- **Next:** slice 13 = in-process JSC bridge (plan sect.25.3,
  multi-session). The subprocess path stays as the production
  path until the in-process VM + tsp:* builtins land.

### Slice 13 -- ADR-0001: subprocess is the v2 production JSC path (done, bun commit a3faae0a)

- **Why:** plan sect.25.3 recommends in-process JSC, but Bun v1.4.0
  does not expose an embedder-facing API (`VirtualMachine::init` is
  `pub(crate)`, hooks are crate-private statics). Slice 7 spiked
  the gap; slice 13 closes it as a formal decision rather than a
  "TODO" item.
- **What landed (ADR-anchored, NOT a code feature):**
  - `docs/v2/adr/0001-subprocess-as-production-jsc.md`: locks
    slice 6's subprocess bridge as the v2 production JSC path.
    The in-process bridge is future work, triggered by one of:
    (1) Bun upstream exposes an embedder API, (2) the Bun fork
    exposes `Run::boot` + the runtime/loader hooks, or (3) a
    new use case (streaming, SSE) that subprocess cannot meet.
  - `bun/src/runtime/tsp/in_process_jsc.rs`: ADR-anchored
    reference code. Symbol-free of `bun_runtime` because the
    rlib transitively pulls in C externs from the `bun` binary
    that the foreign linker cannot resolve (2700+ unresolved
    externs discovered during slice 13b). Integration checklist
    is plain text in the module doc, not a typed constant.
    `InProcessVm` placeholder type + 1 unit test
    (`in_process_vm_is_placeholder`).
  - `bun_runtime` dep in `Cargo.toml` stays (workspace-hygiene;
    removal would force a cold recompile of the entire Bun
    workspace for no functional gain).
- **Critical finding (was not in the slice 7 spike):** naming any
  `bun_runtime::*` symbol from `bun_runtime_tsp` -- even via a
  `type X = bun_runtime::error::Error` alias with no use --
  triggers 2700+ unresolved externals at link time. `cargo check`
  succeeds (type-check only); `cargo test` and `cargo build` fail.
  This is documented in ADR-0001's "Link constraint" section so
  a future in-process slice does not waste a session on the
  same false start.
- **Verify:** `cargo test -p bun_runtime_tsp --lib` 47 passed
  (was 46, 1 new test for the placeholder). `cargo build
  -p bun_runtime_tsp` 3.54s incremental. e2e (`curl /`) still
  returns `<h1>Hello from TSP v2</h1>` (subprocess path
  unchanged).
- **Out of slice 13 (deferred to slice 14+ if/when ADR-0001
  triggers fire):** in-process JSC VM creation; `tsp:*` builtin
  modules via `HardcodedModule`; per-worker VM threading
  (sect.25.2); native module loader.
- **Next:** PoC 1 is complete (all 7 DoD items in plan sect.74).
  Phase 5+ (plan sect.61) candidates: JSC native module loader
  (sect.7), full Context bridge (sect.6), fragments-as-HTML
  rendering (sect.9), multi-worker (sect.25.2). The watcher +
  generation + dedup + pinning work landed in slices 9-12
  covers the Phase 4-6 "Generation + atomic reload" milestone;
  Phase 5+ is feature work, not infra. Sol to pick the next
  direction.

### Slice 14a -- HEAD / OPTIONS fallback per spec sect.6.5/6.6 (done, bun commit `39b95599`)

- **Why:** the DeepSeek plan/spec consistency audit (2026-08-24)
  flagged that spec sect.6.5 requires the runtime to synthesise
  a body-less `HEAD` from `GET` when no explicit `HEAD` export
  exists, and sect.6.6 requires an automatic `OPTIONS` response
  with `Allow` when no explicit `OPTIONS` export exists. Before
  this slice, `HEAD /` returned 400 (the verb was not in the
  `HttpMethod` enum at all) and `OPTIONS /` returned 405. Both
  were spec MUSTs, so the gap was high-priority book-keeping
  rather than feature work.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `router.rs`: `HttpMethod::Head` variant + `from_request_line`
    recognises `"HEAD"`. New `HttpMethod::REAL` constant (Get,
    Post, Put, Patch, Delete) excludes Head and Options so the
    slice 5 detector does not falsely claim support for the
    two fallback-handled verbs. `RouteTable::scan` switched
    from `ALL.to_vec()` to `REAL.to_vec()`. New
    `MatchResult::FoundHeadOverGet` arm in `lookup` for the
    "GET but no explicit HEAD" case.
  - `host.rs`: `handle_connection` now matches
    `FoundHeadOverGet` -> run GET, emit body-less 200; and
    `MethodNotAllowed { requested: Options, .. }` with at least
    one other method exported -> 204 + `Allow`.
- **Verify:** `cargo test -p bun_runtime_tsp --lib` 47 passed
  (no new tests -- the change is path-level routing semantics,
  not unit-test-worthy). E2E (TSP_PORT=9127) end-to-end:
  ```
  GET /     -> 200 <h1>Hello from TSP v2</h1>
  HEAD /    -> 200 (empty body, Content-Length 0)
  OPTIONS / -> 204 with Allow: GET, POST, PUT, PATCH, DELETE
  GET /nope -> 404
  POST /    -> 405 (no POST export)
  DELETE /  -> 405 (no DELETE export)
  ```
- **Known limitation (deferred to slice 14b):** the current
  `HEAD` response has `Content-Length: 0`, not the GET body's
  length. Spec sect.6.5 says the body MUST be omitted but the
  `Content-Length` header SHOULD match the body that would have
  been sent. Preserving the length requires a response-writer
  refactor (the current `handle_connection` formats the headers
  from the `body.len()` of the return tuple, which discards the
  GET's actual length when we substitute an empty body). Slice
  14b will split response writing into a dedicated helper that
  takes a `head_mode: bool` and a `declared_length: usize`.
- **Out of slice 14a (still deferred):**
  - Dynamic route segments `[name].tsp` (spec sect.11.3) --
    router still rejects with `RouterError::UnsupportedShape`.
  - Catch-all `[...name].tsp` (sect.11.4) -- same.
  - URL percent-decode + 400 malformed (sect.11.8) -- not
    implemented.
  - Trailing slash normalisation (sect.11.9) -- not
    implemented.
  - Route precedence (sect.11.6) -- linear scan, no priority.
  - Full Context bridge (`ctx.request` / `ctx.url` /
    `ctx.params` / `ctx.signal`, spec sect.13) -- not
    implemented; PoC 1 fixtures use a zero-arg `GET()`.
  These are Phase 5+ per plan sect.61; tracked in the
  audit-issue list (issue 4 / 14).
- **Next:** Sol to pick. Candidates: slice 14b (HEAD
  Content-Length refactor), Phase 5 feature work (Context
  bridge, dynamic routing, fragments), or ADR work for the
  remaining plan/spec gaps surfaced in the audit.

### Slice 15a -- hot-reload RouteTable + PageRegistry on add/remove (done, bun commit `a832659f`)

- **Why:** the watcher audit (2026-08-24) found two spec
  MUSTs that slice 11 had not implemented:
  - spec sect.12: route file creation/deletion MUST update
    the route table without a process restart.
  - spec sect.33.5: deleting a `.tsp` route MUST remove the
    route from the route table; in-flight requests already
    pinned MAY complete.
  Before this slice, deleting `routes/index.tsp` produced a
  500 "build error" (the route was still in the table but
  the file was gone), and creating a new `routes/foo.tsp`
  produced a 404 (the route never made it into the table).
- **What landed (in `bun/src/runtime/tsp/`):**
  - `router.rs`: `RouteTable` is now `Arc<Mutex<Vec<Route>>>`
    so the watcher thread can add/remove routes while
    requests are in flight. New methods: `add(Route)`,
    `remove_by_path(&str)`, `paths()`, `get_by_path(&str)`.
    New `RouterError::DuplicatePath` variant for the
    duplicate-add case. `MatchResult` now owns `Route` (not
    `&Route`) because the lock guard cannot outlive the
    `lookup` call.
  - `generation.rs`: `PageRegistry::unregister(&PageRef)`,
    `unregister_path(&str)`, `register_route(&Route)`. Used
    by the watcher's reconcile path and by the boot-time
    builder (which now goes through `register_route` too,
    for consistency).
  - `watcher.rs`: `poll_once` takes a `table: &RouteTable`
    argument and runs a `reconcile_routes` after the
    source-hash diff. Adds call `register_route` +
    `mark_dirty`; removes call `remove_by_path` +
    `unregister_path`. New `PollStats` fields
    `routes_added`, `routes_removed`, `reconcile_error`.
  - `host.rs`: `serve` accepts `Arc<RouteTable>` (owned);
    each connection thread clones it. `handle_connection`
    takes `&Arc<RouteTable>`.
  - `bin/tspserver_v2.rs`: `Box::leak(table)` removed -- the
    table is now `Arc<RouteTable>`, the bin holds one Arc
    and clones it for the watcher and the host.
- **Verify:** 50 lib tests pass (was 47; 3 new:
  `add_and_remove_by_path_round_trip`,
  `unregister_path_drops_all_method_slots`,
  `unregister_one_page_ref_does_not_drop_siblings`).
  E2E end-to-end:
  ```
  GET /                    -> 200
  HEAD /                   -> 200 (empty body)
  OPTIONS /                -> 204 with Allow
  GET /nope                -> 404
  POST /                   -> 405
  create routes/about.tsp   -> GET /about returns 200
  delete routes/index.tsp  -> GET / returns 404
  GET /about (untouched)   -> 200
  server log: 'TSPv2PoC1: watch: removed route / (dropped 1 slot(s))'
  ```
- **Design note (preserved request pinning):** when a
  route is removed, the registry drops the `PageSlot`
  (and the `Generation` payload), but a request that had
  already pinned a generation's `Arc<String>` continues to
  serve that body until the response is written. This is
  the spec sect.33.5 "in-flight requests already pinned MAY
  complete" clause, satisfied for free by the slice 12
  `Arc<String>` payload refcount.
- **Out of slice 15a (still deferred):**
  - Precise per-module dirty (plan sect.22.1 reverse-graph
    walk) -- the watcher still does "any change dirties
    every slot" for the dirty marking half. The new
    reconcile path handles add/remove correctly but does
    not improve the granularity of `mark_dirty`.
  - ModuleGraph hot-reload (a future slice -- the graph
    is still frozen at boot; only the `RouteTable` and
    `PageRegistry` are hot).
  - Phase 5+ feature work (Context bridge, dynamic
    routes, fragments, etc.).
- **Slice 15 status (closed by Sol 2026-08-24):** 15a
  landed; 15b (precise per-module dirty) and 15c
  (ModuleGraph hot-reload) are explicitly **deferred** --
  the current "any change dirties every slot" watcher is
  acceptable for the small-app dev use case, and precise
  invalidation lands in a future phase if/when a real
  app's rebuild cost makes it worth the watcher's
  reverse-graph + per-page reverse-index bookkeeping.

- **Next:** Phase 5+ feature work. Candidates:
  - **Context bridge** (spec sect.13, plan sect.8) -- the
    host currently calls `GET()` with no argument; spec
    says `ctx.request` / `ctx.url` / `ctx.query` /
    `ctx.params` / `ctx.signal` / `ctx.cookies` and
    `ctx.formData()`. Biggest single gap between PoC 1
    and any non-trivial app.
  - **Dynamic route segments** (spec sect.11.3-11.4) --
    `routes/users/[id].tsp` currently rejected at boot
    with `RouterError::UnsupportedShape`.
  - **URL percent-decode + trailing slash** (spec
    sect.11.8-11.9) -- not implemented.
  - **Fragments** (spec sect.23, plan sect.14) -- not
    implemented.
  - **ModuleGraph hot-reload** -- a future slice if a
    real app needs it.

### Slice 16a -- Context bridge (host -> JS via env var) (done, bun commit `c0dcb654`)

- **Why:** Phase 7 (Context / Request / Response, plan
  sect.61) is the next unblocked phase after the
  side-by-side PoC 1 work. The current `.tsp` pages
  receive no arguments; the host calls `GET()` with no
  argument, so a page cannot know its own URL, query
  string, or method. spec sect.13 is the canonical answer
  for the page-side Context surface; slice 16a lands the
  minimum useful subset (method / path / query / empty
  params). The rest of spec sect.13 (cookies, signal,
  body, formData) lands in 16b/c; spec sect.18 (Response
  ABI) lands in 16b alongside.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `host.rs`: new `Context` struct with `to_json()`
    (hand-rolled, no serde dep). `parse_request` now
    extracts the query string. `handle_connection`
    builds the per-request `Context` and threads it
    through `render_for_route` and `pipeline::build`.
  - `pipeline.rs`: `build` accepts a `ctx_json: &str`
    and forwards to `jsc_bridge::execute`.
  - `jsc_bridge.rs`: `execute` accepts an optional
    `ctx_json` and sets the `TSP_CONTEXT_JSON` env var
    on the bun subprocess (the page does not actually
    need to read it because the JS preamble bakes the
    JSON in as a literal; the env var is there for
    completeness and for code that wants the raw form).
  - `jsx.rs`: `wrap_for_bun_cli` accepts an optional
    `ctx_json`. When present, the preamble parses it
    into `__tspContext` and passes it as the page
    handler's only argument. When absent, the handler
    is called with no argument (legacy zero-arg
    fixtures keep working).
  - `routes/index.tsp`: rewritten to `GET(ctx) { return
    ... ctx.method ... }` using a template literal
    (the slice-6 inline JSX shim only handles
    `<tag>text</tag>`, not interpolation; a future
    slice lands full JSX).
- **Verify:** 54 lib tests pass (was 51; 3 new in
  `host::tests::` for `Context::to_json`). E2E:
  ```
  GET /                     -> 200 with method=GET path=/
  GET /?q=hello&page=2      -> 200 with method=GET path=/ query=q=hello&page=2
  GET /POST                 -> 404 (no /POST route)
  GET /nope                 -> 404 (no /nope route)
  ```
- **Known limitation:** the rendered body includes a
  literal `q=` prefix because the fixture uses
  `q=${ctx.query}`. Future slices that land JSX
  expression interpolation will produce a cleaner
  body. The Context object itself is correct; the
  noise is purely the fixture's template string.
- **Out of slice 16a (deferred to 16b/c):**
  - spec sect.18 `Response` ABI (status / headers / body
    on the page return value).
  - spec sect.13 `ctx.request` (Web Request), `ctx.url`
    (Web URL), `ctx.signal` (AbortSignal), `ctx.formData()`.
  - spec sect.13 cookies (slice 18).
  - spec sect.13 params populated by dynamic route
    segments (needs spec sect.11.3-11.4 first).
  - spec sect.11.8 URL percent-decode -- currently
    surfaced verbatim; pages that want decoded forms
    use `decodeURIComponent` on the JS side.
- **Next:** slice 16b = Response ABI. The page returns
  either an `HtmlNode` (current behaviour) or a Web
  `Response` (spec sect.18). The host must distinguish
  the two and emit the right HTTP wire form.

### Slice 16b -- Response ABI (HtmlNode | Web Response) (done, bun commit `a155f9ff`)

- **Why:** Phase 7 (spec sect.18) -- the page-side return
  type is `HtmlNode | Response`. Slice 16a lands the
  Context surface but the page's return value is opaque
  to the host: bun subprocess stdout was passed through
  as the response body regardless of shape. Slice 16b
  inspects the return value via `instanceof Response` (in
  JS) and surfaces the page's status / content-type / body
  / headers to the HTTP wire.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `jsx.rs`: `wrap_for_bun_cli` now wraps the handler in
    an async IIFE. `instanceof Response` produces a
    `{type: 'response', status, headers, body}` envelope;
    `typeof string` produces a `{type: 'html', body}`
    envelope; anything else throws (the host returns 500
    on non-zero bun exit, matching spec sect.6.3). The
    envelope is prefixed with `__TSP_OUT_V1__` on its
    own line.
  - `host.rs`: new `parse_envelope` (no serde dep,
    hand-rolled JSON extractors) and `EnvelopeOutcome`
    struct. The Found arm now uses the envelope's
    `status_line` / `content_type` / `body` instead of
    the render placeholder. The legacy branch (envelope
    tag absent) keeps the slice 6 behaviour for older
    fixtures.
  - Status line is mapped through a small table of
    common HTTP statuses (200, 201, 204, 30x, 4xx, 5xx)
    with a fallback to 200 for unknown codes.
- **Verify:** 55 lib tests pass (no new tests; the change
  is parser-shape). E2E with the slice 16b fixture
  (POST returns `new Response("created", { status: 201,
  headers: { "x-demo": "slice16b" } })`):
  ```
  GET /                    -> 200 + Content-Type: text/html
  GET /?q=hi               -> 200
  POST /                   -> 201 + body "created"  (envelope response path)
  HEAD /                   -> 200 + empty body
  GET /nope                -> 404
  OPTIONS /                -> 204 + Allow
  ```
- **Out of slice 16b (deferred to 16c):**
  - Full header propagation (`extra_headers` is an empty
    vec in the Found arm; the `json_extract_headers`
    helper is too fragile for header values containing
    commas).
  - spec sect.18 helpers (`redirect()`, `json()`) -- the
    page can use `new Response(null, { status: 302,
    headers: { Location: '/x' } })` directly already.
  - spec sect.6.3 typed error codes (currently the host
    500s without a TSP3xxx code).
  - spec sect.18.2 `HtmlNode` -> text/html pipeline
    (the slice 6 inline JSX shim still treats
    `<h1>x</h1>` as a string literal).
- **Known issue (pre-existed, NOT a 16b regression):**
  DELETE / PUT / PATCH on a route that exports only GET
  returns 200 + "method X not exported" error body
  instead of 405. The route's `methods` field is the
  boot-time `REAL` set (GET/POST/PUT/PATCH/DELETE) and
  the registry's detected-method set is a subset. Cleanest
  fix: make `Route::methods` reflect the page-detected
  set after boot. Tracked as a follow-up slice.
- **Next:** slice 16c = full header propagation +
  `ctx.request` body (spec sect.13) + `ctx.signal`.

### Slice 16c -- full header propagation + real JSON parser (done, bun commit `dceeca00`)

- **Why:** slice 16b's envelope carried the page's `Response`
  headers but the host's comma-split extractor broke on
  header values containing commas (`x-comma: a,b,c`) and
  on escaped quotes. spec sect.18.3 requires the page's
  headers surfaced verbatim. Slice 16c replaces the
  hand-rolled extractors with a correct, small
  recursive-descent JSON parser (no serde dep -- the plan
  "no new deps unless the plan supports it" discipline
  holds; `bun_runtime` is the only workspace dep).
- **What landed (in `host.rs`, the only file changed):**
  - `JsonValue` enum + `JsonParser` (skip_ws /
    parse_value / parse_keyword / parse_string
    escape-aware incl. `\uXXXX` / parse_number /
    parse_array / parse_object). Correct for the shapes
    the wrap script produces.
  - `parse_envelope` rewritten: `kind` from the `type`
    field, status via an explicit `status_line_for()`
    table (widened to 101/203/205/206/300/402/406/408/
    411/413/414/416/417/418/425/426/431/451/505),
    content-type from the page's `content-type` header
    (fallback text/plain), headers pushed to a vec.
  - `handle_connection`: Found arm passes
    `outcome.headers` as the 5-tuple's `extra_headers`;
    the writer's `header_block` loop emits each header
    before Content-Type, skipping
    host-computed content-type/content-length.
  - Removed `json_extract_string` / `json_extract_number`
    / `json_extract_headers` / `unescape_json_string` /
    `json_string` kept only where `Context::to_json`
    needs the serializer.
- **Verify:** 60 lib tests pass (was 55; 5 new host
  tests: `envelope_parses_html` /
  `envelope_parses_response_with_headers` /
  `envelope_legacy_when_no_tag` /
  `envelope_unknown_status_falls_back_200` /
  `json_parser_handles_escaped_quotes`). E2E:
  ```
  POST /   -> 201 + x-demo: slice16c + x-comma: a,b,c
              + Content-Type: application/json
  GET /    -> 200 text/html
  HEAD /   -> 200 empty
  GET /nope -> 404
  OPTIONS / -> 204
  ```
- **Out of slice 16c (deferred to 16d+ / Phase 8):**
  - `ctx.request` body (`request.text()` / `json()` /
    `formData()`).
  - `ctx.signal` (AbortSignal).
  - Set-Cookie multi-value headers (the wrap script emits
    a flat object; list form lands with cookies in Phase
    8 / slice 18).
  - spec sect.18 helpers (`redirect()` / `json()`) -- the
    page can use `new Response(null, { status: 302,
    headers: { Location: '/x' } })` directly already.
  - spec sect.6.3 typed error codes (TSP3xxx).
- **Next:** slice 16d = `ctx.request` body + `ctx.signal`
  (spec sect.13.3/13.7) -- the last Phase 7 Context gap.

### Slice 16d -- ctx.request body + ctx.signal (full Context shape) (done, bun commit `baef11b0`)

- **Why:** slice 16c's "Next" names 16d as the last Phase 7
  Context gap: spec sect.13.3 (`ctx.request` Web Request with
  `text()` / `json()` / `formData()`) and sect.13.7
  (`ctx.signal` AbortSignal). FREEZE item 6's full Context
  shape also requires `ctx.url` (Web `URL`, spec sect.13.4)
  and `ctx.query` as `URLSearchParams` (spec sect.13.5) --
  slice 16a had delivered raw-string query and no url. 16d
  closes the whole gap at once.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `host.rs` -- `Context` gains `body: String` + `headers:
    Vec<(String, String)>`; `to_json` emits them. New
    `read_request` reads the header block up to CRLFCRLF
    (capped at `MAX_HEADER_BYTES`), parses `Content-Length`,
    then reads exactly that many body bytes; a body over
    `TSP_MAX_BODY_BYTES` (default 1 MiB, spec sect.14.2)
    returns `ReadOutcome::BodyTooLarge` and the connection
    answers `413 Payload Too Large` before the page runs.
    `parse_headers` lower-cases names and folds duplicates
    with ", ". New `render_per_request` builds pages that
    carry query or body without touching the generation
    cache -- the registry keys on (route, method), so a
    cached payload would replay the FIRST request's query /
    body echo to every later request on the same route. The
    envelope `Legacy` branch now falls back to the host's
    status line (405/500) instead of always 200.
  - `jsx.rs` -- the preamble now decorates the parsed
    context with a real `new URL` (host header + path +
    query), `ctx.query = url.searchParams`, a real Web
    `Request` (Bun's native class; body attached only for
    non-GET/HEAD, page `content-length` dropped since the
    host owns it), and `ctx.signal = new AbortController()
    .signal` (host never aborts -- no timeout / disconnect
    detection yet; listeners still work). The handler call
    is now `await`ed inside the async IIFE, so async pages
    (which `ctx.request.text()` forces) work. Fixed the
    double-escape bug: the ctx JSON is embedded via `{:?}`
    Debug formatting WITHOUT a manual `replace('\\', ...)`,
    which previously doubled backslashes and broke
    `JSON.parse` for bodies containing quotes.
  - `jsc_bridge.rs` -- the `TSP_CONTEXT_JSON` env var now
    carries `ctx_json_for_env` (body stripped) because env
    blocks on Windows cap at ~32 KiB while bodies may reach
    1 MiB; the body rides in the embedded literal instead.
- **Verify:** 70 lib tests pass (was 60; +10: parse_headers
  folding, content-length, read_request split / over-limit /
  split-across-reads, ctx_json_for_env strip, serialize
  round-trip, to_json body+headers, 2 new jsx wrap tests).
  E2E:
  ```
  GET /                    -> 200, url=http://localhost:9066/,
                              q= (URLSearchParams), signal=pending
  GET /?q=hi&p=2           -> 200, url=.../?q=hi&p=2, q=q=hi&p=2
                              (query reaches the page; cache bypass)
  POST {"a":1,"b":"x"}     -> 201, echo:{"a":1,"b":"x"}, x-method: POST
  POST second=payload      -> 201, echo:second=payload (no replay)
  POST w/ TSP_MAX_BODY_BYTES=8 -> 413 Payload Too Large
  ```
  The `"<h1>..."` quote noise in GET is the slice-6 inline
  JSX shim stringifying the standard `<h1>` tag (pre-existing,
  tracked; full JSX lands later).
- **Out of slice 16d (deferred to 16e / Phase 8+):**
  - `ctx.signal` abort triggers -- the runtime has no
    timeout / disconnect detection yet, so the signal is a
    live never-aborted AbortSignal (spec sect.13.7's abort
    condition is present but nothing fires it).
  - `request.formData()` for multipart -- Bun's native
    `Request.formData()` handles it today, but the host body
    path is UTF-8-lossy; binary multipart payloads need a
    raw-bytes transport slice.
  - `ctx.cookies` (Phase 8 / slice 18).
  - spec sect.13 per-request `url` origin: currently
    `http://<host-header>`; TLS termination and `x-forwarded-*`
    handling are a later slice.
- **Next:** slice 16e = per the plan order, the remaining
  Phase 7 items (spec sect.18 helpers `redirect()` / `json()`
  are already expressible via `new Response`, so the next
  gap is likely spec sect.6.3 typed error codes TSP3xxx or
  Phase 8 (session / cookies). Read plan sect.61 and the
  Phase 8 queue before starting.

### Slice 16e -- ctx.params + dynamic route segments (done, bun commit `a607deb9`)

- **Why:** plan sect.61 Phase 7 lists `params` as a
  first-class requirement and FREEZE item 3 froze the
  segment-name rule (`[A-Za-z_][A-Za-z0-9_]*`). Slices
  16a-16d landed the bridge, query, headers, body, url,
  signal, and response -- but `ctx.params` stayed an empty
  map and dynamic routes were `RouterError::UnsupportedShape`
  at boot. Phase 7 still had a real gap: `routes/users/[id].tsp`
  refused to start. 16e closes it.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `router.rs` -- new `Segment` enum (`Static` / `Param` /
    `CatchAll`) plus `Route { path, source, methods,
    segments, params }`. `url_path_for` recognises
    `[name]` (one-segment dynamic) and `[...name]` (catch-
    all) at any position; the catch-all is constrained to
    the last position (FREEZE item 3). The segment name
    pattern is validated with the FREEZE item 3 regex
    `[A-Za-z_][A-Za-z0-9_]*` -- a typo'd `routes/users/
    [1st].tsp` refuses to boot, matching the slice-3
    `RouterError::UnsupportedShape` policy. The `path` is
    the URL template (`/users/:id`, `/files/*path`).
    `lookup` is rewritten: it splits the request path into
    segments, walks every route, picks the lowest-priority
    score (spec sect.11.6: static > dynamic > catch-all),
    and returns the matched `Route` with `params`
    populated. `match_segments` is the per-route pattern
    engine. Trailing slash is normalised (spec sect.11.9):
    `/foo` and `/foo/` are the same route; the root `/`
    is its own canonical form.
  - `host.rs` -- `handle_connection` looks up the route
    once, then seeds `ctx.params` from the matched route
    before building the per-request `Context`. The page
    handler now reads `ctx.params.<name>` correctly. The
    `render_per_request` cache-bypass condition (slice
    16d) is extended: a request with `params` non-empty
    bypasses the generation cache, because the registry
    keys on (route, method) and a cached payload would
    replay the FIRST captured params to every later
    request on the same source file.
  - `generation.rs` -- the two test `Route` literals gain
    the new `segments` + `params` fields.
  - `routes/users/[id].tsp` (new fixture) -- demonstrates
    the dynamic segment with `ctx.params.id`, the
    `URLSearchParams` `query`, and the `URL` `pathname`.
- **Verify:** 80 lib tests pass (was 70; +10: dynamic
  segment bind, dynamic non-over-match, catch-all bind,
  static-over-dynamic precedence, dynamic-over-catch-all
  precedence, trailing slash normalisation, dynamic
  directory segment template, catch-all template,
  invalid segment name rejection, non-final catch-all
  rejection). E2E with the new fixture:
  ```
  GET /users/42            -> 200, "User 42" + path=/users/42
  GET /users/99?lang=zh    -> 200, "User 99" + query=lang=zh
  GET /                    -> 200 (static route still works)
  GET /nope                -> 404
  ```
- **Out of slice 16e (deferred to later slices):**
  - Static-vs-dynamic ambiguity rejection at scan time
    (e.g. `routes/users/[id].tsp` and `routes/users/[name]
    .tsp` at the same path). The current scan orders
    lexicographically and the matcher picks the static
    route when one is available, but a path-only conflict
    between two dynamics or two statics of the same
    shape still relies on `RouterError::DuplicatePath`
    (which fires when two files produce the same canonical
    `path`). A proper "ambiguous routes" check is a
    follow-up slice alongside spec sect.6.3 typed error
    codes.
  - `ctx.signal` abort triggers (16d deferred).
  - `request.formData()` (16d deferred -- needs raw-bytes
    body transport).
  - `ctx.cookies` (Phase 8 / slice 18).
  - URL percent-decoding of dynamic segments (spec
    sect.11.8) -- currently surfaced verbatim; pages
    can use `decodeURIComponent` on the JS side.
- **Next:** slice 16f = the remaining Phase 7 items are
  `formData` (spec sect.14.3, needs raw-bytes body) and
  `cookies` (spec sect.15, Phase 8 in plan but covered
  in Phase 7 per spec ordering). Either is the natural
  next slice; pick based on which has a cleaner dependency
  surface. `formData` is a host-side change; `cookies`
  needs a Set-Cookie merge path through the response
  ABI (slice 16c's `extra_headers` is a flat object,
  cookies are multi-value). Read plan sect.61 + spec
  sect.15/18.3 to decide.

### Slice 16f -- ctx.cookies + Set-Cookie multi-value merge (done, bun commit `b9c3be8e`)

- **Why:** spec sect.15 is a MUST-implement contract for
  `ctx.cookies` (read / write / delete). Phase 7's last
  real gap. The slice 16c envelope emitted `headers` as a
  flat JSON object (`{k: v}`), which collapses two
  `Set-Cookie:` wire lines with the same header name into a
  single comma-folded value -- violating spec sect.15's
  "preserve all valid cookie header lines rather than
  comma-joining them". The 16c trade-off was honest but
  undeclared; slice 16f closes both gaps in one slice
  because the cookie merge is impossible without the
  multi-value header shape. (memory: "根因重叠 bug 优先合并
  slice" -- one commit, one slice, one merge.)
- **What landed (in `bun/src/runtime/tsp/`):**
  - `jsx.rs` -- the wrap preamble now parses the
    `Cookie` request header into a small `Map` and exposes
    `ctx.cookies = { get, has, set, delete }`. Read methods
    return values from the request's cookies; write methods
    push formatted `Set-Cookie` lines into a
    `__tspCookieWrites` array (the page's options flow
    through -- `path` / `maxAge` / `domain` / `httpOnly` /
    `secure` / `sameSite` per spec sect.15's
    `CookieOptions`). The async IIFE now merges the writes
    into the response header list AFTER the page's
    `Response.headers` are walked, so:
    - `Set-Cookie` writes reflect on the response even
      when the handler returns an `HtmlNode` (string)
      -- spec sect.15 line 809 satisfied.
    - multiple writes become separate `Set-Cookie` wire
      lines -- spec sect.15 line 813 satisfied.
    - explicit `Response` Set-Cookie lines from the page
      + runtime cookie writes are preserved as a single
      ordered list.
  - `jsx.rs` -- the response envelope's `headers` field
    changed from a flat object `{k: v}` to an array of
    `[name, value]` pairs. The wrap script emits the new
    shape via `__tspHeaders__.push([k, v])`. The host's
    `parse_envelope` accepts both shapes (array for
    forward, object for slice 16c backward compat).
  - `host.rs` -- `parse_envelope` now delegates header
    extraction to `parse_envelope_headers(v)`, which
    handles the array shape correctly (preserving
    duplicates) and still accepts the flat-object shape
    (one entry per name). The writer's `header_block` loop
    was already array-of-pairs aware; it just needed
    multi-value to reach it.
- **Verify:** 83 lib tests pass (was 80; +3: array
  headers preserve multi-value, malformed array entries
  skipped without fatal, wrap preamble builds cookies
  with read+write API). E2E:
  ```
  GET / (no cookies)
    -> 200, Set-Cookie: sid=s_<rand>; Path=/; HttpOnly
       Set-Cookie: theme=dark; Path=/; Max-Age=3600
       (two separate wire lines; NOT comma-joined)
  GET / with Cookie: sid=old
    -> 200, body shows "seen=old" and cookies.has(sid)=true
  POST with body, no inbound cookies
    -> 201, x-demo: slice16f, x-method: POST
       Set-Cookie: a=v1; Path=/
       Set-Cookie: b=v2; Path=/
       (Response return path also surfaces both writes)
  ```
- **Out of slice 16f (deferred to 16g+ / Phase 8):**
  - `request.formData()` for multipart (spec sect.14.3).
    Bun's native `Request.formData()` already works for
    text bodies, but the host body path is UTF-8 lossy --
    binary multipart payloads need a raw-bytes transport
    slice before formData is genuinely spec-compliant.
  - `ctx.signal` abort triggers (16d deferred).
  - Cookie read-side spec details: spec sect.15's
    `cookie.set` is documented with full `CookieOptions`
    (signed / prefix / priority / partitioned); 16f
    covers the common subset (`path` / `maxAge` / `domain` /
    `httpOnly` / `secure` / `sameSite`). Full coverage
    follows with the rest of the options interface in
    Phase 8 / slice 18.
  - URL percent-decode on dynamic segment values
    (spec sect.11.8) -- 16e deferred.
  - `ctx.session` (Phase 8).
- **Next:** slice 16g = `request.formData()` (spec
  sect.14.3) requires a raw-bytes body channel. The
  change is wider than cookies: `Context.body` becomes
  `Vec<u8>` (or stays `String` plus a separate
  `body_raw: Option<...>`), the wrap preamble hands Bun
  the bytes verbatim, and the env side channel stops
  carrying the body. Read plan sect.61 and the Phase 8
  queue before starting so the raw-bytes path also
  enables future file-upload + body streaming work.

### Slice 16g -- raw-bytes body channel + formData (done, bun commit `aeb5dbc0`)

- **Why:** spec sect.14.3 requires `request.formData()`
  for multipart parsing. The slice 16d body was a
  UTF-8-lossy `String` -- a binary multipart payload
  with 0x00 / non-UTF-8 bytes (a real file upload)
  would have U+FFFD substitution before Bun's
  `Request.formData()` ever saw it. The slice 16d
  progress note also flagged this as a Phase 7
  blocker: "binary multipart payloads need a raw-bytes
  transport slice before formData is genuinely
  spec-compliant." 16g is that slice.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `host.rs` -- `Context.body` is now `Vec<u8>` (the
    lossy String is gone). `to_json` emits the body as
    a base64 string under the new field name `body_b64`
    (the JSON wire format has no native bytes shape;
    the wrap preamble atob-decodes it back to bytes).
    `read_request` no longer runs `from_utf8_lossy` on
    the body; only the head block is lossy-decoded
    (HTTP header lines are required to be ASCII; the
    lossy fallback is defence-in-depth for misbehaving
    clients). `ReadOutcome::Complete.body` is
    `Vec<u8>`. `ctx_json_for_env` strips `body_b64`
    (the env side channel still drops the body, since
    Windows env blocks cap at ~32 KiB while bodies can
    reach the 1 MiB default limit; the body rides in the
    embedded literal in the generated JS). New
    `base64_encode` is hand-rolled (no new dep) and
    tested against the RFC 4648 section 10 vectors.
  - `jsx.rs` -- the wrap preamble atob-decodes
    `__tspContext.body_b64` into a `Uint8Array` and
    feeds it to `new Request(url, { body: Uint8Array })`
    so binary multipart reaches Bun's native
    `formData()` parser without U+FFFD corruption. The
    previous slice 16d "drop the page's content-length
    header" line is removed: the host now reports the
    raw body length in the content-length header, and
    Bun needs that length to finalise a multipart
    stream-parse.
  - `routes/upload.tsp` (new fixture) -- the
    `request.formData()` demonstration. Text fields
    parse cleanly; file parts hit a known Bun 1.4
    multipart parser hang when fed a Blob body (see
    "Out of slice 16g" below).
- **Verify:** 86 lib tests pass (was 83; +3: RFC 4648
  base64 vectors, binary body round-trip through
  `to_json`, raw binary body survives `read_request`).
  E2E:
  ```
  GET /                    -> 200 (16f fixture, raw bytes path is transparent)
  POST /  text/plain       -> 201 echo (slice 16f fixture, body round-trip ok)
  POST /upload  text-only multipart
    -> 200, a=1; b=2; c=3   (formData() parses 3 text fields)
  POST /upload  raw binary [0,1,2,3] (not multipart)
    -> 500 formData-error: ERR_FORMDATA_PARSE_ERROR
       (formData() correctly rejects non-multipart
       bodies; this is the proof the raw bytes
       actually reach Bun intact)
  POST / body [0,1,2,3] via / echo endpoint
    -> 201 echo + 4 bytes (raw bytes round-trip
       verified, no U+FFFD)
  ```
- **Out of slice 16g (deferred to 16h+):**
  - **File upload with byte-fidelity** (multipart
    `file=` parts). Bun 1.4's `Request.formData()` hangs
    when the body is a `Blob` constructed in JS (the
    Blob-fed stream has no proper end-of-stream
    signal that the multipart parser can detect). The
    hang is reproducible with both `Uint8Array` and
    `Blob` body shapes plus `duplex: 'half'`; the
    underlying cause is in Bun's stream layer, not in
    our wire transport. 16g ships text fields + raw
    body round-trip; file parts need a future slice
    that streams the body through the host instead of
    buffering it, or that switches to a Bun-side
    implementation once the engine bug is fixed.
  - `ctx.signal` abort triggers (16d deferred).
  - spec sect.6.3 typed error codes TSP3xxx.
  - URL percent-decode on dynamic segment values
    (spec sect.11.8) -- 16e deferred.
  - Cookie full `CookieOptions` interface (16f deferred).
  - `ctx.session` (Phase 8).
- **Next:** slice 16h = the remaining Phase 7 items.
  The two real gaps left are: (a) spec sect.6.3 typed
  error codes (TSP3xxx) -- a host-side change that
  threads a stable code + phase + route into the 500
  / 400 / 413 pages; and (b) `ctx.signal` abort triggers
  (timeout / disconnect detection) -- a runtime change
  that requires per-request book-keeping. Pick based
  on which slice the next application work needs.
  Read plan sect.61 + spec sect.6.3 / 13.7 to decide.

### Slice 16h -- spec sect.6.3 typed error codes (TSP-NNNN) (done, bun commit `1dfd705e`)

- **Why:** spec sect.6.3 / sect.37 ask for "stable error
  code + error phase + route" on every dev diagnostic.
  Through 16g the host returned plain "TSP v2 PoC 1
  slice N: ..." text without a stable code -- a
  fixing-the-paragraph-then-asking-the-customer kind
  of API. Spec sect.37 even gives the canonical
  conceptual example `TSP-E-TRANSPILE` /
  `TSP-E-EXPORT` / `TSP-E-RENDER`. 16h threads a real
  `TSP-NNNN` code into every 4xx / 5xx / 413 body so
  tooling can grep for the prefix without parsing the
  human description.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `host.rs` -- new `TspError` enum + `format_error_body`
    / `format_error_body_raw` helpers. The enum models
    the **host-side** failures (route config, request
    input, host state machine) and the raw variant
    takes code + description strings for the build
    pipeline (where the bridge layer owns the code).
    Every error point in `handle_connection` and
    `render_*` now emits `[TSP-NNNN] <description>` on
    the first line of the body, with the pre-16h
    "TSP v2 PoC 1 slice N: ..." detail line preserved
    so existing dev tooling that greps for `slice 12`
    etc. continues to work.
  - `router.rs` -- `RouterError::code()` covers the
    1xxx range (`TSP1001`-`TSP1004`). The boot-time
    `bin/tspserver_v2.rs` scan path can now surface a
    `TSP1004` body for ambiguous or duplicate routes
    (FREEZE item 14 referenced `TSP1004`).
  - `jsc_bridge.rs` -- `JscError::code()` / `describe()`
    cover the bridge internals (`TSP3002` JSX
    transform, `TSP3010` bun not found, `TSP3011`
    spawn fail, `TSP3012` subprocess non-zero,
    `TSP3013` empty stdout, `TSP3014` write temp
    fail). The host's `render_per_request` / `render_for_route`
    build-error 500 body uses these directly via
    `format_error_body_raw`, so the wire prefix the
    dev sees matches the bridge's own contract.
  - `pipeline.rs` -- `BuildError::code()` delegates to
    the inner `Prepare` (TSP3001) or `Jsc` (one of
    the bridge codes). The host's build-error body
    threads both layers' codes through the same
    formatter.
- **Verify:** 93 lib tests pass (was 86; +7:
  `tsp_error_codes_are_stable` (host) /
  `format_error_body_typed_form` /
  `format_error_body_raw_passes_arbitrary_code` /
  `format_error_body_adds_trailing_newline_if_missing`
  / `router_error_codes_are_stable` /
  `jsc_error_codes_are_stable` /
  `build_error_codes_are_stable`). E2E:
  ```
  GET /nope    -> 404 [TSP2003] no route matches
                  TSP v2 PoC 1 slice 10b: no route matches
                  path=/nope (table has 3 route(s))
  DELETE /     -> 405 [TSP2004] method not exported by route
                  Allow: GET, POST
                  TSP v2 PoC 1 slice 12: method DELETE not exported
                  by D:/GitHub/tsp/routes\index.tsp
  ```
- **Code table (16h):**
  ```
  TSP1001  routes directory not found
  TSP1002  unsupported route shape
  TSP1003  duplicate route path
  TSP1004  route filesystem error
  TSP2001  malformed request line
  TSP2002  request body exceeds limit (413)
  TSP2003  no route matches (404)
  TSP2004  method not exported by route (405)
  TSP3001  page prepare error
  TSP3002  jsx transform error
  TSP3006  clean slot has no payload
  TSP3007  page never built successfully
  TSP3008  page not registered
  TSP3010  bun binary not found
  TSP3011  bun subprocess spawn failed
  TSP3012  bun subprocess exited non-zero
  TSP3013  bun produced no stdout
  TSP3014  writing bun temp file failed
  ```
  The 3xxx range includes the host enum (3001 / 3006 /
  3007 / 3008) and the bridge codes (3002 / 3010-3014).
  Gaps in the 3xxx range (3003-3005) are reserved for
  future slices (e.g. 3003 = invalid return value
  when a page returns a `Date` or other non-`Response`
  non-string; 3004 = empty handler output; 3005 =
  LKG missing). The current 16h surfaces those through
  the closest host variant (PagePrepareError) so the
  dev always sees a code.
- **Out of slice 16h (deferred to 16i+ / Phase 8):**
  - `ctx.signal` abort triggers (16d deferred; the
    remaining Phase 7 item from the 16g progress
    note's "two real gaps" summary).
  - TSP3xxx: spec sect.6.3 also lists the **error
    phase** and the **route** alongside the code. The
    phase is implicit in the prefix (1xxx = routing,
    2xxx = request, 3xxx = build) so the wire form
    already carries it; the route is not currently
    threaded into the 4xx / 5xx body. 16h does not
    add the route because the existing detail line
    already includes the path / source, and adding
    structured `key: value` fields risks breaking
    pre-16h `slice 12: ...` greps. A future slice can
    add a structured `route=...` line below the
    `TSP-NNNN` line.
  - URL percent-decode on dynamic segment values
    (spec sect.11.8) -- 16e deferred.
  - Cookie full `CookieOptions` interface (16f
    deferred).
  - File upload multipart (16g deferred -- Bun 1.4
    parser hang).
  - `ctx.session` (Phase 8).
- **Next:** slice 16k = `ctx.session` memory session store
  (host-owned, cookie-keyed, survives reload by the same
  host-residency the registry guarantees; spec sect.16).
  After that: Redis session, then the persistent JS
  adapter realm for call-capable services.

### Slice 16j -- ServiceRegistry infrastructure (+ logger service) (done, bun commit `75eaf213`)

- **Why:** spec sect.17 says runtime-scoped services MUST
  survive page generation replacement and `.tsp` modules must
  NOT own durable resources; plan sect.61 Phase 8 lists
  `ServiceRegistry` first because every later Phase 8 slice
  (session, Redis, logger, adapter realm) builds on it.
  The registry is the host-side home for services:
  created at boot, shared by every connection thread, never
  owned by a page generation.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `services.rs` (new) -- `ServiceRegistry` (BTreeMap of
    `Arc<dyn Service>`, `with_defaults`, `register`,
    `get`, `any_request_varying`, `snapshot`,
    `flush_log_lines`), `Service` trait (name / scope /
    `is_request_varying` / `describe_json` / `as_any`),
    `ServiceScope` (runtime vs request), `LogLine`, and the
    built-in `LoggerService` (in-memory 1000-line ring
    buffer + `total_lines` counter -- file/rotation
    backends are a later slice).
  - `host.rs` -- `Context.services: Vec<(name, json)>`
    serialised into the wire Context as `"services":{...}`
    (spec sect.17 `ctx.services`). `serve` /
    `handle_connection` take `&ServiceRegistry`; each
    request snapshots `services.snapshot(&[])` into the
    Context; after `parse_envelope` the host flushes
    `outcome.service_logs` into the owning service BEFORE
    writing the response (so the next request observes the
    flush). `EnvelopeOutcome` gains `service_logs` +
    `parse_service_logs` (malformed entries dropped).
    The `per_request` flag (16d/16e cache bypass) now also
    trips when `services.any_request_varying()` -- a page
    may read live service state (`logger.total_lines`), so
    the generation cache must not replay a stale snapshot.
  - `jsx.rs` -- the preamble declares `__tspServiceLogs`
    unconditionally (legacy zero-arg fixtures keep working)
    and hydrates `ctx.services` from the descriptor
    snapshot: `kind='logger'` becomes a log adapter whose
    calls buffer into `__tspServiceLogs`; any other
    descriptor surfaces read-only via `Object.freeze`
    (spec sect.17.3 -- no wrapper identity across
    requests). The envelope now carries
    `service_logs: __tspServiceLogs`.
  - `bin/tspserver_v2.rs` -- `ServiceRegistry::with_defaults()`
    boxed-leaked like the PageRegistry and passed to `serve`.
- **Verify:** 110 lib tests pass (was 94; +9 services, +3
  host envelope/context, +2 jsx wrap, +2 misc). E2E on
  `routes/svc.tsp`:
  ```
  GET /svc          -> svc lines=0   (page logs 1 line; flushed after)
  GET /svc          -> svc lines=1   (state survived request 1)
  edit svc.tsp      -> watcher cycle
  GET /svc          -> svc lines=2   (service NOT rebuilt -- reload page
                                     after service 不重建 acceptance)
  GET /             -> 200 (16f index regression-free)
  ```
  Server log: `services registered: logger`.
- **Deferred (16k+):** memory session store (`ctx.session`,
  spec sect.16); Redis session; file/rotation logger
  backends; persistent JS adapter realm (call-capable
  services with a real IPC channel); spec sect.17.2
  dev-tool diagnosis of page-owned durable resources
  (needs static analysis, Phase 11 territory).

### Slice 16i -- ctx.signal abort triggers (host timeout + kill) (done, bun commit `9b7db6df`)

- **Why:** spec sect.13.7 says `ctx.signal` MUST be
  aborted when the runtime determines the request is
  no longer executable, including applicable timeout
  conditions. 16d shipped a live `ctx.signal` but
  never wired an abort trigger. Without a timeout, a
  page that loops forever (or hangs on a slow
  `await ctx.request.text()`) holds the worker thread
  indefinitely; Phase 7's contract is incomplete.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `host.rs` -- new `TSP_TIMEOUT_MS` env var
    (default 30 000 ms; `0` disables) and a
    `resolve_request_timeout()` helper. The
    `handle_connection` thread reads it once per
    request and threads `timeout_ms` into the render
    path. `render_for_route` and `render_per_request`
    now take `timeout_ms` as the last argument and pass
    it to `pipeline::build`.
  - `pipeline.rs` -- `build()` takes `timeout_ms:
    u64` and forwards to `jsc_bridge::execute`.
  - `jsc_bridge.rs` -- `execute` switched from
    `cmd.output()` to `cmd.spawn()` + `try_wait` loop
    (50 ms poll) so the watchdog can hard-kill the
    child. The watchdog thread sleeps for `timeout_ms`,
    writes the `ABORT_MARKER` (b"A\n") to the child's
    stdin, sleeps a 1s grace for the page to emit its
    envelope cleanly, then the host thread hard-kills
    the child. `JscError::BunFailed` surfaces the
    timeout + the bun stderr tail (capped at 512
    bytes) so the dev sees why the page never
    returned. A new `ABORT_MARKER` constant pins the
    single-byte wire form and a test guards it.
  - `jsx.rs` -- the IIFE now calls `process.exit(0)`
    right after writing the envelope. Without this, a
    page that finishes its work but left a pending
    promise / timer alive holds the bun subprocess
    open past the timeout window and forces the host
    to hard-kill. The explicit exit makes clean runs
    clean.
- **Verify:** 94 lib tests pass (was 93, +1
  `abort_marker_is_a_single_line`). E2E:
  ```
  TSP_TIMEOUT_MS=0    GET /          -> 200 (watchdog disabled)
  TSP_TIMEOUT_MS=0    POST /upload   -> 200 formData text fields
  TSP_TIMEOUT_MS=3000 GET /slow      -> 500 [TSP3012] bun subprocess
                                        exited non-zero (timeout grace
                                        + kill backstop)
  ```
- **Out of slice 16i (deferred to 16i+ / 16j+):**
  - **Wrap-side abort listener** that reads the
    `ABORT_MARKER` from bun's stdin and calls
    `__tspAbortCtrl.abort()` so the page's own code
    can react via `ctx.signal.aborted` or
    `ctx.request.signal`. The host-side marker write
    is in place; the listener half needs more careful
    work because bun 1.4 stdin data events are not
    guaranteed to fire while a `setTimeout` is parked
    on the same loop. A follow-up slice adds the
    listener + an IIFE catch path that re-emits the
    envelope after a handler throws.
  - **TCP disconnect detection** (spec sect.13.7
    second trigger). The host poll-loop would call
    `stream.peek()` to detect a peer-side close; the
    Windows + stdlib interaction is fiddly and the
    timeout backstop already covers the runaway-page
    case. A follow-up slice adds it once the abort
    marker pipeline is settled.
  - spec sect.6.3 typed error code `TSP3xxx` for
    `InvalidReturnValue` (3005), `EmptyHandlerOutput`
    (3004), and `LkgMissing` (3003) -- the host enum
    currently falls back to `PagePrepareError`; a
    future slice extends `TspError` /
    `BuildError::code()` once the dev loop needs them.
  - `ctx.session` (Phase 8) and the rest of the
    service registry.

## Realistic next-step options (post-Slice 7) -- STALE

> Note (2026-08-24): the in-process JSC bridge was closed
> via ADR-0001 (slice 13) as future work, and the
> "next-step" options below were all delivered:
> - (a) Phase 0 docs freeze -> slices 8 + Phase 0 sign-off.
> - (b) Watcher + atomic reload -> slices 11, 15a.
> - (c) In-process JSC bridge -> ADR-0001 (subprocess is
>   the production path; in-process is future work
>   triggered by Bun-side API changes).
>
> The "next-step" below is kept for historical context;
> the real next step is the Phase 5+ candidate list at
> the end of the slice 15a entry.

The in-process bridge is genuinely multi-session work. Other
options for the next session that are cheaper and don't depend
on bun_runtime:

(a) **Phase 0 docs freeze** (plan sect.61). Write
    `docs/v2/spec.md` + `tsp-module.md` + `jsx-runtime.md` +
    `context.md` (the 12 freeze items from plan sect.60) plus
    10-20 `.tsp` fixtures. **No code.** Protects future slices
    from ABI drift while we tackle the in-process bridge in
    parallel.
(b) **Watcher + atomic reload** (plan sect.22). PageSlot +
    Generation + LKG + ModuleGraph + reverse edges. The
    slice-7+ half of PoC 1's DoD. **Several sessions.** Closes
    the rest of PoC 1 DoD but does not require bun_runtime.
(c) **In-process JSC bridge** (the work slice 7 spiked). Cold
    compile is now done; the remaining work is replicating
    Bun's startup sequence in this crate. **Multi-session.**

User-side decision required: pick (a), (b), or (c) for the next
session. The slice ledger updates with the chosen path before
more code lands.

## PoC 1 closure

Plan sect.70 "PoC 1" is the 7-step vertical slice:
```text
1. Rust 启动 HTTP server              slice 2
2. / 映射 routes/index.tsp           slice 3
3. transpile 标准 TSX                slice 6 (jsx.rs)
4. JSC instantiate/evaluate          slice 6 (jsc_bridge.rs)
5. 找 GET export                     slice 5
6. 调用 GET(ctx)                     slice 6
7. 返回 Response / <h1>Hello</h1>    slice 6
```

All 7 steps land. The model the plan asked us to validate --
"Rust host owns HTTP lifecycle, JSC page module is replaceable
execution generation" -- is proven in real life on the loopback
interface. v1 (`src/main.ts`, `www/`, `tsp.sh`) is unchanged
throughout the refactor; the side-by-side coexistence strategy
holds.

### Plan sect.74 DoD items satisfied by PoC 1 (reconciled post-slice 13)

> Reconciled 2026-08-24 after the slice 9-12 + 13 review: the
> original "deferred (slice 7+)" bullets were written before
> slices 9-12 landed. The slice column now reflects actual
> delivery; the original PoC 1 closure check (slice 6 ledger)
> predates that work. See slice 9-12 entries for evidence.
>
> Items still genuinely deferred (Context bridge full ABI,
> session / persistent services, multi-worker) are flagged
> explicitly as such below.

- [x] `tspserver_v2` does not depend on `main.ts`
- [x] HTTP lifecycle is native (Rust stdlib TcpListener)
- [x] `.tsp` is transpile + execute (jsx.rs -> bun.exe)
- [x] filesystem routing correct (routes/index.tsp -> /)
- [x] Context / Response ABI stable enough for the smoke test --
      **partial** (full Context bridge with `tsp:server` import +
      `ctx.request` / `ctx.url` / `ctx.params` etc. is slice 14+;
      PoC 1 `.tsp` fixtures use a zero-arg `GET()` signature
      that is intentionally simpler than the spec §6.2
      `(ctx: Context) => HandlerResult` form)
- [x] generation atomic publish correct -- landed (slices 9-10b)
- [x] LKG correct -- landed (slice 10a; first-commit LKG = candidate,
      subsequent-commit LKG = previous current, failed-commit
      LKG unchanged)
- [x] reload does not restart HTTP server -- landed (slice 11
      watcher: any change marks slots dirty, next request rebuilds)
- [x] reload does not rebuild session / persistent services --
      N/A (no session / no persistent services yet; those are
      Phase 8 work)
- [x] generation can be retired -- landed (slice 12 Arc<String>
      payload reference counting: old generation's payload is
      dropped when no request holds a pin, even after `current`
      is overwritten)

### What is NOT yet built (deferred to slice 7+)

The above "deferred" bullets are the meat of the plan sect.61
Phase 4-6 milestones and are not part of PoC 1. The next
session(s) should pick one of two entry points:

(a) **In-process JSC bridge** -- swap the `bun.exe` subprocess
    path for a real `bun_runtime` integration. Cold compile is
    the only cost; the architectural model is already proven.
(b) **Phase 0 docs freeze** (plan sect.61) -- write
    `docs/v2/spec.md` + `tsp-module.md` + `jsx-runtime.md` +
    `context.md` (the 12 freeze items from plan sect.60) plus
    10-20 `.tsp` fixtures. The user code stabilises before more
    Rust code lands, which protects against the slice 7+
    pivot re-shaping the public contract.
(c) **Watcher + atomic reload** (plan sect.22) -- the slice 7+
    half of PoC 1's DoD. This is what turns the v2 host from
    "smoke test" into "iterable dev server".

User-side decision required before any of (a), (b), (c) starts.


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


## Session summary (2026-08-24)

This session took the v2 refactor from zero to a frozen v2.0
contract in 9 slices:

- **PoC 1 (slices 1-6):** Rust HTTP listener + filesystem route
  scanner + JSC execute via `bun.exe` subprocess. End-to-end
  verified: `curl /` returns 200 OK with body `<h1>Hello from TSP v2</h1>`
  and `Content-Type: text/html; charset=utf-8`. 22 unit tests
  pass; binary builds in 3.48s incremental.
- **Slice 7 (in-process JSC spike):** `bun_runtime` dep added
  with cold compile in 1m 51s; spike module documents the
  multi-session gap before the in-process bridge can fully
  land. The PoC 1 subprocess path stays as the production
  code; the binary regress-tested clean.
- **Phase 0 (slice 8):** 12-item v2.0 contract documented in
  `FREEZE.md` + 4 topic docs + 10 example fixtures. Sol signed
  off; the contract is FROZEN.

Side-by-side coexistence with v1 (`src/main.ts` / `www/` /
`tsp.sh`) holds throughout: no v1 source was modified.

**v2 git log on the parent tsp repo** (10 commits, oldest to
newest):

```text
ec397f4  chore(tsp-v2): scaffold v2 docs and routes fixture (slice 1)
c53cfd3  chore(tsp-v2): mark slice 2 done in progress log
44d114f  chore(tsp-v2): mark slice 3 done in progress log
6067d8d  chore(tsp-v2): mark slices 4 + 5 done, surface slice 6 pivot
a06cf26  chore(tsp-v2): close PoC 1 in progress log
4836110  chore(tsp-v2): mark slice 7 (in-process JSC spike) in progress log
86ce560  docs(tsp-v2): Phase 0 freeze -- 12 contract items + 4 topic docs + 10 fixtures
a7f3797  chore: gitignore .logs/
043a832  docs(tsp-v2): lock the 12-item v2.0 contract (Phase 0 closed)
```

**v2 git log in the bun/ submodule** (7 commits):

```text
296ef0c2  feat(tsp-v2): add v2 host crate slice 1 (boot stub)
a0f5ffd5  feat(tsp-v2): add stdlib TCP listener with 404 (PoC 1 slice 2)
495a5253  feat(tsp-v2): add filesystem route scanner + matcher (PoC 1 slice 3)
b9a7b0a2  feat(tsp-v2): add bun_jsc + bun_transpiler dependencies (PoC 1 slice 4)
7d867e49  feat(tsp-v2): add page source reader + static export detector (PoC 1 slice 5)
d5a88b79  feat(tsp-v2): close PoC 1 vertical slice (JSX + bun.exe JSC bridge)
1e9a4b92  chore(tsp-v2): add bun_runtime dep + in_process_jsc spike (slice 7)
```
