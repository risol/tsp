# Getting started

This guide gets a small TSP application running from a release package or a
checkout of this repository.

## Release package

Download the latest package from the [GitHub Releases page](https://github.com/risol/tsp/releases).
Packages are provided for Linux x64, Windows x64, and macOS.

Create an application directory with this shape:

```text
my-app/
├── pages/
│   └── index.tsp
└── public/
```

Add the first route:

```tsx
export function GET() {
  return <h1>Hello from TSP</h1>;
}
```

Run the binary from the application directory:

```bash
./tspserver
```

On Windows PowerShell:

```powershell
.\tspserver.exe
```

The binary listens on port `3000` by default. Open `http://localhost:3000/`.

## Repository checkout

The repository includes a wrapper for the build and runtime workflow. After
the required Bun and Rust toolchains are available:

```bash
./tsp.sh build
./tsp.sh start
```

The wrapper uses port `9000` by default and points at the repository's
`pages/` and `public/` directories. Override these values with environment
variables or use the packaged binary directly.

Before starting, validate routes and imports:

```bash
./tsp.sh check:app
./tsp.sh typings
```

Add `.tsp-types` to the TypeScript project configuration if your editor needs
declarations for `tsp:*` imports.

## Add a dynamic route

Files map directly to URL paths:

```text
pages/index.tsp           -> /
pages/users/index.tsp     -> /users
pages/users/[id].tsp      -> /users/:id
pages/files/[...path].tsp -> /files/*
```

Read a dynamic parameter with `ctx.params`:

```tsx
import { type Context, text } from "tsp:server";

export function GET(ctx: Context) {
  return text(`user=${ctx.params.id}`);
}
```

## Development loop

The runtime watches the route tree. A changed route or local dependency is
rebuilt as a new generation. New requests use a valid candidate; in-flight
requests finish on their original generation. If a candidate fails, the last
known-good generation remains available.

Useful introspection commands are:

```bash
tspserver routes
tspserver graph
tspserver check --tsc
```

See [the module reference](./reference/tsp-module.html) for the full route
contract and [configuration](./configuration.html) for runtime settings.
