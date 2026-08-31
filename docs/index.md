# TSP Documentation

TSP is a native Rust web runtime for `.tsp` pages. It combines a Rust HTTP
host with embedded Bun workers and a page-oriented TypeScript development model.

## Start here

- [Project overview and design features](https://github.com/risol/tsp#readme)
- [Download the latest prebuilt release](https://github.com/risol/tsp/releases/latest)
- [Embedded worker deployment guide](./worker.html)
- [Release and CI guide](./github-ci-cd.html)

## Architecture

- [TSP specification](./tsp-specification.html) — normative runtime and
  application contract.
- [Architecture and implementation plan](./tsp-plan.html) — design rationale,
  milestones, risks, and implementation details.
- [Phase 0 specification index](./reference/spec.html) — a guided map of the
  reference documents.

## Application reference

- [Frozen application contract](./reference/contract.html)
- [`.tsp` module format](./reference/tsp-module.html)
- [Request context and built-in modules](./reference/context.html)
- [JSX runtime](./reference/jsx-runtime.html)
- [Route examples](./reference/examples/)

## Engineering records

- [Architecture decision records](./reference/adr/)
- [Verified bugs and regressions](./reference/bugs/)
- [Changelog](./changelog.html)

This directory is the documentation source for the project's future GitHub
Pages site. Configure GitHub Pages to publish from the `docs/` directory on the
default branch.
