# TSP

![Banner](./docs/images/banner.png)

A TypeScript server that executes `.tsp` files directly like PHP, designed for AI-driven development.

## Features

- **Simple to Use** - Execute `.tsp` files directly like PHP
- **Smart Caching** - File modification time-based module caching with excellent performance
- **Hot Reload** - Support for hot reloading with nested dependencies at any depth
- **Secure** - Comprehensive path checking and permission control
- **Full-featured** - Query parameters, POST data, Cookies, redirects, and more
- **Component-based** - Using TSX + React, supporting modern frontend component development
- **Type-safe** - Complete TypeScript type support, Schema-first database API
- **File Manager** - Built-in web file manager with password protection
- **Config Auto-reload** - Configuration changes take effect automatically without restart
- **Static Files** - Support for HTML, CSS, JS, images, and other static files
- **Port Management** - Automatically detect and clean up processes occupying ports
- **Database Integration** - Schema-first MySQL/Redis/LDAP support, type-safe database queries

## Why TSP for AI Code Generation?

TSP is designed specifically for AI-driven development with unique advantages:

### 1. Minimal Behavioral Space

Unlike full-stack frameworks with endless patterns (MVC, hooks, contexts, providers), TSP has **only one way** to write server-side code:

```tsx
export default Page(async function(ctx, deps) {
  // Your logic here
  return <html>...</html>;
});
```

This constrains AI to a tiny, predictable pattern—**no choice paralysis**, **no framework-hopping**.

### 2. Built-in Dependency Injection

All dependencies must be obtained through **function factories**:

```tsx
export default Page(async function(ctx, { createMySQL, createRedis, z, response }) {
  const db = await createMySQL(config, z);
  const redis = await createRedis(config);
  // ...
});
```

This ensures:
- **Consistency** - AI always uses the same patterns
- **Type safety** - Schemas are enforced at the factory level
- **No ad-hoc imports** - Can't bypass the system

### 3. Zero Configuration

AI can generate working code without:
- No `package.json` to manage
- No `tsconfig.json` to configure
- No router files to wire up
- No environment variables to set

Just create a `.tsp` file and it works.

### 4. Type-Safe Dependency Injection

All types are injected through the Page function—no imports needed:

```tsx
// Types come from dependency injection:
export default Page(async function(ctx, { response, session, z }) {
  // ctx: PageContext (method, url, query, body, cookies, files)
  // response: ResponseBuilder (json, html, redirect, file, error)
  // session: SessionManager
  // z: Zod (for schema validation)
});
```

This ensures **full type inference** without any imports.

### 5. Schema-First Database API

Database operations require schemas upfront:

```tsx
const UserSchema = z.object({
  id: z.number(),
  name: z.string(),
  email: z.string().email()
});

const users = await db.query(UserSchema, 'SELECT * FROM users');
```

AI can't produce unsafe queries—**Zod validates everything**.

### 6. Instant Hot Reload

Save a `.tsp` file and **instantly see changes**—no rebuild, no restart:

```bash
sh ./tsp.sh dev
# Edit any .tsp file, refresh browser to see changes
```

This enables **rapid feedback loops**:
- AI generates code → save → see result → iterate
- Fix bugs in seconds, not minutes
- Test database queries live without restart

Works with **nested dependencies** too—edit a component, all pages using it update instantly.

## Quick Start

### Option 1: Download Pre-built Release (Recommended)

Download the latest release from [GitHub Releases](https://github.com/risol/tsp/releases):

```bash
# Download and extract (replace with your platform)
curl -L https://github.com/risol/tsp/releases/latest/download/tsp-linux-x64.tar.gz -o tsp.tar.gz
tar -xzf tsp.tar.gz
cd tsp-linux-x64

# Start the server
./tspserver --root ./www --port 9000
```

### Option 2: Build from Source

If you want to build from source, you need Rust and C build tools:

```bash
# Clone the repository (with submodules)
git clone --recursive https://github.com/risol/tsp.git
cd tsp

# Or if already cloned, init submodules
git submodule update --init --recursive

# Build deno-tsp runtime
sh ./tsp.sh build:denort
sh ./tsp.sh build:deno

# Start development server
sh ./tsp.sh dev

# Start production mode
sh ./tsp.sh start
```

### 3. Access the Application

Open browser and visit `http://localhost:9000`

## Example

A simple `.tsp` file looks like this:

```tsx
// www/hello.tsp - Access at /hello
export default Page(async function(ctx, { response }) {
  return (
    <html>
      <head>
        <title>Hello TSP!</title>
        <style>{`
          body { font-family: sans-serif; max-width: 600px; margin: 50px auto; padding: 20px; }
          h1 { color: #3178c6; }
        `}</style>
      </head>
      <body>
        <h1>Hello, TSP!</h1>
        <p>URL: {ctx.url.pathname}</p>
        <p>Method: {ctx.method}</p>
        <p>Time: {new Date().toLocaleString()}</p>
      </body>
    </html>
  );
});
```

Key features:
- **No imports needed** - `Page`, `ctx`, `response` are all global
- **JSX support** - Write HTML directly in TypeScript
- **Type-safe** - Full TypeScript support with auto-completion

## Build from Source (Advanced)

Most users should download pre-built releases instead. Building from source requires:

- Rust toolchain
- C compiler (gcc/clang)
- For Linux static builds: sysroot (auto-downloaded)

```bash
# Build release binary for current platform
sh ./tsp.sh build:tspserver

# Build debug binary
sh ./tsp.sh build:tspserver:dev

# Build release binary (alias)
sh ./tsp.sh build:tspserver:rel
```

Build output is in the `dist/` directory.

## Docker Test Services

The project includes Docker Compose configuration for quickly starting MySQL and Redis services needed for testing.

See [DOCKER_SERVICES.md](docker/DOCKER_SERVICES.md)

## Documentation

- [Getting Started](./docs/getting-started.md) - Quick start guide
- [Development Guide](./docs/development.md) - Development setup
- [Configuration](./docs/configuration.md) - Server configuration
- [Features](./docs/features/readme.md) - Feature documentation
- [Testing](./docs/testing/readme.md) - Testing guide
- [Changelog](./docs/changelog.md) - Version change log
