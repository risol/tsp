# TSP reference map

This page is an index, not a second specification. The normative source for
the shipped runtime is [`tsp-specification.md`](../tsp-specification.html).

## Which document should I read?

| Need | Document |
| --- | --- |
| Install and run a first route | [`getting-started.md`](../getting-started.html) |
| Configure the server | [`configuration.md`](../configuration.html) |
| Understand the runtime | [`architecture.md`](../architecture.html) |
| Confirm the supported application surface | [`contract.md`](./contract.html) |
| Write a `.tsp` route | [`tsp-module.md`](./tsp-module.html) |
| Use request, response, session, or services APIs | [`context.md`](./context.html) |
| Render TSX and HTML safely | [`jsx-runtime.md`](./jsx-runtime.html) |
| Deploy and operate workers | [`worker.md`](../worker.html) |
| Diagnose a native regression | [`adr/`](./adr/) or [`bugs/`](./bugs/) |

## Source-of-truth policy

- `tsp-specification.md` defines current runtime behavior.
- `reference/contract.md` is the compact compatibility checklist.
- Reference topic pages explain one API area and link back to the
  specification for normative details.
- `tsp-plan.md` explains architecture and planned work. It does not add
  public API behavior.
- ADRs and bug records preserve engineering history and verified rationale.

## Current implementation checks

Use the packaged binary to inspect the application that is actually running:

```text
tspserver check --tsc
tspserver routes --json
tspserver graph --json
tspserver typings --out .tsp-types
```

The repository's `tests/` and `docs/reference/examples/` are the examples to
prefer when a page claims a behavior. The old `progress.md` link was removed:
progress is now represented by the changelog, contract status, and tests.
