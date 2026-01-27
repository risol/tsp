# 二进制测试类型错误修复

## 问题描述

运行 `deno task test:binary` 时出现 TypeScript 类型错误：

```
TS18046 [ERROR]: 'error' is of type 'unknown'.
    throw new Error(`请求失败: ${error.message}`);
                             ~~~~~
    at file:///D:/GitHub/tsp/tests/binary_build_test.ts:159:30
```

## 根本原因

在 TypeScript 的 `catch` 块中，`error` 的类型是 `unknown`，而不是 `Error`。这是 TypeScript 4.0+ 的设计，因为：
1. 任何值都可能被抛出（字符串、数字、对象等）
2. 强制开发者先检查类型再使用

## 错误代码

```typescript
try {
  await fetch(url);
} catch (error) {
  throw new Error(`请求失败: ${error.message}`);  // ❌ 类型错误
}
```

## 修复方案

### 修复前（错误）

```typescript
} catch (error) {
  throw new Error(`请求失败: ${error.message}`);
}
```

### 修复后（正确）

```typescript
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  throw new Error(`请求失败: ${message}`);
}
```

## 其他 catch 块的处理

文件中有多个 catch 块，根据使用情况不同处理：

### 1. 重新抛出错误（无需修改）

```typescript
} catch (error) {
  if (error instanceof Deno.errors.NotFound) {
    // 文件不存在，忽略
  } else {
    throw error;  // ✅ 可以直接重新抛出
  }
}
```

### 2. 打印错误（无需修改）

```typescript
} catch (error) {
  console.error(error);  // ✅ console.error 接受 any 类型
}
```

### 3. 访问错误属性（需要类型检查）

```typescript
} catch (error) {
  // ❌ 错误：不能直接访问 error.message
  console.log(error.message);

  // ✅ 正确：先检查类型
  if (error instanceof Error) {
    console.log(error.message);
  } else {
    console.log(String(error));
  }
}
```

## 最佳实践

### 1. 使用类型保护

```typescript
try {
  // ...
} catch (error) {
  if (error instanceof Error) {
    console.error(error.message);
    console.error(error.stack);
  } else {
    console.error("未知错误:", error);
  }
}
```

### 2. 类型断言（谨慎使用）

```typescript
try {
  // ...
} catch (error) {
  // 如果确定错误类型，可以使用断言
  const err = error as Error;
  console.error(err.message);
}
```

### 3. 提取辅助函数

```typescript
function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

try {
  // ...
} catch (error) {
  console.error(getErrorMessage(error));
}
```

## TypeScript 版本差异

### TypeScript < 4.0

```typescript
try {
  // ...
} catch (error) {
  // error 的类型是 Error（不安全）
  console.log(error.message);
}
```

### TypeScript >= 4.0

```typescript
try {
  // ...
} catch (error) {
  // error 的类型是 unknown（更安全）
  // 必须先检查类型
  if (error instanceof Error) {
    console.log(error.message);
  }
}
```

## 测试结果

### 修复前
```
TS18046 [ERROR]: 'error' is of type 'unknown'.
error: Type checking failed.
```

### 修复后
```
✓ Check tests/binary_build_test.ts
✓ 所有类型检查通过
```

## 相关资源

- [TypeScript 4.0 Release Notes](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-4-0.html)
- [Unknown vs Any](https://www.typescriptlang.org/docs/handbook/2/basic-types.html#unknown)
- [Type Narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html)

## 运行测试

```bash
# 类型检查
deno check tests/binary_build_test.ts

# 运行测试
deno task test:binary

# 所有测试
deno task test
```

## 更新时间

2026-01-27
