# 自定义 Response 对象测试

## 新增测试

已添加 `custom_response.tsx` 页面，用于测试自定义 Response 对象的直接返回功能。

## 测试页面功能

### custom_response.tsx

**位置：** `tests/test_www/custom_response.tsx`

**功能：**
- 默认返回 HTML 响应（自定义 Content-Type）
- 支持 `?format=json` 参数返回 JSON 响应
- 测试自定义响应头

### 代码示例

```tsx
export default async function (context: PageContext) {
  const { query } = context;
  const format = query.format;

  // JSON 格式
  if (format === "json") {
    return new Response(
      JSON.stringify({
        message: "自定义 JSON 响应",
        timestamp: new Date().toISOString(),
      }),
      {
        status: 200,
        headers: {
          "Content-Type": "application/json",
          "X-Custom-Header": "test-value",
        },
      }
    );
  }

  // HTML 格式（默认）
  return new Response(
    "<!DOCTYPE html>...",
    {
      status: 200,
      headers: {
        "Content-Type": "text/html; charset=utf-8",
        "X-Response-Type": "custom",
      },
    }
  );
}
```

## 测试用例

### 测试 1: HTML Response

```bash
curl http://localhost:9100/custom_response.tsx
```

**预期结果：**
- 状态码：200
- Content-Type: text/html; charset=utf-8
- 包含自定义响应头：X-Response-Type: custom

### 测试 2: JSON Response

```bash
curl http://localhost:9100/custom_response.tsx?format=json
```

**预期结果：**
- 状态码：200
- Content-Type: application/json
- 返回 JSON 数据
- 包含自定义响应头：X-Custom-Header: test-value

## 测试代码

```typescript
await testHttpRequest(`http://localhost:${TEST_PORT}/custom_response.tsx`, 200);
await testHttpRequest(
  `http://localhost:${TEST_PORT}/custom_response.tsx?format=json`,
  200,
  "application/json"
);
```

## Response 对象的优势

### 1. 完全控制

```tsx
return new Response(body, {
  status: 200,
  headers: {
    "Content-Type": "text/html",
    "Cache-Control": "no-cache",
    "Set-Cookie": "name=value",
  },
});
```

### 2. 支持各种内容类型

**HTML：**
```tsx
return new Response("<h1>Hello</h1>", {
  headers: { "Content-Type": "text/html" }
});
```

**JSON：**
```tsx
return new Response(
  JSON.stringify({ message: "Hello" }),
  {
    headers: { "Content-Type": "application/json" }
  }
);
```

**纯文本：**
```tsx
return new Response("Plain text", {
  headers: { "Content-Type": "text/plain" }
});
```

**二进制：**
```tsx
const data = await Deno.readFile("./image.png");
return new Response(data, {
  headers: { "Content-Type": "image/png" }
});
```

### 3. 灵活的状态码

```tsx
// 200 OK
return new Response("Success", { status: 200 });

// 201 Created
return new Response("Created", { status: 201 });

// 204 No Content
return new Response(null, { status: 204 });

// 404 Not Found
return new Response("Not Found", { status: 404 });

// 500 Internal Server Error
return new Response("Server Error", { status: 500 });
```

### 4. 流式响应

```tsx
// 创建可读流
const file = await Deno.open("./large-file.txt");

return new Response(file.readable, {
  headers: {
    "Content-Type": "text/plain",
    "Content-Disposition": "attachment; filename=file.txt",
  },
});
```

## 与 JSX 返回的对比

### JSX 返回（默认）

```tsx
export default async function (context: PageContext) {
  // 自动渲染为 HTML
  return <h1>Hello</h1>;
}
```

**处理流程：**
1. 页面函数返回 JSX
2. 调用 `renderJSX()` 渲染为 HTML
3. 返回 200 HTML 响应

### Response 返回（自定义）

```tsx
export default async function (context: PageContext) {
  // 直接返回 Response
  return new Response("<h1>Hello</h1>", {
    headers: { "Content-Type": "text/html" }
  });
}
```

**处理流程：**
1. 页面函数返回 Response 对象
2. 检测到 `instanceof Response`
3. 直接返回，不做任何处理

## 测试覆盖

### 当前测试包含

| 测试 | URL | 验证内容 |
|------|-----|---------|
| HTML 响应 | `/custom_response.tsx` | 默认 HTML 响应 |
| JSON 响应 | `/custom_response.tsx?format=json` | JSON 格式 |
| 自定义头 | 两者 | 自定义响应头 |

### 验证点

- ✅ Response 对象能被正确识别
- ✅ 自定义状态码能正常工作
- ✅ 自定义响应头能正常设置
- ✅ 不同 Content-Type 能正确返回

## 代码修改

### 文件：tests/test_www/custom_response.tsx

**新增文件**，包含：
- HTML 响应示例
- JSON 响应示例
- 自定义响应头示例

### 文件：tests/binary_build_test.ts

**修改内容：**
1. 添加 `testHttpRequest()` 函数的 `expectedContentType` 参数
2. 添加两个自定义响应测试：
   - HTML 响应测试
   - JSON 响应测试（带 Content-Type 验证）

## 运行测试

```bash
# 运行完整测试
deno task test:binary

# 预期输出
✓ custom_response.tsx - 状态码: 200
✓ custom_response.tsx?format=json - 状态码: 200
✓ Content-Type: application/json
✓ 所有测试通过
```

## 手动测试

```bash
# 启动服务器（使用测试目录）
deno run --allow-net --allow-read src/main.ts --root ./tests/test_www

# 测试 HTML 响应
curl http://localhost:9000/custom_response.tsx

# 测试 JSON 响应
curl http://localhost:9000/custom_response.tsx?format=json

# 测试自定义头
curl -I http://localhost:9000/custom_response.tsx
```

## 总结

自定义 Response 对象提供了：
- ✅ 完全的 HTTP 响应控制
- ✅ 灵活的内容类型支持
- ✅ 自定义状态码和响应头
- ✅ 无需经过 JSX 渲染

这是 TSP-FPM 的高级功能，适用于需要精确控制 HTTP 响应的场景。

## 更新时间

2026-01-27
