# AppDeps 使用指南

## 概述

`AppDeps` 是 TSP 的依赖注入系统，提供类型安全的依赖管理。通过 `AppDeps`，你可以：

- 在页面函数中注入 Session、Cookies 等内置依赖
- 创建自定义依赖（数据库、缓存、API 客户端等）
- 享受完整的 TypeScript 类型提示
- 使用懒加载机制优化性能

## 快速开始

### 1. 声明依赖类型

在 `types.d.ts` 中声明依赖：

```typescript
// types.d.ts
declare global {
  interface AppDeps extends Record<string, unknown> {
    // 内置依赖（必需）
    session: import("./src/session.ts").SessionManager;
    cookies: import("./src/cookies.ts").CookieManager;
    testFunc: () => string;

    // 可选依赖
    db?: {
      query: (sql: string) => Promise<unknown[]>;
    };
    logger?: typeof console.log;
  }
}

export {};
```

### 2. 注册依赖

在 `main.ts` 中注册依赖：

```typescript
// main.ts
import { registerDep } from "./src/injection-typed.ts";
import { createSessionManager, SessionStore, getDefaultOptions } from "./src/session.ts";
import { createCookieManager } from "./src/cookies.ts";

// Session 依赖（全局单例）
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

// Cookies 依赖
registerDep('cookies', (ctx) => {
  return createCookieManager(ctx);
});
```

### 3. 在页面中使用

```tsx
// www/my-page.tsx
export default Page(async function(ctx, { session, cookies }) {
  // 使用 session
  const user = await session.getUser();

  // 使用 cookies
  cookies.set('theme', 'dark');

  return <div>Hello {user?.name}</div>;
});
```

## 内置依赖

### Session

Session 提供用户会话管理功能：

```tsx
export default Page(async function(ctx, { session }) {
  // 获取当前用户
  const user = await session.getUser();

  if (!user) {
    // 未登录，跳转到登录页
    return { redirect: '/login', status: 302 };
  }

  // 登录用户
  await session.login('user-123', {
    name: 'John Doe',
    email: 'john@example.com',
  });

  // 存储 session 数据
  await session.set('cart', { items: [], total: 0 });

  // 读取 session 数据
  const cart = await session.get('cart');

  // 删除 session 数据
  await session.delete('cart');

  // 刷新 session 过期时间
  await session.touch();

  // 重新生成 session ID（防固定攻击）
  await session.regenerateId();

  // 登出
  await session.logout();

  return <div>Hello {user.name}</div>;
});
```

**完整 API**：

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `getUser()` | - | `Promise<SessionUser \| null>` | 获取当前登录用户 |
| `login(userId, userData)` | `userId: string`, `userData?: Partial<SessionUser>` | `Promise<void>` | 用户登录 |
| `logout()` | - | `Promise<void>` | 用户登出 |
| `set(key, value)` | `key: string`, `value: unknown` | `Promise<void>` | 存储 session 数据 |
| `get<T>(key)` | `key: string` | `Promise<T \| null>` | 读取 session 数据 |
| `delete(key)` | `key: string` | `Promise<void>` | 删除 session 数据 |
| `regenerateId()` | - | `Promise<void>` | 重新生成 session ID |
| `touch()` | - | `Promise<void>` | 刷新过期时间 |
| `isValid()` | - | `Promise<boolean>` | 检查 session 是否有效 |
| `getId()` | - | `string` | 获取 session ID |

**SessionUser 接口**：

```typescript
interface SessionUser {
  id: string;
  name: string;
  email?: string;
  role?: string;
  [key: string]: unknown;
}
```

详见：[Session 功能文档](./session.md)

### Cookies

Cookies 提供 HTTP Cookie 管理功能：

```tsx
export default Page(async function(ctx, { cookies }) {
  // 设置 Cookie（基础）
  cookies.set('theme', 'dark');

  // 设置 Cookie（带选项）
  cookies.set('sessionId', 'abc123', {
    httpOnly: true,      // 防止 XSS 攻击
    secure: true,        // 仅 HTTPS
    sameSite: 'Strict',  // 防止 CSRF 攻击
    maxAge: 3600,        // 1 小时过期
    path: '/',           // 路径
    domain: '.example.com', // 域名
  });

  // 删除 Cookie
  cookies.delete('theme');

  // 批量设置
  cookies.setMultiple({
    'theme': { value: 'dark', options: { maxAge: 31536000 } },
    'language': { value: 'zh-CN', options: { maxAge: 31536000 } },
    'fontSize': { value: '14px', options: { maxAge: 31536000 } },
  });

  // 读取 Cookie（从 ctx.cookies）
  const theme = ctx.cookies.theme;

  return <div>Theme: {theme}</div>;
});
```

**完整 API**：

| 方法 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `set(name, value, options?)` | `name: string`, `value: string`, `options?: CookieOptions` | `void` | 设置 Cookie |
| `get(name)` | `name: string` | `string \| undefined` | 获取响应 Cookie |
| `delete(name, options?)` | `name: string`, `options?: CookieOptions` | `void` | 删除 Cookie |
| `setMultiple(cookies)` | `Record<string, { value: string, options?: CookieOptions }>` | `void` | 批量设置 |

**CookieOptions 接口**：

```typescript
interface CookieOptions {
  httpOnly?: boolean;   // 默认: true
  secure?: boolean;     // 默认: true
  sameSite?: 'Strict' | 'Lax' | 'None';  // 默认: 'Strict'
  maxAge?: number;      // 秒
  expires?: Date;       // 绝对过期时间
  path?: string;        // 默认: '/'
  domain?: string;
}
```

**注意**：读取请求 Cookie 使用 `ctx.cookies`，设置响应 Cookie 使用 `cookies` 依赖。

详见：[Cookie 功能文档](./cookies.md)

## 自定义依赖

### 创建数据库依赖

```typescript
// 1. types.d.ts - 声明类型
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

// 2. main.ts - 注册依赖
registerDep('db', (ctx) => {
  // 使用连接池
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

// 3. page.tsx - 使用依赖
export default Page(async function(ctx, { db }) {
  const users = await db?.query('SELECT * FROM users');
  return <div>{JSON.stringify(users)}</div>;
});
```

### 创建缓存依赖

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
  // 尝试从缓存获取
  let data = await cache?.get('homepage-data');

  if (!data) {
    // 缓存未命中，从数据库获取
    data = await fetchDataFromDB();
    await cache?.set('homepage-data', data, 600); // 缓存 10 分钟
  }

  return <div>{JSON.stringify(data)}</div>;
});
```

### 创建 API 客户端依赖

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

### 创建邮件服务依赖

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

## 最佳实践

### 1. 依赖应该是可选的

对于大多数自定义依赖，应该使用可选类型（`?`）：

```typescript
// ✅ 推荐 - 可选依赖
interface AppDeps {
  db?: Database;
  cache?: Cache;
}

// ❌ 不推荐 - 必需依赖
interface AppDeps {
  db: Database;
  cache: Cache;
}
```

### 2. 使用全局单例

对于需要全局状态的依赖（如 SessionStore），在 builder 外部缓存实例：

```typescript
// ✅ 推荐 - 全局单例
let sessionStore: SessionStore | null = null;

registerDep('session', (ctx) => {
  if (!sessionStore) {
    sessionStore = new SessionStore(options);
  }
  return createSessionManager(ctx, sessionStore);
});

// ❌ 不推荐 - 每次请求创建新实例
registerDep('session', (ctx) => {
  const store = new SessionStore(options);  // 每次请求都创建新 store
  return createSessionManager(ctx, store);
});
```

### 3. 使用连接池

对于数据库、Redis 等资源，使用连接池而不是单个连接：

```typescript
// ✅ 推荐 - 使用连接池
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

// ❌ 不推荐 - 单个连接
registerDep('db', (ctx) => {
  const conn = new DatabaseConnection(DB_URL);
  return {
    query: async (sql: string) => conn.query(sql),
  };
});
```

### 4. 环境变量配置

从环境变量读取配置，而不是硬编码：

```typescript
// ✅ 推荐 - 使用环境变量
registerDep('db', (ctx) => {
  const url = Deno.env.get('DATABASE_URL');
  if (!url) {
    throw new Error('DATABASE_URL environment variable is required');
  }
  return new Database(url);
});

// ❌ 不推荐 - 硬编码配置
registerDep('db', (ctx) => {
  return new Database('postgresql://localhost:5432/mydb');
});
```

### 5. 错误处理

在依赖中提供良好的错误处理：

```typescript
// ✅ 推荐 - 提供错误处理
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

## 常见场景

### 场景 1: 用户认证

```tsx
export default Page(async function(ctx, { session }) {
  const user = await session.getUser();

  if (!user) {
    return { redirect: '/login', status: 302 };
  }

  // 检查权限
  if (user.role !== 'admin') {
    return <div>Access denied</div>;
  }

  return <div>Admin dashboard</div>;
});
```

### 场景 2: 购物车

```tsx
export default Page(async function(ctx, { session }) {
  const user = await session.getUser();

  if (!user) {
    return { redirect: '/login', status: 302 };
  }

  // 获取购物车
  let cart = await session.get('cart') || { items: [], total: 0 };

  // 添加商品
  if (ctx.method === 'POST') {
    const { productId, quantity } = ctx.body as { productId: string, quantity: number };
    cart.items.push({ productId, quantity });
    cart.total += quantity * 100; // 假设单价 100
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

### 场景 3: 数据分页

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

### 场景 4: 缓存优化

```tsx
export default Page(async function(ctx, { db, cache }) {
  const cacheKey = 'homepage-stats';

  // 尝试从缓存获取
  let stats = await cache?.get(cacheKey);

  if (!stats) {
    // 缓存未命中，从数据库获取
    stats = await db?.query(`
      SELECT
        (SELECT COUNT(*) FROM users) as userCount,
        (SELECT COUNT(*) FROM posts) as postCount,
        (SELECT COUNT(*) FROM comments) as commentCount
    `);

    // 缓存 5 分钟
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

## 测试

### 测试带依赖的页面

```typescript
// tests/unit/my-page_test.ts
import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { registerDep } from "../../src/injection-typed.ts";

// 注册测试依赖
registerDep('session', (ctx) => ({
  getUser: async () => ({ id: '123', name: 'Test User' }),
  login: async () => {},
  logout: async () => {},
}));

registerDep('db', (ctx) => ({
  query: async () => [{ id: '1', title: 'Test Post' }],
}));

// 测试页面
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

  // 动态导入页面
  const page = await import('../../www/my-page.tsx');
  const result = await page.default(ctx);

  // 断言结果
  assertStringIncludes(result, 'Test User');
  assertStringIncludes(result, 'Test Post');
});
```

## 相关文档

- [依赖注入功能](./injection.md) - 依赖注入详细文档
- [Session 功能](./session.md) - Session 管理详细文档
- [Cookie 功能](./cookies.md) - Cookie 管理详细文档
- [架构设计](../architecture.md) - 了解系统架构

---

[← 返回功能特性](./README.md) | [← 返回文档中心](../README.md)
