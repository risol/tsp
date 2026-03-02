# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Language and Communication Rules

### Language Requirements

**All communication and content must be in English**, except in the following cases:

1. **Code and Technical Terms**: Keep in English
   - Variable names, function names, class names, type names
   - Technical terms: TypeScript, Deno, JSX, API, HTTP, SQL, etc.
   - Programming concepts: async/await, Promise, Proxy, Map, etc.

2. **Code Comments**: Must be in English
   - Single line comments: `// This is a comment`
   - Multi-line comments: `/* This is a multi-line comment */`
   - JSDoc comments: `/** Function description */`

3. **Documentation and Instructions**: Must be in English
   - Markdown documentation (*.md)
   - User interface text in TSX
   - Error messages (user-visible)
   - README, documentation, etc.

4. **Response Language**: Use English
   - Use English when communicating with users
   - Use English when explaining concepts
   - Use English for code explanations

### Examples

```typescript
/**
 * Get current user information
 * @returns User object or null
 */
export async function getUser() {
  // Get user from database
  const user = await db.query('SELECT * FROM users');
  return user;
}
```

```tsx
// Correct - English comments and UI text
export default Page(async function(ctx, { session }) {
  // Get current user
  const user = await session.getUser();

  if (!user) {
    return <div>Please login first</div>;
  }

  return <div>Welcome, {user.name}!</div>;
});
```

## Project Overview

TSP (TypeScript Server Page) is a TypeScript server that executes `.tsp` files directly like PHP, designed for AI-driven development.

**Detailed Feature Documentation**: See [TSP TypeScript Features](./docs/TSP_FEATURES.md)

## Core Features

### Main Features
- **Direct TSP Execution**: Execute `.tsp` files directly like PHP, no build required
- **Intelligent Module Caching**: Automatic cache invalidation based on file modification time
- **Hot Reload**: Supports hot reload for nested dependencies at any depth
- **Type Safety**: Global type declarations, no imports needed
- **Data Validation**: Schema-first data validation using Zod, type-safe database queries
- **Dependency Injection**: Type-safe dependency injection system
- **Static File Service**: Built-in static file service with caching support
- **File Manager**: Built-in web file manager with password protection
- **Configuration Auto-Reload**: Automatically reload configuration after changes, no restart needed
- **Cross-Platform Compilation**: Supports Windows, Linux, macOS
- **.ts File Support**: Preprocessor handles `.tsp`, `.tsx`, and `.ts` files

## Version Management

### Version Number Rules

TSP follows [Semantic Versioning](https://semver.org/) specification, with version format `MAJOR.MINOR.PATCH`.

#### Version Format: `x.y.z`

- **MAJOR**: Incompatible API changes
- **MINOR**: Backward-compatible new features
- **PATCH**: Backward-compatible bug fixes

#### Example: `2.0.0`

- `2` - Major version
- `0` - Minor version
- `0` - Patch version

### Version Upgrade Rules

#### When to Upgrade Major Version?
- Modified `Page` function signature causing existing code to fail
- Removed public API or important features
- Changed configuration file format incompatibly
- Major changes to dependency injection system
- Incompatible changes to global types in `types.d.ts`

**Example**: `2.0.0` -> `3.0.0`
- Remove `Page` function, use new routing system
- Modify `AppDeps` interface, not backward compatible

#### When to Upgrade Minor Version?
- Add new features while maintaining backward compatibility
- Add new dependency injection items
- Add new helper functions or types
- Expand configuration options while maintaining backward compatibility

**Example**: `2.0.0` -> `2.1.0`
- Add `testHelper` dependency injection
- Add WebSocket support
- Add new static file type support

#### When to Upgrade Patch Version?
- Fix bugs without changing API
- Optimize performance without changing functionality
- Fix documentation errors
- Improve error messages
- Fix security issues without changing API

**Example**: `2.0.0` -> `2.0.1`
- Fix memory leak
- Fix hot reload not working in some cases
- Optimize cache performance
- Fix path traversal security vulnerability

### Current Version Locations

Version numbers are defined in the following locations (must be synchronized when releasing):

1. **`src/main.ts:36`** - Main version definition
   ```typescript
   const TSP_VERSION = "4.0.0";
   ```

2. **`deno.json:2`** - Deno configuration file
   ```json
   {
     "version": "4.0.0",
     ...
   }
   ```

3. **`CHANGELOG.md`** - Changelog (records changes)

### Release New Version Process

1. **Update version number**
   ```bash
   # Update TSP_VERSION in src/main.ts
   # Update version in deno.json
   ```

2. **Update CHANGELOG.md**
   ```markdown
   ## [2.1.0] - 2026-02-XX

   ### Added
   - New feature description

   ### Improved
   - Improvement content

   ### Fixed
   - Fix content
   ```

3. **Run tests**
   ```bash
   ./tsp.sh test
   ```

4. **Build release package**
   ```bash
   ./tsp.sh build:tspserver:rel
   ```

5. **Create Git tag** (optional)
   ```bash
   git tag -a v0.1.0 -m "Release v0.1.0"
   git push origin v0.1.0
   ```

### Version History

| Version | Date       | Major Changes                      |
|---------|------------|-------------------------------------|
| 4.0.0   | 2026-02-25 | .tsp file suffix support, precompiler upgrade |
| 2.0.0   | 2026-02-04 | Configuration auto-reload, port cleanup tool |
| 0.5.0   | 2026-01-29 | Access log feature                 |
| 0.4.0   | 2026-01-28 | Dependency injection system refactor |

For detailed changelog, see [CHANGELOG.md](./docs/changelog.md)

## Common Commands

### Development

```bash
# First compile custom Deno (deno-tsp)
./tsp.sh build:denort   # Compile denort-tsp runtime (debug)
./tsp.sh build:deno      # Compile deno-tsp CLI (debug)

# Run development server (hot reload)
./tsp.sh dev

# Run production server
./tsp.sh start

# Compile TSP server as standalone binary
./tsp.sh build:tspserver      # debug -> dist/debug/
./tsp.sh build:tspserver:rel  # release -> dist/release/
```

### Release Version

```bash
# Compile Release version
./tsp.sh build:denort:rel
./tsp.sh build:deno:rel

# Compile TSP server Release version
./tsp.sh build:tspserver:rel
```

### Port Management

```bash
# Kill processes using config port
bash ./kill-port.sh
```

### Testing

```bash
# Run all tests
./tsp.sh test

# Run unit tests
./tsp.sh test:unit

# Run E2E tests
./tsp.sh test:e2e

# Run single unit test
./tsp.sh test:unit
```

### Code Quality

```bash
# Type check
./tsp.sh check

# Format code
./tsp.sh fmt

# Lint code
./tsp.sh lint
```

### Binary Execution

```bash
# Run compiled binary (hot reload always enabled)
./tspserver --root ./www --port 9000

# With dev mode (skips precompilation, shows detailed errors)
./tspserver --root ./www --port 9000 --dev

# From distribution package
cd dist
./tspserver --root ./www --port 9000
```

## Critical Architecture Rules

### MOST IMPORTANT: Import Restrictions

**NEVER import from `src/` in TSP files under `www/` or `tests/test_www/`**

```tsx
// CORRECT - Global types are available everywhere
export default async function(context: PageContext) {
  return <div>Hello</div>;
}

// CORRECT - Page function is global
export default Page(async function(context, { db, logger }) {
  const users = await db?.query('SELECT * FROM users');
  logger?.('Page loaded');
  return <div>{users}</div>;
});

// WRONG - Do NOT import from src/
import type { PageContext } from "../src/cache.ts";
import { Page } from "../src/injection-typed.ts";
```

**Why**: Types (`PageContext`, `RedirectResult`, `AppDeps`) and the `Page` function are declared globally in `types.d.ts`. Compiled binaries cannot resolve relative paths to `src/` during runtime.

**Allowed imports**:
- `www/` files can import from other `www/` files (components, etc.)
- `src/` files can import from other `src/` files
- `tests/unit/` can import from `src/`
- `tests/test_www/` CANNOT import from `src/`

### TSP File Suffix Rules

**TSP uses `.tsp` as the route file suffix**:

- `.tsp` files are route files accessible via URL
- `.tsp` files can only import `.ts` and `.tsx` files, not other `.tsp` files
- `.ts` and `.tsx` files cannot be accessed directly via HTTP (return 404)
- Files starting with `__` cannot be accessed directly via HTTP but can be imported by other files

```typescript
// www/index.tsp - access / or /index.tsp
// www/page.tsp - access /page or /page.tsp
// www/api/user.tsp - access /api/user or /api/user.tsp
// www/components/Layout.tsx - component, can be imported, cannot be accessed directly
// www/lib/utils.ts - utility module, can be imported, cannot be accessed directly
```

## High-Level Architecture

### Module Loading Strategy

The system uses **Deno's native module loading** to support both development and compiled binary modes:

1. **Development Mode** (using `./tsp.sh dev`):
   - Uses Deno's native `import()` with JSX support
   - TSP files are natively supported by the custom `deno-tsp` binary

2. **Compiled Binary Mode** (using `./tsp.sh build:tspserver`):
   - Uses `deno-tsp compile` with embedded runtime
   - Supports `.tsp` files via custom deno-tsp binary

This logic is in `src/main.ts`.

**Custom Deno (deno-tsp)**:
- Binary renamed to `deno-tsp` (avoids conflict with official Deno)
- Supports `.tsp` file extension natively
- Includes `--dynamic-import-no-cache` flag for hot reload

### Request Processing Flow

```
HTTP Request -> src/main.ts:handleRequest()
  ->
src/router.ts:resolvePath() -> maps URL to file path (.tsp)
  ->
src/router.ts:securityCheck() -> validates path and permissions
  ->
src/main.ts:reloadConfigIfNeeded() -> checks and auto-reloads config
  ->
src/context.ts:buildContext() -> creates PageContext object
  ->
src/cache.ts:getPage() -> loads/transpiles TSP module (with caching)
  ->
Execute page function with PageContext
  ->
Process return value:
  - RedirectResult -> HTTP redirect
  - Response object -> direct return
  - JSX -> render via React -> HTML
```

### Configuration Auto-Reload

**Core Feature**: No server restart needed after modifying config file, changes take effect automatically

**Implementation**:
1. Global variables track config file state:
   - `currentConfig`: Current config object
   - `configFilepath`: Config file path
   - `configMtime`: Config file modification time

2. Check config file on each request:
   - Compare `configMtime` with current file modification time
   - If file is modified, reload config
   - Validate and apply new config

3. Config items that support auto-reload:
   - File manager password
   - Logger configuration
   - Static file extensions
   - Port (requires restart, as port is bound at startup)
   - Root directory (requires restart, as path is resolved at startup)

**Related Files**:
- `src/main.ts:reloadConfigIfNeeded()` - Core config reload logic
- `src/main.ts:parseArgs()` - Records config file path and modification time
- `src/main.ts:handleRequest()` - Checks config on each request

### Module Loading & Hot Reload

#### New Architecture (Simplified)

TSP uses a simplified module loading architecture:

1. **On-demand compilation**: Check if source file is newer than cached file on each request, recompile if needed
2. **Hot reload**: Implemented via Deno's watch mode (server auto-restarts on file changes)
3. **Simplified caching**: No longer maintains complex in-memory cache and dependency graph

#### Cache Mechanism
- **Key**: File modification time (mtime)
- **Storage**: Compiled JS files in `.cache/tsp/` directory
- **Invalidation**: Automatic detection of source file modifications
- **Supported files**: `.tsp`, `.tsx`, `.ts`

#### Hot Reload

Hot reload is implemented via Deno's built-in watch feature:

```bash
# Dev mode: monitors file changes and auto-restarts server
./tsp.sh dev

# Or use the compiled binary (hot reload always enabled)
./dist/tspserver --dev
```

**Note**: Hot reload is now implemented via Deno's built-in watch feature, which auto-restarts the entire server when files change.

#### Cache Directory
- **All modes**: Cache stored in `project_root/.cache/tsp/`

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

This design means TSP files (`.tsp`) need NO imports for types or the Page function.

## Key Source Files

### Core Features
- **src/main.ts**: Server entry point, CLI argument parsing, HTTP request handling, config auto-reload
- **src/router.ts**: URL to file path resolution, security checks
- **src/context.ts**: PageContext builder
- **src/injection-typed.ts**: Type-safe dependency injection, global Page function
- **src/cookies.ts**: Cookie management, Set-Cookie header support
- **src/static.ts**: Static file service, caching support

### File Manager
- **src/filemanager/mod.ts**: File manager main module
- **src/filemanager/handlers.ts**: Request handling (list, upload, delete, etc.)
- **src/filemanager/auth.ts**: Authentication module (password hash, session, CSRF)
- **src/filemanager/security.ts**: Security checks (path traversal, file type restrictions)
- **src/filemanager/config.ts**: Config validation and defaults

### Database Integration
- **src/mysql/factory.ts**: MySQL connection factory
- **src/redis/factory.ts**: Redis connection factory
- **src/ldap/client.ts**: LDAP client

### Utility Scripts
- **kill-port.sh**: Port cleanup script (Bash)

### Type Definitions
- **types.d.ts**: Global type declarations (CRITICAL - must read)

## MySQL Schema-first API

TSP provides a brand new **Schema-first** style MySQL client API, with all methods using Zod schema for type validation and inference.

### Core Features

- **Type Safe**: All return values validated through Zod schema
- **Auto Type Inference**: No manual return type definition needed
- **Singleton Connection**: No connection pool needed in TSX mode
- **Transaction Support**: Auto commit/rollback

### Core Methods

| Method | Description | SQL Requirement |
|--------|-------------|-----------------|
| `query(schema, sql, params?)` | Multi-row query | - |
| `queryOne(schema, sql, params?)` | Strict single row (0 or multiple rows throws error) | - |
| `queryMaybe(schema, sql, params?)` | Optional single row (0 rows returns null) | - |
| `scalar(schema, sql, params?)` | Single value query | Must use `AS value` alias |
| `execute(schema, sql, params?)` | Write operation | INSERT/UPDATE/DELETE |
| `tx(callback)` | Transaction operation | Auto commit/rollback |
| `queryPage(schema, sql, params, pageArgs)` | Paginated query | Must include `total` field |

### Usage Examples

#### 1. Basic Query

```tsx
export default Page(async function(ctx, { createMySQL, z, response }) {
  // Note: createMySQL now requires two parameters
  const db = await createMySQL({
    host: '127.0.0.1',
    port: 3306,
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  }, z);

  // Define Schema
  const UserSchema = z.object({
    id: z.number(),
    username: z.string(),
    email: z.string()
  });

  // Multi-row query
  const users = await db.query(UserSchema, 'SELECT * FROM users WHERE age > ?', [18]);

  // Single row query (must return exactly one row)
  const user = await db.queryOne(UserSchema, 'SELECT * FROM users WHERE id = ?', [userId]);

  // Optional single row query (returns 0 or 1 row)
  const maybeUser = await db.queryMaybe(UserSchema, 'SELECT * FROM users WHERE email = ?', [email]);

  // Single value query (SQL must use AS value alias)
  const count = await db.scalar(z.number(), 'SELECT COUNT(*) as value FROM users');

  return response.json({ users, user, maybeUser, count });
});
```

#### 2. Write Operations

```tsx
export default Page(async function(ctx, { createMySQL, z, response }) {
  const db = await createMySQL({...}, z);

  const ResultSchema = z.object({
    affectedRows: z.number(),
    insertId: z.number()
  });

  // INSERT operation
  const result = await db.execute(ResultSchema,
    'INSERT INTO users (username, email) VALUES (?, ?)',
    [username, email]
  );

  // UPDATE operation
  await db.execute(ResultSchema,
    'UPDATE users SET email = ? WHERE id = ?',
    [newEmail, userId]
  );

  // DELETE operation
  await db.execute(ResultSchema,
    'DELETE FROM users WHERE id = ?',
    [userId]
  );

  return response.json({
    success: true,
    insertId: result.insertId
  });
});
```

#### 3. Transaction Operations

```tsx
export default Page(async function(ctx, { createMySQL, z, response }) {
  const db = await createMySQL({...}, z);

  const UserSchema = z.object({
    id: z.number(),
    balance: z.number()
  });

  const ResultSchema = z.object({
    affectedRows: z.number(),
    insertId: z.number()
  });

  try {
    const result = await db.tx(async (tx) => {
      // Lock user record
      const sender = await tx.queryOne(UserSchema,
        'SELECT * FROM users WHERE id = ? FOR UPDATE',
        [senderId]
      );

      if (sender.balance < amount) {
        throw new Error('Insufficient balance');
      }

      // Transfer operation
      await tx.execute(ResultSchema,
        'UPDATE users SET balance = balance - ? WHERE id = ?',
        [amount, senderId]
      );

      await tx.execute(ResultSchema,
        'UPDATE users SET balance = balance + ? WHERE id = ?',
        [amount, receiverId]
      );

      return { success: true, newBalance: sender.balance - amount };
    });

    return response.json(result);
  } catch (error) {
    return response.json({
      success: false,
      error: (error as Error).message
    }, 500);
  }
});
```

#### 4. Paginated Query

```tsx
export default Page(async function(ctx, { createMySQL, z, response }) {
  const db = await createMySQL({...}, z);

  const UserSchema = z.object({
    id: z.number(),
    username: z.string(),
    email: z.string(),
    total: z.number()  // Must include total field
  });

  const result = await db.queryPage(
    UserSchema,
    `SELECT *, COUNT(*) OVER() as total FROM users WHERE age > ? LIMIT ? OFFSET ?`,
    [18, 10, 20],  // params: [age, pageSize, offset]
    { page: 3, pageSize: 10 }
  );

  // result.items: User[]
  // result.total: 150
  // result.page: 3
  // result.pageSize: 10
  // result.totalPages: 15

  return response.json(result);
});
```

### Important Notes

#### 1. MySQL LIMIT/OFFSET Placeholder Limitation

**Problem**: MySQL does not support parameterized placeholders in `LIMIT` and `OFFSET` clauses.

**Solution**: TSP's `queryPage` method automatically detects and replaces placeholders with actual values.

```tsx
// Correct - queryPage handles automatically
await db.queryPage(
  UserSchema,
  'SELECT * FROM users LIMIT ? OFFSET ?',  // Placeholders allowed
  [],
  { page: 2, pageSize: 5 }
);
// Internally converted to: 'SELECT * FROM users LIMIT 5 OFFSET 5'
```

#### 2. DECIMAL Type Handling

MySQL's `DECIMAL` type returns strings and requires special handling:

```tsx
// Option 1: Use union type
const BalanceSchema = z.object({
  id: z.number(),
  balance: z.union([z.number(), z.string()]),  // Accept number or string
});

// Option 2: Convert to number
const balance = typeof row.balance === 'string'
  ? parseFloat(row.balance)
  : row.balance;
```

#### 3. DATE/DATETIME Types

```tsx
const UserSchema = z.object({
  created_at: z.string().optional(),  // Handle as string
  // or use z.any() to accept any type
});
```

#### 4. Transaction Usage Recommendations

- Use `FOR UPDATE` to lock queried rows to prevent concurrent modifications
- Any error thrown in transaction callback triggers auto rollback
- Must use `tx` object methods in transaction, not external `db` object

### Error Handling Strategy

| Method | Error Condition | Handling |
|--------|-----------------|----------|
| `query` | Schema validation failure | Throws `ZodError` |
| `queryOne` | 0 rows | Throws `Error: Expected 1 row, got 0` |
| `queryOne` | >1 rows | Throws `Error: Expected 1 row, got N` |
| `queryMaybe` | >1 rows | Throws `Error: Expected 0 or 1 row, got N` |
| `scalar` | No `value` field | Throws `Error: Missing 'value' field` |
| `execute` | SQL error | Throws original MySQL error |
| `tx` | Callback throws error | Auto rollback, re-throws error |

## ExcelJS Integration

TSP provides ExcelJS integration via factory function that returns ExcelJS library instance, allowing direct use of its full API.

### Basic Usage

#### 1. Write Excel File

```tsx
export default Page(async function(ctx, { createExcelJS, response }) {
  const ExcelJS = await createExcelJS();

  // Create workbook
  const workbook = new ExcelJS.Workbook();
  const worksheet = workbook.addWorksheet("Users");

  // Define columns
  worksheet.columns = [
    { header: "Name", key: "name", width: 20 },
    { header: "Age", key: "age", width: 10 },
    { header: "Email", key: "email", width: 30 },
  ];

  // Add data
  worksheet.addRow({ name: "Alice", age: 25, email: "alice@example.com" });
  worksheet.addRow({ name: "Bob", age: 30, email: "bob@example.com" });

  // Save file
  await workbook.xlsx.writeFile("./output.xlsx");

  return response.json({ success: true });
});
```

#### 2. Read Excel File

```tsx
export default Page(async function(ctx, { createExcelJS, z, response }) {
  const ExcelJS = await createExcelJS();

  // Read file
  const workbook = new ExcelJS.Workbook();
  await workbook.xlsx.readFile("./input.xlsx");

  // Get worksheet
  const worksheet = workbook.getWorksheet("Users");

  // Read data
  const users: any[] = [];
  worksheet?.eachRow((row, rowNumber) => {
    if (rowNumber > 1) { // Skip header
      users.push({
        name: row.getCell(1).value,
        age: row.getCell(2).value,
        email: row.getCell(3).value,
      });
    }
  });

  return response.json({ users });
});
```

#### 3. Read with Zod Validation

```tsx
export default Page(async function(ctx, { createExcelJS, z, response }) {
  const ExcelJS = await createExcelJS();
  const UserSchema = z.object({
    name: z.string(),
    age: z.number(),
    email: z.string().email(),
  });

  const workbook = new ExcelJS.Workbook();
  await workbook.xlsx.readFile("./data.xlsx");
  const worksheet = workbook.getWorksheet("Sheet1");

  const users: any[] = [];
  worksheet?.eachRow((row, rowNumber) => {
    if (rowNumber > 1) {
      users.push({
        name: row.getCell(1).value,
        age: row.getCell(2).value,
        email: row.getCell(3).value,
      });
    }
  });

  // Validate with Zod
  const validated = UserSchema.parse(users[0]);

  return response.json({ validated });
});
```

#### 4. Set Styles

```tsx
export default Page(async function(ctx, { createExcelJS, response }) {
  const ExcelJS = await createExcelJS();

  const workbook = new ExcelJS.Workbook();
  const worksheet = workbook.addWorksheet("Products");

  worksheet.columns = [
    { header: "Product", key: "product", width: 20 },
    { header: "Price", key: "price", width: 10 },
  ];

  worksheet.addRow({ product: "Apple", price: 1.5 });
  worksheet.addRow({ product: "Banana", price: 0.8 });

  // Set header styles
  const headerRow = worksheet.getRow(1);
  headerRow.font = { bold: true, size: 14, color: { argb: "FFFFFFFF" } };
  headerRow.fill = {
    type: "pattern",
    pattern: "solid",
    fgColor: { argb: "FF4472C4" },
  };
  headerRow.alignment = { horizontal: "center", vertical: "middle" };

  await workbook.xlsx.writeFile("./products.xlsx");

  return response.json({ success: true });
});
```

#### 5. Template Filling

```tsx
export default Page(async function(ctx, { createExcelJS, response }) {
  const ExcelJS = await createExcelJS();

  // Read template
  const workbook = new ExcelJS.Workbook();
  await workbook.xlsx.readFile("./template.xlsx");

  // Replace placeholders
  workbook.eachSheet((worksheet) => {
    worksheet.eachRow((row) => {
      row.eachCell((cell) => {
        if (typeof cell.value === "string") {
          let value = cell.value;
          value = value.replace(/\{\{name\}\}/g, "Alice");
          value = value.replace(/\{\{age\}\}/g, "25");
          cell.value = value;
        }
      });
    });
  });

  await workbook.xlsx.writeFile("./output.xlsx");

  return response.json({ success: true });
});
```

Template placeholders format: `{{key}}`

```
Name: {{name}}
Age: {{age}}
Email: {{email}}
```

#### 6. Export as Buffer/Base64

```tsx
export default Page(async function(ctx, { createExcelJS, response }) {
  const ExcelJS = await createExcelJS();

  const workbook = new ExcelJS.Workbook();
  const worksheet = workbook.addWorksheet("Report");
  worksheet.addRow(["Name", "Value"]);
  worksheet.addRow(["Item 1", 100]);

  // Export as Buffer
  const buffer = await workbook.xlsx.writeBuffer();

  // Or convert to base64
  const base64 = btoa(String.fromCharCode(...new Uint8Array(buffer)));

  // Return file download
  return response.file(buffer, "report.xlsx", {
    "Content-Type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  });
});
```

#### 7. Cell Operations

```tsx
export default Page(async function(ctx, { createExcelJS, response }) {
  const ExcelJS = await createExcelJS();

  const workbook = new ExcelJS.Workbook();
  const worksheet = workbook.addWorksheet("Sheet1");

  // Set cell values
  worksheet.getCell("A1").value = "Hello ExcelJS!";
  worksheet.getCell("B1").value = 123;
  worksheet.getCell("C1").value = true;

  // Set styles
  worksheet.getCell("A1").font = { bold: true, size: 16, color: { argb: "FFFF0000" } };
  worksheet.getCell("A1").fill = {
    type: "pattern",
    pattern: "solid",
    fgColor: { argb: "FFCCCCCC" },
  };

  await workbook.xlsx.writeFile("./output.xlsx");

  return response.json({ success: true });
});
```

#### 8. Worksheet Operations

```tsx
export default Page(async function(ctx, { createExcelJS, response }) {
  const ExcelJS = await createExcelJS();

  // Create workbook
  const workbook = new ExcelJS.Workbook();
  workbook.addWorksheet("Sheet1");
  workbook.addWorksheet("Sheet2");
  await workbook.xlsx.writeFile("./workbook.xlsx");

  // Read and manipulate
  const wb = new ExcelJS.Workbook();
  await wb.xlsx.readFile("./workbook.xlsx");

  // Rename worksheet
  const sheet = wb.getWorksheet("Sheet2");
  if (sheet) sheet.name = "RenamedSheet";

  // Delete worksheet
  const sheetToDelete = wb.getWorksheet("RenamedSheet");
  if (sheetToDelete) wb.removeWorksheet(sheetToDelete.id);

  await wb.xlsx.writeFile("./workbook.xlsx");

  // Get worksheet info
  const sheets = workbook.worksheets.map((ws) => ({
    name: ws.name,
    rowCount: ws.rowCount,
  }));

  return response.json({ sheets });
});
```

### Color Format

ExcelJS uses ARGB format for colors:

```
FF000000  // Black (Alpha=FF, R=00, G=00, B=00)
FFFF0000  // Red
FF00FF00  // Green
FF0000FF  // Blue
FFFFFFFF  // White
```

### More Information

For complete API, refer to [ExcelJS Official Documentation](https://github.com/exceljs/exceljs).

## Configuration

### Configuration Method Priority
1. **CLI arguments** (highest): `--root ./www --port 9000 --dev`
2. **Config file**: `config.json` or `config.jsonc` (auto-discovered)
3. **Defaults** (lowest): root="./www", port=9000, dev=false

### Configuration File Example (config.jsonc)

```jsonc
{
  // Basic configuration
  "root": "./www",
  "port": 9000,
  "dev": false,

  // Hot reload configuration (optional, default true)
  // Set to false to disable hot reload, file changes won't auto-refresh cache
  // Dev mode: first access generates cache, subsequent accesses don't refresh cache
  // Production mode: after precompilation, cache won't refresh
  "hotReload": true,

  // Access log
  "accessLogPath": ".logs/access.log",

  // Session configuration
  "session": {
    "secure": false,  // Set to false for HTTP, true for HTTPS
    "maxAge": 86400,  // Session validity in seconds, default 1 day
    "sameSite": "Strict"  // SameSite attribute: Strict, Lax, or None
  },

  // Logger configuration
  "logger": {
    "level": "INFO",
    "file": ".logs/app.log",
    "colorize": true,
    "format": "text"
  },

  // File manager configuration
  "fileManager": {
    "enabled": true,
    "path": "/__filemanager",
    "password": "your_password_here",  // At least 6 characters
    "allowOutsideRoot": false,
    "deniedPaths": [".git", ".deno", "node_modules", ".cache"],
    "maxUploadSize": 104857600
    // Note: Cookie Secure attribute is controlled by session.secure
  }
}
```

### Configuration Auto-Reload

**Behavior after modifying config file**:
- Session secure config, file manager password, logger config, hot reload config, etc.: Take effect automatically, no restart needed
- Port, root directory: Server restart required

**Prompt message**:
```
File manager enabled
  Access path: /__filemanager
  Config file changes will auto-reload
```

### Path Resolution

- Root directory path is resolved to absolute path
- Cache directory is created relative to current working directory
- Ensures cache consistency when running from different directories

## Test Structure

### Test Directory Structure

```
tests/
├── unit/                    # Unit tests (can import src/)
│   ├── filemanager_test.ts   # File manager tests (35+ tests)
│   └── ...
├── test_www/                # E2E test pages (cannot import src/)
│   ├── index.tsp             # .tsp route file
│   ├── api.tsp
│   └── ...
├── run_e2e_tests.ts        # E2E test runner
├── run_unit_tests.ts        # Unit test runner
├── test_kill_port_scripts.ts # Port cleanup script tests
└── ...
```

### E2E Test Content

E2E tests include:
- Basic HTTP functionality
- TSP routing (`.tsp` file access)
- API tests
- Dependency injection
- Error handling
- Security (path traversal protection, `.ts`/`.tsx` direct access blocked)
- **Hot reload with nested dependencies** (2+ level deep)
- **File manager login tests**
- **Configuration auto-reload tests**
- **Concurrent compilation tests**

### Running Tests

```bash
# All tests
./tsp.sh test

# Unit tests only
./tsp.sh test:unit

# E2E tests
./tsp.sh test:e2e
```

## Compilation & Distribution

### Compilation Notes

When using `deno-tsp compile`:
- Binary can be run from any directory
- Cache directory is created relative to working directory
- Uses custom `deno-tsp` for compilation, supports `.tsp` files
- Test binary uses port 9001 to avoid conflict with dev server (9000)

### Distribution Package Structure

```bash
# Build debug package
./tsp.sh build:tspserver

# Build release package
./tsp.sh build:tspserver:rel

# Build output (organized by OS):
dist/
├── debug/
│   └── windows-x64/          # Windows x64
│       ├── tspserver.exe     # Compiled binary
│       ├── config.jsonc      # Configuration file
│       ├── types.d.ts       # Type definitions
│       └── CLAUDE_GUIDE_README.md
├── release/
│   └── windows-x64/          # Windows x ├── tspserver64
│      .exe
│       ├── config.jsonc
│       ├── types.d.ts
│       └── CLAUDE_GUIDE_README.md
```

## Port Management

### Port Cleanup Tool

**Problem**: Server startup shows "Port in use"

**Solution**: Use port cleanup script to automatically free the port

```bash
# Cleanup occupied port from config
bash ./kill-port.sh
```

**Features**:
- Automatically reads port number from config file
- Detects and kills processes using that port
- Supports multiple config files (config.jsonc, config.json)
- Uses default port 9000 if no config file found

**Integrated into startup process**:

Run cleanup script before starting server:
```bash
# One-click startup (cleanup port + start server)
bash ./kill-port.sh && ./tsp.sh start

# Or for development with hot reload
bash ./kill-port.sh && ./tsp.sh dev
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

### Configuration Auto-Reload Implementation

**Core Mechanism**:
1. Record config file path and modification time at startup
2. Check if file is modified on each request
3. If modification detected, immediately reload config
4. Apply new config to subsequent requests

**Code Locations**:
- Global variables: `src/main.ts:238-253`
- Reload function: `src/main.ts:257-322`
- Integration point: `src/main.ts:387-389`

**Performance Impact**:
- Config check overhead: ~0.01ms per request (one file stat syscall)
- Config reload overhead: ~1.5ms (only when file is modified)
- Overall impact: Negligible (config changes are infrequent)

## File Manager

### Features

- Web file management (browse, upload, download, delete, rename)
- Password protection (PBKDF2 hash, 100,000 iterations)
- Security features (path traversal protection, file type restrictions)
- File upload (multiple files, progress display, size limit)
- File management (create directory, move, extract/compress)
- Access control (IP whitelist, blacklist)

### Access Paths

```
/__filemanager/          # File manager main page
/__filemanager/api/*     # API endpoints
  /login               # Login
  /list                # List files
  /upload              # Upload files
  /delete              # Delete files
  /rename              # Rename
  /mkdir               # Create directory
  /download            # Download files
```

### Configuration Example

```jsonc
{
  "fileManager": {
    "enabled": true,
    "path": "/__filemanager",
    "password": "secure_password_123",  // At least 6 characters
    "allowOutsideRoot": false,
    "deniedPaths": [".git", ".deno", "node_modules", ".cache"],
    "maxUploadSize": 104857600,           // 100MB
    "allowDelete": true,
    "allowRename": true,
    "allowMkdir": true,
    "allowMove": true,
    "allowExtract": true,
    "allowCompress": true,
    "secure": false  // Set to false for HTTP, true for HTTPS
  }
}
```

### Login API

```bash
curl -X POST http://localhost:9000/__filemanager/api/login \
  -H "Content-Type: application/json" \
  -d '{"password":"your_password"}'
```

**Response**:
```json
{
  "success": true,
  "data": {
    "csrfToken": "..."
  }
}
```

## Troubleshooting

### Issue 1: Port In Use

**Error message**:
```
Fatal error: AddrInUse: Only one usage of each socket address (protocol/network address/port) is normally permitted.
```

**Solution**:

Run port cleanup script:
```bash
# Cleanup port
bash ./kill-port.sh
```

Or manually find and terminate:
```bash
# Windows
netstat -ano | findstr :9000
taskkill /PID <pid> /F

# Linux/macOS
lsof -ti :9000 | xargs kill -9
```

### Issue 2: Cannot Login After Password Change

**Cause**: In older versions, server restart was required after config file changes.

**Solution (fixed)**:
- Modify password in config file
- Refresh browser, login with new password
- No server restart needed!

If still cannot login:
1. Ensure password is at least 6 characters
2. Check config file format is correct
3. Check server logs to confirm config loaded

### Issue 3: TSP File Not Taking Effect

**Cause**: Possibly a cache issue

**Solution**:
- Dev mode: Auto hot reload on file changes
- Production mode: Delete `.cache/tsp/` directory, restart server

### Issue 4: Compiled Binary Cannot Find Config File

**Cause**: When binary runs from different directory, config file path may be relative

**Solution**:
- Ensure config file and binary are in same directory
- Or use absolute path to specify config file:
  ```bash
  ./tspserver --config /path/to/config.jsonc
  ```

### Issue 5: File Manager 404 Error

**Checklist**:
1. Is `fileManager.enabled` set to `true` in config?
2. Is `password` set in config? (at least 6 characters)
3. Does server startup log show "File manager enabled"?

**Debug method**:
```bash
# Start dev server, check logs
./tsp.sh dev

# Check log output
# Should see:
# Config file found: config.jsonc
# File manager enabled
#   Access path: /__filemanager
```

## Changelog

See [CHANGELOG.md](./docs/changelog.md) for version history.
