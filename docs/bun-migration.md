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
`src/runtime/module-graph.ts`; it is used by the page-loading boundary and by
unit tests for reverse-dependent dirty propagation and last-known-good
behavior. Recursive invalidation of already-loaded dependencies remains a
native Bun follow-up once the loader exposes its resolved module graph.

Build a standalone server with:

```bash
./tsp.sh build:tspserver
```

The resulting executable is paired with the external `www/` directory. A
plain official Bun binary can type-check and bundle the application, but page
requests require the TSP-enabled Bun fork.
