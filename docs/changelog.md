# Changelog

All notable changes to TSP will be documented in this file.

## [Unreleased]

### Added
- Persistent v2.4 Master + IPC embedded Bun Workers with cross-platform
  Worker Manager integration tests for reuse, heartbeat, crash replacement,
  timeout replacement, and pool backpressure.
- v2.4 build, package, benchmark, and hot-reload smoke-test workflows.
- Free GitHub Actions CI/CD coverage for `master` and `main`.
- Fragment routing: a single `.tsp` file can expose named sub-renders via
  `<page>/__fragment/<name>`. Each fragment is a `Fragment()`-wrapped
  function and returns bare HTML (no `<!DOCTYPE>` wrapper) suitable for
  htmx `hx-get` / `hx-swap`. Fragments may also return a `Response` (e.g.
  JSON) — headers pass through. See `tests/test_www/fragments_demo.tsp`
  for a working example.
- Built-in htmx client: vendored `htmx.org@1.9.10` (~47KB minified) is
  served at `/__static/htmx.js` with `application/javascript` MIME and
  `Cache-Control: public, max-age=3600`. Drop
  `<script src="/__static/htmx.js">` into a page to enable htmx
  attributes without any external network fetch.

### Globals
- `Fragment<T>(fn)` — semantic alias of `Page<T>` for declaring named
  sub-renders inside `.tsp` files.
- `FragmentMap` — `Record<string, (ctx: PageContext) => Promise<any>>`
  matching the `fragments` named export convention.
- `hxUrl(page, name)` — builds the fragment URL, normalizing `.tsp`
  and the `__fragment` marker.
- `HtmxScript(props?)` — emits the vendored htmx client + optional
  `<meta name="htmx-config">` for global config (defaultSwap, timeout,
  historyCacheSize, withCredentials, indicatorClass, ...).
- `HtmxFragment(props)` — emits the hx-* wrapper div. Initial content
  is auto-fetched from the same page's `fragments[name]`; pass
  `children` explicitly to override.

## [0.1.0] - 2026-03-02

### Added
- Initial release
- TSP (TypeScript Server Page) template server using Deno + TSX + React
- Direct `.tsp` file execution (like PHP)
- Intelligent module caching with hot reload support
- Type-safe dependency injection system
- Built-in file manager with password protection
- MySQL Schema-first API with Zod validation
- Redis client support
- LDAP client support
- ExcelJS integration for Excel file operations
- Session management
- Cookie management
- Static file serving with caching
- Configuration auto-reload

### Features
- `.tsp` file suffix as route files
- Global type declarations (no imports needed)
- Schema-first data validation
- Hot reload via Deno's watch mode
- Cross-platform compilation
