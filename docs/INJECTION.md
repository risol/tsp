# 依赖注入功能文档

## 概述

TSP 现在支持依赖注入功能，允许在页面函数中注入辅助函数和服务。

## 核心概念

依赖注入使用包装器模式，将需要的依赖作为第二个参数传递给页面函数：

```tsx
// 原来的写法
export default async function(context) {
  // ...
}

// 使用依赖注入
export default withDeps(async function(context, { dep1, dep2 }) {
  // 使用 dep1 和 dep2
});
```

## 使用方法

### 1. 注册依赖

在应用启动时（例如 `main.ts`）注册依赖：

```typescript
import { registerDepBuilder } from "./injection.ts";

// 注册单个依赖
registerDepBuilder('db', (ctx) => new Database(ctx));

// 注册异步依赖
registerDepBuilder('session', async (ctx) => await getSession(ctx));

// 注册工具函数
registerDepBuilder('logger', (ctx) => console.log);
```

### 2. 在页面中使用

```tsx
import { withDeps } from "../src/injection.ts";

export default withDeps(async function(context, { db, session, logger }) {
  // 使用注入的依赖
  const user = await session.getUser();
  const data = await db.query('SELECT * FROM users');
  logger('页面渲染成功');

  return <div>Hello {user.name}</div>;
});
```

## 实现原理

依赖注入不需要源码扫描，而是通过以下方式工作：

1. **依赖注册**：使用 `registerDepBuilder(name, builder)` 注册依赖构建器
2. **函数包装**：使用 `withDeps(fn)` 包装页面函数
3. **自动注入**：包装器在调用页面函数时，自动构建所有已注册的依赖并注入

### 代码流程

```typescript
// 1. 注册依赖
registerDepBuilder('testFunc', (ctx) => () => {
  console.log('testFunc called');
  return 'testFunc called';
});

// 2. 包装函数
export default withDeps(async function(ctx, { testFunc }) {
  const result = testFunc();  // 调用注入的函数
  return <div>{result}</div>;
});

// 3. 内部执行流程（由 withDeps 自动处理）
const deps = { testFunc: await builder(ctx) };
return fn(ctx, deps);
```

## 测试

### 单元测试

运行依赖注入单元测试：

```bash
deno test --allow-net tests/unit/injection_test.ts
```

测试覆盖：
- ✓ 注册单个依赖
- ✓ 注册多个依赖
- ✓ 取消注册依赖
- ✓ 获取已注册的依赖列表
- ✓ 单个依赖注入
- ✓ 多个依赖注入
- ✓ 异步依赖构建
- ✓ 依赖可以访问 context
- ✓ 未注册的依赖返回 undefined
- ✓ 页面函数返回 JSX

### E2E 测试

E2E 测试会编译二进制文件并启动真实服务器进行测试：

```bash
deno run --allow-all tests/run_e2e_tests.ts
```

## 类型定义

```typescript
/**
 * 依赖对象类型（任意类型的键值对）
 */
export type Deps = Record<string, unknown>;

/**
 * 依赖构建器类型
 */
export type DepBuilder<T> = (ctx: PageContext) => Promise<T> | T;

/**
 * 注册依赖构建器
 */
export function registerDepBuilder<T>(name: string, builder: DepBuilder<T>): void;

/**
 * 包装页面函数，自动注入依赖
 */
export function withDeps<T>(
  fn: (ctx: PageContext, deps: Deps) => Promise<T> | T
): (ctx: PageContext) => Promise<T>;
```

## 示例

### 示例 1：数据库连接

```typescript
// main.ts
registerDepBuilder('db', (ctx) => {
  return {
    query: (sql: string) => database.execute(sql),
    insert: (table: string, data: Record<string, unknown>) => {
      return database.insert(table, data);
    },
  };
});

// page.tsx
export default withDeps(async function(ctx, { db }) {
  const users = await db.query('SELECT * FROM users');
  return <div>{JSON.stringify(users)}</div>;
});
```

### 示例 2：Session 管理

```typescript
// main.ts
registerDepBuilder('session', async (ctx) => {
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
export default withDeps(async function(ctx, { session }) {
  const user = await session.getUser();
  return <div>Hello {user.name}</div>;
});
```

### 示例 3：多个依赖

```typescript
// main.ts
registerDepBuilder('db', (ctx) => new Database());
registerDepBuilder('cache', (ctx) => new Cache());
registerDepBuilder('logger', (ctx) => console.log);

// page.tsx
export default withDeps(async function(ctx, { db, cache, logger }) {
  logger('开始处理请求');

  const cached = await cache.get('data');
  if (cached) return <div>{cached}</div>;

  const data = await db.query('SELECT * FROM data');
  await cache.set('data', data);

  return <div>{JSON.stringify(data)}</div>;
});
```

## 注意事项

1. **依赖构建时机**：依赖在每次请求时构建，不是单例
2. **访问 Context**：依赖构建器可以访问 `context`，根据请求信息构建不同的依赖
3. **异步支持**：依赖构建器可以是异步的，`withDeps` 会自动等待所有依赖构建完成
4. **类型安全**：建议为依赖定义明确的类型，便于 TypeScript 类型检查
