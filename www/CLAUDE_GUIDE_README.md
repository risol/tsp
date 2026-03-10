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

## Logging (logger)

```tsx
export default Page(async function(ctx, { logger }) {
  logger.debug('Debug info:', { key: 'value' });
  logger.info('Request:', ctx.method, ctx.url.pathname);
  logger.warn('Warning:', 'deprecated feature');
  logger.error('Error:', new Error('failed'));

  return <div>Check logs for output</div>;
});
```

**Methods:** `debug()`, `info()`, `warn()`, `error()`

## Dependency Injection

Available in second parameter (auto-typed):

```tsx
export default Page(async function(ctx, { logger, session, response, createZod, crypto, createBcryptjs, createMySQL, createRedis, createExcelJS }) {
  // logger - structured logging (debug, info, warn, error)
  // session - cookie session management
  // response - helper methods (json, redirect, file)
  // createZod - Zod validation library (factory, requires await)
  // crypto - encryption utilities (native Deno API)
  // createBcryptjs - password hashing (factory, requires await)
  // createMySQL - MySQL database (factory, requires await)
  // createRedis - Redis cache (factory, requires await)
  // createExcelJS - Excel file handling (factory, requires await)
});
```

**Common dependencies:** `logger`, `session`, `response`, `createZod`, `crypto`, `createBcryptjs`, `createMySQL`, `createRedis`, `createLdap`, `createExcelJS`

## Encryption

### Crypto (Native Deno API)

```tsx
export default Page(async function(ctx, { crypto, response }) {
  // Generate random values
  const iv = crypto.getRandomValues(12);

  // Hash data
  const hash = await crypto.digest('SHA-256', 'Hello');

  // Generate key
  const key = await crypto.generateKey('AES-GCM', 256);

  // Encrypt/Decrypt
  const data = new TextEncoder().encode('secret');
  const encrypted = await crypto.encrypt(data, key, iv);
  const decrypted = await crypto.decrypt(encrypted, key, iv);

  return response.json({ decrypted: new TextDecoder().decode(decrypted) });
});
```

## Validation (Zod)

```tsx
export default Page(async function(ctx, { createZod, body, formatZodError, response }) {
  const z = await createZod();

  // Define schema
  const UserSchema = z.object({
    name: z.string().min(2).max(50),
    email: z.string().email(),
    age: z.coerce.number().min(1).max(150).optional()
  });

  // Validate request body
  try {
    const userData = body(UserSchema);
    return response.json({ success: true, data: userData });
  } catch (error) {
    if (error instanceof z.ZodError) {
      return response.json(formatZodError(error), 400);
    }
    throw error;
  }
});
```

**Schema methods:** `string()`, `number()`, `boolean()`, `object()`, `array()`, `email()`, `min()`, `max()`, `optional()`, `nullable()`, `default()`

## Nanoid (Unique IDs)

```tsx
export default Page(async function(ctx, { createNanoid, response }) {
  const nanoid = await createNanoid();

  const id = nanoid(); // "V1StGXR8_Z5jdHi6B-myT"
  const shortId = nanoid(10); // Custom length

  return response.json({ id, shortId });
});
```

### Bcryptjs (Password Hashing)

```tsx
export default Page(async function(ctx, { createBcryptjs, response }) {
  const bcrypt = await createBcryptjs({ saltRounds: 10 });

  // Hash password
  const hash = bcrypt.hash('myPassword');

  // Verify password
  const valid = bcrypt.compare('myPassword', hash);

  return response.json({ hash, valid });
});
```

## Database (MySQL)

```tsx
export default Page(async function(ctx, { createMySQL, createZod, response }) {
  const z = await createZod();
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

## Cache (Redis)

```tsx
export default Page(async function(ctx, { createRedis, response }) {
  const redis = await createRedis({
    host: '127.0.0.1',
    port: 6379,
    password: 'pass',
    database: 0
  });

  // String
  await redis.set('key', 'value', 3600); // TTL in seconds
  const value = await redis.get('key');

  // List
  await redis.lpush('list', 'item1');
  const list = await redis.lrange('list', 0, -1);

  // Hash
  await redis.hset('hash', 'field', 'value');
  const hash = await redis.hgetall('hash');

  return response.json({ value, list, hash });
});
```

**Methods:** `set`, `get`, `del`, `exists`, `expire`, `ttl`, `lpush`, `rpush`, `lpop`, `rpop`, `lrange`, `sadd`, `smembers`, `sismember`, `hset`, `hget`, `hgetall`, `hdel`

## LDAP (Directory)

```tsx
export default Page(async function(ctx, { createLdap, response }) {
  const ldap = await createLdap({
    url: 'ldap://ldap.example.com:389',
    bindDN: 'cn=admin,dc=example,dc=com',
    bindCredentials: 'password'
  });

  // Search
  const users = await ldap.search('ou=users,dc=example,dc=com', {
    scope: 'sub',
    filter: '(objectClass=person)',
    attributes: ['cn', 'mail']
  });

  // Authenticate
  try {
    await ldap.bind(`uid=user,ou=users,dc=example,dc=com`, 'userpass');
    return response.json({ success: true });
  } catch {
    return response.json({ success: false }, 401);
  }

  await ldap.close();
});
```

**Methods:** `bind`, `anonymousBind`, `search`, `add`, `modify`, `del`, `modifyDN`, `compare`, `close`

## Excel Files

```tsx
export default Page(async function(ctx, { createExcelJS, response }) {
  const ExcelJS = await createExcelJS();

  const workbook = new ExcelJS.Workbook();
  const sheet = workbook.addWorksheet('Users');

  sheet.columns = [
    { header: 'ID', key: 'id' },
    { header: 'Name', key: 'name' }
  ];

  sheet.addRows([
    { id: 1, name: 'John' },
    { id: 2, name: 'Jane' }
  ]);

  await workbook.xlsx.writeFile('./users.xlsx');
  return response.json({ success: true });
});
```

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
