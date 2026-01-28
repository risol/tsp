# 依赖注入功能文档

## 概述

TSP 支持类型安全的依赖注入功能，允许在页面函数中注入辅助函数和服务。

## 核心概念

依赖注入使用包装器模式，将需要的依赖作为第二个参数传递给页面函数：

```tsx
// 原来的写法
export default async function(context) {
  // ...
}

// 使用依赖注入
import { Page } from "../src/injection-typed.ts";

export default Page(async function(context, { dep1, dep2 }) {
  // 使用 dep1 和 dep2
});

// Page 已在全局作用域，无需 import
export default Page(async function(context, { dep1, dep2 }) {
  // 使用 dep1 和 dep2
});
```

## 使用方法

### 1. 声明依赖类型

在 `types.d.ts` 中声明依赖类型：

```typescript
declare global {
  interface AppDeps {
    db?: {
      query: (sql: string) => Promise<unknown[]>;
      insert: (table: string, data: Record<string, unknown>) => Promise<void>;
    };
    session?: {
      getUser: () => Promise<{ name: string }>;
      set: (key: string, value: unknown) => Promise<void>;
    };
    logger?: (message: string) => void;
  }
}
```

### 2. 注册依赖

在应用启动时（例如 `main.ts`）注册依赖：

```typescript
import { registerDep } from "./src/injection-typed.ts";

// 注册单个依赖
registerDep('db', (ctx) => {
  return {
    query: async (sql: string) => database.execute(sql),
    insert: async (table: string, data: Record<string, unknown>) => {
      return database.insert(table, data);
    },
  };
});

// 注册异步依赖
registerDep('session', async (ctx) => {
  const sessionId = ctx.cookies.session_id;
  const session = await redis.get(`session:${sessionId}`);
  return {
    getUser: async () => session.user,
    set: async (key: string, value: unknown) => {
      session[key] = value;
      await redis.set(`session:${sessionId}`, session);
    },
  };
});

// 注册工具函数
registerDep('logger', (ctx) => {
  return (message: string) => console.log(`[LOG] ${message}`);
});
```

### 3. 在页面中使用

```tsx
import { Page } from "../src/injection-typed.ts";

export default Page(async function(context, { db, session, logger }) {
  // 使用注入的依赖（完整的类型提示）
  const user = await session?.getUser();
  const data = await db?.query('SELECT * FROM users');
  logger?.('页面渲染成功');

  return <div>Hello {user?.name}</div>;
});
```

或者使用全局 `Page`（无需 import）：

```tsx
export default Page(async function(context, { db, session, logger }) {
  // Page 已在全局作用域中
  const user = await session?.getUser();
  return <div>Hello {user?.name}</div>;
});
```

## 实现原理

依赖注入不需要源码扫描，而是通过以下方式工作：

1. **类型声明**：在 `types.d.ts` 中声明 `AppDeps` 接口
2. **依赖注册**：使用 `registerDep(name, builder)` 注册依赖构建器
3. **函数包装**：使用 `Page(fn)` 包装页面函数
4. **自动注入**：包装器在调用页面函数时，自动构建所有已注册的依赖并注入

### 代码流程

```typescript
// 1. 在 types.d.ts 中声明类型
declare global {
  interface AppDeps {
    testFunc?: () => string;
  }
}

// 2. 注册依赖
registerDep('testFunc', (ctx) => () => {
  console.log('testFunc called');
  return 'testFunc called';
});

// 3. 包装函数
export default Page(async function(ctx, { testFunc }) {
  const result = testFunc();  // 调用注入的函数
  return <div>{result}</div>;
});

// 4. 内部执行流程（由 Page 自动处理）
const deps = { testFunc: await builder(ctx) };
return fn(ctx, deps);
```

### 全局可用性

`Page` 在全局作用域中自动初始化，无需 import：

```tsx
// 无需 import，直接使用
export default Page(async function(ctx, { testFunc }) {
  // ...
});
```

### createPage()

`Page` 函数由 `createPage()` 工厂函数创建：

```typescript
export function createPage() {
  return function Page<T>(
    fn: (ctx: PageContext, deps: AppDeps) => Promise<T> | T
  ): (ctx: PageContext) => Promise<T> {
    // 实现细节...
  };
}
```

如果你需要创建自定义的 Page 函数，可以使用 `createPage()`：

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

### E2E 测试

E2E 测试会编译二进制文件并启动真实服务器进行测试：

```bash
deno run --allow-all tests/run_e2e_tests.ts
```

注意：由于编译环境的限制，E2E 测试中无法实际测试依赖注入功能，依赖注入主要由单元测试覆盖。

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
- `builder`: 依赖构建器函数，接收 `PageContext`，返回依赖实例

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

### createPage()

创建自定义的 Page 函数。

```typescript
function createPage(): <T>(
  fn: (ctx: PageContext, deps: AppDeps) => Promise<T> | T
) => (ctx: PageContext) => Promise<T>
```

### 类型定义

```typescript
// 在 types.d.ts 中声明
declare global {
  interface AppDeps {
    db?: {
      query: (sql: string) => Promise<unknown[]>;
      insert: (table: string, data: Record<string, unknown>) => Promise<void>;
    };
    session?: {
      getUser: () => Promise<{ name: string }>;
      set: (key: string, value: unknown) => Promise<void>;
    };
    logger?: (message: string) => void;
  }
}
```

## 示例

### 示例 1：数据库连接

```typescript
// types.d.ts
declare global {
  interface AppDeps {
    db?: {
      query: (sql: string) => Promise<unknown[]>;
      insert: (table: string, data: Record<string, unknown>) => Promise<void>;
    };
  }
}

// main.ts
registerDep('db', (ctx) => {
  return {
    query: async (sql: string) => {
      console.log(`Executing: ${sql}`);
      return database.execute(sql);
    },
    insert: async (table: string, data: Record<string, unknown>) => {
      return database.insert(table, data);
    },
  };
});

// page.tsx
export default Page(async function(ctx, { db }) {
  const users = await db?.query('SELECT * FROM users');
  return <div>{JSON.stringify(users)}</div>;
});
```

### 示例 2：Session 管理

```typescript
// types.d.ts
declare global {
  interface AppDeps {
    session?: {
      getUser: () => Promise<{ name: string }>;
      set: (key: string, value: unknown) => Promise<void>;
    };
  }
}

// main.ts
registerDep('session', async (ctx) => {
  const sessionId = ctx.cookies.session_id;
  const session = await redis.get(`session:${sessionId}`);
  return {
    getUser: async () => session.user,
    set: async (key: string, value: unknown) => {
      session[key] = value;
      await redis.set(`session:${sessionId}`, session);
    },
  };
});

// page.tsx
export default Page(async function(ctx, { session }) {
  const user = await session?.getUser();
  return <div>Hello {user?.name}</div>;
});
```

### 示例 3：多个依赖

```typescript
// types.d.ts
declare global {
  interface AppDeps {
    db?: Database;
    cache?: Cache;
    logger?: (message: string) => void;
  }
}

// main.ts
registerDeps({
  db: (ctx) => new Database(),
  cache: (ctx) => new Cache(),
  logger: (ctx) => (message: string) => console.log(`[LOG] ${message}`),
});

// page.tsx
export default Page(async function(ctx, { db, cache, logger }) {
  logger?.('开始处理请求');

  const cached = await cache?.get('data');
  if (cached) return <div>{cached}</div>;

  const data = await db?.query('SELECT * FROM data');
  await cache?.set('data', data);

  return <div>{JSON.stringify(data)}</div>;
});
```

## 注意事项

1. **依赖构建时机**：依赖在每次请求时构建，不是单例
2. **访问 Context**：依赖构建器可以访问 `context`，根据请求信息构建不同的依赖
3. **异步支持**：依赖构建器可以是异步的，`Page` 会自动等待所有依赖构建完成
4. **类型安全**：必须在 `types.d.ts` 中声明依赖类型，才能获得完整的类型提示
5. **全局可用**：`Page` 已在全局作用域中，无需 import

## 相关文档

- [功能特性首页](./README.md) - 查看其他功能
- [架构设计](../architecture.md) - 了解依赖注入的实现原理
- [开发指南](../development.md) - 如何开发新功能
- [历史文档](../history/README.md#依赖注入) - 依赖注入功能演进历史

---

[← 返回功能特性](./README.md) | [← 返回文档中心](../README.md)
