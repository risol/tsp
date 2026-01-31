# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 语言和交流规则

### 🌏 语言要求

**所有交流和内容必须使用中文**，除非遇到以下情况：

1. **代码和技术术语**：保留英文
   - 变量名、函数名、类名、类型名
   - 技术术语：TypeScript、Deno、JSX、API、HTTP、SQL 等
   - 编程概念：async/await、Promise、Proxy、Map 等

2. **代码注释**：必须使用中文
   - 单行注释：`// 这是一个注释`
   - 多行注释：`/* 这是多行注释 */`
   - JSDoc 注释：`/** 函数说明 */`

3. **文档和说明**：必须使用中文
   - Markdown 文档（*.md）
   - TSX 中的用户界面文本
   - 错误提示信息（用户可见的）
   - README、说明文档等

4. **回复语言**：使用中文
   - 与用户交流时使用中文
   - 解释概念时使用中文
   - 代码说明使用中文

### 📝 示例

```typescript
/**
 * 获取当前用户信息
 * @returns 用户对象或 null
 */
export async function getUser() {
  // 从数据库获取用户
  const user = await db.query('SELECT * FROM users');
  return user;
}
```

```tsx
// ✅ 正确 - 中文注释和界面文本
export default Page(async function(ctx, { session }) {
  // 获取当前用户
  const user = await session.getUser();

  if (!user) {
    return <div>请先登录</div>;  // 中文界面文本
  }

  return <div>欢迎, {user.name}!</div>;
});
```

```tsx
// ❌ 错误 - 英文注释和界面文本
export default Page(async function(ctx, { session }) {
  // Get current user
  const user = await session.getUser();

  if (!user) {
    return <div>Please login</div>;
  }

  return <div>Welcome, {user.name}!</div>;
});
```

## Project Overview

TSP (TypeScript Server Page) is a template server built with Deno + TSX + Preact. It executes `.tsx` files directly (like PHP) and serves them as HTML, with intelligent module caching and hot reload support for nested dependencies.

## Common Commands

### Development
```bash
# Start dev server with hot reload
deno task dev

# Start production server
deno task start

# Compile to standalone binary
deno task compile

# Build distribution package
deno task build

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
```

### Code Quality
```bash
# Type check
deno task check

# Format code
deno task fmt

# Lint
deno task lint
```

### Binary Execution
```bash
# Run compiled binary from project root
./tspserver --root ./www --port 9000

# With dev mode (hot reload enabled)
./tspserver --root ./www --port 9000 --dev

# From distribution package
cd dist
./tspserver --root ./www --port 9000
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

### Module Caching & Hot Reload

#### Cache Mechanism
- **Key**: File modification time (mtime)
- **Storage**: `Map<string, CacheEntry>` in memory
- **Location**: `.cache/tsp/` relative to current working directory
- **Invalidation**: Automatic when file mtime changes

#### Hot Reload System (Dev Mode)

The hot reload system supports **nested dependencies** through:

1. **Versioned Filenames**: Compiled files use version numbers to bypass Deno's import cache
   - Example: `Component.v7.js` instead of `Component.js`
   - Version increments on each recompilation

2. **Reverse Dependency Graph**: Tracks which files depend on which
   - When `Navigation.tsx` changes, `Layout.tsx` and `index.tsx` are automatically recompiled
   - Supports unlimited nesting depth

3. **Recursive Dependency Checking**:
   - Checks all transitive dependencies for modifications
   - Invalidates caches recursively up the dependency tree

#### Cache Directory
- **Development mode**: Cache at `project_root/.cache/tsp/`
- **Production mode (from root)**: Cache at `project_root/.cache/tsp/`
- **Production mode (from dist/)**: Cache at `dist/.cache/tsp/`

The cache directory is always relative to where the binary is run from.

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
- **src/cache.ts**: Module caching, hot reload, reverse dependency tracking
- **src/precompiler_lib.ts**: TSX → JS transpilation with versioned filenames
- **src/injection-typed.ts**: Type-safe dependency injection with global Page function
- **src/cookies.ts**: Cookie management module with Set-Cookie header support
- **src/static.ts**: Static file serving with caching
- **types.d.ts**: Global type declarations (CRITICAL - read this first)

## Configuration

The server supports three configuration methods (priority: CLI args > config file > defaults):

1. **CLI arguments**: `--root ./www --port 9000 --dev`
2. **Config file**: `config.json` or `config.jsonc` (auto-discovered)
3. **Defaults**: root="./www", port=9000, dev=false

The root path is automatically resolved to an absolute path, ensuring cache directory consistency regardless of where the binary is run from.

## Test Structure

- **tests/unit/**: Unit tests without server (can import src/)
- **tests/test_www/**: Test pages for E2E (CANNOT import src/)
- **tests/run_e2e_tests.ts**: E2E runner that compiles binary and tests it
- **tests/run_unit_tests.ts**: Unit test runner

E2E tests include:
- Basic HTTP functionality
- API tests
- Dependency injection
- Error handling
- Security (path traversal protection)
- **Hot reload with nested dependencies** (2+ level deep)

E2E tests compile a binary named `tspserver-test.exe` to avoid conflicts with development binaries.

## Compilation & Distribution

### Compilation Notes
When compiling with `deno compile`:
- Binary can be run from any directory
- Cache directory is created relative to working directory
- Use `@deno/loader` for TSX transpilation (fallback mechanism)
- Test binaries use port 9001 to avoid conflicts with dev server (9000)

### Distribution Package
```bash
# Build distribution package
deno task build

# The build creates:
dist/
  tspserver.exe          # Compiled binary
  www/                   # Copied from project root
  .deno/                 # Deno cache directory
  README.md              # Distribution readme
```

## Important Implementation Details

### Hot Reload Implementation
- **Versioned filenames**: `Component.v7.js` instead of query parameters or fragments
- **Source maps removed**: To avoid path resolution issues in compiled binaries
- **Recursive dependency tracking**: Supports unlimited nesting depth
- **Efficient invalidation**: Only recompiles affected files, not entire dependency tree

### Path Resolution
- All file paths are resolved to absolute paths at server startup
- This prevents issues when running from different directories
- Cache directory location is calculated based on current working directory

### Performance Optimizations
- In-memory module cache with mtime-based invalidation
- Static file caching with ETag and If-Modified-Since support
- Reverse dependency graph for efficient cascade invalidation
- Precompilation mode for production (no --dev flag)
