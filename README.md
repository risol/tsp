# TSP v2 — TypeScript Server Page

![TSP banner](./docs/images/banner.png)

[![CI](https://github.com/risol/tsp/actions/workflows/ci.yml/badge.svg)](https://github.com/risol/tsp/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/risol/tsp?display_name=tag)](https://github.com/risol/tsp/releases)

TSP v2 is a native Rust HTTP runtime that embeds Bun in worker child processes
for executing `.tsp` route modules. It is intentionally incompatible with the former TSP v1
`Page()`/React application host.

## Quick start

Requirements:

- Rust toolchain required by `bun/Cargo.toml`
- Bun 1.x for building the embedded worker
- Git with submodules when starting from a fresh checkout

Build the v2 runtime package:

```bash
git clone --recursive https://github.com/risol/tsp.git
cd tsp
./tsp.sh build
```

Build a Linux x64 `tspserver_v2` entirely inside Docker. The container uses
the same Bun, Rust nightly, and LLVM versions as the Linux GitHub Actions
release job, so a Windows or macOS host does not need the native build
toolchain:

```bash
# Optional: build the reusable compiler environment once.
bash docker/build-builder-image.sh

# Compile the server using that environment.
bash docker/build-linux.sh
```

The executable is written to `dist/tsp-v2-linux-x64/tspserver_v2`. To build the
runtime image after compiling it, use:

```bash
bash docker/build.sh
```

Run the development server:

```bash
./tsp.sh dev
```

The server listens on port `9000` by default and loads routes from `routes/`.
Route changes are watched and published without restarting the host.

## v2 route contract

Routes are `.tsp` modules. They export HTTP method handlers and may import the
reserved v2 modules `tsp:server` and `tsp:html`.

```tsx
import { type Context, type PageConfig } from "tsp:server";

export const config = {
  cache: "no-store",
} satisfies PageConfig;

export function GET(ctx: Context) {
  return <h1>Hello from {ctx.url.pathname}</h1>;
}
```

The v2 context exposes the request, URL, route parameters, query parameters,
cookies, session, services, abort signal, and route metadata. Handlers may
return JSX, strings, or `Response` values created with `json`, `html`, `text`,
`redirect`, or `notFound` from `tsp:server`.

Dynamic routes use filesystem segments such as `routes/users/[id].tsp`.
Static assets belong under `public/` and are served independently from route
modules.

## Configuration

The native host is configured through environment variables:

```text
TSP_PORT=9000
TSP_ROUTES_DIR=./routes
TSP_PUBLIC_DIR=./public
TSP_EMBEDDED_WORKER=1
TSP_WORKER_COUNT=2
```

See `./tsp.sh --help` and `tspserver_v2 --help` for worker recycling,
timeouts, Redis sessions, cgroup limits, and diagnostics.

## Commands

```bash
./tsp.sh build          # Build the single-file runtime and dist/tsp-v2 package
./tsp.sh build:host     # Copy the built runtime to dist/tsp-v2
./tsp.sh build:worker   # Build the single-file runtime
./tsp.sh start          # Run the packaged v2 server
./tsp.sh dev            # Run with route hot reload
./tsp.sh check          # cargo check for the v2 host
./tsp.sh test           # Rust tests plus embedded-worker smoke test
./tsp.sh test:rust      # Rust unit and Worker IPC tests
./tsp.sh test:smoke     # HTTP, metrics, and hot-reload smoke test
./tsp.sh package        # Package the single runtime binary
```

## Repository layout

```text
.
├── bun/src/runtime/tsp/       Native v2 host, router, watcher, services, worker
├── routes/                    Application route fixtures
├── public/                    Optional static assets
├── tests/v2_smoke/            End-to-end v2 route fixture
├── scripts/                   Build, package, benchmark, and smoke workflows
├── docs/v2/                   Frozen v2 contract and examples
├── types/                     TypeScript declarations for v2 builtin modules
└── tsp.sh                    Root v2 workflow wrapper
```

The v2 specification is in [`tsp-v2-specification.md`](./tsp-v2-specification.md).
The frozen application contract is [`docs/v2/FREEZE.md`](./docs/v2/FREEZE.md).
The embedded worker deployment guide is [`docs/v2.4-worker.md`](./docs/v2.4-worker.md).

## License

TSP is released under the MIT License.
