# 依赖注入功能文档

## 概述

TSP 支持类型安全的依赖注入功能，通过 `Page` 包装器自动将依赖注入到页面函数中。

**重要**：Page 函数使用 **Proxy 懒加载机制**，只有在访问依赖时才构建它们。

## 核心概念

### 正确的 Page 用法

```tsx
// ✅ 正确 - 直接使用 Page 包装函数
export default Page(async function(ctx, { session, db, logger }) {
  const user = await session.getUser();
  const data = await db.query('SELECT * FROM users');
  logger('页面渲染成功');
  return <div>Hello {user?.name}</div>;
});

// ✅ 也可以不使用 async
export default Page(function(ctx) {
  return <div>No dependencies needed</div>;
});

// ❌ 错误 - 不要使用数组参数
// export default Page(['session', 'db'], async function(ctx, { session, db }) {
//   这是错误的语法！
// });
```

### 工作原理

依赖注入使用 **Proxy 懒加载机制**：

1. **声明阶段**：在 `types.d.ts` 中声明 `AppDeps` 接口
2. **注册阶段**：在 `main.ts` 中使用 `registerDep()` 注册依赖
3. **注入阶段**：使用 `Page()` 包装函数，自动注入依赖
4. **懒加载**：只有在访问依赖时才构建，未访问的依赖不会被构建

```typescript
// 1. types.d.ts - 声明依赖类型
declare global {
  interface AppDeps {
    session: import("./src/session.ts").SessionManager;
    cookies: import("./src/cookies.ts").CookieManager;
    db: { query: (sql: string) => Promise<unknown[]> };
    logger: typeof console.log;
  }
}

// 2. main.ts - 注册依赖
registerDep('db', (ctx) => {
  return {
    query: async (sql: string) => database.execute(sql),
  };
});

// 3. page.tsx - 使用依赖
export default Page(async function(ctx, { db, session }) {
  // 只有 session 和 db 会被构建
  // logger 未被访问，不会被构建
  const user = await session.getUser();
  const data = await db.query('SELECT * FROM users');
  return <div>{data}</div>;
});
```

## 完整使用流程

### 步骤 1: 声明依赖类型

在 `types.d.ts` 中声明所有依赖的类型：

```typescript
// types.d.ts
declare global {
  interface AppDeps extends Record<string, unknown> {
    // Session 管理
    session: import("./src/session.ts").SessionManager;

    // Cookie 管理
    cookies: import("./src/cookies.ts").CookieManager;

    // 数据库（示例）
    db?: {
      query: (sql: string) => Promise<unknown[]>;
      insert: (table: string, data: Record<string, unknown>) => Promise<void>;
    };

    // 日志函数
    logger?: typeof console.log;

    // 测试函数
    testFunc: () => string;
  }
}

export {}; // 确保类型被视为全局的
```

### 步骤 2: 注册依赖

在 `main.ts` 中注册依赖实现：

```typescript
// main.ts
import { registerDep } from "./src/injection-typed.ts";
import { createSessionManager, SessionStore, getDefaultOptions } from "./src/session.ts";
import { createCookieManager } from "./src/cookies.ts";

// 全局 SessionStore 单例
let sessionStore: SessionStore | null = null;

// 注册 Session 依赖
registerDep('session', (ctx) => {
  if (!sessionStore) {
    const secret = Deno.env.get('TSP_SESSION_SECRET');
    const secretBytes = secret
      ? new TextEncoder().encode(secret)
      : new Uint8Array(32); // 开发环境使用随机密钥

    const options = {
      ...getDefaultOptions(),
      secret: secretBytes,
    };

    sessionStore = new SessionStore(options);
  }

  const cookieManager = createCookieManager(ctx);
  return createSessionManager(ctx, sessionStore, cookieManager);
});

// 注册 Cookies 依赖
registerDep('cookies', (ctx) => {
  return createCookieManager(ctx);
});

// 注册数据库依赖（示例）
registerDep('db', (ctx) => {
  return {
    query: async (sql: string) => {
      // 实际的数据库查询逻辑
      return database.execute(sql);
    },
    insert: async (table: string, data: Record<string, unknown>) => {
      return database.insert(table, data);
    },
  };
});

// 注册日志函数
registerDep('logger', (ctx) => {
  return (message: string) => console.log(`[LOG] ${message}`);
});

// 注册测试函数
registerDep('testFunc', (ctx) => {
  return () => {
    console.log('testFunc called');
    return 'testFunc called';
  };
});
```

### 步骤 3: 在页面中使用

```tsx
// www/my-page.tsx
// 注意：无需 import Page，它已经是全局函数

export default Page(async function(ctx, { session, db, logger }) {
  // 获取当前用户
  const user = await session.getUser();

  if (!user) {
    return { redirect: '/login', status: 302 };
  }

  // 查询数据
  const data = await db.query('SELECT * FROM posts WHERE user_id = ?', [user.id]);

  // 记录日志
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

## 懒加载机制

依赖注入使用 Proxy 实现懒加载：

```tsx
export default Page(async function(ctx, { db, session, logger }) {
  // 只有访问了的依赖会被构建

  await session.getUser();  // ✅ session 会被构建

  const data = await db.query('...');  // ✅ db 会被构建

  // logger 没有被使用，不会被构建！
  // 这样可以节省资源

  return <div>Done</div>;
});
```

**优点**：
- 只有使用的依赖才会被构建
- 减少不必要的资源消耗
- 提高性能

## 内置依赖

TSP 内置了以下依赖：

### Session

用户会话管理：

```tsx
export default Page(async function(ctx, { session }) {
  // 获取当前用户
  const user = await session.getUser();

  // 登录
  await session.login('user-123', {
    name: 'John Doe',
    email: 'john@example.com',
  });

  // 存储数据
  await session.set('cart', { items: [] });

  // 读取数据
  const cart = await session.get('cart');

  // 登出
  await session.logout();

  return <div>Done</div>;
});
```

详见：[Session 功能文档](./session.md)

### Cookies

HTTP Cookie 管理：

```tsx
export default Page(async function(ctx, { cookies }) {
  // 设置 Cookie
  cookies.set('theme', 'dark', {
    httpOnly: true,
    secure: true,
    sameSite: 'Strict',
    maxAge: 3600,
  });

  // 读取 Cookie（从 ctx.cookies）
  const theme = ctx.cookies.theme;

  // 删除 Cookie
  cookies.delete('theme');

  // 批量设置
  cookies.setMultiple({
    'theme': { value: 'dark', options: { maxAge: 31536000 } },
    'language': { value: 'zh-CN', options: { maxAge: 31536000 } },
  });

  return <div>Done</div>;
});
```

详见：[Cookie 功能文档](./cookies.md)

## 自定义依赖

### 示例 1: 数据库连接

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

### 示例 2: 缓存服务

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
  // 尝试从缓存获取
  let data = await cache?.get('my-data');

  if (!data) {
    // 缓存未命中，从数据库获取
    data = await fetchFromDatabase();
    await cache?.set('my-data', data, 600); // 缓存 10 分钟
  }

  return <div>{JSON.stringify(data)}</div>;
});
```

### 示例 3: API 客户端

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

## API 参考

### registerDep(name, builder)

注册单个依赖构建器。

```typescript
function registerDep<K extends keyof AppDeps>(
  name: K,
  builder: (ctx: PageContext) => Promise<AppDeps[K]> | AppDeps[K]
): void
```

**参数**:
- `name`: 依赖名称（必须是 `AppDeps` 接口的键）
- `builder`: 依赖构建器函数，接收 `PageContext`，返回依赖实例或 Promise

**示例**:
```typescript
registerDep('db', (ctx) => new Database());
```

### registerDeps(deps)

批量注册依赖。

```typescript
function registerDeps<T extends Record<string, (ctx: PageContext) => unknown>>(
  deps: T
): void
```

**参数**:
- `deps`: 依赖对象，键为依赖名称，值为构建器函数

**示例**:
```typescript
registerDeps({
  db: (ctx) => new Database(),
  cache: (ctx) => new Cache(),
  logger: (ctx) => console.log,
});
```

### unregisterDep(name)

取消注册依赖。

```typescript
function unregisterDep(name: keyof AppDeps): void
```

### getRegisteredDeps()

获取已注册的依赖列表。

```typescript
function getRegisteredDeps(): string[]
```

### Page(fn)

包装页面函数，自动注入依赖。

```typescript
function Page<T>(
  fn: (ctx: PageContext, deps: AppDeps) => Promise<T> | T
): (ctx: PageContext) => Promise<T>
```

**参数**:
- `fn`: 页面函数，接收 `PageContext` 和 `AppDeps`

**返回**:
- 包装后的函数，只接收 `PageContext`

**示例**:
```typescript
export default Page(async function(ctx, { db, logger }) {
  // ...
});
```

## 类型安全

完整的类型推断：

```tsx
// ✅ 完整的类型提示
export default Page(async function(ctx, { session, db }) {
  // session 是 SessionManager 类型
  // await session.getUser() 返回 SessionUser | null
  const user = await session.getUser();

  // db 是 db 类型
  // db.query() 返回 Promise<unknown[]>
  const data = await db.query('SELECT * FROM users');

  return <div>{user?.name}</div>;
});
```

## 常见问题

### Q: Page 函数需要 async 吗？

**A**: 不一定。只有当你需要使用 `await` 时才需要 `async`：

```tsx
// ✅ 不需要 async - 没有异步操作
export default Page(function(ctx) {
  return <div>Hello</div>;
});

// ✅ 需要 async - 有异步操作
export default Page(async function(ctx, { session }) {
  const user = await session.getUser();
  return <div>Hello {user?.name}</div>;
});
```

### Q: 为什么不能写成 `export default async Page(...)`？

**A**: 因为 `Page` 已经返回了一个 async 函数，再加 `async` 会造成嵌套的 Promise：

```tsx
// ❌ 错误 - 会创建 Promise<Promise<T>>
export default async Page(async function(ctx, { session }) {
  // ...
});

// ✅ 正确 - 直接返回 Promise<T>
export default Page(async function(ctx, { session }) {
  // ...
});
```

### Q: 可以在 www/ 目录中 import Page 吗？

**A**: **不可以**！Page 函数是全局的，无需 import：

```tsx
// ❌ 错误 - 不要 import
import { Page } from "../src/injection-typed.ts";

// ✅ 正确 - 直接使用
export default Page(async function(ctx, { session }) {
  // ...
});
```

### Q: 依赖每次请求都会重新构建吗？

**A**: 是的，每个请求都会调用 `builder(ctx)` 重新构建依赖。这是设计使然，确保：
1. 每个请求获得独立的依赖实例
2. 可以基于请求上下文构建不同的依赖
3. 避免请求间的状态污染

如果需要单例，可以在 builder 外部缓存实例。

### Q: 如何调试依赖注入？

**A**: 使用 `getRegisteredDeps()` 查看已注册的依赖：

```typescript
// main.ts
registerDep('db', (ctx) => new Database());

console.log('已注册的依赖:', getRegisteredDeps());
// 输出: ['db', 'session', 'cookies', ...]
```

## 测试

### 单元测试

运行依赖注入单元测试：

```bash
deno test --allow-all tests/unit/injection_test.ts
```

测试覆盖：
- ✓ 注册单个依赖
- ✓ 注册多个依赖
- ✓ 取消注册依赖
- ✓ 获取已注册的依赖列表
- ✓ Page 函数包装
- ✓ 单个依赖注入
- ✓ 多个依赖注入
- ✓ 异步依赖构建
- ✓ 依赖可以访问 context
- ✓ 类型安全和类型推断
- ✓ 全局 Page 函数
- ✓ 懒加载机制

## 相关文档

- [功能特性首页](./README.md) - 查看其他功能
- [Session 功能](./session.md) - Session 管理详细文档
- [Cookie 功能](./cookies.md) - Cookie 管理详细文档
- [AppDeps 使用指南](./appdeps.md) - 依赖注入最佳实践
- [架构设计](../architecture.md) - 了解依赖注入的实现原理

---

[← 返回功能特性](./README.md) | [← 返回文档中心](../README.md)
