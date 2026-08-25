# ADR-0001: Subprocess is the v2 production JSC execution path

> Status: **Accepted (2026-08-24)**
> Decider: Sol + Mavis
> Slice: 13 (in-process JSC bridge, plan §25.3)

## Context

TSP v2 PoC 1 needs a JSC execution engine to evaluate `.tsp` page
modules. The plan (`tsp-v2-plan.md` §25.3) recommends reusing Bun's
in-process runtime:

> 若 TSP 基于 Bun fork,优先复用 Bun 自身 HTTP/runtime/event-loop 能力。

Slice 6 (PoC 1) closed with a subprocess bridge — the host spawns
the vendored `bun.exe` to run a per-request temp `.js` file. The
subprocess path was always intended as a placeholder while the
in-process bridge was scoped.

Slice 7 added `bun_runtime` as a dep + a `in_process_jsc` spike that
documented the gap. The current source does expose
`bun_jsc::VirtualMachine::init` and `bun_jsc::runtime_hooks()`, but
those are low-level pieces. The higher-level
`bun_runtime::cli::run_command::Run::boot` and `Run::start` path is
not an embedder API (`boot` is `pub(crate)`), and assumes a fully
bootstrapped CLI / dispatch / loader-hook environment. In addition,
referencing `bun_runtime` from this foreign TSP binary pulls in
transitive crates whose C/ABI symbols are provided by the Bun
executable, not by the `bun_runtime` rlib itself.

## Decision

**v2 production uses the subprocess bridge** (slice 6's
`jsc_bridge::execute` → vendored `bun.exe` → temp `.js` file →
captured stdout). The in-process JSC bridge is **future work**
requiring either:

1. A public `bun_runtime::create_isolated_vm()` API in the Bun
   fork, **or**
2. Significant Bun-fork work to extract the `Run::boot` flow into
   an embedder-facing entry point and make the full Bun runtime link
   surface available to the embedding binary.

Neither is in v2 scope.

The `bun_runtime` dep stays in `Cargo.toml` because the spike
module (`in_process_jsc.rs`) still uses it as a compile check and
removal would force a cold recompile of the entire Bun workspace
(1m51s on top of the per-slice incremental cost). The dep does
**not** enter the host's request hot path.

## What this freezes

- **Production JSC execution = subprocess** (plan §25.3
  interpretation: "re-use Bun" is satisfied by shelling to
  `bun.exe`; the alternative — a separate `axum + JSC worker`
  process — is what the plan explicitly warns against).
- **In-process JSC bridge = out of v2 scope** until Bun exposes
  an embedder API. This is not a regression from the plan; it is
  a documented gap.
- **No new dep enters v2 host** unless it lands on the subprocess
  hot path. Adding e.g. `tokio`, `uWS`, or `hyper` would
  duplicate the listener + event-loop work that the subprocess
  path already gets from `bun.exe`.
- **The `bun_runtime` dep is workspace-hygiene only** — it lets
  the spike stay green but is not a contract surface.

## Re-evaluation triggers

This decision should be revisited when **any** of the following
becomes true:

1. Bun ships a `bun_runtime::embed` (or similarly named) public
   API that creates an isolated VM with overridable loader / module
   hooks.
2. The Bun fork we maintain (vendored at `bun/` submodule, branch
   `bun-v1.4.0`) is willing to expose an embedder-safe equivalent of
   `Run::boot` and provide the runtime's required C/ABI symbols to the
   embedding binary.
3. A new use case requires in-process JSC for latency (e.g.
   streaming responses, server-sent events) that the subprocess
   path cannot meet.

If (1) lands upstream, the `bun_runtime` dep + `in_process_jsc.rs`
spike become the starting point for slice 14+. The spike's
"integration checklist" is the work list.

If (2) happens, the fork-side work is itself a new v2 slice
(probably slice 14), and slice 15 wires the in-process bridge.

## What did NOT change

- Plan §25.3 wording is preserved as-is. The plan said "re-use
  Bun"; we are still re-using Bun — just via subprocess rather
  than in-process. The plan's "do NOT use axum + separate JSC
  worker" warning is honoured (we use Bun, not a separate
  process).
- PoC 1 DoD items in plan §74 remain 100% satisfied by the
  subprocess path. The in-process bridge is a Phase 5+
  optimisation, not a PoC 1 requirement.
- The watcher's "any change dirties every slot" granularity
  (slice 11) and the request-pinning + in-flight dedup
  semantics (slice 12) are upstream of the JSC execution
  choice; they work identically against either path.

## How the spike is now treated

`bun/src/runtime/tsp/in_process_jsc.rs` is no longer a "slice 7
spike" — it is the ADR's reference code. It must:

1. Stay compile-green against the current `bun_runtime`.
2. Document the integration checklist (init sequence, hooks,
   `tsp:*` builtin module wiring).
3. NOT be wired into `host::render_for_route`. The host continues
   to call `jsc_bridge::execute` (subprocess) per request.
4. Be the FIRST thing a future slice reads when (1)/(2) of the
   re-evaluation triggers above is met.

## Link constraint (discovered during slice 13b)

un_runtime is an 
lib whose public surface compiles fine in
isolation, but its transitive deps (un_simdutf_sys, un_alloc,
un_s3_signing, un_socket_handlers, ...) each reference C
extern symbols that live in the un binary and are not exported
by the un_runtime rlib. The result: any un_runtime_tsp
binary that names a un_runtime::* symbol -- even via a
	ype X = bun_runtime::error::Error alias with no use -- triggers
2700+ unresolved externals at link time. cargo check succeeds
(type-check only); cargo test and cargo build fail.

Implications for a future in-process slice:

* The work to embed bun_runtime is more than the slice 7 spike
  described. It is not just VirtualMachine::init + hooks; the
  foreign linker has to see the C externs the un binary
  provides. Options are (a) Bun-side rlib hardening (export the
  C externs from un_runtime itself), (b) a feature-gated stub
  that only enables the in-process test path when the un
  symbols are available, or (c) keeping the in-process test as a
  separate integration test, not in cargo test.
* The in_process_jsc.rs module is symbol-free of un_runtime
  to keep cargo test green. The integration checklist that
  future work needs is plain text in the module doc comment, not
  a typed constant.

The spike is gated behind a compile check only — there is no
runtime branch. If a future change accidentally puts the
in-process path on the hot path, this ADR + the spike's
documentation + the 46-test regression suite should make the
regression obvious to a reviewer.
