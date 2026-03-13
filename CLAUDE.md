# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Language Requirements

All communication and content must be in English. Code comments, variable names, and documentation should be in English.

## Project Overview

TSP (TypeScript Server Page) is a TypeScript server that executes `.tsp` files directly like PHP, designed for AI-driven development.

## Core Features

- Direct TSP Execution - execute `.tsp` files without build
- Hot Reload - via Deno's watch mode
- Type Safety - global type declarations, no imports needed
- Data Validation - schema-first with Zod
- Dependency Injection - type-safe DI system
- Static File Service - with caching support
- File Manager - web-based file management with password protection
- Configuration Auto-Reload - no restart needed
- Cross-Platform - Windows, Linux, macOS

## Common Commands

```bash
# Development
./tsp.sh dev          # Run with hot reload
./tsp.sh start        # Run production server

# Build
./tsp.sh build:tspserver       # Debug build
./tsp.sh build:tspserver:rel   # Release build

# Testing
./tsp.sh test        # Run all tests
./tsp.sh test:unit   # Unit tests only
./tsp.sh test:e2e    # E2E tests

# Code quality
./tsp.sh check       # Type check
./tsp.sh fmt         # Format code
./tsp.sh lint        # Lint

# Binary execution
./tspserver --root ./www --port 9000
./tspserver --root ./www --port 9000 --dev

# Workers mode (Linux/macOS only - uses SO_REUSEPORT)
./tspserver --workers 4          # 4 workers on port 9000
./tspserver -w 4 -p 8080        # 4 workers on port 8080
```

## Version Locations

Version is defined in:
- `src/version.ts` - `TSP_VERSION`
- `deno.json:2` - version field
- `CHANGELOG.md` - changelog

## Critical Architecture Rules

### Import Restrictions

**NEVER import from `src/` in TSP files under `www/` or `tests/test_www/`**

Types (`PageContext`, `RedirectResult`, `AppDeps`) and the `Page` function are declared globally in `types.d.ts`.

### TSP File Suffix Rules

- `.tsp` files are route files accessible via URL
- `.tsp` files can import `.ts`, `.tsx`, and other `.tsp` files
- `.ts` and `.tsx` files cannot be accessed directly via HTTP (return 404)
- Files starting with `__` cannot be accessed directly but can be imported

## Architecture

### Request Flow

```
HTTP Request -> router -> security check -> build context -> load module -> execute page -> render
```

### Dependency Injection

Use `Page` wrapper for type-safe DI:

```tsx
export default Page(async function(ctx, { db, logger }) {
  // Full type inference, no imports needed
});
```

### Global Types

All types declared in `types.d.ts`:
- `PageContext` - request context
- `RedirectResult` - redirect config
- `AppDeps` - application dependencies
- `Page` function - DI wrapper

## Key Source Files

- `src/main.ts` - Server entry, CLI, request handling
- `src/router.ts` - URL resolution, security
- `src/context.ts` - PageContext builder
- `src/injection-typed.ts` - DI system
- `src/cookies.ts` - Cookie management
- `src/static.ts` - Static file service
- `src/filemanager/` - File manager modules
- `types.d.ts` - Global type declarations

## Database - MySQL Schema-first API

```tsx
export default Page(async function(ctx, { createMySQL, createZod, response }) {
  const z = await createZod();
  const db = await createMySQL({ host: '127.0.0.1', port: 3306, user: 'user', password: 'pass', database: 'db' }, z);

  const UserSchema = z.object({ id: z.number(), username: z.string() });

  const users = await db.query(UserSchema, 'SELECT * FROM users');
  const user = await db.queryOne(UserSchema, 'SELECT * FROM users WHERE id = ?', [id]);
  const maybeUser = await db.queryMaybe(UserSchema, 'SELECT * FROM users WHERE email = ?', [email]);
  const count = await db.scalar(z.number(), 'SELECT COUNT(*) as value FROM users');

  const result = await db.execute(z.object({ affectedRows: z.number() }), 'INSERT INTO users (name) VALUES (?)', [name]);

  await db.tx(async (tx) => {
    // transaction operations
  });

  return response.json({ users });
});
```

Core methods: `query`, `queryOne`, `queryMaybe`, `scalar`, `execute`, `tx`, `queryPage`

## ExcelJS

```tsx
export default Page(async function(ctx, { createExcelJS, response }) {
  const ExcelJS = await createExcelJS();
  const workbook = new ExcelJS.Workbook();
  const worksheet = workbook.addWorksheet("Sheet1");
  worksheet.addRow(["Header1", "Header2"]);
  worksheet.addRow(["Data1", "Data2"]);
  await workbook.xlsx.writeFile("./output.xlsx");
  return response.json({ success: true });
});
```

Reference: [ExcelJS Documentation](https://github.com/exceljs/exceljs)

## Crypto & Bcrypt

```tsx
export default Page(async function(ctx, { crypto, createBcryptjs, response }) {
  const hash = await crypto.digest("SHA-256", "data");
  const key = await crypto.generateKey("AES-GCM", 256);

  const bcrypt = await createBcryptjs({ saltRounds: 10 });
  const hash2 = bcrypt.hash("password");
  const valid = bcrypt.compare("password", hash2);
});
```

## Configuration

Config file: `config.json` or `config.jsonc` (auto-discovered)

Priority: CLI args > config file > defaults

```jsonc
{
  "root": "./www",
  "port": 9000,
  "dev": false,
  "accessLog": { "file": ".logs/access.log", "rotation": { "maxSize": 10485760, "maxFiles": 5 } },
  "session": { "secure": false, "maxAge": 86400 },
  "logger": { "level": "INFO", "file": ".logs/app.log" },
  "fileManager": { "enabled": true, "path": "/__filemanager", "password": "xxx" },
  // Redis for session sharing (stateless sessions)
  "redis": { "host": "127.0.0.1", "port": 6379, "db": 0 }
}
```

## Test Structure

```
tests/
├── unit/           # Unit tests (can import src/)
├── test_www/       # E2E tests (cannot import src/)
├── run_e2e_tests.ts
└── run_unit_tests.ts
```

## File Manager

Access: `/__filemanager/`

```bash
curl -X POST http://localhost:9000/__filemanager/api/login -H "Content-Type: application/json" -d '{"password":"xxx"}'
```

## Port Management

```bash
bash ./kill-port.sh   # Cleanup port from config
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Port in use | `bash ./kill-port.sh` |
| Config not found | Use absolute path: `./tspserver --config /path/to/config.jsonc` |

## Compilation

```bash
# Build output
dist/tsp-win-x64-v0.1.4/tspserver.exe
dist/tsp-linux-x64-v0.1.4/tspserver
```

For full changelog, see [CHANGELOG.md](./docs/changelog.md)
