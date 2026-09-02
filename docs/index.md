# TSP documentation

TSP is a native Rust web runtime for TypeScript Server Pages. A Rust host
owns HTTP, routing, configuration, sessions, workers, and reloads. Embedded
Bun workers evaluate application TypeScript and TSX.

This documentation describes the runtime shipped by this repository. Future
ideas are labelled explicitly and are not part of the current contract.

## Choose a path

### New to TSP

- [Getting started](./getting-started.html) — install a release, create a
  route, and run the server.
- [Application configuration](./configuration.html) — environment variables,
  `tsp.config.json`, and generated typings.
- [Examples](./reference/examples/) — small route modules covering common
  request and rendering patterns.

### Building an application

- [`.tsp` module format](./reference/tsp-module.html) — exports, imports, and
  filesystem routing.
- [Context and server APIs](./reference/context.html) — requests, cookies,
  sessions, services, responses, and built-in modules.
- [JSX runtime](./reference/jsx-runtime.html) — server rendering, escaping,
  components, and fragments.
- [Current contract](./reference/contract.html) — the short compatibility
  checklist for application code.

### Operating TSP

- [Worker and deployment guide](./worker.html) — the embedded worker model,
  limits, diagnostics, and smoke tests.
- [Configuration reference](./configuration.html) — complete runtime
  settings and precedence rules.
- [Troubleshooting](./troubleshooting.html) — common startup, route, reload,
  and request failures.
- [GitHub CI/CD](./github-ci-cd.html) — checks, release builds, and local
  verification.

### Maintaining TSP

- [Runtime specification](./tsp-specification.html) — normative behavior.
- [Architecture and roadmap](./tsp-plan.html) — implementation boundaries,
  lifecycle, and planned work.
- [Reference map](./reference/spec.html) — where each rule is documented.
- [Architecture decision records](./reference/adr/)
- [Verified bugs and regressions](./reference/bugs/)
- [Bun upstream tracking](./reference/dependencies/bun-upstream.html)
- [Changelog](./changelog.html)

## Documentation rules

`reference/contract.md` summarizes the current public surface. The detailed
normative rules live in `tsp-specification.md`. The architecture plan is
non-normative: it explains ownership and direction but cannot expand the
current API by itself.

Examples should be checked with `tspserver check --tsc` before they are
treated as canonical. If an implementation and a document disagree, record
the discrepancy first; do not silently document an unimplemented feature.
