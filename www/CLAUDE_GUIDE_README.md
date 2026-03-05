# TSP Server Guide

> Quick reference for building TSP applications. TSP runs `.tsp` files like PHP - no build step needed.

## Server-Side Rendering (SSR)

TSP is a **server-side rendering** framework, NOT a SPA (Single Page Application). Each request executes the `.tsp` file on the server and returns rendered HTML.

**Key points:**
- Each URL maps to a `.tsp` file - server executes it and returns HTML
- No client-side routing - traditional multi-page app pattern
- Like PHP, but with TypeScript and modern tooling
- Great for SEO - search engines see full HTML

## File Structure

```
www/
├── index.tsp          # Route: /
├── about.tsp          # Route: /about
├── api/
│   └── users.tsp     # Route: /api/users
├── components/       # .tsx files here (not accessible via URL)
│   └── Header.tsx
└── types.d.ts       # Type definitions (auto-copied in release)
```

**Rules:**
- `.tsp` files = routes (accessible via URL)
- `.tsx`/`.ts` files = components/modules (importable, not accessible)
- Files starting with `__` = private (accessible by other files, not via URL)
- `types.d.ts` = global types (available automatically, updated in release builds)

## Basic Page

```tsx
export default Page(async function(ctx, { logger, response }) {
  // ctx: PageContext (method, url, query, body, cookies, files)
  // { logger, response }: injected dependencies

  return (
    <html>
      <head><title>My Page</title></head>
      <body>
        <h1>Hello TSP</h1>
      </body>
    </html>
  );
});
```

## PageContext (ctx)

| Property | Description |
|----------|-------------|
| `ctx.method` | GET, POST, PUT, DELETE |
| `ctx.url` | URL object (pathname, searchParams) |
| `ctx.query` | Query params as object |
| `ctx.body` | Request body (JSON parsed) |
| `ctx.headers` | Request headers |
| `ctx.cookies` | Cookies object |
| `ctx.files` | Uploaded files |
| `ctx.root` | www directory path |
| `ctx.file` | Current file path |

## Return Types

```tsx
// 1. JSX (rendered as HTML)
return <div>Hello</div>;

// 2. Plain text
return 'Hello';

// 3. Redirect
return response.redirect('/login');

// 4. JSON
return response.json({ success: true });

// 5. Custom Response
return new Response('custom', { status: 200 });
```

## Dependency Injection

Available in second parameter (auto-typed):

```tsx
export default Page(async function(ctx, { logger, session, response, z }) {
  // logger - logging
  // session - cookie session management
  // response - helper methods (json, redirect, file)
  // z - Zod for validation
});
```

**Common dependencies:** `logger`, `session`, `response`, `z`, `createMySQL`, `createRedis`

## Database (MySQL)

```tsx
export default Page(async function(ctx, { createMySQL, z, response }) {
  const db = await createMySQL({
    host: '127.0.0.1',
    port: 3306,
    user: 'root',
    password: 'pass',
    database: 'mydb'
  }, z);

  // Query with Zod schema
  const UserSchema = z.object({ id: z.number(), name: z.string() });
  const users = await db.query(UserSchema, 'SELECT * FROM users');

  return response.json({ users });
});
```

**Query methods:** `query()`, `queryOne()`, `queryMaybe()`, `scalar()`, `execute()`, `tx()`, `queryPage()`

## Configuration

Create `config.jsonc` in project root:

```jsonc
{
  "root": "./www",
  "port": 9000,
  "dev": true,
  "fileManager": {
    "enabled": true,
    "password": "your_password"
  }
}
```

## Important Rules

1. **NEVER import from `src/`** - Use global types from `types.d.ts`
2. **Use `.tsp` suffix** for route files
3. **Components in `./components/`** - import with `./components/Header.tsx`
4. **No parent directory imports** - Use `./` not `../`
5. **Error pages** - 500 errors show stack trace in dev mode only

## CLI Commands

```bash
# Start server
./tspserver --root ./www --port 9000

# With config file
./tspserver --config config.jsonc
```
