# AppDeps Usage Guide

## Overview

`AppDeps` is TSP's dependency injection system, providing type-safe dependency management. Through `AppDeps`, you can:

- Inject built-in dependencies like Session, Cookies into page functions
- Create custom dependencies (database, cache, API client, etc.)
- Enjoy complete TypeScript type hints
- Use lazy loading mechanism for performance optimization

## Quick Start

### 1. Declare Dependency Types

Declare dependencies in `types.d.ts`:

```typescript
// types.d.ts
declare global {
  interface AppDeps extends Record<string, unknown> {
    // Built-in dependencies (required)
    session: import("./src/session.ts").SessionManager;
    cookies: import("./src/cookies.ts").CookieManager;
    testFunc: () => string;

    // Optional dependencies
    db?: {
      query: (sql: string) => Promise<unknown[]>;
    };
    logger?: typeof console.log;
  }
}

export {};
```

### 2. Register Dependencies

Register dependencies in `main.ts`:

```typescript
// main.ts
import { registerDep } from "./src/injection-typed.ts";
import { createSessionManager, SessionStore, getDefaultOptions } from "./src/session.ts";
import { createCookieManager } from "./src/cookies.ts";

// Session dependency (global singleton)
let sessionStore: SessionStore | null = null;

registerDep('session', (ctx) => {
  if (!sessionStore) {
    const secret = Deno.env.get('TSP_SESSION_SECRET');
    const secretBytes = secret
      ? new TextEncoder().encode(secret)
      : new Uint8Array(32);

    const options = {
      ...getDefaultOptions(),
      secret: secretBytes,
    };

    sessionStore = new SessionStore(options);
  }

  const cookieManager = createCookieManager(ctx);
  return createSessionManager(ctx, sessionStore, cookieManager);
});

// Cookies dependency
registerDep('cookies', (ctx) => {
  return createCookieManager(ctx);
});
```

### 3. Use in Pages

```tsx
// www/my-page.tsx
export default Page(async function(ctx, { session, cookies }) {
  // Use session
  const user = await session.getUser();

  // Use cookies
  cookies.set('theme', 'dark');

  return <div>Hello {user?.name}</div>;
});
```

## Built-in Dependencies

### Session

Session provides user session management functions:

```tsx
export default Page(async function(ctx, { session }) {
  // Get current user
  const user = await session.getUser();

  if (!user) {
    // Not logged in, redirect to login page
    return { redirect: '/login', status: 302 };
  }

  // Login user
  await session.login('user-123', {
    name: 'John Doe',
    email: 'john@example.com',
  });

  // Store session data
  await session.set('cart', { items: [], total: 0 });

  // Read session data
  const cart = await session.get('cart');

  // Delete session data
  await session.delete('cart');

  // Refresh session expiration time
  await session.touch();

  // Regenerate session ID (prevent fixation attack)
  await session.regenerateId();

  // Logout
  await session.logout();

  return <div>Hello {user.name}</div>;
});
```

**Complete API**:

| Method | Parameters | Return | Description |
|------|------|--------|------|
| `getUser()` | - | `Promise<SessionUser \| null>` | Get current logged in user |
| `login(userId, userData)` | `userId: string`, `userData?: Partial<SessionUser>` | `Promise<void>` | User login |
| `logout()` | - | `Promise<void>` | User logout |
| `set(key, value)` | `key: string`, `value: unknown` | `Promise<void>` | Store session data |
| `get<T>(key)` | `key: string` | `Promise<T \| null>` | Read session data |
| `delete(key)` | `key: string` | `Promise<void>` | Delete session data |
| `regenerateId()` | - | `Promise<void>` | Regenerate session ID |
| `touch()` | - | `Promise<void>` | Refresh expiration time |
| `isValid()` | - | `Promise<boolean>` | Check if session is valid |
| `getId()` | - | `string` | Get session ID |

**SessionUser Interface**:

```typescript
interface SessionUser {
  id: string;
  name: string;
  email?: string;
  role?: string;
  [key: string]: unknown;
}
```

See: [Session Feature Documentation](./session.md)

### Cookies

Cookies provides HTTP Cookie management functions:

```tsx
export default Page(async function(ctx, { cookies }) {
  // Set Cookie (basic)
  cookies.set('theme', 'dark');

  // Set Cookie (with options)
  cookies.set('sessionId', 'abc123', {
    httpOnly: true,      // Prevent XSS attacks
    secure: true,        // HTTPS only
    sameSite: 'Strict',  // Prevent CSRF attacks
    maxAge: 3600,        // Expires in 1 hour
    path: '/',           // Path
    domain: '.example.com', // Domain
  });

  // Delete Cookie
  cookies.delete('theme');

  // Batch set
  cookies.setMultiple({
    'theme': { value: 'dark', options: { maxAge: 31536000 } },
    'language': { value: 'zh-CN', options: { maxAge: 31536000 } },
    'fontSize': { value: '14px', options: { maxAge: 31536000 } },
  });

  // Read Cookie (from ctx.cookies)
  const theme = ctx.cookies.theme;

  return <div>Theme: {theme}</div>;
});
```

**Complete API**:

| Method | Parameters | Return | Description |
|------|------|--------|------|
| `set(name, value, options?)` | `name: string`, `value: string`, `options?: CookieOptions` | `void` | Set Cookie |
| `get(name)` | `name: string` | `string \| undefined` | Get response Cookie |
| `delete(name, options?)` | `name: string`, `options?: CookieOptions` | `void` | Delete Cookie |
| `setMultiple(cookies)` | `Record<string, { value: string, options?: CookieOptions }>` | `void` | Batch set |

**CookieOptions Interface**:

```typescript
interface CookieOptions {
  httpOnly?: boolean;   // Default: true
  secure?: boolean;     // Default: true
  sameSite?: 'Strict' | 'Lax' | 'None';  // Default: 'Strict'
  maxAge?: number;      // Seconds
  expires?: Date;       // Absolute expiration time
  path?: string;        // Default: '/'
  domain?: string;
}
```

**Note**: Read request Cookies using `ctx.cookies`, set response Cookies using `cookies` dependency.

See: [Cookie Feature Documentation](./cookies.md)

### Crypto

Crypto provides encryption utilities based on Deno's native Web Crypto API:

```tsx
export default Page(async function(ctx, { crypto, response }) {
  // Generate random values
  const iv = crypto.getRandomValues(12);

  // SHA-256 hash
  const hash = await crypto.digest('SHA-256', 'Hello World');

  // Generate AES-GCM key
  const key = await crypto.generateKey('AES-GCM', 256);

  // Encrypt data
  const data = new TextEncoder().encode('secret message');
  const encrypted = await crypto.encrypt(data, key, iv);

  // Decrypt data
  const decrypted = await crypto.decrypt(encrypted, key, iv);

  return response.json({
    hash: Array.from(new Uint8Array(hash)),
    decrypted: new TextDecoder().decode(decrypted)
  });
});
```

**Complete API**:

| Method | Parameters | Return | Description |
|------|------|--------|------|
| `getRandomValues(length)` | `length: number` | `Uint8Array` | Generate random values |
| `digest(algo, data)` | `algo: string`, `data: string \| Uint8Array` | `Promise<ArrayBuffer>` | Hash data (SHA-1, SHA-256, SHA-384, SHA-512, MD5) |
| `generateKey(algo, length?, extractable?, usages?)` | `algo: "AES-GCM" \| "HMAC"`, `length?: number` | `Promise<CryptoKey>` | Generate encryption key |
| `importKey(algo, keyData, extractable?, usages?)` | `algo: "AES-GCM" \| "HMAC"`, `keyData: string \| Uint8Array` | `Promise<CryptoKey>` | Import key |
| `encrypt(data, key, iv)` | `data: Uint8Array`, `key: CryptoKey`, `iv: Uint8Array` | `Promise<ArrayBuffer>` | AES-GCM encryption |
| `decrypt(data, key, iv)` | `data: Uint8Array`, `key: CryptoKey`, `iv: Uint8Array` | `Promise<ArrayBuffer>` | AES-GCM decryption |
| `sign(algo, key, data)` | `algo: string`, `key: CryptoKey`, `data: Uint8Array` | `Promise<ArrayBuffer>` | HMAC signature |
| `verify(algo, key, signature, data)` | `algo: string`, `key: CryptoKey`, `signature: Uint8Array`, `data: Uint8Array` | `Promise<boolean>` | HMAC verification |

### Bcryptjs (createBcryptjs)

Bcryptjs provides password hashing and verification using bcryptjs library:

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

**API**:

| Method | Parameters | Return | Description |
|------|------|--------|------|
| `hash(password)` | `password: string` | `string` | Hash password using bcryptjs |
| `compare(password, hash)` | `password: string`, `hash: string` | `boolean` | Verify password |

**Note**: `createBcryptjs` is a factory function that returns the bcryptjs object. It requires `await` to get the actual bcryptjs methods.

## Custom Dependencies

### Create Database Dependency

```typescript
// 1. types.d.ts - Declare types
declare global {
  interface AppDeps {
    db?: {
      query: (sql: string, params?: unknown[]) => Promise<unknown[]>;
      insert: (table: string, data: Record<string, unknown>) => Promise<void>;
      update: (table: string, id: string, data: Record<string, unknown>) => Promise<void>;
      delete: (table: string, id: string) => Promise<void>;
    };
  }
}

// 2. main.ts - Register dependency
registerDep('db', (ctx) => {
  // Use connection pool
  const pool = new DatabasePool(DB_URL);

  return {
    query: async (sql: string, params?: unknown[]) => {
      const conn = await pool.getConnection();
      try {
        return await conn.query(sql, params);
      } finally {
        conn.release();
      }
    },
    insert: async (table: string, data: Record<string, unknown>) => {
      await pool.insert(table, data);
    },
    update: async (table: string, id: string, data: Record<string, unknown>) => {
      await pool.update(table, id, data);
    },
    delete: async (table: string, id: string) => {
      await pool.delete(table, id);
    },
  };
});

// 3. page.tsx - Use dependency
export default Page(async function(ctx, { db }) {
  const users = await db?.query('SELECT * FROM users');
  return <div>{JSON.stringify(users)}</div>;
});
```

### Create Cache Dependency

```typescript
// 1. types.d.ts
declare global {
  interface AppDeps {
    cache?: {
      get: <T>(key: string) => Promise<T | null>;
      set: (key: string, value: unknown, ttl?: number) => Promise<void>;
      delete: (key: string) => Promise<void>;
      clear: () => Promise<void>;
    };
  }
}

// 2. main.ts
registerDep('cache', (ctx) => {
  const redis = new RedisClient(REDIS_URL);

  return {
    get: async <T>(key: string) => {
      const data = await redis.get(key);
      return data ? JSON.parse(data) as T : null;
    },
    set: async (key: string, value: unknown, ttl = 3600) => {
      await redis.setex(key, ttl, JSON.stringify(value));
    },
    delete: async (key: string) => {
      await redis.del(key);
    },
    clear: async () => {
      await redis.flushdb();
    },
  };
});

// 3. page.tsx
export default Page(async function(ctx, { cache }) {
  // Try to get from cache
  let data = await cache?.get('homepage-data');

  if (!data) {
    // Cache miss, get from database
    data = await fetchDataFromDB();
    await cache?.set('homepage-data', data, 600); // Cache for 10 minutes
  }

  return <div>{JSON.stringify(data)}</div>;
});
```

### Create API Client Dependency

```typescript
// 1. types.d.ts
declare global {
  interface AppDeps {
    api?: {
      get: <T>(url: string) => Promise<T>;
      post: <T>(url: string, data: unknown) => Promise<T>;
      put: <T>(url: string, data: unknown) => Promise<T>;
      delete: <T>(url: string) => Promise<T>;
    };
  }
}

// 2. main.ts
registerDep('api', (ctx) => {
  const baseUrl = 'https://api.example.com';
  const apiKey = Deno.env.get('API_KEY');

  const request = async <T>(
    method: string,
    url: string,
    data?: unknown,
  ): Promise<T> => {
    const response = await fetch(`${baseUrl}${url}`, {
      method,
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${apiKey}`,
      },
      body: data ? JSON.stringify(data) : undefined,
    });

    if (!response.ok) {
      throw new Error(`API request failed: ${response.statusText}`);
    }

    return response.json() as T;
  };

  return {
    get: <T>(url: string) => request<T>('GET', url),
    post: <T>(url: string, data: unknown) => request<T>('POST', url, data),
    put: <T>(url: string, data: unknown) => request<T>('PUT', url, data),
    delete: <T>(url: string) => request<T>('DELETE', url),
  };
});

// 3. page.tsx
export default Page(async function(ctx, { api }) {
  const posts = await api?.get('/posts');
  return <div>{JSON.stringify(posts)}</div>;
});
```

### Create Mail Service Dependency

```typescript
// 1. types.d.ts
declare global {
  interface AppDeps {
    mailer?: {
      send: (to: string, subject: string, body: string) => Promise<void>;
      sendTemplate: (to: string, template: string, data: Record<string, unknown>) => Promise<void>;
    };
  }
}

// 2. main.ts
registerDep('mailer', (ctx) => {
  const transporter = new SMTPTransporter({
    host: 'smtp.example.com',
    port: 587,
    auth: {
      user: Deno.env.get('SMTP_USER'),
      pass: Deno.env.get('SMTP_PASS'),
    },
  });

  return {
    send: async (to: string, subject: string, body: string) => {
      await transporter.sendMail({
        from: 'noreply@example.com',
        to,
        subject,
        html: body,
      });
    },
    sendTemplate: async (to: string, template: string, data: Record<string, unknown>) => {
      const body = await renderTemplate(template, data);
      await transporter.sendMail({
        from: 'noreply@example.com',
        to,
        subject: data.subject as string,
        html: body,
      });
    },
  };
});

// 3. page.tsx
export default Page(async function(ctx, { mailer }) {
  if (ctx.method === 'POST') {
    const { email } = ctx.body as { email: string };
    await mailer?.send(email, 'Welcome!', '<h1>Welcome to our site!</h1>');
    return <div>Email sent!</div>;
  }

  return <form method="POST">
    <input type="email" name="email" />
    <button type="submit">Send Email</button>
  </form>;
});
```

## Best Practices

### 1. Dependencies Should Be Optional

For most custom dependencies, use optional types (`?`):

```typescript
// ✅ Recommended - Optional dependencies
interface AppDeps {
  db?: Database;
  cache?: Cache;
}

// ❌ Not recommended - Required dependencies
interface AppDeps {
  db: Database;
  cache: Cache;
}
```

### 2. Use Global Singletons

For dependencies requiring global state (like SessionStore), cache instances outside the builder:

```typescript
// ✅ Recommended - Global singleton
let sessionStore: SessionStore | null = null;

registerDep('session', (ctx) => {
  if (!sessionStore) {
    sessionStore = new SessionStore(options);
  }
  return createSessionManager(ctx, sessionStore);
});

// ❌ Not recommended - Create new instance on each request
registerDep('session', (ctx) => {
  const store = new SessionStore(options);  // Creates new store on each request
  return createSessionManager(ctx, store);
});
```

### 3. Use Connection Pools

For resources like databases and Redis, use connection pools instead of single connections:

```typescript
// ✅ Recommended - Use connection pool
registerDep('db', (ctx) => {
  const pool = new DatabasePool(DB_URL);
  return {
    query: async (sql: string) => {
      const conn = await pool.getConnection();
      try {
        return await conn.query(sql);
      } finally {
        conn.release();
      }
    },
  };
});

// ❌ Not recommended - Single connection
registerDep('db', (ctx) => {
  const conn = new DatabaseConnection(DB_URL);
  return {
    query: async (sql: string) => conn.query(sql),
  };
});
```

### 4. Environment Variable Configuration

Read configuration from environment variables instead of hardcoding:

```typescript
// ✅ Recommended - Use environment variables
registerDep('db', (ctx) => {
  const url = Deno.env.get('DATABASE_URL');
  if (!url) {
    throw new Error('DATABASE_URL environment variable is required');
  }
  return new Database(url);
});

// ❌ Not recommended - Hardcoded configuration
registerDep('db', (ctx) => {
  return new Database('postgresql://localhost:5432/mydb');
});
```

### 5. Error Handling

Provide good error handling in dependencies:

```typescript
// ✅ Recommended - Provide error handling
registerDep('api', (ctx) => {
  const baseUrl = Deno.env.get('API_URL') || 'https://api.example.com';

  return {
    get: async <T>(url: string) => {
      try {
        const response = await fetch(`${baseUrl}${url}`);
        if (!response.ok) {
          throw new Error(`API error: ${response.status} ${response.statusText}`);
        }
        return await response.json() as T;
      } catch (error) {
        console.error('API request failed:', error);
        throw error;
      }
    },
  };
});
```

## Common Scenarios

### Scenario 1: User Authentication

```tsx
export default Page(async function(ctx, { session }) {
  const user = await session.getUser();

  if (!user) {
    return { redirect: '/login', status: 302 };
  }

  // Check permissions
  if (user.role !== 'admin') {
    return <div>Access denied</div>;
  }

  return <div>Admin dashboard</div>;
});
```

### Scenario 2: Shopping Cart

```tsx
export default Page(async function(ctx, { session }) {
  const user = await session.getUser();

  if (!user) {
    return { redirect: '/login', status: 302 };
  }

  // Get shopping cart
  let cart = await session.get('cart') || { items: [], total: 0 };

  // Add items
  if (ctx.method === 'POST') {
    const { productId, quantity } = ctx.body as { productId: string, quantity: number };
    cart.items.push({ productId, quantity });
    cart.total += quantity * 100; // Assume unit price is 100
    await session.set('cart', cart);
  }

  return (
    <div>
      <h1>Shopping Cart</h1>
      <ul>
        {cart.items.map((item: any) => <li>{item.productId} x {item.quantity}</li>)}
      </ul>
      <p>Total: {cart.total}</p>
    </div>
  );
});
```

### Scenario 3: Data Pagination

```tsx
export default Page(async function(ctx, { db }) {
  const page = parseInt(ctx.query.page || '1');
  const limit = 20;
  const offset = (page - 1) * limit;

  const posts = await db?.query(
    'SELECT * FROM posts LIMIT ? OFFSET ?',
    [limit, offset]
  );

  const totalResult = await db?.query('SELECT COUNT(*) as total FROM posts');
  const total = totalResult?.[0]?.total as number;
  const totalPages = Math.ceil(total / limit);

  return (
    <div>
      <h1>Posts</h1>
      <ul>
        {posts?.map((post: any) => <li>{post.title}</li>)}
      </ul>
      <div>
        Page {page} of {totalPages}
        {page > 1 && <a href={`?page=${page - 1}`}>Previous</a>}
        {page < totalPages && <a href={`?page=${page + 1}`}>Next</a>}
      </div>
    </div>
  );
});
```

### Scenario 4: Cache Optimization

```tsx
export default Page(async function(ctx, { db, cache }) {
  const cacheKey = 'homepage-stats';

  // Try to get from cache
  let stats = await cache?.get(cacheKey);

  if (!stats) {
    // Cache miss, get from database
    stats = await db?.query(`
      SELECT
        (SELECT COUNT(*) FROM users) as userCount,
        (SELECT COUNT(*) FROM posts) as postCount,
        (SELECT COUNT(*) FROM comments) as commentCount
    `);

    // Cache for 5 minutes
    await cache?.set(cacheKey, stats, 300);
  }

  return (
    <div>
      <h1>Site Statistics</h1>
      <p>Users: {stats?.[0]?.userCount}</p>
      <p>Posts: {stats?.[0]?.postCount}</p>
      <p>Comments: {stats?.[0]?.commentCount}</p>
    </div>
  );
});
```

## Testing

### Test Pages with Dependencies

```typescript
// tests/unit/my-page_test.ts
import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { registerDep } from "../../src/injection-typed.ts";

// Register test dependencies
registerDep('session', (ctx) => ({
  getUser: async () => ({ id: '123', name: 'Test User' }),
  login: async () => {},
  logout: async () => {},
}));

registerDep('db', (ctx) => ({
  query: async () => [{ id: '1', title: 'Test Post' }],
}));

// Test page
Deno.test('my page renders correctly', async () => {
  const ctx = {
    method: 'GET',
    url: new URL('http://localhost:9000/test'),
    headers: new Headers(),
    query: {},
    body: null,
    cookies: {},
    file: '/www/test.tsx',
    root: '/www',
  };

  // Dynamically import page
  const page = await import('../../www/my-page.tsx');
  const result = await page.default(ctx);

  // Assert results
  assertStringIncludes(result, 'Test User');
  assertStringIncludes(result, 'Test Post');
});
```

## Related Documentation

- [Dependency Injection Feature](./injection.md) - Detailed dependency injection documentation
- [Session Feature](./session.md) - Session management detailed documentation
- [Cookie Feature](./cookies.md) - Cookie management detailed documentation
- [Architecture](../architecture.md) - Learn about system architecture

---

[← Back to Features](./README.md) | [← Back to Documentation Center](../README.md)
