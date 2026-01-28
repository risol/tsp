# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TSP (TypeScript Server Page) is a template server built with Deno + TSX + Preact. It executes `.tsx` files directly (like PHP) and serves them as HTML, with intelligent module caching based on file modification time.

## Common Commands

### Development
```bash
# Start dev server with hot reload
deno task dev

# Start production server
deno task start

# Compile to standalone binary
deno task compile

# Clean generated files
deno task clean
```

### Testing
```bash
# Run all tests
deno task test

# Run unit tests only
deno task test:unit

# Run E2E tests (compiles binary)
deno task test:e2e

# Run single unit test
deno test --allow-all tests/unit/injection_test.ts

# Run single E2E test
deno test --allow-all tests/unit/router_test.ts
```

### Code Quality
```bash
# Type check
deno task check
deno task check:all

# Format code
deno task fmt
deno task fmt:check

# Lint
deno task lint
```

### Binary Execution
```bash
# Run compiled binary (Windows)
DENO_DIR=./.deno ./tspserver.exe --root ./www --port 9000

# Run compiled binary (Linux/Mac)
DENO_DIR=./.deno ./tspserver --root ./www --port 9000

# With dev mode
./tspserver --root ./www --port 9000 --dev
```

## Critical Architecture Rules

### ⭐ MOST IMPORTANT: Import Restrictions

**NEVER import from `src/` in TSX files under `www/` or `tests/test_www/`**

```tsx
// ✅ CORRECT - Global types are available everywhere
export default async function(context: PageContext) {
  return <div>Hello</div>;
}

// ✅ CORRECT - Page function is global
export default Page(async function(context, { db, logger }) {
  const users = await db?.query('SELECT * FROM users');
  logger?.('Page loaded');
  return <div>{users}</div>;
});

// ❌ WRONG - Do NOT import from src/
import type { PageContext } from "../src/cache.ts";
import { Page } from "../src/injection-typed.ts";
```

**Why**: Types (`PageContext`, `RedirectResult`, `AppDeps`) and the `Page` function are declared globally in `types.d.ts`. Compiled binaries cannot resolve relative paths to `src/` during runtime.

**Allowed imports**:
- `www/` files can import from other `www/` files (components, etc.)
- `src/` files can import from other `src/` files
- `tests/unit/` can import from `src/`
- `tests/test_www/` CANNOT import from `src/`

See `docs/CODING_STANDARDS.md` for complete rules.

## High-Level Architecture

### Module Loading Strategy

The system uses a **hybrid loading approach** to support both `deno run` and compiled binary modes:

1. **Direct Import First** (for `deno run`):
   - Try `import(fileUrl)` directly for better performance
   - TSX files load natively with Deno's JSX support

2. **Fallback to @deno/loader** (for compiled binaries):
   - If direct import fails, use `@deno/loader` to transpile TSX → JavaScript
   - Load transpiled code via data URL
   - Required because compiled binaries can't bundle TSX transpiler

This logic is in `src/cache.ts:getPage()`.

### Request Processing Flow

```
HTTP Request → src/main.ts:handleRequest()
  ↓
src/router.ts:resolvePath() → maps URL to file path
  ↓
src/router.ts:securityCheck() → validates path and permissions
  ↓
src/context.ts:buildContext() → creates PageContext object
  ↓
src/cache.ts:getPage() → loads/transpiles TSX module (with caching)
  ↓
Execute page function with PageContext
  ↓
Process return value:
  - RedirectResult → HTTP redirect
  - Response object → direct return
  - JSX → render via Preact → HTML
```

### Module Caching

- **Key**: File modification time (mtime)
- **Storage**: `Map<string, CacheEntry>` in memory
- **Invalidation**: Automatic when file mtime changes
- **Cache invalidation**: Clears global loader cache when cache miss occurs

### Dependency Injection System

Type-safe dependency injection using the `Page` wrapper:

1. **Declaration** (`types.d.ts`):
   ```typescript
   interface AppDeps {
     db?: Database;
     logger?: (msg: string) => void;
   }
   ```

2. **Registration** (`main.ts` or app init):
   ```typescript
   import { registerDep } from "./src/injection-typed.ts";
   registerDep('db', (ctx) => new Database());
   ```

3. **Usage** (TSX pages):
   ```typescript
   export default Page(async function(ctx, { db, logger }) {
     // Full type inference, no imports needed
   });
   ```

The `Page` function:
- Wraps the page function
- Parses function signature to extract expected dependencies
- Builds all registered dependencies for each request
- Injects them as second parameter
- Validates all expected deps are registered

### Global Type System

All types are declared globally in `types.d.ts`:
- `PageContext` - Request context with method, URL, headers, query, body, cookies
- `RedirectResult` - HTTP redirect configuration
- `AppDeps` - Application dependencies (extendable)
- `Page` function - Dependency injection wrapper

This design means TSX files need NO imports for types or the Page function.

## Key Source Files

- **src/main.ts**: Server entry point, CLI argument parsing, HTTP request handler
- **src/router.ts**: URL → file path resolution, security checks
- **src/context.ts**: PageContext builder
- **src/cache.ts**: Module caching, TSX loading with hybrid strategy (direct import + @deno/loader fallback)
- **src/injection-typed.ts**: Type-safe dependency injection with global Page function
- **types.d.ts**: Global type declarations (CRITICAL - read this first)

## Configuration

The server supports three configuration methods (priority: CLI args > config file > defaults):

1. **CLI arguments**: `--root ./www --port 9000 --dev`
2. **Config file**: `config.json` or `config.jsonc` (auto-discovered)
3. **Defaults**: root="./www", port=9000, dev=false

## Test Structure

- **tests/unit/**: Unit tests without server (can import src/)
- **tests/test_www/**: Test pages for E2E (CANNOT import src/)
- **tests/run_e2e_tests.ts**: E2E runner that compiles binary and tests it
- **tests/run_unit_tests.ts**: Unit test runner

E2E tests compile a binary named `tspserver-test` to avoid conflicts.

## Compilation Notes

When compiling with `deno compile`:
- Binary must be run with `DENO_DIR=./.deno` environment variable
- Use `@deno/loader` for TSX transpilation (fallback mechanism)
- Test binaries use port 9100 to avoid conflicts with dev server (9000)
