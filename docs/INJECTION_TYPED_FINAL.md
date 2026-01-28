# 类型安全的依赖注入 - Page 函数

## 概述

`Page` 是一个类型安全的依赖注入函数，通过函数签名自动推断类型，提供完整的 IDE 支持。

## 快速开始

### 第一步：在 types.d.ts 中声明依赖类型

```typescript
// types.d.ts

declare global {
  interface AppDeps {
    // 声明所有依赖类型
    testFunc: () => string;

    db: {
      query: (sql: string) => Promise<unknown[]>;
      insert: (table: string, data: Record<string, unknown>) => Promise<void>;
    };

    session: {
      getUser: () => Promise<{ id: string; name: string } | null>;
    };
  }
}
```

### 第二步：在 main.ts 中注册依赖

```typescript
// main.ts
import { registerDep, registerDeps } from "./src/injection-typed.ts";

// 注册单个依赖
registerDep('testFunc', () => {
  return function testFunc() {
    console.log('testFunc called');
    return 'testFunc called';
  };
});

// 批量注册依赖
registerDeps({
  db: (ctx) => ({
    query: async (sql: string) => {
      console.log('Executing SQL:', sql);
      return await database.execute(sql);
    },
    insert: async (table: string, data: Record<string, unknown>) => {
      await database.insert(table, data);
    },
  }),

  session: async (ctx) => {
    const sessionId = ctx.cookies.session_id;
    return {
      getUser: async () => {
        return await getUserFromSession(sessionId);
      },
    };
  },
});
```

### 第三步：在页面中使用 Page

```tsx
// www/example.tsx
import { Page } from "../src/injection-typed.ts";

export default Page(async function(ctx, { testFunc, db, session }) {
  // ✅ 完整的类型提示！
  const result = testFunc();  // 类型: string
  const users = await db.query('SELECT * FROM users');  // 类型: unknown[]
  const user = await session.getUser();  // 类型: { id: string; name: string } | null

  return <div>Hello {user?.name}</div>;
});
```

## 类型提示效果

使用 `Page` 后，你会得到：

✅ **自动补全**：输入 `{` 时会显示所有可用的依赖
✅ **类型检查**：使用错误的参数类型会报错
✅ **参数提示**：显示函数的参数类型和返回值类型
✅ **跳转定义**：可以跳转到依赖的类型声明

## 对比

### 普通页面函数

```tsx
// 无依赖注入
export default async function(ctx) {
  return <div>Hello</div>;
}
```

### 使用 Page 的依赖注入

```tsx
// 有依赖注入 + 类型提示
import { Page } from "../src/injection-typed.ts";

export default Page(async function(ctx, { db, session }) {
  // db 和 session 都有完整的类型提示 ✅
  const user = await session.getUser();
  return <div>Hello {user?.name}</div>;
});
```

## API 参考

### Page(fn)

包装页面函数，自动注入依赖。

**参数：**
- `fn`: 页面函数，接收 `(context, deps)`

**返回：**
- 包装后的函数，只接收 `context`

**示例：**
```typescript
import { Page } from "../src/injection-typed.ts";

export default Page(async function(ctx, { db }) {
  const users = await db.query('SELECT *');
  return <div>{JSON.stringify(users)}</div>;
});
```

### registerDep(name, builder)

注册单个依赖。

**参数：**
- `name`: 依赖名称（必须是 AppDeps 的属性）
- `builder`: 依赖构建函数，接收 context，返回依赖实例

**示例：**
```typescript
registerDep('testFunc', () => {
  return function testFunc() {
    console.log('testFunc called');
    return 'testFunc called';
  };
});
```

### registerDeps(deps)

批量注册依赖。

**参数：**
- `deps`: 依赖对象

**示例：**
```typescript
registerDeps({
  db: (ctx) => ({
    query: async (sql: string) => database.execute(sql),
  }),
  session: async (ctx) => ({
    getUser: async () => await getUser(ctx),
  }),
  logger: (ctx) => console.log,
});
```

## 完整示例

### types.d.ts

```typescript
declare global {
  interface AppDeps {
    testFunc: () => string;
    db: {
      query: (sql: string) => Promise<unknown[]>;
    };
    session: {
      getUser: () => Promise<{ id: string; name: string } | null>;
    };
  }
}
```

### main.ts

```typescript
import { registerDeps } from "./src/injection-typed.ts";

async function main() {
  const config = await parseArgs();

  // 注册所有依赖
  registerDeps({
    testFunc: () => {
      console.log('testFunc called');
      return 'testFunc called';
    },

    db: (ctx) => ({
      query: async (sql: string) => {
        console.log('Executing SQL:', sql);
        return await database.execute(sql);
      },
    }),

    session: async (ctx) => {
      const sessionId = ctx.cookies.session_id;
      return {
        getUser: async () => {
          return await getUserFromSession(sessionId);
        },
      };
    },
  });

  // 启动服务器...
  Deno.serve({ port: config.port }, (req) => handleRequest(req, config));
}
```

### www/users.tsx

```tsx
import { Page } from "../src/injection-typed.ts";

export default Page(async function(ctx, { db, session }) {
  // ✅ 完整的类型提示
  const user = await session.getUser();

  if (!user) {
    return <div>Please login</div>;
  }

  const users = await db.query('SELECT * FROM users');

  return (
    <html>
      <head>
        <title>User List</title>
      </head>
      <body>
        <h1>Welcome, {user.name}</h1>
        <ul>
          {users.map(u => <li>{JSON.stringify(u)}</li>)}
        </ul>
      </body>
    </html>
  );
});
```

## 命名说明

### 为什么叫 Page？

1. **简洁**：比 `withDeps` 更短
2. **语义化**：明确表示这是一个页面函数
3. **一致性**：与其他框架的概念保持一致（如 Next.js 的 Page）

### 与 withDeps 的关系

`Page` 和 `withDeps` 是同一个函数的不同命名：

```typescript
export function createWithDeps() {
  return function Page<T>(...) { ... };
}
```

你可以选择使用哪个名称：
- `Page` - 推荐，更简洁
- `withDeps` - 别名，更描述性

## 迁移指南

### 从普通页面函数迁移

**之前：**
```tsx
export default async function(ctx) {
  return <div>Hello</div>;
}
```

**之后：**
```tsx
import { Page } from "../src/injection-typed.ts";

export default Page(async function(ctx, { /* deps */ }) {
  return <div>Hello</div>;
});
```

### 从 withDeps 迁移

**之前：**
```tsx
import { withDeps } from "../src/injection-typed.ts";

export default withDeps(async function(ctx, { db }) {
  // ...
});
```

**之后：**
```tsx
import { Page } from "../src/injection-typed.ts";

export default Page(async function(ctx, { db }) {
  // ...
});
```

只需要把 `withDeps` 改为 `Page` 即可！
