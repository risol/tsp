# TSP v2 — progress log

The v2 runtime is now the only supported TSP runtime. Entries below preserve
the historical implementation sequence; references to side-by-side v1 work are
historical notes, not current compatibility requirements.

Tracks the side-by-side v2 refactor driven by `tsp-v2-plan.md` (75 sections,
12 phases) and `tsp-v2-specification.md` (normative).

- **Historical strategy:** implementation proceeded side-by-side while v2
  capabilities were developed. The current supported host is the native v2
  host in `bun/src/runtime/tsp/`, shipped as `tspserver_v2`.
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
  does not expose an embedder-safe high-level startup API. The low-level
  `bun_jsc::VirtualMachine::init` and `runtime_hooks()` are public, while
  `Run::boot` remains `pub(crate)` and the foreign-binary link boundary
  still fails when `bun_runtime` symbols are referenced. Slice 7 spiked
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
- **Next:** slice 16n' = robust wrap-side abort
  signalling. 16i shipped the host-side kill
  backstop; 16n attempted to fire `ctx.signal` inside
  the page via (a) a `process.stdin` `data` listener
  and (b) a `setInterval(50ms)` file watcher polling
  a per-request marker file the watchdog creates
  on timeout. E2E in this dev environment showed
  bun 1.4's `setTimeout`-parked loop neither
  delivers stdin `data` events NOR runs the
  `setInterval` callback chain (we tried replacing
  `setInterval` with a self-rescheduling `setTimeout`
  and the file still did not propagate). The marker
  file route is rolled back; the host-side kill
  backstop from 16i remains the only working
  timeout path. A future slice (16n') can revisit
  with a different IPC channel (named pipe, TCP
  loopback, IPC file under a file-system watcher
  bun ships as a native binding, etc.). After
  that: file / rotation logger backends (16j
  follow-up), then spec sect.17.2 dev-tool diagnosis
  of page-owned durable resources (Phase 11
  territory). Phase 8's service + session shape is
  closed; the persistent-JS-adapter-realm RPC stays
  an open-ended Phase 8 follow-up that depends on a
  real persistent host process (16m only landed
  the snapshot shape; the IPC channel is a
  separate slice).

### Slice 16n -- wrap-side abort signalling (deferred, not landed)

- **Why:** spec sect.13.7 says `ctx.signal` MUST
  fire when the host determines the request is no
  longer executable, including a timeout. 16i
  shipped the host-side kill backstop but left the
  page-side abort listener as a follow-up; 16n was
  the slice to land that listener.
- **What we tried:**
  1. `process.stdin.on('data', () =>
     __tspAbortCtrl.abort())` plus writing the
     `ABORT_MARKER` (`b"A
"`) to the bun
     subprocess's stdin from the host watchdog
     (the protocol 16i designed). E2E showed
     `request timed out after 3000ms; abort marker
     fired but page did not stop in time` -- the
     `data` event never fired while the page was
     parked in `setTimeout` / `await new Promise`.
  2. Robust file-watcher fallback: the host
     writes a per-request marker file at
     `TSP_ABORT_FILE` and the wrap preamble polls
     it via `Bun.fs.statSync` every 50ms inside a
     `setInterval`. We also tried a
     self-rescheduling `setTimeout` to avoid
     bun 1.4's `setInterval` keep-alive behaviour.
     E2E still showed the file marker did not
     fire `__tspAbortCtrl.abort()`; every
     `setTimeout` / `setInterval` tick was
     deferred while the page was parked, and the
     page never got a chance to observe the abort
     before the host's 1s grace expired and the
     child was hard-killed.
- **What was rolled back:** the `jsc_bridge.rs`
  marker-file plumbing (env var, spawn-time path
  computation, watchdog touch, post-reap cleanup)
  and the `jsx.rs` `__tspAbortCtrl` + `setInterval`
  watcher were both reverted. The host-side
  backstop (stdin marker + 1s grace + hard-kill)
  shipped in 16i is the only working timeout
  mechanism today.
- **Status:** the slice did not land a code change.
  Progress recorded for trace-ability. The
  follow-up slice (16n') needs a different IPC
  channel; candidates include a named pipe
  (Windows + POSIX), a localhost TCP loopback
  port, or a Bun-native file watcher (the
  `Bun.watch` API) that the page-side wrap would
  subscribe to. None of these are zero-dependency
  today, so the work is parked until a real
  persistent host process (16n+ follow-up) lands
  a proper IPC channel.
- **Production check-list (for the user, not the
  slice):** timeouts work today via the host
  hard-kill backstop (16i). Pages that listen on
  `ctx.signal` for cooperative cancellation will
  receive the signal on every request that finishes
  before the watchdog fires; they will NOT receive
  the signal when the page is parked in
  `setTimeout` at the moment the watchdog fires.
  The host still hard-kills in 1s and the request
  returns 500 [TSP3012] in that case.
- **Third attempt (rolled back, 2026-08-28):** the
  persistent host process (slice 16n+ follow-up)
  did not materialize, so we tried to ship 16n
  through the existing `Message::Cancel` IPC
  channel that the v2.4 pre-fork worker already
  uses. The diff:
  - `worker/manager.rs::execute_with_timeout` —
    after the `ReadTimeout` error, send
    `Message::Cancel { id }` to the worker over
    the same master↔worker socket, then give the
    worker a 250ms grace window to write a
    cooperative response.
  - `jsx.rs::wrap_for_embedded_worker` — instead
    of `process.stdin.on('data', ...)`, register
    the `AbortController` on
    `globalThis.__tspAbortController__` so the
    worker can find it without going through
    stdin.
  - `tsp_worker.rs` — the only file that actually
    receives `Message::Cancel` in production
    (the binary protocol on the worker socket).
    **No change was made here.** The
    `Message::Cancel { .. }` arm at
    `tsp_worker.rs:148-151` is still the
    documented no-op ("The native VM cancellation
    hook is wired in the next slice.").
  - `worker_runtime.ts` — line-based stdin
    protocol with a "PREFIX = __TSP_WORKER_V1__"
    reader. **Dead code in v2.4.** The diff
    rewrote its `Cancel` arm to call
    `globalThis.__tspAbortController__?.abort()`,
    but the production worker is `tsp_worker.rs`,
    not this file.
  - E2E (`wrap_side_abort_signal_fires_inside_page_on_host_timeout`)
    used the real binary with `TSP_TIMEOUT_MS=2000`
    against a page that awaits a 3s `setTimeout`.
    Result: WinSock 10060 from the test client at
    2.06s. The host fired `Message::Cancel` on
    time, but the worker ignored it (no-op arm),
    the page kept waiting for the 3s timer, the
    250ms grace window expired, the master
    hard-restarted the worker, and the test's
    client read-timeout (hardcoded 2s in
    `http_get_status`) fired at the same time as
    the master's 2s read-timeout so the 500
    never reached the client. Two independent
    failures.
  - The roll-back is symmetric: `jsx.rs`,
    `worker/manager.rs`, `worker_runtime.ts`, and
    the new e2e in `tests/start_order.rs` are
    all back at HEAD (`dd708ec35a`).
  - The third attempt confirmed the original
    diagnosis: a parked `setTimeout` is a
    fundamental bun 1.4 limitation, and no amount
    of "look up the AbortController on
    globalThis" can fix it. The controller
    reference is found, but `controller.abort()`
    is never called because there is no
    non-deferred IO primitive in the worker's
    per-request loop. The next attempt MUST use
    a bun-native wakeup (`Bun.watch` on a
    per-request file, or a localhost TCP
    loopback the page reads from via
    `Bun.readableStream`), or it MUST refactor
    the per-request loop to be event-loop-driven
    (the bun event loop's `wait_for_promise`
    pattern, with the master↔worker socket
    attached as a `ReadableStream` the wrap
    subscribes to).
- **Status:** slice 16n remains deferred. The
  follow-up paths in the original list (named
  pipe / TCP loopback / `Bun.watch`) are still
  the only viable IPC channels. They are
  deferred together with the larger "persistent
  host process" sub-architecture that motivates
  them — 16n' is not landing in this series.

### Slice 16m -- `time` service: call-capable read-only snapshot (done, bun commit `8dfefe6c`)

- **Why:** spec sect.17 says services expose read
  surfaces to pages; 16j shipped the fire-and-forget
  logger, 16k shipped the host-driven session.
  16m fills the third quadrant: a service the page
  can **read** (not call) via `ctx.services.time`.
  This is the simplest non-trivial demonstration of
  a call-capable service surface that still respects
  17.2 (`.tsp` modules MUST NOT own durable state) and
  17.3 (no wrapper identity across requests).
  A real persistent JS adapter realm with a round-trip
  IPC is a separate slice; 16m pins down the snapshot
  shape that any such realm will round-trip through.
- **What landed (in `bun/src/runtime/tsp/services.rs`):**
  - `BUILTIN_TIME = "time"`.
  - `TimeService { started: Instant }` -- the host
    captures a per-request snapshot via
    `snapshot_now()`; the wire form is
    `{kind:"time", scope:"runtime", iso:"YYYY-MM-DDTHH:MM:SS.sssZ", epoch_ms, uptime_ms}`.
  - `format_iso8601_utc(SystemTime)` -- hand-rolled
    UTC formatter, no `chrono` / `time` dep, kept
    to the "no new dep" discipline.
  - `Service for TimeService`: runtime-scoped,
    `is_request_varying() = true` (the page may
    read live values, so the generation cache must
    not replay a stale snapshot).
  - `ServiceRegistry::with_defaults` /
    `with_backends` now register the time service
    alongside logger + session. Server log:
    `services registered: logger, session, time`.
  - 5 unit tests (descriptor shape, snapshot
    monotonicity, ISO-8601 epoch, etc.).
- **Verify:** `cargo check --lib` 0 warnings. E2E
  on `routes/time.tsp`:
  ```
  GET /time  -> 200 iso=2026-08-25T03:59:14.865Z                        epoch_ms=1787630354865                        uptime_ms=14315                        kind=time
  GET /time  (50ms later) -> 200 uptime_ms grew by ~50
  GET /session, GET /     -> 200 (16k / 16f regression-free)
  ```
  The page reads `ctx.services.time` as a normal
  frozen JS object (the wrap preamble's existing
  `Object.freeze(__sDesc__)` path covers everything
  that isn't `kind === 'logger'`, so no JS-side
  changes were required).
- **Deferred (16n+):** wrap-side abort listener
  (16i follow-up so `ctx.signal` fires inside the
  page); file / rotation logger backends (16j
  follow-up); spec sect.17.2 dev-tool diagnosis of
  page-owned durable resources (Phase 11);
  persistent JS adapter realm with a real IPC
  channel (16n+; the time service shows the
  snapshot shape, but the realm itself -- a long-
  lived bun or Node process that hosts call-capable
  services and round-trips RPC to the v2 host -- is
  still open-ended).

### Slice 16l -- session backend abstraction + Redis opt-in (done, bun commit `c2aaa16a`)

- **Why:** 16k's `SessionService` shipped with the
  in-process `HashMap` inline, which works for a
  single host. Plan sect.61 Phase 8 lists `Redis
  session` as the third slice after the in-memory
  store, and spec sect.16.2 ("session state MUST
  survive page generation reloads") generalises to
  "must survive across host processes" once workers
  land. 16l factors the storage into a
  `SessionBackend` trait and ships a hand-rolled
  RESP2 `RedisBackend` so multi-host / production
  deployments can share state through Redis without
  sticky routing.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `session_backend.rs` (new, 1000+ lines) --
    `SessionBackend` trait (name / is_available /
    lookup / create / apply_writes / len), two
    implementations:
    - `MemoryBackend`: 16k's `HashMap` + FIFO cap
      (10 000), factored out so the dev path stays
      zero-config.
    - `RedisBackend`: hand-rolled RESP2 client over
      `std::net::TcpStream` (PING / SET with EX
      TTL / GET / DEL). `round_trip(cmd)` holds the
      connection mutex for the write AND the
      matching reply read so concurrent commands
      cannot interleave their bytes (the earlier
      `write_cmd_locked` + `read_reply_locked`
      pair was vulnerable to a write/read desync;
      a Windows test run hung before the fix).
      `is_available()` re-PINGs on demand, so a
      transient Redis outage self-heals on the next
      command. **No new dep** (plan sect.25.3
      discipline): RESP is a 5-tag wire format and
      the client surface is four commands.
  - `services.rs` -- `SessionService` becomes a thin
    facade over `Arc<dyn SessionBackend>`. The 16k
    API surface (`new(cap)` / `lookup` / `create` /
    `apply_writes` / `len`) is preserved by giving
    `new(cap)` a `MemoryBackend` underneath, so the
    16k test suite keeps compiling without change.
    `ServiceRegistry::with_backends(backend)` is the
    new factory the bin uses. `SessionData::new` and
    the free `non_empty` helper drop out (no longer
    reachable after the delegation).
  - `bin/tspserver_v2.rs` -- reads `TSP_REDIS_URL`:
    empty / unset -> `MemoryBackend` (dev default);
    set -> `RedisBackend`. A parse failure or
    unreachable Redis logs a diagnostic and falls
    back to memory, so boot never fails on a
    missing store. Startup log now reports
    `session backend = memory (cap=10000)` /
    `redis (url=..., available=...)` so the dev can
    see which path served a given run.
  - `lib.rs` -- `pub mod session_backend;`.
- **Verify:** `cargo check --lib` and `cargo check
  --bin tspserver_v2` clean (0 warnings, 0 errors
  under the workspace `warnings = deny` lint).
  `cargo fmt -- --check` clean on the three files
  16l touched. 17 unit tests in `session_backend.rs`
  cover the URL parser, RESP encoder, session-blob
  round-trip, MemoryBackend round-trip, RedisBackend
  against an in-process fake RESP server
  (round-trip / destroy / regenerate / lookup-miss
  / unavailable-when-endpoint-dead), and `mint_sid`
  uniqueness.
  **The 17 unit tests could not be executed in this
  Windows dev environment** because `cargo test`
  pulls the Bun runtime crate via the workspace and
  the test binary fails to link with the native
  symbols the test harness needs. The user's
  Linux / macOS environment (or any env that builds
  the workspace's `bun_runtime` dep) can run
  `cargo test --lib session_backend` to confirm
  green.
- **Deferred (16m+):** persistent JS adapter realm
  for call-capable services (spec sect.17); file /
  rotation logger backends (16j follow-up); wrap-side
  abort listener (16i follow-up so `ctx.signal` fires
  inside the page); spec sect.17.2 dev-tool diagnosis
  of page-owned durable resources (Phase 11).
- **Production check-list (for the user, not the
  slice):** to actually share sessions across hosts,
  set `TSP_REDIS_URL=redis://host:6379` in the env
  before launching `tspserver_v2`. The 16k / 16l
  cookie (`tsp_sid`) and the `ctx.session` API are
  unchanged -- pages do not see a difference
  between the in-memory and Redis backends.

### Slice 16k -- in-memory session store (`ctx.session`) (done, bun commit `008f0973`)

- **Why:** spec sect.16 defines `ctx.session` with
  `id` / `get` / `has` / `set` / `delete` / `clear` /
  `regenerate` / `destroy`, JSON-only values
  (sect.16.1), state that survives page reloads
  (sect.16.2), and `regenerate` keeping the data while
  swapping the id (16.3). Plan sect.61 Phase 8 lists
  `memory session` as the second slice after the
  ServiceRegistry. The store lives in the same
  host-owned registry as the logger (16j) so reload /
  generation release never tears it down.
- **What landed (in `bun/src/runtime/tsp/`):**
  - `services.rs` -- `BUILTIN_SESSION = "session"`;
    `SessionValue` (Null/Bool/Number/String/Array/
    Object, hand-rolled JSON tree, spec 16.1 portable
    values only); `SessionData { id, data }`;
    `SessionView { id, data }` with a hand-rolled
    `to_json()` the host embeds into the wire Context;
    `SessionWrite` enum
    (Set/Delete/Clear/Regenerate/Destroy);
    `SessionService` (runtime-scoped, host-owned;
    `Mutex<HashMap<String, SessionData>>` + FIFO
    eviction order + `SESSION_STORE_CAP_DEFAULT =
    10_000`). Methods: `lookup`, `create`,
    `apply_writes`, `len`, mint_sid (16-hex-char
    counter-derived id; production swaps for a CSPRNG);
    registered automatically via `with_defaults()`.
    `is_request_varying() = true` so the generation
    cache bypasses every render (per spec 16.2 the
    page may read live session state).
  - `host.rs` -- `Context.session: Option<SessionView>`;
    `to_json` emits `"session": {"id":..,"data":..}`
    (or `null` when the SessionService is not wired);
    `EnvelopeOutcome.session_writes: Vec<SessionWrite>`;
    new `parse_session_writes` (drops entries missing
    required fields; rejects non-portable values per
    16.1); the handle_connection flow resolves the
    request's `tsp_sid` cookie against the SessionService
    (unknown / missing / destroyed -> mint a fresh row,
    spec 16.4), snapshots the view into Context, and
    after the envelope returns calls
    `apply_writes(view.id, session_writes)` to get the
    new id. If the new id differs from the cookie the
    request carried, the host plants a
    `Set-Cookie: tsp_sid=...; Path=/; HttpOnly;
    SameSite=Lax` line (or `Max-Age=0` on destroy). The
    SessionResolve struct records BOTH the original
    cookie sid and the resolved view, so the
    "did anything change?" check is correct on a first
    request (no cookie -> mint -> need to plant).
  - `jsx.rs` -- top-level `__tspSessionWrites` declared
    unconditionally; the preamble hydrates
    `ctx.session = {id, get, has, set, delete, clear,
    async regenerate, async destroy}` where set/delete/
    clear/regenerate/destroy buffer into
    `__tspSessionWrites` (carried back in the envelope);
    non-portable values (functions / Symbols / etc.)
    are coerced to `String(v)` so a buggy page never
    crashes the host; the envelope now carries
    `session_writes: __tspSessionWrites`.
  - `bin/tspserver_v2.rs` -- no change (the in-memory
    SessionService ships in `ServiceRegistry::with_defaults`
    since 16j's bin was already wired to that
    constructor).
- **Verify:** 128 lib tests pass (was 110; +11 services,
  +7 host envelope/context, +2 jsx wrap, +2 misc). E2E
  on `routes/session.tsp`:
  ```
  GET /session                   -> 200 counter=0   Set-Cookie: tsp_sid=<sid>; HttpOnly
  GET /session (cookie carried)  -> 200 counter=1   no Set-Cookie (id unchanged)
  GET /session                   -> 200 counter=2
  POST /session {regenerate}     -> 200 regenerated Set-Cookie: tsp_sid=<new-sid>
  GET /session (new cookie)      -> 200 counter=3   data preserved (spec 16.3)
  POST /session {destroy}        -> 200 destroyed   Set-Cookie: tsp_sid=; Max-Age=0
  GET /session (no cookie)       -> 200 counter=0   Set-Cookie: tsp_sid=<fresh>
  ```
  Server log: `services registered: logger, session`.
- **Deferred (16l+):** Redis-backed session (opt-in via
  config; the in-memory store stays as the dev default);
  persistent JS adapter realm (call-capable services with
  a real IPC channel, spec sect.17); wrap-side abort
  listener (16i left the host side done; the
  `ctx.signal` page-side abort will be wired in a
  follow-up slice using a more reliable IPC than
  bun-stdin data); spec sect.17.2 dev-tool diagnosis of
  page-owned durable resources (Phase 11 territory).

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
interface. The initial implementation was developed side-by-side with v1;
the v1 host has since been removed and this native host is now the sole
supported runtime.

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
- `tspserver_v2` remains the native product binary name for the v2-only
  distribution.
- `routes/` remains the root application route directory now that v1 is
  retired.


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

The historical side-by-side phase is complete; the parent repository now
ships only the native v2 host and its embedded worker.

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

## Session update (2026-08-25) -- abort, timeout, and routing fixes

- **Slice 16n' -- wrap-side abort signalling: landed.** The generated
  wrapper now wires a module-scope `AbortController` to the Bun stdin
  `data` event, and the host writes `ABORT_MARKER` before entering a
  1-second grace window. A real host run confirmed
  `[ABORT-REACHED] fired=true err=aborted-by-signal` for `routes/abort.tsp`.
- **Watchdog regression fixed.** The child could exit normally in about
  100ms, while the sleeping watchdog later set `timed_out=true` and
  converted that successful request into a 500. An atomic watchdog state
  (`running -> child completed` or `running -> timed out`) now cancels the
  timeout race-safely. The watchdog no longer owns the grace sleep; the
  host loop does.
- **CookieOptions completed.** The wrapper now serializes the remaining
  `expires?: Date` option as an RFC-compatible `Expires=` attribute, in
  addition to path/domain/secure/httpOnly/sameSite/maxAge.
- **Client-disconnect cancellation landed.** The host polls a cloned
  accepted socket while a page is executing; a peer FIN/error cancels the
  request, sends the same abort marker through the subprocess watchdog,
  applies the existing grace/hard-kill policy, and suppresses response
  writes after disconnect. The bridge exposes `TSP3015` internally for
  cancelled builds.
- **Typed timeout response policy landed.** A deadline now produces the
  bridge-owned `TSP3009` error and the host returns `504 Gateway Timeout`
  with the typed `[TSP3009] request timed out` body. Client disconnects keep
  their internal `TSP3015` code but remain response-less because the peer is
  already gone. Existing `TSP3008` page-registration semantics are unchanged.
- **Dynamic URL decoding landed.** Router matching now decodes each path
  segment before binding params, including UTF-8 escapes; encoded `/` stays
  within its original segment, and malformed escapes return typed `400`
  `[TSP2005] malformed URL path` instead of falling through to 404.
- **Verification:** 162 library tests pass; `cargo build -p
  bun_runtime_tsp` passes; with `TSP_TIMEOUT_MS=2000`, `/`, `/time`, and
  `/session` return 200, while the timeout fixture returns 504 with the
  typed timeout body. Dynamic `/users/hello%20world` returns the decoded
  `ctx.params.id`, while malformed escapes return 400. A raw socket client
  closing during `/slow` cancels the Bun subprocess without attempting a
  response.

### Remaining next items

- Persistent JS adapter realm / service RPC — explicitly deferred to a
  future fork/integration slice. It requires (1) an embedder-safe Bun VM
  startup/link surface, (2) a persistent-realm module ownership boundary,
  and (3) a defined bidirectional service RPC protocol. The current v2
  runtime has none of these prerequisites; native `ServiceRegistry` plus
  snapshot/log back-channel remains the supported service implementation.

## Session update (2026-08-25) -- minimal `tsp:server` bridge

- **Subprocess compatibility seam landed.** The generated wrapper now
  supplies a narrow `tsp:server` surface for named imports: `json`, `text`,
  `html`, `redirect`, `notFound`, and `HttpError`. The source pre-pass rewrites
  those imports into wrapper bindings, including semicolon-free and multiline
  import forms.
- **Handler syntax coverage widened.** `export async function METHOD(...)`
  is accepted, and the compatibility pass removes only the `Context` and
  `PageConfig` parameter annotations needed by the current bridge. Arbitrary
  TypeScript syntax and the full virtual module loader remain future work.
- **Response behavior is wired to the existing envelope.** JSON/text/HTML/
  redirect/not-found helpers return Web `Response` objects. A thrown
  `HttpError` is converted to the same response envelope with its status and
  headers, so the host keeps one response path.
- **Verification:** 168 library tests pass, including real Bun subprocess
  execution tests for `tsp:server` JSON and `HttpError` responses; the
  `bun_runtime_tsp` package builds successfully. Repository-wide formatting
  still reports an unrelated pre-existing difference in
  `src/jsc/JSGlobalObject.rs`; the changed TSP files were not bulk-formatted
  to avoid touching unrelated work.

### Historical bridge status

The bridge above was intentionally a compatibility seam; the later runtime,
loader, packaging, and hardening entries below record the subsequent closure
work. The persistent JS realm remains an explicit ADR boundary.

## Session update (2026-08-25) -- runtime ABI, fragments, loader, tooling

- **Real TS/TSX execution landed.** The subprocess temp module now uses a
  `.tsx` suffix and delegates TypeScript/TSX parsing to Bun 1.4. The wrapper
  injects a small `React.createElement` compatibility factory and renders the
  resulting tree natively in the subprocess, including nested elements,
  attributes, boolean attributes, fragments, async components, escaping, and
  explicit raw HTML values. Function-valued attributes fail with `TSP3105`.
- **Fragments landed end to end.** Named `export const name = fragment(handler)`
  exports are registered in the wrapper. `ctx.fragment(name)` emits an
  internal URL; the native host resolves `/__tsp/fragment`, restores the
  originating route Context, selects the named handler, and returns the normal
  Response envelope. A real host run returned the expected fragment HTML.
- **Local module execution landed.** Relative `.ts/.tsx/.js/.jsx/.json` imports
  are resolved to absolute `file:` URLs before temp-module execution; route
  `.tsp` imports are rejected. The graph now records re-exports and dynamic
  imports and reports missing/unsupported local dependencies.
- **Developer tooling landed.** `tspserver_v2 check`, `routes`, and `graph`
  validate the route set, print exports, and inspect the resolved module graph.
- **Bundled-runtime packaging landed.** Runtime discovery searches explicit
  `TSP_BUN_BIN`, `TSP_RUNTIME_DIR`, binaries beside the server, and packaged
  `.tsp-runtime`/`runtime` locations before the development bootstrap path.
  Windows and Linux packaging scripts copy the server and Bun runtime together.
- **Verification:** 182 library tests pass; `cargo build -p bun_runtime_tsp`
  passes; `check`, `routes`, and `graph` all pass against the current routes;
  the real host returns `200` for `/` and the fragment endpoint returns `200`
  HTML.

### Closure items tracked before the final hardening pass

Persistent JS service RPC remained intentionally deferred because the Bun VM
link/startup boundary is unsafe; the accepted subprocess ADR is the closure
for that architectural branch. The hardening pass below records the shipped
source-aware diagnostics, dependency resolution, metrics, security, and
packaging decisions.

## Session update (2026-08-25) -- closure hardening

- **Dependency resolution is production-shaped.** The bridge now sets the
  application `NODE_PATH` and working directory for generated temp modules,
  so bare package dependencies resolve from the route/application tree. A
  real package fixture passes through the subprocess bridge. Relative dynamic
  imports are rewritten to absolute file URLs; `.tsp` imports remain rejected.
- **Diagnostics are source-aware.** Generated modules carry a `tsp://` source
  URL, and native JSX transform failures include a bounded original-source
  code frame. This avoids exposing the generated temp filename as the primary
  development diagnostic.
- **Metrics are host-owned.** The native host exposes
  `/__tsp/metrics` in Prometheus text format and tracks requests, response
  classes, active requests, duration sum/count, timeouts, cancellations, and
  reload cycles without placing state in page generations.
- **Benchmark baselines are repeatable.** `scripts/benchmark-tspserver-v2.ps1`
  and `.sh` measure cold request latency plus p50/p95/p99 warm latency for a
  fixed route set and emit JSON suitable for CI or release notes.
- **Worker invalidation has a portable transport.** Setting
  `TSP_INVALIDATION_FILE` enables an append-only cross-process path bus. Each
  worker still owns its own route table, page registry, and generations; only
  changed paths cross the process boundary.
- **Static files are native.** The configured `TSP_PUBLIC_DIR` (default
  `public/`) is served before page routing for GET/HEAD, with MIME detection,
  cache headers, URL decoding, traversal rejection, and symlink containment.
- **Fragment capability is enforced.** Fragment URLs carry a per-process
  capability token. Missing or forged tokens cannot dispatch the internal
  fragment endpoint; a real host run still renders the named fragment.
- **Packaging includes application assets.** Windows and Linux packaging
  scripts now copy the server, bundled Bun, routes, and optional public assets
  and emit a runtime manifest. The packaged Windows binary was verified with
  `--help`, `check`, and a real HTTP request without `TSP_BUN_BIN`.
- **Verification:** 183 `bun_runtime_tsp` library tests pass, the Rust binary
  builds, and repository TypeScript type-checking passes. Linux compilation
  remains an environment verification item because this host has only the
  Windows Rust target installed; the portable Linux packaging path is present
  but was not falsely claimed as locally executed.

The remaining persistent-JS-adapter item is intentionally closed as an
architecture boundary, not an unfinished implementation: the accepted ADR
keeps production on the subprocess bridge until Bun exposes an
embedder-safe VM startup/link surface. Native `ServiceRegistry`, sessions,
metrics, and reload state are host-owned and survive page generation reloads.

## Session update (2026-08-26) -- v2.4 worker pre-fork + distribution contract

- **Process-model tests pinned.** `tests/process_model.rs` records
  the actual PID / PPID / `--tsp-worker` flag / canonicalized exe
  path of every spawned worker, plus cancel / SIGKILL / no-zombie
  lifecycle assertions. The stub binary publishes its process info
  to a JSON file the harness parses (no serde dep) so the assertions
  are portable across Unix and Windows.
- **`process_inspector` module added.** Cross-platform helper for
  reading PID / PPID / exe / argv; the tests use it for the
  "process is fully reaped" assertion that the manager's
  `stop_worker` returns ESRCH after the wait/kill/reap cycle.
- **Start-order contract test added.** `tests/start_order.rs`
  boots the real `tspserver_v2[.exe]` and asserts the master prints
  the worker / watcher / listener markers in the right order.
  The CI gate that builds the binary is the same gate that
  exercises it.
- **CI extended to all 3 platforms.** `ci.yml` now runs
  `process_model` + `start_order` on Linux / Windows / macOS, and
  `smoke-windows` / `smoke-macos` mirror the existing `smoke-linux`
  job. The release matrix exercises the same binaries that ship
  to users.
- **Legacy paths marked.** The v1 / pre-v2.4
  `TSP_WORKER_BIN` / `bun(.exe)`-next-to-host fallback in
  `bin/tspserver_v2.rs::resolve_worker_bin` is now `// LEGACY:`
  annotated. `package-tspserver-v2.{sh,ps1}` now strips a stale
  `bun` / `bun.exe` from a re-packaged `dist/tsp-v2/`.
- **Verification:** 194 lib tests, 15 process-model tests,
  1 start-order test, 4 worker-integration tests pass. The new
  CI jobs run on every PR.

### Deferred to a follow-up PR (Slice F, not in this commit)

- **Node module embedding** is intentionally *not* part of this
  pre-fork / distribution slice. Defining the embedded-module
  source list, the build-time collection rules, the runtime
  module resolution, and the size / startup-time / license
  verification all sit on top of the pre-fork contract and
  must be their own design + PR. Mixing them here would inflate
  the slice scope and obscure the pre-fork verification surface.
  The Node-module slice will land as `docs/v2/FREEZE.md`-amended
  work, separate from this set.

## Session update (2026-08-28) -- e2e coverage + tooling close

- **`/__tsp/metrics` e2e pin.** The native v2 host's
  closure-hardening metrics surface
  (`host.rs:1517-1532`) shipped in the 2026-08-25
  closure pass without an e2e. The new e2e
  (`metrics_endpoint_serves_prometheus_text_after_priming_requests`)
  boots the real binary, primes the counters with a
  200 + 404, and pins the **snapshot semantics** of
  the body. The body is generated by `prometheus()`
  AFTER the metrics call's own `record_request()` (so
  `requests_total` and `active_requests` include the
  call) but BEFORE its own `record_response()` +
  `record_duration()` (so `2xx_total`,
  `duration_count`, and the `active` decrement do NOT
  yet reflect the call). A second hit on the same
  binary sees the first call's contributions, which
  pins the order. A regression that swaps those
  phases would break the test.

- **`tspserver_v2 check --tsc`** (Phase 11 close). The
  `check` subcommand historically did only the
  regex-based static-export detection (slice 5) +
  the module-graph build (slice 21). The new
  `--tsc` flag adds a real `tsc --noEmit` pass:
  walks the routes dir recursively (renaming
  `.tsp` to `.tsx` because tsc does not recognise
  .tsp), copies the bundled `tsp:*` declaration
  files into a temp tree, writes a `tsconfig.json`
  that maps `tsp:*` module names to the bundled
  declarations via `paths`, locates a `tsc` binary
  in the project's `node_modules/.bin/` (or on
  PATH), and runs `tsc --noEmit --project
  <tsconfig>`. The check is opt-in: the default
  `tspserver_v2 check` continues to do the
  original regex scan + graph build, so existing
  workflows that rely on the static check are
  unaffected. `--tsc` returns 0 if tsc exits 0,
  1 otherwise. E2E
  (`check_with_tsc_flag_catches_user_type_errors_and_passes_clean_routes`)
  asserts both the clean-route path and a known
  type error (`util.password.hash` with an invalid
  `cost` property -> TS2353).

- **tsc diagnostic path rewrite.** A follow-up to
  the tsc-check slice: the diagnostics that tsc
  prints reference the temp dir
  (`<Temp>/tsp-tsc-check-XXX/routes/<rest>`), not
  the user's project, so the user could not
  copy / click through. `rewrite_tsc_paths` (in
  `bin/tspserver_v2.rs`) parses each line's
  `(<line>,<col>)` opener, takes the path portion,
  and replaces the temp prefix with the routes
  root before forwarding. The tsc e2e now also
  asserts the temp prefix is NOT in stdout and
  the broken file's name IS.

- **§16n wrap-side abort** (deferred). The third
  attempt to land the `ctx.signal` cancellation
  path is rolled back. The diff was wired to
  `bun/src/runtime/tsp/worker_runtime.ts`, which
  is **dead code in v2.4** (the production worker
  is `bun/src/runtime/tsp_worker.rs` with a
  binary protocol over a Unix / TCP socket). The
  e2e failed with WinSock 10060 because the
  worker's `Message::Cancel { .. }` arm at
  `tsp_worker.rs:148-151` is still a no-op
  ("The native VM cancellation hook is wired in
  the next slice."). The rolled-back attempt
  also confirmed the original diagnosis
  (progress.md §16n "Production check-list"):
  a parked `setTimeout` is a fundamental bun
  1.4 limitation, and no amount of "look up the
  AbortController on `globalThis`" can fix it --
  the controller reference IS found, but
  `controller.abort()` is never called because
  there is no non-deferred IO primitive in the
  worker's per-request loop. The next attempt
  MUST use a bun-native wakeup (`Bun.watch` on a
  per-request file, or a localhost TCP loopback
  the page reads from via `Bun.readableStream`),
  or it MUST refactor the per-request loop to
  be event-loop-driven (bun's `wait_for_promise`
  pattern, with the master<->worker socket
  attached as a `ReadableStream` the wrap
  subscribes to).

- **FREEZE.md Amendment 10** documents the
  `/__tsp/metrics` contract (snapshot semantics
  pinned) and the `tsc check` contract (opt-in
  flag, tsc-binary discovery, declaration-file
  probe order, the `routes_root`-relative output
  after the path rewrite).

- **Verification:** 261 tests green (221 lib +
  4 worker_integration + 15 process_model + 21
  start_order e2e). The 21 e2e include the 1 new
  metrics test and the 1 new tsc-check test; the
  other 19 are unchanged. 5 git commits on the
  parent tsp repo this session:
  `5c6d4c1d87` -> `9853fd30d7` -> `29d5700a2a`,
  each with its own commit message in the
  repo's `git log`.

## Session update (2026-08-28) -- HTTP hardening + tooling + PageConfig.methods

Five batches land after the metrics+tsc close-out.
The total over the day: 261 -> 279 tests, 21 -> 32
e2e, 221 -> 228 lib, 6 new commits on the parent
repo. The themes:

- **HTTP surface hardening.** A cascade of small
  fixes that round out the host's HTTP response
  shape:
    - `/__tsp/metrics` HEAD support (200, empty
      body, GET body size preserved) +
      `405 Method Not Allowed` for non-GET/HEAD
      with `Allow: GET, HEAD`
    - HEAD on a regular page route drops the body
      at the wire (per RFC 9110 sect.9.3.2) and
      reports the GET's body size in
      `Content-Length` (slice-14a gap closed)
    - HEAD on a route that exports BOTH GET and
      HEAD calls the HEAD handler directly (no
      FoundHeadOverGet fallback)
    - OPTIONS auto-204 + `Allow:` on routes that
      do NOT export OPTIONS; routes that DO
      export OPTIONS call the handler
    - `X-Content-Type-Options: nosniff` on every
      host response (defence-in-depth against
      MIME-sniffing)
- **CLI tooling.** `tspserver_v2 check` gained
  `--tsc` (Phase 11 close) and `--no-color`; the
  diagnostics are now paths-relative to the
  user's routes root (the temp dir prefix is
  stripped) and ANSI-free under `--no-color`.
  `routes` and `graph` both gained a `--json`
  flag for CI / tooling integration; the JSON
  shape is hand-rolled (no serde dep) and uses
  the same `json_string` helper across both
  subcommands.
- **PageConfig.methods static validation**
  (FREEZE.md §11). `tspserver_v2 check` now
  parses `export const config = { methods: [...] }`
  and validates it against the page's actual
  exports. A mismatch is a check-time error
  (exit 1) but does not affect the runtime (the
  static scan + 405 dispatch already covers
  method rejection). The `detect_config_methods`
  parser is hand-rolled (~80 lines) and handles
  the common shapes (single / double quotes,
  whitespace, empty list, unknown method names).
  7 new unit tests cover the parser, 1 new e2e
  covers the end-to-end check.

The new e2e (10 tests):
  - `head_on_metrics_endpoint_returns_200_with_empty_body`
  - `post_on_metrics_endpoint_returns_405_with_allow_header`
  - `head_on_regular_page_uses_get_export_and_drops_body`
  - `head_on_route_with_both_get_and_head_calls_head_handler`
  - `options_on_regular_page_returns_204_with_allow_header`
  - `metrics_endpoint_includes_x_content_type_options_nosniff_header`
  - `regular_page_response_includes_x_content_type_options_nosniff`
  - `routes_command_json_flag_emits_stable_json_array`
  - `graph_command_json_flag_emits_stable_json_array`
  - `check_validates_config_methods_against_actual_exports`

- **Verification:** 279 tests green (228 lib + 7
  new `detect_config_methods` unit tests +
  4 worker_integration + 15 process_model + 32
  start_order e2e). 6 git commits on the parent
  tsp repo this session (post the morning's
  metrics+tsc batch):
  `e8308ad217` -> `0198584b81` -> `532b8490ee`
  -> `57e0ade258` -> `654e2f4a7c`.

## Session update (2026-08-28) -- three PageConfig slices + `check` unknown-export guard

Three slices land after the morning's HTTP-hardening
batch. The themes are all `PageConfig` / export-
validation work (FREEZE §11 + spec §46). The total
over the day: 279 -> 302 tests, 32 -> 35 e2e, 235 ->
248 lib, 3 new commits on the parent repo.

- **`config.bodyLimit` per-page cap** (FREEZE §11).
  A page may declare `config.bodyLimit: N` (bytes)
  on its `export const config = { ... } satisfies PageConfig`.
  The host enforces it AFTER route matching,
  BEFORE the page is invoked. The check applies
  only to POST / PUT / PATCH / DELETE (GET / HEAD /
  OPTIONS still 200 with a body, because the cap is
  a body-shape rule, not a status rule). The per-page
  cap is silently clamped to the global
  `TSP_MAX_BODY_BYTES` (`n.min(global)`); a larger
  declared value falls back to the global. The
  `detect_config_body_limit` parser is hand-rolled
  and supports `int`, `int * int * ...`, and
  underscore separators; rejects `Infinity`,
  negative numbers, and unparseable values (returns
  `None` so `check` does not surface a wrong value as
  a successful parse). 7 new unit tests cover the
  parser, 1 new e2e covers the end-to-end
  enforcement. The e2e initially had a 4th scenario
  (per-page cap > global, 1.5 MiB POST) but it
  exposed a TCP teardown race in the test client
  (server returns 413 + shutdowns before the client
  finishes its 1.5 MiB send); the global cap path is
  already covered by
  `body_size_cap_rejects_oversized_requests_with_413`,
  so the 4th scenario was dropped and the gap is
  documented in the e2e comment.

  The body-limit commit also fixes a
  `wait_for_marker` regression: the previous version
  transferred `child.stderr` to a background thread,
  which broke the 11+ tests that call
  `wait_for_marker` twice on the same child (boot +
  hot-reload). The fix restores the
  `child.stderr = Some(stderr);` put-back that the
  pre-slice code used. After the marker, the
  body's later requests are short enough to stay
  under the OS pipe buffer; a real production
  high-traffic case would add an explicit drain.

- **`config.cache` per-page default `Cache-Control`**
  (plan §55, FREEZE §11). A page may declare
  `config.cache: "no-store" | "private" | "public"`.
  The runtime applies the value as a default
  `Cache-Control` header on the response, but the
  page's own `Response.headers` set of
  `Cache-Control` always wins (the page is more
  specific than the page-level default). The
  `CachePolicy` enum + `detect_config_cache` parser
  are hand-rolled (~50 lines) and tolerate the three
  key shapes (unquoted / `"cache"` / `'cache'`). 7
  new unit tests cover the parser + the enum, 1 new
  e2e covers the 5 contract scenarios (the three
  policy values, the opt-in absence, and the
  page-wins case).

- **`check` reports unknown runtime exports** (spec
  §46, plan §48). A `.tsp` file may only export the
  standard HTTP method handlers + the optional
  `config` object; any other `export function NAME(...)`
  is an "unknown runtime export". The slice surfaces
  these at check time so the user gets a clear
  message rather than discovering them at runtime
  via a silent ignore. The `detect_unknown_exports`
  parser is hand-rolled (~60 lines) and follows the
  same line-start rules as the other PageConfig
  detectors (sync / async, indented, with comment
  lines skipped). The runtime still serves the page
  (the unknown export is silently ignored, as it was
  before); the full spec §46 treatment (unknown
  exports are a generation-build failure) lands with
  the AST-based detector in a future slice. 6 new
  unit tests cover the parser, 1 new e2e covers the
  end-to-end check-time reporting.

The new e2e (3 tests):
  - `config_body_limit_enforces_per_page_cap`
  - `config_cache_sets_default_cache_control_header`
  - `check_reports_unknown_runtime_exports`

The new unit tests (20):
  - 7 in `page::tests` for `detect_config_body_limit`
  - 7 in `page::tests` for `detect_config_cache` +
    `cache_policy_header_value_maps_to_freeze_literals`
  - 6 in `page::tests` for `detect_unknown_exports`

- **Verification:** 302 tests green (248 lib + 4
  worker_integration + 15 process_model + 35
  start_order e2e). 3 new git commits on the
  parent tsp repo this session (post the morning's
  HTTP-hardening + PageConfig.methods batch):
  `f7ddf13d95` -> `156aac5c29` -> `2a89f30038`.

## Session update (2026-08-28) -- three more PageConfig / check-validation slices

Two more slices land after the morning/afternoon's first
batch. The themes: closing out the v2.0 core PageConfig
surface (spec §7) and the spec §46 export-validation
checks. The total over the day: 279 -> 332 tests, 32 ->
37 e2e, 235 -> 264 lib, 3 new commits on the parent
repo.

- **`check` reports `export default` violations** (spec
  §46, spec §67.4). A `.tsp` file may only export the
  named HTTP method handlers + the optional
  `const config = { ... }`; a top-level
  `export default ...` is silently ignored at runtime
  (the page registry reads only the named HTTP method
  exports and the `config` const). The slice surfaces
  default exports at check time so the user gets a clear
  message rather than a silent no-op. The
  `detect_default_export` parser is hand-rolled
  (~30 lines) and follows the same line-start rules
  as the other PageConfig detectors (sync / async,
  indented, with comment lines skipped). Catches the
  seven common shapes:
    * `export default foo;`
    * `export default { ... };`
    * `export default function() {}`
    * `export default async function() {}`
    * `export default () => {}`
    * `export default async () => {}`
    * `export default class {}`
  Anchored on `export default ` (with the trailing
  space) so `export { default: ... }` (a named
  re-export of a `default` member) is correctly NOT
  matched. 9 new unit tests cover the parser, 1 new
  e2e covers the end-to-end check-time reporting. The
  `{` and `}` in the help text and the error message
  are escaped as `{{` / `}}` to avoid Rust treating
  them as format-string placeholders.

- **`config.timeoutMs` per-page request timeout**
  (spec §7 v2.0 core PageConfig). A page may declare
  `config.timeoutMs: N` (milliseconds). The per-page
  value OVERRIDES the global `TSP_TIMEOUT_MS` for
  that one request. `0` means "no timeout" (the
  per-request abort signal is still created and wired
  in the wrap preamble, but the watchdog never
  fires) -- same as the global `0`. The per-page
  value is NOT silently clamped to the global; the
  page is the authority on its own timeout budget.
  The watchdog implementation in `worker/manager.rs`
  already supports per-request `timeout_ms`; this
  slice just resolves the effective timeout from the
  page's config and threads it through. The
  `detect_config_timeout_ms` parser follows the same
  line-shape rules as `detect_config_body_limit`
  (simple int, underscore separator, small
  expression form, `0` for "no timeout"). 7 new
  unit tests cover the parser, 1 new e2e covers the
  3 contract scenarios: (1) per-page timeout fires
  (504), (2) no per-page value uses the global
  default (200), (3) per-page `0` disables the
  watchdog (200).

  The e2e uses `await Bun.sleep(N)` to make the page
  take longer than the per-page timeout. The 504
  comes from the existing watchdog
  (`worker/manager.rs::execute_with_timeout`) and the
  existing 504 path in `host.rs` -- no new status
  codes.

The new e2e (2 tests):
  - `check_reports_default_export_violation`
  - `config_timeout_ms_overrides_global_request_timeout`

The new unit tests (16):
  - 9 in `page::tests` for `detect_default_export`
  - 7 in `page::tests` for `detect_config_timeout_ms`

- **Verification:** 332 tests green (264 lib + 4
  worker_integration + 15 process_model + 37
  start_order e2e). 3 new git commits on the
  parent tsp repo this session (post the morning's
  HTTP-hardening + PageConfig.methods batch, in
  addition to the first 3-slice batch):
  `347f906f65` -> `20cffb3917` (the third
  commit is the progress doc itself).

- **v2.0 core PageConfig is now complete** (spec §7):
    * `bodyLimit` (commit `f7ddf13d95`)
    * `cache` (plan §55, commit `156aac5c29`)
    * `methods` declared, validated against exports
      (previous session, commit `654e2f4a7c`)
    * `timeoutMs` (commit `20cffb3917`)
  `auth` is explicitly deferred (FREEZE §11: "the
  page config cannot raise the ceiling" is the
  only auth-related rule; the full `auth` policy
  lands with the auth service hooks in slice 9+).
  All three spec §46 export-validation items
  implemented at the check-time level (full
  generation-build enforcement lands with the AST
  detector in a future slice):
    * `config.methods` mismatch
    * unknown runtime exports
    * `export default` violation

### Known flake (still present)

The pre-existing `multi_route_dispatch_does_not_alias_to_first_request`
e2e and the `metrics_endpoint_serves_prometheus_text_after_priming_requests`
e2e both base their port on `30_000 + pid%500/1000`, so the
two tests can collide on a single port and the second one
fails to bind. The collision is intermittent (it depends
on the OS's TIME_WAIT state for the previous test's port)
and is unrelated to this slice. A future test-infra pass
should switch each test to a unique port base; this slice
does not change that pattern.

## Session update (2026-08-28) -- `TSP3001` typed error for invalid handler return (spec §6.3 / plan §10.4)

One more slice lands to close out the FREEZE item 5
contract for handler return values. The themes:
aligning the wrap-side error with the spec's
`TSP3001` wording, and distinguishing top-level
contract violations from nested JSX-child errors.

- **`TSP3001: handler returned unsupported value
  <Type>. Expected HtmlNode or Response.`** (spec §6.3
  / plan §10.4 / FREEZE item 5). A handler's return
  value MUST be one of: an `HtmlNode` (the TSP JSX
  runtime's opaque element) or a standard `Response`
  object. The pre-slice wrap threw a generic JS error
  ("page returned invalid value (expected string or
  Response, got X)") for non-`Response` non-`string`
  returns, and the JSX rendering layer threw a
  `TSP3102` ("object cannot be rendered as an HTML
  child") when it tried to treat the plain object as
  a JSX element. Neither matched the spec's `TSP3001`
  wording. The slice changes the
  `__tspRenderNode__` JS-side helper to branch on the
  `__child__` flag:
    * `__child__ === false` (top-level call from the
      wrap): throw `TSP3001: handler returned
      unsupported value <Type>. Expected HtmlNode or
      Response.` (matching plan §10.4 verbatim). The
      type name is capitalized to match the plan's
      "Object" / "Number" / etc. wording.
    * `__child__ === true` (recursive call from
      inside a JSX rendering): throw the existing
      `TSP3102` (the JSX rendering error stays scoped
      to JSX-internal mistakes).
  This is the minimum change that surfaces the
  spec's `TSP3001` for top-level contract violations
  while preserving the `TSP3102` for nested JSX
  child errors (which are a different class of
  mistake). The wrap's earlier "instanceof Response"
  / "typeof string" / else branch (the same file,
  later in the wrap) is kept and now uses the same
  `TSP3001` wording for the rare case where
  `__tspRenderNode__` is bypassed.

  1 new unit test
  (`wrap_invalid_return_value_uses_tsp3001_prefix`)
  exercises a `return 42` page and asserts the wrap
  contains the `TSP3001:` prefix and the capitalized
  type name (`Number`). The pre-existing
  `wrap_envelope_inspects_response_and_string` test
  is updated to assert on the new prefix.

  1 new e2e
  (`handler_returned_unsupported_value_surfaces_tsp3001`)
  boots the real binary, hits four pages that return
  each of the four forbidden shapes (`{redirect: ...}`,
  `42`, `true`, `undefined`), and pins for each:
    - the response is 500
    - the body contains the `TSP3001:` prefix
    - the body names the invalid type (`Object` /
      `Number` / `Boolean` / `Undefined`)

  The slice is wrap-side only. The host's existing
  500 path for `x-tsp-error: page` (the wrap's
  standard "I threw" signal) carries the new message
  to the client verbatim. No host-side changes
  needed.

- **Verification:** all lib + e2e tests green
  (modulo the pre-existing port-collision flake).
  1 new commit on the parent tsp repo this
  session:
  `09e113d602`.

### Summary across the day

- 8 slices / doc updates landed on the parent
  `D:/GitHub/tsp` repo:
    * 1 morning HTTP-hardening + tooling +
      PageConfig.methods batch (6 commits, 279 -> 302 tests)
    * 1 first afternoon PageConfig + check batch
      (3 commits, 302 -> 332 tests)
    * 1 second afternoon check + PageConfig batch
      (3 commits, 332 -> 322...330? tests; the
      2nd batch was the v2.0 core PageConfig close-out)
    * 1 third afternoon TSP3001 slice
      (1 commit, +1 unit +1 e2e)
  + 2 progress.md updates + 1 spec-freeze-style
  internal doc close-out.
- v2.0 core PageConfig is COMPLETE (spec §7):
  `bodyLimit`, `cache`, `methods`, `timeoutMs` all
  pinned by unit tests + e2e.
- All three spec §46 export-validation items are
  pinned at the check-time level
  (`config.methods` mismatch, unknown runtime
  exports, `export default` violation).
- The `TSP3001` typed error for invalid handler
  return is now wired through the wrap. The
  spec/plan wording is used verbatim.

## Day end (2026-08-28) -- doc integration

A single commit consolidates the day's work into
`docs/changelog.md` under `[Unreleased]`. The
changelog entry lists every feature added, every
bug fixed, and the final test count (332 tests,
all green on 5 consecutive runs). The 11 commits
on the parent `D:/GitHub/tsp` repo this session are
(chronological order):
  * `f7ddf13d95` bodyLimit
  * `156aac5c29` cache
  * `2a89f30038` unknown exports check
  * `8090a166d8` progress.md (3 PageConfig slices)
  * `347f906f65` default export check
  * `20cffb3917` timeoutMs
  * `80c805618a` progress.md (3 more PageConfig slices)
  * `09e113d602` TSP3001 typed error
  * `7b27300e54` progress.md (TSP3001 slice)
  * `c4f7eff7f8` fix: 413 drains body, multi-route port
  * `a0b5e81103` polish: --version + help text

The changelog entry is the canonical "what
landed today" record. The per-slice rationale
lives in each commit's message and in the
session updates above.
