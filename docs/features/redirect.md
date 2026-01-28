# 重定向测试修复

## 问题描述

运行二进制测试时，重定向测试失败：

```
测试请求: http://localhost:9100/redirect.tsx
✓ 状态码: 200

Error: 请求失败: Values are not equal: 期望状态码 302
-   200
+   302
```

## 根本原因

`/redirect.tsx` 页面需要查询参数才会触发重定向：

```tsx
// redirect.tsx 的逻辑
export default async function (context: PageContext) {
  const { query } = context;

  // 只有特定查询参数才重定向
  if (query.to === "home") {
    return { redirect: "/" };  // 302
  }

  // 默认显示页面（200）
  return <html>...</html>;
}
```

测试代码直接访问 `/redirect.tsx`（不带参数），期望返回 302，但实际返回 200。

## 修复方案

### 方案 1: 创建简单的重定向测试页面

创建 `www/redirect_simple.tsx`，总是重定向：

```tsx
export default async function (_context: PageContext) {
  // 简单的重定向测试页面
  // 总是重定向到首页
  return { redirect: "/" };
}
```

### 方案 2: 使用正确的查询参数

修改测试代码，使用触发重定向的 URL：

```typescript
// 修复前
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx`, 302); // ❌ 失败

// 修复后
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx`, 200);  // 默认页面
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx?to=home`, 302); // 触发重定向
```

### 方案 3: 组合测试（最终方案）

```typescript
// 4. 测试各种 HTTP 请求
await testHttpRequest(`http://localhost:${TEST_PORT}/`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/index.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/form.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/api.tsx`, 200);

// 重定向测试 - 多种场景
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect_simple.tsx`, 302); // 简单重定向
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx`, 200);  // 重定向示例页面
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx?to=home`, 302); // 带参数重定向

await testHttpRequest(`http://localhost:${TEST_PORT}/nonexistent.tsx`, 404); // 404
```

## redirect.tsx 的完整行为

| URL | 行为 | 状态码 |
|-----|------|--------|
| `/redirect.tsx` | 显示重定向示例页面 | 200 |
| `/redirect.tsx?to=home` | 重定向到首页 | 302 |
| `/redirect.tsx?to=new-home` | 永久重定向到首页 | 301 |
| `/redirect.tsx?to=protected` (未登录) | 重定向到登录页 | 302 |
| `/redirect.tsx?to=protected` (已登录) | 显示受保护页面 | 200 |

## 测试覆盖

现在测试包含三种重定向场景：

1. **简单重定向**
   ```typescript
   await testHttpRequest(`/redirect_simple.tsx`, 302);
   ```
   - 总是返回 `{ redirect: "/" }`
   - 验证基本重定向功能

2. **重定向示例页面**
   ```typescript
   await testHttpRequest(`/redirect.tsx`, 200);
   ```
   - 默认显示重定向示例页面
   - 验证页面渲染正常

3. **带参数的重定向**
   ```typescript
   await testHttpRequest(`/redirect.tsx?to=home`, 302);
   ```
   - 触发条件重定向
   - 验证查询参数处理

## 代码修改

### 新增文件: www/redirect_simple.tsx

```tsx
export default async function (_context: PageContext) {
  return { redirect: "/" };
}
```

### 修改文件: tests/binary_build_test.ts

**第 225-233 行：**
```typescript
await testHttpRequest(`http://localhost:${TEST_PORT}/`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/index.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/form.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/api.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect_simple.tsx`, 302); // 新增
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx?to=home`, 302); // 修改
await testHttpRequest(`http://localhost:${TEST_PORT}/nonexistent.tsx`, 404);
```

## 验证结果

### 修复前

```
✗ 期望状态码 302
- 实际: 200
```

### 修复后（预期）

```
✓ redirect_simple.tsx - 状态码: 302
✓ redirect.tsx - 状态码: 200
✓ redirect.tsx?to=home - 状态码: 302
✓ 所有测试通过
```

## 重定向功能说明

### 返回值类型

TSP 支持三种返回值类型：

1. **JSX 元素** - 渲染为 HTML (200)
   ```tsx
   return <div>Hello</div>;
   ```

2. **重定向对象** - 触发 HTTP 重定向 (301/302/303/307/308)
   ```tsx
   return { redirect: "/" };
   return { redirect: "/", status: 301 };
   ```

3. **Response 对象** - 自定义响应
   ```tsx
   return new Response("...", { status: 200, headers: {...} });
   ```

### 重定向状态码

| 状态码 | 含义 | 使用场景 |
|--------|------|---------|
| 301 | Moved Permanently | 永久重定向 |
| 302 | Found | 临时重定向（默认） |
| 303 | See Other | POST 后重定向到 GET |
| 307 | Temporary Redirect | 保持请求方法的临时重定向 |
| 308 | Permanent Redirect | 保持请求方法的永久重定向 |

### 示例代码

**基本重定向：**
```tsx
export default async function (context: PageContext) {
  return { redirect: "/target" };
}
```

**永久重定向：**
```tsx
export default async function (context: PageContext) {
  return { redirect: "/target", status: 301 };
}
```

**条件重定向：**
```tsx
export default async function (context: PageContext) {
  const { cookies } = context;

  if (!cookies.sessionId) {
    return { redirect: "/login" };
  }

  return <div>已登录内容</div>;
}
```

## 运行测试

```bash
# 完整的二进制测试
deno task test:binary

# 手动测试重定向
deno run --allow-net --allow-read src/main.ts

# 访问
# http://localhost:9000/redirect_simple.tsx  -> 重定向到首页
# http://localhost:9000/redirect.tsx?to=home -> 重定向到首页
```

## 其他注意事项

### 重定向循环

避免创建重定向循环：

```tsx
// ❌ 错误：无限循环
// index.tsx 重定向到 /home
// home.tsx 重定向到 /index

// ✅ 正确：有条件的重定向
export default async function (context: PageContext) {
  const { query } = context;

  if (query.redirect !== "done") {
    return { redirect: "/?redirect=done" };
  }

  return <div>首页</div>;
}
```

### 相对路径 vs 绝对路径

重定向推荐使用绝对路径：

```tsx
// ✅ 推荐
return { redirect: "/login" };
return { redirect: "https://example.com/page" };

// ⚠️ 可能有问题
return { redirect: "login" };  // 相对路径
```

### 测试重定向

使用 `curl` 测试重定向：

```bash
# 不跟随重定向
curl -I http://localhost:9000/redirect_simple.tsx

# 输出：
# HTTP/1.1 302 Found
# Location: /

# 跟随重定向
curl -L http://localhost:9000/redirect_simple.tsx
```

## 更新时间

2026-01-27

## 相关文档

- [功能特性首页](./README.md) - 查看其他功能
- [自定义响应](./custom-response.md) - 另一种响应控制方式
- [架构设计](../architecture.md) - 了解重定向的实现原理
- [历史文档](../history/README.md#重定向功能) - 重定向功能演进历史

---

[← 返回功能特性](./README.md) | [← 返回文档中心](../README.md)
