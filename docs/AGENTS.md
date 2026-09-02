# TSP User Guide for AI Assistants

This file is shipped with the TSP runtime package. When helping a user build
or modify a TSP application, read this file first and follow the route and
runtime contract below. The application directory is the directory containing
this `AGENTS.md`, `tspserver` (or `tspserver.exe`), `pages/`, and `public/`.

## Application layout

```text
my-app/
├── AGENTS.md
├── tspserver                 # Linux/macOS
├── tspserver.exe             # Windows
├── tsp.config.json           # optional runtime configuration
├── pages/                    # HTTP route modules
│   └── index.tsp
└── public/                   # static files
```

If `tsp.config.json` is absent, the binary uses its built-in defaults. Keep
application code in `pages/` and `public/`; do not edit the binary.

## Writing `.tsp` routes

A `.tsp` file is a standard TypeScript/TSX HTTP route module. It has no
frontmatter, template delimiters, decorators, or special page class.

```tsx
import { type Context } from "tsp:server";

export function GET(ctx: Context) {
  return <h1>Hello from {ctx.url.pathname}</h1>;
}
```

Supported handler exports are `GET`, `POST`, `PUT`, `PATCH`, `DELETE`,
`HEAD`, `OPTIONS`, and `ANY`. Handlers may be synchronous or asynchronous.
Use `ANY` only when a wildcard handler is intended.

Use the built-in modules and explicit response helpers:

```tsx
import { type Context, json, redirect, text } from "tsp:server";

export async function POST(ctx: Context) {
  const body = await ctx.request.json<{ name: string }>();
  return json({ ok: true, name: body.name });
}

export function PUT(ctx: Context) {
  return text(`updated ${ctx.params.id}`, { status: 202 });
}

export function DELETE() {
  return redirect("/done", 303);
}
```

Important rules:

- Return JSX, a string, a trusted node, or `Response`. Use `json(value)` for
  objects; do not return arbitrary objects as responses.
- Import `Context`, helpers, fragments, and errors from `tsp:server`.
- Use `tsp:html` for escaping and explicitly trusted HTML.
- Dynamic segments use `[name]`; catch-all segments use `[...name]`. Their
  values are strings in `ctx.params`.
- A default export is not a route handler. Do not use a `Page()` wrapper or
  legacy global framework objects.
- Do not import one `.tsp` route from another. Put shared logic in `.ts` or
  `.tsx` modules.
- Static assets belong in `public/`, not in route handlers.

The file path maps to the URL:

```text
pages/index.tsp             -> /
pages/login.tsp             -> /login
pages/users/index.tsp       -> /users
pages/users/[id].tsp        -> /users/:id
pages/files/[...path].tsp  -> /files/*
```

Route-local options may be declared as follows:

```tsx
export const config = {
  methods: ["GET", "POST"],
  cache: "no-store",
  bodyLimit: 256 * 1024,
  timeoutMs: 10_000,
};
```

When `config.methods` is present, it must exactly match the exported handlers.

## Validate an application

Run these commands from the application directory. They inspect the route tree
and exit; they do not start the HTTP server.

```bash
./tspserver check --tsc --no-color
./tspserver routes --json
./tspserver graph --json
./tspserver typings --out .tsp-types
```

Use `check` after changing routes or local imports. Add `.tsp-types` to the
TypeScript project if the editor needs declarations for `tsp:*` imports.

## Start the server

The simplest start command is:

```bash
./tspserver
```

The default listener is `0.0.0.0:3000`. Open `http://localhost:3000/` after
adding `pages/index.tsp`.

For an explicit production-style setup on Linux or macOS:

```bash
TSP_PORT=9000 \
TSP_ROUTES_DIR="$PWD/pages" \
TSP_PUBLIC_DIR="$PWD/public" \
TSP_CONFIG="$PWD/tsp.config.json" \
TSP_EMBEDDED_WORKER=1 \
TSP_WORKER_COUNT=2 \
./tspserver
```

On Windows PowerShell:

```powershell
$env:TSP_PORT = "9000"
$env:TSP_ROUTES_DIR = "$PWD\pages"
$env:TSP_PUBLIC_DIR = "$PWD\public"
$env:TSP_CONFIG = "$PWD\tsp.config.json"
$env:TSP_EMBEDDED_WORKER = "1"
$env:TSP_WORKER_COUNT = "2"
.\tspserver.exe
```

The same `tspserver` executable owns the host and embedded workers. Do not
look for or create a separate worker binary.

## Configure the runtime

The precedence for the JSON configuration file is:

```text
--config/-c -> TSP_CONFIG -> ./tsp.config.json
```

For server settings, environment variables override `tsp.config.json`, which
overrides built-in defaults. Common environment variables are:

| Variable | Meaning | Default |
| --- | --- | --- |
| `TSP_PORT` | HTTP listener port | `3000` |
| `TSP_ROUTES_DIR` | `.tsp` route root | `pages` |
| `TSP_PUBLIC_DIR` | static asset root | `public` |
| `TSP_CONFIG` | JSON configuration path | `tsp.config.json` |
| `TSP_APPLICATION_NAME` | application registry name | `main` |
| `TSP_WORKER_COUNT` | embedded worker process count | `1` |
| `TSP_WORKER_MAX_IN_FLIGHT` | admission limit per worker | `2 * count` |
| `TSP_TIMEOUT_MS` | request watchdog in milliseconds | `30000` |
| `TSP_MAX_BODY_BYTES` | global request body cap | `1048576` |
| `TSP_DEVELOPMENT` | detailed HTML errors when `1` | `0` |
| `TSP_REDIS_URL` | optional Redis session backend | memory sessions |

The JSON configuration can contain these top-level sections:

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 3000,
    "routesDir": "./pages",
    "publicDir": "./public",
    "publicPrefix": "/static",
    "timeoutMs": 30000,
    "maxBodyBytes": 1048576,
    "development": false
  },
  "worker": {
    "count": 1,
    "maxInFlight": 2
  },
  "application": { "name": "main" },
  "session": { "redisUrl": null },
  "services": {}
}
```

Relative `server.routesDir` and `server.publicDir` paths are resolved relative
to the configuration file. `server.publicPrefix` controls the URL prefix for
files in `public/`; for example, `public/app.css` is served at
`/static/app.css` with the default prefix.

Server and worker startup settings are read when the process starts. Restart
the server after changing them. If a particular binary exposes additional
options, use `./tspserver --help` as the final authority.

## Guidance for AI-assisted changes

When asked to modify an application:

1. Read this file and inspect the existing `pages/`, `public/`, and config
   before editing.
2. Preserve the existing route contract and style. Do not introduce another
   web framework, frontmatter format, or server entrypoint.
3. Add or update the smallest `.tsp` route/module needed, and keep secrets out
   of source files.
4. Run `./tspserver check --tsc --no-color`, then exercise the affected URL.
5. If configuration changed, explain the required environment variables and
   whether a restart is needed.

For a route-only change, no runtime rebuild is needed. The development server
reloads route generations; a packaged binary is replaced only when rebuilding
the TSP runtime itself.
