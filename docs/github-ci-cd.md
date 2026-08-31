# GitHub CI/CD

TSP uses GitHub Actions for native checks, embedded-worker smoke tests, and
tagged release packages.

## Workflows

### `ci.yml`

Pull requests and pushes to `master` and `main` run:

- Linux native Rust tests and host checking;
- Windows native checks and process-model tests;
- macOS native checks and process-model tests;
- Linux, Windows, and macOS embedded-worker smoke tests; and
- release-build packaging checks.

The workflow pins the Rust toolchain to `nightly-2026-07-20` and uses Bun
`1.3.14` for the bootstrap step.

### `release.yml`

Pushing a tag matching `v*` creates a draft release, builds the three supported
targets, runs smoke tests, uploads archives, and publishes the release after
all target jobs succeed.

## Local verification

From the repository root:

```bash
./tsp.sh check
./tsp.sh test:rust
./tsp.sh test:smoke
```

For a complete packaged runtime:

```bash
./tsp.sh build
./tsp.sh test
```

The build creates the single-file runtime, copies the host executable into
`dist/tspserver`, and packages the route and public roots with
`tsp.config.json`.

Application-level inspection is available without a full build when a host
binary already exists:

```bash
./tsp.sh check:app
./tsp.sh routes
./tsp.sh graph
./tsp.sh typings
```

## Release checklist

1. Run `tspserver check --tsc` against the application routes.
2. Run focused Rust and worker integration tests.
3. Build and package the release binary.
4. Verify the packaged `tsp.config.json`, `pages/`, and `public/` content.
5. Run the platform smoke tests using redirected, non-interactive I/O.
6. Push the version tag and wait for every release matrix job.
7. Confirm the generated archives and release notes before publishing.

## Cost and permission boundaries

The workflows use read-only repository permissions for CI. The release
workflow requests contents write permission only to create and upload tagged
release assets. Keep third-party action versions pinned or reviewed before
changing them.

## Failure diagnosis

Start with the failing stage: native check, worker startup, module evaluation,
packaging, or smoke request. On Windows, set `TSP_WORKER_STARTUP_TRACE=1` to
split JSC initialization, VM construction, and protocol readiness. Do not infer
a native root cause from a fault address without a symbolicated frame or a
reproducible startup boundary.

See [worker operations](./worker.html), the [architecture overview](./architecture.html),
and the [verified bug records](./reference/bugs/) for follow-up guidance.
