# TSP — TypeScript Server Page

<p align="center">
  <img src="./logo.png" alt="TSP logo" width="220">
</p>

<p align="center">
  A native Rust web runtime for <code>.tsp</code> pages, powered by Bun and
  inspired by the simplicity of classic PHP.
</p>

<p align="center">
  <a href="https://github.com/risol/tsp/releases/latest">Download the latest release</a>
  ·
  <a href="https://github.com/risol/tsp/releases">All releases</a>
</p>

[![CI](https://github.com/risol/tsp/actions/workflows/ci.yml/badge.svg)](https://github.com/risol/tsp/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/risol/tsp?display_name=tag)](https://github.com/risol/tsp/releases/latest)

TSP is a self-contained server runtime for building dynamic websites and HTTP
services with TypeScript. A Rust host handles HTTP, routing, request context,
and process management, while embedded Bun workers execute `.tsp` route
modules. The result is one native `tspserver` binary with a familiar
page-oriented development model and modern TypeScript APIs.

## Download

Download a prebuilt package from the [latest GitHub Release](https://github.com/risol/tsp/releases/latest).
Release packages are currently provided for:

- Linux x64 (`.tar.gz`)
- Windows x64 (`.zip`)
- macOS (`.tar.gz`)

No Rust or Bun installation is required to run a release package. Extract the
archive, then start the platform-specific `tspserver` executable.

## Quick start

After extracting a release package:

```bash
./tspserver
```

On Windows PowerShell:

```powershell
.\tspserver.exe
```

The server listens on port `9000` by default, loads routes from `pages/`, and
serves static assets from `public/`. Point the runtime at an existing project
with environment variables:

```text
TSP_PORT=9000
TSP_ROUTES_DIR=./pages
TSP_PUBLIC_DIR=./public
TSP_WORKER_COUNT=2
```

## Design features

TSP is designed around a small, explicit boundary between native infrastructure
and application code. The host owns the server lifecycle and the page runtime
owns request-level application behavior.

### Native host with embedded Bun workers

Rust owns HTTP, routing, request scheduling, deadlines, process management, and
long-lived services. Bun workers execute TypeScript and JSX in isolated,
persistent processes. This keeps native resources out of disposable page code
and gives the release package a single self-contained `tspserver` binary.

### `.tsp` is standard TSX

There are no PHP-style template delimiters, special page classes, or hidden
global framework objects. A `.tsp` file is a normal TypeScript/JSX module and a
route entry point. Export `GET`, `POST`, `PUT`, or `DELETE` explicitly so the
HTTP contract is visible in the file itself.

### Filesystem routing with predictable behavior

The route tree maps directly to URLs: `pages/index.tsp` serves `/`, while
`pages/users/[id].tsp` exposes a dynamic `id` parameter. Static files stay in
`public/` and are handled separately from executable route modules. Route
precedence and ambiguous routes are validated deterministically at startup.

### Disposable page generations

Each route is evaluated as an immutable page generation. A source change builds
a new candidate generation, validates it, and publishes it atomically. New
requests use the new generation while in-flight requests finish on the one they
started with. If a reload fails, TSP keeps serving the last known-good
generation instead of taking down the route.

### Native ownership of durable state

Page generations are replaceable; durable state is not hidden inside them.
Sessions, services, worker coordination, and other long-lived resources belong
to the native runtime or explicit external modules, so a route reload does not
silently reset application infrastructure.

### JSX for server rendering, without React

TSP's JSX runtime produces an HTML node tree for server rendering. It provides
components, async components, escaping, trusted HTML helpers, layouts, and
fragments without requiring a client-side React runtime or client state model.

### Explicit request and response APIs

Every handler receives a request-scoped `Context` with the URL, route
parameters, query, cookies, session, services, and abort signal. Responses are
returned as JSX, text, HTML, JSON, redirects, or standard `Response` values;
TSP does not infer response meaning from arbitrary object shapes.

### Development ergonomics with production boundaries

Development hot reload updates route generations without restarting the host.
The same route contract, worker isolation, request lifetime rules, and native
ownership boundaries remain in place when the prebuilt release binary is
deployed.

## Route contract

Routes are `.tsp` modules. Export the HTTP methods your route supports and use
the reserved `tsp:server` and `tsp:html` modules for runtime helpers.

```tsx
import { type Context, type PageConfig } from "tsp:server";

export const config = {
  cache: "no-store",
} satisfies PageConfig;

export function GET(ctx: Context) {
  return <h1>Hello from {ctx.url.pathname}</h1>;
}
```

Handlers may return JSX, strings, or standard `Response` values created with
helpers such as `json`, `html`, `text`, `redirect`, and `notFound`. Static
assets belong in `public/` and are served independently from route modules.

## Configuration

The runtime is configured through environment variables. In addition to the
paths and port above, `tspserver` supports worker pool sizing, request and
worker limits, timeouts, Redis-backed sessions, Linux cgroup limits, and
diagnostics. Run:

```text
tspserver --help
```

for the complete list of options.

## Documentation

- [TSP specification](./docs/tsp-specification.md)
- [Architecture and implementation plan](./docs/tsp-plan.md)
- [Application contract](./docs/reference/contract.md)
- [Embedded worker guide](./docs/worker.md)
- [Release and CI guide](./docs/github-ci-cd.md)

## License

TSP's original source code and contributions are released under the [MIT
License](./LICENSE). The runtime embeds Bun and includes third-party
components with their own licenses; see [Bun's license and notices](./bun/LICENSE.md).
