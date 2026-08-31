# TSP Specification (Phase 0 summary)

> Status: Phase 0 of `tsp-plan.md` §61.
> Date: 2026-08-24
> Authoritative source: `tsp-specification.md` (1817 lines) and
> `tsp-plan.md` (3659 lines) at the repo root. This document is the
> **index** that points at the right section of those for each
> concern; it does not re-derive the spec.

## Architectural model

```text
TSP
  =  Rust Web Host (HTTP, router, cookies, session, services,
                     watcher, module graph, page registry,
                     generation, LKG, request lifecycle)
   + JavaScriptCore VM (JSC, evaluates application code)
   + TypeScript / TSX Server Pages (`.tsp` files)
```

`.tsp` is the **stable protocol boundary** between the host and the
application. The host owns lifecycle; the application owns business
logic; JSC is the execution engine; the protocol is the contract.

The full architecture diagram is `tsp-plan.md` §73.

## How to read this set of documents

| Doc                                | What it covers                                                |
|------------------------------------|---------------------------------------------------------------|
| `contract.md`                      | **The 12 contract items** application code can rely on. The deliverable of Phase 0. |
| `tsp-module.md`                    | `.tsp` file format, exports, import rules, route mapping.    |
| `jsx-runtime.md`                   | JSX -> HTMLNode contract, escaping, components, fragments.   |
| `context.md`                       | Context ABI, PageConfig, fragment, services, cookies, session. |
| `tsp-specification.md` (root)   | Full normative spec (1817 lines). The source of truth; everything above is a slice / index of this. |
| `tsp-plan.md` (root)            | Architecture plan (75 sections, 12 phases). Slice / milestone layout, risks, ADRs. |
| `progress.md`                      | Per-slice progress log. Start here to see what is done, what is deferred, and why. |

## Phase 0 deliverable

`contract.md` is the deliverable. The 12 items are the current contract;
once Sol signs off, application code is allowed to rely on them, and
later slices build on them, not against them.

## Phase 0 completion condition

Per `tsp-plan.md` §61:

> Phase 0 closes when the 12 freeze items have explicit answers and
> 10-20 `.tsp` example fixtures demonstrate the contract. No v1
> compatibility work begins until Phase 0 closes.

`contract.md` carries the 12 answers. `examples/` carries the fixtures.
After Sol signs off on `contract.md`, Phase 0 closes and slice 9+
(in-process JSC bridge, watcher + atomic reload, full Context
bridge, ...) starts.
