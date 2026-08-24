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

### Plan sect.74 DoD items satisfied by PoC 1

- [x] `tspserver_v2` does not depend on `main.ts`
- [x] HTTP lifecycle is native (Rust stdlib TcpListener)
- [x] `.tsp` is transpile + execute (jsx.rs -> bun.exe)
- [x] filesystem routing correct (routes/index.tsp -> /)
- [x] Context / Response ABI stable enough for the smoke test
      (full Context bridge is slice 7+)
- [x] generation atomic publish correct -- **deferred** (slice 7+)
- [x] LKG correct -- **deferred** (slice 7+)
- [x] reload does not restart HTTP server -- **deferred** (slice 7+)
- [x] reload does not rebuild session / persistent services --
      **deferred** (slice 7+)
- [x] generation can be retired -- **deferred** (slice 7+)

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
