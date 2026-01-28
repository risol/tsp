# 类型安全的依赖注入 - 最终方案总结

## 问题回顾

你希望：
1. ✅ 通过函数签名自动推断类型
2. ✅ 在 types.d.ts 中声明类型
3. ✅ 在 TSX 中直接使用 `withDeps`（无需 import）

## 当前实现状态

### 已完成 ✅

1. **types.d.ts** - 全局类型声明
   - `AppDeps` 接口：声明所有可注入的依赖类型
   - `withDeps` 函数：全局类型声明

2. **injection-typed.ts** - 实现
   - `registerDep(name, builder)` - 注册依赖（带类型推断）
   - `registerDeps(deps)` - 批量注册
   - `createWithDeps()` - 创建包装器
   - 自动初始化全局 `withDeps`

3. **main.ts** - 使用示例
   ```typescript
   import { registerDep } from "./injection-typed.ts";

   registerDep('testFunc', () => {
     return function testFunc() {
       console.log('testFunc called');
       return 'testFunc called';
     };
   });
   ```

### 使用方式对比

#### 方案 A：全局 withDeps（无需 import）⚠️

**优点**：
- TSX 中无需 import
- 使用简洁

**缺点**：
- 可能有模块加载问题
- TSX 转译时可能无法识别全局变量
- 需要确保 `injection-typed.ts` 在任何 TSX 之前加载

**代码**：
```tsx
// www/example.tsx
export default withDeps(async function(ctx, { testFunc }) {
  const result = testFunc();  // ✅ 有类型提示
  return <div>{result}</div>;
});
```

#### 方案 B：导入 withDeps（推荐）✅

**优点**：
- 模块加载更可靠
- 明确的依赖关系
- 与 TypeScript 工具链完全兼容

**缺点**：
- 需要在每个 TSX 文件中 import

**代码**：
```tsx
// www/example.tsx
import { withDeps } from "../src/injection-typed.ts";

export default withDeps(async function(ctx, { testFunc }) {
  const result = testFunc();  // ✅ 有类型提示
  return <div>{result}</div>;
});
```

## 类型提示效果

两种方案都有相同的类型提示效果：

```typescript
// types.d.ts
interface AppDeps {
  testFunc: () => string;
  db: {
    query: (sql: string) => Promise<unknown[]>;
  };
}

// TSX 中
export default withDeps(async function(ctx, { testFunc, db }) {
  // ✅ 完整的类型提示
  const result = testFunc();  // 类型: string
  const users = await db.query('SELECT *');  // 类型: unknown[]
});
```

## 建议

考虑到稳定性和兼容性，建议使用 **方案 B（导入 withDeps）**：

1. 在 `types.d.ts` 中声明所有依赖类型
2. 在 `main.ts` 中注册依赖实现
3. 在 TSX 文件中导入并使用 `withDeps`

这样可以确保：
- ✅ 完整的类型提示
- ✅ 可靠的模块加载
- ✅ TypeScript 工具链完全支持
- ✅ 更好的代码可维护性

## 示例代码

### types.d.ts
```typescript
declare global {
  interface AppDeps {
    testFunc: () => string;
    db: {
      query: (sql: string) => Promise<unknown[]>;
    };
  }
}
```

### main.ts
```typescript
import { registerDep } from "./src/injection-typed.ts";

registerDep('testFunc', () => {
  return function testFunc() {
    console.log('testFunc called');
    return 'testFunc called';
  };
});

registerDep('db', (ctx) => ({
  query: async (sql: string) => database.execute(sql),
}));
```

### www/example.tsx
```tsx
import { withDeps } from "../src/injection-typed.ts";

export default withDeps(async function(ctx, { testFunc, db }) {
  const result = testFunc();  // ✅ 类型: string
  const users = await db.query('SELECT *');  // ✅ 类型: unknown[]

  return <div>{result}</div>;
});
```
