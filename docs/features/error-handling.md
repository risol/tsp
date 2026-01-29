# TSP 错误处理测试

## 新增测试

已添加 `error.tsx` 页面，用于测试生产模式和生产模式的错误处理。

## 测试页面

### error.tsx

**位置：** `tests/test_www/error.tsx`

**功能：**
- 故意抛出错误
- 验证错误捕获机制
- 测试开发和生产模式的错误显示差异

```tsx
export default async function (_context: PageContext) {
  // 故意抛出错误
  throw new Error("这是一个测试错误，用于验证错误处理机制");
}
```

## 错误处理机制

### 开发模式 (--dev)

```bash
deno run --allow-net --allow-read src/main.ts --dev --root ./tests/test_www
```

**行为：**
- 显示详细错误信息
- 显示完整的堆栈跟踪
- 显示错误文件和行号
- 方便调试

**示例输出：**
```html
<h1>500 Internal Server Error</h1>
<pre>错误消息: 这是一个测试错误，用于验证错误处理机制</pre>
<pre>堆栈跟踪:
    at error.tsx:3:15
    at getPage (cache.ts:45:20)
    ...
</pre>
```

### 生产模式（默认）

```bash
deno run --allow-net --allow-read src/main.ts --root ./tests/test_www
```

**行为：**
- 隐藏敏感堆栈信息
- 显示友好的错误页面
- 只显示错误消息
- 不暴露内部实现

**示例输出：**
```html
<h1>500 Internal Server Error</h1>
<pre>错误消息: 这是一个测试错误，用于验证错误处理机制</pre>
```

## 测试用例

### 测试错误捕获

```typescript
await testHttpRequest(`http://localhost:9001/error.tsx`, 500, undefined, true);
```

**验证点：**
- ✅ 返回 500 状态码
- ✅ 包含错误信息
- ✅ 生产模式不包含堆栈跟踪

## 实现代码

### src/main.ts 错误处理

```typescript
try {
  // 执行页面函数
  const result = await pageFn(context);

  // 处理结果...
} catch (error) {
  // 错误处理
  const errorMessage = error instanceof Error ? error.message : String(error);
  const stackTrace = error instanceof Error ? error.stack : "";

  console.error("Request error:", errorMessage);

  if (config.dev) {
    // 开发模式：显示详细错误
    const html = `
<!DOCTYPE html>
<html>
<head><title>Error</title></head>
<body>
  <h1>500 Internal Server Error</h1>
  <pre>${errorMessage}</pre>
  <pre>${stackTrace}</pre>
</body>
</html>
    `;
    return new Response(html, {
      status: 500,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  } else {
    // 生产模式：隐藏堆栈
    const html = `
<!DOCTYPE html>
<html>
<head><title>Error</title></head>
<body>
  <h1>500 Internal Server Error</h1>
  <pre>${errorMessage}</pre>
</body>
</html>
    `;
    return new Response(html, {
      status: 500,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }
}
```

## 测试场景

### 场景 1: 运行时错误

```tsx
export default async function (context: PageContext) {
  // 访问未定义变量
  const obj = null as unknown;
  console.log((obj as any).value);
}
```

**测试：** 访问 `/error.tsx`
**期望：** 500 错误页面

### 场景 2: 异步错误

```tsx
export default async function (context: PageContext) {
  // Promise 拒绝
  await Promise.reject("异步错误");
}
```

**测试：** 访问 `/error.tsx`
**期望：** 500 错误页面

### 场景 3: 类型错误

```tsx
export default async function (context: PageContext) {
  // 类型错误
  const str: string = null as unknown;
  console.log(str.toUpperCase());
}
```

**测试：** 访问 `/error.tsx`
**期望：** 500 错误页面

## 安全特性

### 开发模式

✅ 显示完整错误信息
✅ 显示堆栈跟踪
✅ 显示文件路径
✅ 显示行号

### 生产模式

✅ 隐藏敏感信息
✅ 只显示错误消息
✅ 不暴露代码路径
✅ 不暴露堆栈跟踪

## 测试验证

### 运行测试

```bash
# 开发模式测试
deno run --allow-net --allow-read src/main.ts --dev --root ./tests/test_www
curl http://localhost:9000/error.tsx

# 生产模式测试
deno run --allow-net --allow-read src/main.ts --root ./tests/test_www
curl http://localhost:9000/error.tsx
```

### 二进制测试

```bash
deno task test:binary
```

**测试包含：**
- ✅ 访问 `/error.tsx`
- ✅ 验证 500 状态码
- ✅ 验证错误信息显示
- ✅ 验证生产模式不显示堆栈

## 错误处理最佳实践

### 1. 抛出有意义的错误

```tsx
export default async function (context: PageContext) {
  const userId = context.query.userId;

  if (!userId) {
    throw new Error("缺少必需参数: userId");
  }

  // ...
}
```

### 2. 捕获和处理错误

```tsx
export default async function (context: PageContext) {
  try {
    // 可能出错的操作
    const data = await fetchData();
    return <div>{data}</div>;
  } catch (error) {
    // 处理错误，返回友好页面
    console.error("数据加载失败:", error);
    return <div>加载失败，请稍后重试</div>;
  }
}
```

### 3. 记录错误日志

```tsx
export default async function (context: PageContext) {
  try {
    // ...
  } catch (error) {
    // 记录错误日志
    console.error("[ERROR]", {
      file: context.file,
      error: error instanceof Error ? error.message : String(error),
      timestamp: new Date().toISOString(),
    });

    throw error; // 重新抛出，让框架处理
  }
}
```

## 测试目录更新

```
tests/test_www/
├── index.tsx            # 首页测试
├── form.tsx             # 表单/POST 测试
├── api.tsx              # API/上下文测试
├── custom_response.tsx  # 自定义 Response 测试
├── error.tsx            # 错误处理测试 ✨ 新增
├── redirect.tsx         # 重定向测试（暂时禁用）
└── README.md
```

## 运行测试

```bash
deno task test:binary
```

**预期结果：**
```
✓ error.tsx - 状态码: 500
✓ 错误信息显示正确
✓ 生产模式隐藏堆栈
✓ 所有测试通过
```

## 总结

错误处理测试验证了：
- ✅ 开发模式显示详细错误
- ✅ 生产模式隐藏敏感信息
- ✅ 500 错误页面正确显示
- ✅ 错误被正确捕获和处理
- ✅ 堆栈跟踪在开发模式显示，生产模式隐藏

这是确保应用在生产环境安全性的重要测试。

## 更新时间

2026-01-27

## 相关文档

- [功能特性首页](./README.md) - 查看其他功能
- [自定义响应](./custom-response.md) - 自定义 HTTP 响应
- [开发指南](../development.md) - 如何调试错误
- [架构设计](../architecture.md) - 了解错误处理的实现原理

---

[← 返回功能特性](./README.md) | [← 返回文档中心](../README.md)
