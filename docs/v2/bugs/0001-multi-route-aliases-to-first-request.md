# BUG-0001: Multi-route requests alias to the first request's body

> Status: **Resolved (2026-08-26)**
> Discovered: 2026-08-26
> Reporter: Mavis (during slice-12 smoke re-verification)
> Fix author: Mavis
> Fix reviewer: Sol
> Affected: TSP v2 master+worker IPC path on multi-route `routes/`
> Severity: Blocker for any deployment with `> 1` `.tsp` file
> Workaround: None needed — fixed in one line

## Resolution summary

One-line fix in `bun/src/runtime/tsp_worker.rs::execute_path_with_api_lock`
calling `vm.clear_entry_point()` before `vm.load_entry_point()`. The API
was already in the Bun fork (`VirtualMachine::clear_entry_point` at
`bun/src/jsc/VirtualMachine.rs:4879`); Bun's own test runner already
uses it for repeat runs. See "The fix" section below for the diff
context. Regression test `multi_route_dispatch_does_not_alias_to_first_request`
added to `bun/src/runtime/tsp/tests/start_order.rs`; 187/187 lib tests
+ the new regression test pass.

## TL;DR

When `routes/` contains more than one `.tsp` file, every URL on the
running server returns **the first request's response body verbatim**,
regardless of method or path. The first request was to `/`, so all
subsequent requests (`/time`, `/svc`, `/users/42`, `POST /`, ...) get
`routes/index.tsp`'s `GET` handler output. The single-route smoke test
(`tests/v2_smoke/routes/` contains only `index.tsp`) does NOT catch
this.

## Repro

```powershell
# from D:/GitHub/tsp
./tsp.sh build            # 7-10 min, builds bun_runtime_tsp + bun bundle
./tsp.sh build:host       # copy bun.exe to dist/tsp-v2/tspserver_v2.exe

$env:TSP_PORT            = 3000
$env:TSP_ROUTES_DIR      = "D:\GitHub\tsp\dist\tsp-v2\routes"  # 8 .tsp files
$env:TSP_EMBEDDED_WORKER = 1
$env:TSP_WORKER_COUNT    = 2
& ".\dist\tsp-v2\tspserver_v2.exe"
# in another shell:
foreach ($u in '/','/time','/svc','/session','/abort','/slow','/upload','/users/42') {
  (Invoke-WebRequest "http://127.0.0.1:3000$u" -UseBasicParsing).Content
}
```

**Observed (all 8 URLs):**

```
<!doctype html><h1>Hello GET /</h1><p>seen=none</p><p>cookies.has(sid)=false</p>
```

That's `routes/index.tsp` line 16 verbatim:
```js
return `<!doctype html><h1>Hello ${ctx.method} ${ctx.path}</h1>...`;
```

**Expected**: each URL returns its own handler's body, e.g.
`GET /time` → `iso=2026-... epoch_ms=... uptime_ms=... kind=time`.

**Also observed**: `POST /` returns 200 with the GET body (not
201 with `echo:...`), confirming the request's HTTP method is
also not reaching the right handler export. `POST /time` correctly
returns 405 (time.tsp has no `POST`), because the *method-not-allowed*
check fires before the cache-alias can.

## Where the bug lives

**It is NOT in the master.** Master-side dispatch has been fully
verified by the regression tests in
`bun/src/runtime/tsp/{router,page,jsx}.rs` (see "Tests that pin
master-side correctness" below). Live eprintln from a debug build
confirms the master sends a **per-request, source-specific** script
to the worker:

```
DEBUG page::prepare: route.path="/time"  source=".../time.tsp"  byte_len=1106
DEBUG page::prepare: route.path="/svc"   source=".../svc.tsp"   byte_len=1158
...
DEBUG worker execute_request:
    request.path=".../time.tsp"  request.method=GET
    script_bytes=17463
    source_url=//# sourceURL=tsp://D:/.../routes/time.tsp?generation=1
DEBUG worker execute_request:
    request.path=".../svc.tsp"   request.method=GET
    script_bytes=17512
    source_url=//# sourceURL=tsp://D:/.../routes/svc.tsp?generation=1
```

Each request produces a **different** `script_bytes` (16523–18469)
and a **different** `source_url`. The master is doing the right
thing on every leg.

**It IS in `bun/src/runtime/tsp_worker.rs`** (the worker process
that runs the JS). Specifically, the call chain:

```
Message::Execute { id, request: ExecuteRequest { method, path, script, ... } }
  -> tsp_worker.rs::execute_request
     -> writes request.script to a unique temp file
        `tsp-embedded-worker-<pid>-<NEXT_SCRIPT_ID>.tsx`
     -> calls execute_path(temp)
        -> calls vm.load_entry_point(temp_path)
           -> calls VirtualMachine::reload_entry_point
              -> regenerates entry.contents to `import * from "<temp_path>"`
              -> calls JSC__JSModuleLoader__loadAndEvaluateModule
                 with module name = "bun:main" (the hardcoded MAIN_FILE_NAME)
```

The JSC module registry keys `bun:main` by its **name** (a constant
in `bun/src/jsc/VirtualMachine.rs:626`: `pub const MAIN_FILE_NAME:
&[u8] = b"bun:main";`). The first call to `loadAndEvaluateModule`
loads + evaluates the synthetic entry, sets the `__tspEmbeddedResponse`
global, and stores the resolved promise in the registry under that
key. **All subsequent calls return the cached promise** (already
resolved), without re-evaluating. The new `entry.contents` (whose
import target points at a NEW temp file) is never run, the
`globalThis.__tspEmbeddedResponse = undefined` reset at the top of
the wrap preamble never fires, the wait loop in
`tsp_worker.rs::execute_path_with_api_lock` sees the stale value, and
returns the first request's body.

In short: Bun's per-VM `bun:main` cache outlives the request. The
worker calls the right API but the cache is per-key and the key is
constant.

## Tests that pin master-side correctness

Added in this investigation, all pass (`cargo test -p
bun_runtime_tsp --lib`, 187/187 green):

| File | Test | What it pins |
|---|---|---|
| `bun/src/runtime/tsp/router.rs` | `lookup_returns_route_specific_source_for_each_url` | `RouteTable::lookup("/time", GET)` returns the time route, not the root; `/users/42` binds `id=42`; `POST /time` is 405 not Found |
| `bun/src/runtime/tsp/page.rs` | `prepare_reads_route_source_not_a_hardcoded_path` | `page::prepare(&route)` reads `route.source` verbatim — two different routes resolve to two different file bodies, no shared aliasing |
| `bun/src/runtime/tsp/jsx.rs` | `wrap_for_embedded_worker_distinguishes_two_routes` | `wrap_for_embedded_worker(index_src, "GET", json)` and `wrap_for_embedded_worker(time_src, "GET", json)` produce two byte-different wraps, each carrying its own source-specific template |
| `bun/src/runtime/tsp/jsx.rs` | `wrap_for_embedded_worker_method_bakes_into_handler_selection` | The wrap's `__tspHandler__ = METHOD;` line stamps the actual request method into the script — `GET` for GET requests, `POST` for POST requests |

If any of these regress in the future, the bug class can re-surface
from a different stage; together they form a "master is correct"
invariant.

## What I tried that did NOT fix it

1. **Set `vm.main_is_html_entrypoint = true` for the duration of
   `load_entry_point`**: my hypothesis was that `loadAndEvaluateModule`
   would then key on `vm.main()` (= the temp file path, unique per
   request) instead of `bun:main`. **Result: broke the build.** The
   HTML entry-point branch fires `Bun__loadHTMLEntryPoint` which
   iterates the temp dir looking for `*.html`; there are none, so
   every request fails with `No HTML files found matching
   "...tsp-embedded-worker-...tsx"`. Wrong toggle.

2. **Pass a query string in the path** (e.g. `temp.tsx?req=N`): my
   hypothesis was that the module loader's URL cache key would
   include the query, so the synthetic entry's `import "temp.tsx?req=1"`
   would miss against the cached `import "temp.tsx?req=0"`.
   **Result: bug unchanged.** Confirmed via debug eprintln that
   `load_entry_point` was called with the new path, but the resolved
   promise was still the first request's. The `bun:main` cache lives
   in the C++ module registry, not the URL -> module resolution
   cache, so the import target's URL doesn't matter.

## The fix (already available)

**The fix is one line in `tsp_worker.rs::execute_path_with_api_lock`.**
Bun's `VirtualMachine` already exposes a public API for exactly this
case — `clear_entry_point()` at
`bun/src/jsc/VirtualMachine.rs:4879` — and Bun's own test runner
already uses it for repeat runs:

```rust
// bun/src/runtime/cli/test_command.rs:3205, 3236
vm.clear_entry_point()?;
```

`clear_entry_point` is a thin wrapper that calls
`JSGlobalObject::delete_module_registry_entry("bun:main")`, which
the C++ side implements as
`moduleLoader->removeEntry(identifier)`
(`bun/src/jsc/bindings/bindings.cpp:3041`).

The TSP worker should call it before every `load_entry_point`:

```rust
// in tsp_worker.rs::execute_path_with_api_lock
self.vm.clear_entry_point()
    .map_err(|error| format!("{error:?}"))?;
self.vm.load_entry_point(path.as_bytes())
    .map_err(|error| format!("{error:?}"))?;
```

Only `bun:main` needs to be cleared. The temp file path is unique
per request, so the imported module is automatically a fresh entry
in the registry, and any shared `.ts/.tsx` modules that the page
imports (e.g. `tsp:server` shims) keep their cache hits across
requests — no over-invalidation.

An earlier revision of this report (incorrect) suggested this API
had to be added and estimated "1 day of Bun work". That was wrong —
the API is in the current fork and is already used in
`runtime/cli/test_command.rs`. The TSP-side change is the one
line above plus a regression test.

### Multi-worker cache isolation

The `bun:main` cache is **per worker VM**, not global. Each spawned
worker is a separate `tspserver_v2.exe` process with its own
`VirtualMachine` and its own `clear_entry_point` lifecycle. The
implications for the bug's surface area, now that the fix is
applied:

- **Single worker, serial requests**: every request after the first
  aliases to the first request's body. Reproduces 100% of the time
  on `TSP_WORKER_COUNT=1`.
- **Single worker, concurrent requests**: same alias, race-free
  (the bug is not a race, it's a deterministic cache).
- **N workers, serial requests from one client**: typically the
  load-balancing pool pins the client to one worker, so the alias
  persists across requests. If pool rotation kicks in, different
  workers can show *different* first-request bodies — a more
  subtle, intermittent failure mode.
- **N workers, concurrent requests across clients**: each worker
  aliases to its own first request, so two clients can see two
  different wrong bodies for the same URL.

A regression test on `TSP_WORKER_COUNT=1` is sufficient to catch
the deterministic case (which is what the original report
described). A second test with `TSP_WORKER_COUNT=2` and concurrent
requests would catch the multi-worker case.

## What I tried that did NOT fix it (and why)

1. **Set `vm.main_is_html_entrypoint = true` for the duration of
   `load_entry_point`**: hypothesis was that `loadAndEvaluateModule`
   would then key on `vm.main()` (= the temp file path, unique per
   request) instead of `bun:main`. **Result: broke the build.** The
   HTML entry-point branch fires `Bun__loadHTMLEntryPoint` which
   iterates the temp dir looking for `*.html`; there are none, so
   every request fails with `No HTML files found matching
   "...tsp-embedded-worker-...tsx"`. Wrong toggle. (Also: this
   would have been a workaround for a bug that has a real
   solution, see "The fix" above.)

2. **Pass a query string in the path** (e.g. `temp.tsx?req=N`):
   hypothesis was that the module loader's URL cache key would
   include the query, so the synthetic entry's
   `import "temp.tsx?req=1"` would miss against the cached
   `import "temp.tsx?req=0"`. **Result: bug unchanged.** Confirmed
   via debug eprintln that `load_entry_point` was called with the
   new path, but the resolved promise was still the first request's.
   The `bun:main` cache lives in the C++ module registry, not the
   URL -> module resolution cache, so the import target's URL
   doesn't matter. (Correct fix is `clear_entry_point()`, see above.)

## Test coverage gap this exposes

The existing test suite did not catch this:

- **Master-side unit tests** (the four new ones listed above, plus
  the existing router / page / jsx tests) verify that the master
  produces a per-request, source-specific script. They do not
  exercise the worker, so they cannot catch a worker-side bug.
- **Worker integration tests** (`bun/src/runtime/tsp/tests/`)
  use `tsp_worker_test_stub`, which echoes `request.script` back
  without evaluating it. They cannot catch a worker-side bug in
  the Bun VM evaluation path.
- **End-to-end smoke** (`scripts/smoke-tspserver-v2.{sh,ps1}`)
  uses `tests/v2_smoke/routes/` which contains a single
  `index.tsp`. The bug only manifests with `>= 2` `.tsp` files
  in the routes dir.

**Required new regression test**: spawn the real
`dist/tsp-v2/tspserver_v2.exe` against a temp `routes/` dir that
contains at least two distinct `.tsp` files, send one request to
each (in sequence, on a single-worker pool so the alias is
deterministic), and assert that each request's body matches the
respective route's own handler output. A second test with
`TSP_WORKER_COUNT=2` and two concurrent clients would cover the
multi-worker case. Both should live in
`bun/src/runtime/tsp/tests/start_order.rs` (already has the real
binary spawn scaffold) or a new sibling file.

## Files involved

- `bun/src/runtime/tsp_worker.rs` — worker entry, `execute_request`,
  `execute_path`, `execute_path_with_api_lock`. **Primary site of
  the bug.**
- `bun/src/jsc/VirtualMachine.rs` — defines `load_entry_point`,
  `reload_entry_point`, `MAIN_FILE_NAME = b"bun:main"`. Holds the
  hardcoded cache key.
- `bun/src/bundler/entry_points.rs` — `ServerEntryPoint::generate`
  produces the synthetic `bun:main` contents. The contents
  change per request but the module registry does not re-evaluate
  them.
- `bun/src/runtime/tsp/bin/tspserver_v2.rs` — master entry, spawns
  the worker, dispatches requests. Confirmed correct.
- `bun/src/runtime/tsp/host.rs` — `handle_connection`,
  `render_per_request`, `render_for_route`. Confirmed correct
  (dispatch reaches the worker with the right script per request).
- `bun/src/runtime/tsp/{router,page,jsx}.rs` — host-side logic. All
  confirmed correct via the new unit tests.

## Build / test commands the next person will need

```bash
# full rebuild after touching tsp_worker.rs
cd D:/GitHub/tsp
./tsp.sh build              # 7-10 min, only need this after a Rust change
./tsp.sh build:host         # copy bun.exe -> dist/tsp-v2/

# fast iteration on host-side logic (no bun bundle needed)
cd D:/GitHub/tsp/bun
rustup run nightly-2026-07-20 cargo test -p bun_runtime_tsp --lib --locked
# 187 tests, ~5s

# full v2 smoke + integration tests
cd D:/GitHub/tsp
./tsp.sh test               # cargo tests + smoke against the real binary
```
