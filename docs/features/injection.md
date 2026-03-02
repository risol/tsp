# Dependency Injection Feature Documentation

## Overview

TSP supports type-safe dependency injection, automatically injecting dependencies into page functions through the `Page` wrapper.

**Important**: The Page function uses **Proxy lazy loading mechanism** - dependencies are only built when accessed.

## Core Concepts

### Correct Page Usage

```tsx
// ✅ Correct - Use Page wrapper function directly
export default Page(async function(ctx, { session, db, logger }) {
  const user = await session.getUser();
  const data = await db.query('SELECT * FROM users');
  logger('Page rendered successfully');
  return <div>Hello {user?.name}</div>;
});

// ✅ Can also use without async
export default Page(function(ctx) {
  return <div>No dependencies needed</div>;
});

// ❌ Wrong - Don't use array parameters
// export default Page(['session', 'db'], async function(ctx, { session, db }) {
//   This is incorrect syntax!
// });
```

### How It Works

Dependency injection uses **Proxy lazy loading mechanism**:

1. **Declaration Phase**: Declare `AppDeps` interface in `types.d.ts`
2. **Registration Phase**: Register dependencies in `main.ts` using `registerDep()`
3. **Injection Phase**: Wrap function with `Page()`, automatically inject dependencies
4. **Lazy Loading**: Dependencies are only built when accessed, unaccessed dependencies won't be built

```typescript
// 1. types.d.ts - Declare dependency types
declare global {
  interface AppDeps {
    session: import("./src/session.ts").SessionManager;
    cookies: import("./src/cookies.ts").CookieManager;
    db: { query: (sql: string) => Promise<unknown[]> };
    logger: typeof console.log;
  }
}

// 2. main.ts - Register dependencies
registerDep('db', (ctx) => {
  return {
    query: async (sql: string) => database.execute(sql),
  };
});

// 3. page.tsx - Use dependencies
export default Page(async function(ctx, { db, session }) {
  // Only session and db will be built
  // logger is not accessed, won't be built
  const user = await session.getUser();
  const data = await db.query('SELECT * FROM users');
  return <div>{data}</div>;
});
```

## Complete Usage Flow

### Step 1: Declare Dependency Types

Declare all dependency types in `types.d.ts`:

```typescript
// types.d.ts
declare global {
  interface AppDeps extends Record<string, unknown> {
    // Session Management
    session: import("./src/session.ts").SessionManager;

    // Cookie Management
    cookies: import("./src/cookies.ts").CookieManager;

    // Database (example)
    db?: {
      query: (sql: string) => Promise<unknown[]>;
      insert: (table: string, data: Record<string, unknown>) => Promise<void>;
    };

    // Logger function
    logger?: typeof console.log;

    // Test function
    testFunc: () => string;
  }
}

export {}; // Ensure types are treated as global
```

### Step 2: Register Dependencies

Register dependency implementations in `main.ts`:

```typescript
// main.ts
import { registerDep } from "./src/injection-typed.ts";
import { createSessionManager, SessionStore, getDefaultOptions } from "./src/session.ts";
import { createCookieManager } from "./src/cookies.ts";

// Global SessionStore singleton
let sessionStore: SessionStore | null = null;

// Register Session dependency
registerDep('session', (ctx) => {
  if (!sessionStore) {
    const secret = Deno.env.get('TSP_SESSION_SECRET');
    const secretBytes = secret
      ? new TextEncoder().encode(secret)
      : new Uint8Array(32); // Random key for development

    const options = {
      ...getDefaultOptions(),
      secret: secretBytes,
    };

    sessionStore = new SessionStore(options);
  }

  const cookieManager = createCookieManager(ctx);
  return createSessionManager(ctx, sessionStore, cookieManager);
});

// Register Cookies dependency
registerDep('cookies', (ctx) => {
  return createCookieManager(ctx);
});

// Register database dependency (example)
registerDep('db', (ctx) => {
  return {
    query: async (sql: string) => {
      // Actual database query logic
      return database.execute(sql);
    },
    insert: async (table: string, data: Record<string, unknown>) => {
      return database.insert(table, data);
    },
  };
});

// Register logger function
registerDep('logger', (ctx) => {
  return (message: string) => console.log(`[LOG] ${message}`);
});

// Register test function
registerDep('testFunc', (ctx) => {
  return () => {
    console.log('testFunc called');
    return 'testFunc called';
  };
});
```

### Step 3: Use in Pages

```tsx
// www/my-page.tsx
// Note: No need to import Page, it's already a global function

export default Page(async function(ctx, { session, db, logger }) {
  // Get current user
  const user = await session.getUser();

  if (!user) {
    return { redirect: '/login', status: 302 };
  }

  // Query data
  const data = await db.query('SELECT * FROM posts WHERE user_id = ?', [user.id]);

  // Log
  logger(`Loaded ${data.length} posts for user ${user.id}`);

  return (
    <div>
      <h1>Welcome, {user.name}!</h1>
      <ul>
        {data.map(post => <li>{post.title}</li>)}
      </ul>
    </div>
  );
});
```

## Lazy Loading Mechanism

Dependency injection uses Proxy to implement lazy loading:

```tsx
export default Page(async function(ctx, { db, session, logger }) {
  // Only accessed dependencies will be built

  await session.getUser();  // ✅ session will be built

  const data = await db.query('...');  // ✅ db will be built

  // logger is not used, won't be built!
  // This saves resources

  return <div>Done</div>;
});
```

**Advantages**:
- Only used dependencies are built
- Reduces unnecessary resource consumption
- Improves performance

## Built-in Dependencies

TSP has the following built-in dependencies:

### Session

User session management:

```tsx
export default Page(async function(ctx, { session }) {
  // Get current user
  const user = await session.getUser();

  // Login
  await session.login('user-123', {
    name: 'John Doe',
    email: 'john@example.com',
  });

  // Store data
  await session.set('cart', { items: [] });

  // Read data
  const cart = await session.get('cart');

  // Logout
  await session.logout();

  return <div>Done</div>;
});
```

See: [Session Feature Documentation](./session.md)

### Cookies

HTTP Cookie management:

```tsx
export default Page(async function(ctx, { cookies }) {
  // Set Cookie
  cookies.set('theme', 'dark', {
    httpOnly: true,
    secure: true,
    sameSite: 'Strict',
    maxAge: 3600,
  });

  // Read Cookie (from ctx.cookies)
  const theme = ctx.cookies.theme;

  // Delete Cookie
  cookies.delete('theme');

  // Batch set
  cookies.setMultiple({
    'theme': { value: 'dark', options: { maxAge: 31536000 } },
    'language': { value: 'zh-CN', options: { maxAge: 31536000 } },
  });

  return <div>Done</div>;
});
```

See: [Cookie Feature Documentation](./cookies.md)

## Custom Dependencies

### Example 1: Database Connection

```typescript
// types.d.ts
declare global {
  interface AppDeps {
    db?: {
      query: (sql: string, params?: unknown[]) => Promise<unknown[]>;
      insert: (table: string, data: Record<string, unknown>) => Promise<void>;
    };
  }
}

// main.ts
registerDep('db', (ctx) => {
  const pool = new DatabasePool();

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
      return pool.insert(table, data);
    },
  };
});

// page.tsx
export default Page(async function(ctx, { db }) {
  const users = await db?.query('SELECT * FROM users');
  return <div>{JSON.stringify(users)}</div>;
});
```

### Example 2: Cache Service

```typescript
// types.d.ts
declare global {
  interface AppDeps {
    cache?: {
      get: <T>(key: string) => Promise<T | null>;
      set: (key: string, value: unknown, ttl?: number) => Promise<void>;
      delete: (key: string) => Promise<void>;
    };
  }
}

// main.ts
registerDep('cache', (ctx) => {
  const redis = new RedisClient();

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
  };
});

// page.tsx
export default Page(async function(ctx, { cache }) {
  // Try to get from cache
  let data = await cache?.get('my-data');

  if (!data) {
    // Cache miss, get from database
    data = await fetchFromDatabase();
    await cache?.set('my-data', data, 600); // Cache for 10 minutes
  }

  return <div>{JSON.stringify(data)}</div>;
});
```

### Example 3: API Client

```typescript
// types.d.ts
declare global {
  interface AppDeps {
    api?: {
      get: (url: string) => Promise<unknown>;
      post: (url: string, data: unknown) => Promise<unknown>;
    };
  }
}

// main.ts
registerDep('api', (ctx) => {
  const baseUrl = 'https://api.example.com';

  return {
    get: async (url: string) => {
      const response = await fetch(`${baseUrl}${url}`);
      return response.json();
    },
    post: async (url: string, data: unknown) => {
      const response = await fetch(`${baseUrl}${url}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
      });
      return response.json();
    },
  };
});

// page.tsx
export default Page(async function(ctx, { api }) {
  const posts = await api?.get('/posts');
  return <div>{JSON.stringify(posts)}</div>;
});
```

## API Reference

### registerDep(name, builder)

Register a single dependency builder.

```typescript
function registerDep<K extends keyof AppDeps>(
  name: K,
  builder: (ctx: PageContext) => Promise<AppDeps[K]> | AppDeps[K]
): void
```

**Parameters**:
- `name`: Dependency name (must be a key of `AppDeps` interface)
- `builder`: Dependency builder function, receives `PageContext`, returns dependency instance or Promise

**Example**:
```typescript
registerDep('db', (ctx) => new Database());
```

### registerDeps(deps)

Batch register dependencies.

```typescript
function registerDeps<T extends Record<string, (ctx: PageContext) => unknown>>(
  deps: T
): void
```

**Parameters**:
- `deps`: Dependency object, keys are dependency names, values are builder functions

**Example**:
```typescript
registerDeps({
  db: (ctx) => new Database(),
  cache: (ctx) => new Cache(),
  logger: (ctx) => console.log,
});
```

### unregisterDep(name)

Unregister a dependency.

```typescript
function unregisterDep(name: keyof AppDeps): void
```

### getRegisteredDeps()

Get list of registered dependencies.

```typescript
function getRegisteredDeps(): string[]
```

### Page(fn)

Wrap page function, automatically inject dependencies.

```typescript
function Page<T>(
  fn: (ctx: PageContext, deps: AppDeps) => Promise<T> | T
): (ctx: PageContext) => Promise<T>
```

**Parameters**:
- `fn`: Page function, receives `PageContext` and `AppDeps`

**Returns**:
- Wrapped function, only receives `PageContext`

**Example**:
```typescript
export default Page(async function(ctx, { db, logger }) {
  // ...
});
```

## Type Safety

Complete type inference:

```tsx
// ✅ Complete type hints
export default Page(async function(ctx, { session, db }) {
  // session is SessionManager type
  // await session.getUser() returns SessionUser | null
  const user = await session.getUser();

  // db is db type
  // db.query() returns Promise<unknown[]>
  const data = await db.query('SELECT * FROM users');

  return <div>{user?.name}</div>;
});
```

## FAQ

### Q: Does Page function need async?

**A**: Not necessarily. Only need `async` when using `await`:

```tsx
// ✅ No async needed - no async operations
export default Page(function(ctx) {
  return <div>Hello</div>;
});

// ✅ Need async - has async operations
export default Page(async function(ctx, { session }) {
  const user = await session.getUser();
  return <div>Hello {user?.name}</div>;
});
```

### Q: Why can't we write `export default async Page(...)`?

**A**: Because `Page` already returns an async function, adding `async` would create nested Promise:

```tsx
// ❌ Wrong - would create Promise<Promise<T>>
export default async Page(async function(ctx, { session }) {
  // ...
});

// ✅ Correct - directly returns Promise<T>
export default Page(async function(ctx, { session }) {
  // ...
});
```

### Q: Can we import Page in www/ directory?

**A**: **No!** Page function is global, no need to import:

```tsx
// ❌ Wrong - don't import
import { Page } from "../src/injection-typed.ts";

// ✅ Correct - use directly
export default Page(async function(ctx, { session }) {
  // ...
});
```

### Q: Are dependencies rebuilt on each request?

**A**: Yes, `builder(ctx)` is called to rebuild dependencies on each request. This is by design to ensure:
1. Each request gets independent dependency instance
2. Can build different dependencies based on request context
3. Avoid state pollution between requests

If singleton is needed, cache instance outside builder.

### Q: How to debug dependency injection?

**A**: Use `getRegisteredDeps()` to see registered dependencies:

```typescript
// main.ts
registerDep('db', (ctx) => new Database());

console.log('Registered dependencies:', getRegisteredDeps());
// Output: ['db', 'session', 'cookies', ...]
```

## Testing

### Unit Tests

Run dependency injection unit tests:

```bash
./tsp.sh test:unit
```

Test coverage:
- ✓ Register single dependency
- ✓ Register multiple dependencies
- ✓ Unregister dependency
- ✓ Get registered dependencies list
- ✓ Page function wrapping
- ✓ Single dependency injection
- ✓ Multiple dependency injection
- ✓ Async dependency building
- ✓ Dependencies can access context
- ✓ Type safety and type inference
- ✓ Global Page function
- ✓ Lazy loading mechanism

## Related Documentation

- [Features Home](./README.md) - View other features
- [Session Feature](./session.md) - Session management detailed documentation
- [Cookie Feature](./cookies.md) - Cookie management detailed documentation
- [AppDeps Guide](./appdeps.md) - Dependency injection best practices
- [Architecture](../architecture.md) - Learn about dependency injection implementation

---

[← Back to Features](./README.md) | [← Back to Documentation Center](../README.md)
