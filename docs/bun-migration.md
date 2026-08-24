# Bun migration boundary

The Bun branch uses a persistent Bun HTTP process and keeps `www/` outside the
compiled application bundle. Start it with:

```bash
bun install
./tsp.sh dev
```

Page modules are loaded through:

```ts
await Bun.TSP.loadPage(filepath, reloadInDevelopment)
```

The TSP-enabled Bun fork owns canonical module identity, `.tsp`/TSX loading,
and same-key `config.root`-scoped invalidation in development. This invalidates
the page and imported project modules without changing their module keys. The
application layer owns page generations and last-known-good fallback, and deliberately does not use
`bun --hot`, `bun --watch`, query-string cache busting, or remote HTTP imports.

The application repository contains the portable graph model in
`src/runtime/module-graph.ts`. In development, it discovers static file
dependencies under `config.root`, checks their source stamps on requests, and
propagates changes through reverse dependencies so shared modules mark all
owning pages dirty. The page-loading boundary reloads only dirty pages and
keeps the last-known-good generation when a candidate fails. Precise native
registry eviction of only the resolved affected modules remains a Bun follow-up
once the loader exposes its resolved module graph; the current native fallback
invalidates the configured source scope for a dirty page.

Build a standalone server with:

```bash
./tsp.sh build:tspserver
```

The resulting executable is paired with the external `www/` directory. A
plain official Bun binary can type-check and bundle the application, but page
requests require the TSP-enabled Bun fork.
