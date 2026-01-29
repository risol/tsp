# 重定向功能修复

## 问题描述

重定向页面返回 200 状态码而不是预期的 302：

```
测试请求: http://localhost:9001/redirect_simple.tsx
✓ 状态码: 200

Error: 期望状态码 302
- 实际: 200
+ 期望: 302
```

## 根本原因

### 1. Preact VNode 与重定向对象的冲突

**问题代码：**
```typescript
// 检查是否是重定向对象
if (result && typeof result === "object" && "redirect" in result) {
  // 处理重定向
}
```

**问题：**
- JSX 元素（Preact VNode）也是对象
- 理论上可能包含 `redirect` 属性
- 检查不够精确，可能误判

### 2. Preact VNode 的结构

```typescript
// Preact VNode 的结构
interface VNode {
  type: string | ComponentType;  // 元素类型
  props: Record<string, unknown>; // 属性
  // ... 其他属性
}

// 重定向对象的结构
interface RedirectResult {
  redirect: string;
  status?: 301 | 302 | 303 | 307 | 308;
}
```

**区别：**
- VNode 有 `type` 和 `props` 属性
- 纯重定向对象只有 `redirect` 和可选的 `status` 属性

## 修复方案

### 增强的类型检查

**修复前：**
```typescript
if (result && typeof result === "object" && "redirect" in result) {
  // 可能包含 JSX 元素
  const redirectResult = result as RedirectResult;
  // ...
}
```

**修复后：**
```typescript
if (result &&
    typeof result === "object" &&
    "redirect" in result &&
    !("type" in result) &&    // 排除 VNode
    !("props" in result)) {   // 排除 VNode
  // 确保是纯重定向对象
  const redirectResult = result as RedirectResult;
  const targetUrl = redirectResult.redirect;
  const status = redirectResult.status ?? 302;

  // 验证重定向状态码
  const validStatuses = [301, 302, 303, 307, 308];
  const finalStatus = validStatuses.includes(status) ? status : 302;

  if (config.dev) {
    console.log(`[重定向] ${filepath} -> ${targetUrl} (${finalStatus})`);
  }

  return new Response(null, {
    status: finalStatus,
    headers: {
      "Location": targetUrl,
    },
  });
}
```

### 添加开发模式日志

在开发模式下输出重定向信息：

```typescript
if (config.dev) {
  console.log(`[重定向] ${filepath} -> ${targetUrl} (${finalStatus})`);
}
```

**示例输出：**
```
[重定向] www/redirect_simple.tsx -> / (302)
[重定向] www/redirect.tsx -> / (302)
```

## 类型保护原理

### 检查顺序

1. **基本类型检查**
   ```typescript
   result && typeof result === "object"
   ```
   - 确保 result 存在且是对象

2. **重定向属性检查**
   ```typescript
   "redirect" in result
   ```
   - 包含 `redirect` 属性

3. **排除 VNode（关键）**
   ```typescript
   !("type" in result) && !("props" in result)
   ```
   - 不包含 `type` 属性
   - 不包含 `props` 属性
   - **确保不是 Preact VNode**

### 为什么这样检查有效

**Preact VNode：**
```typescript
{
  type: "div" | Component,
  props: { className: "..." },
  key: null,
  ref: null,
  // ...
}
```
- ✅ 有 `type` 属性 → 被排除

**重定向对象：**
```typescript
{
  redirect: "/",
  status: 302
}
```
- ✅ 有 `redirect` 属性
- ✅ 没有 `type` 属性
- ✅ 没有 `props` 属性
- → 被识别为重定向

**普通 JSX 元素：**
```tsx
<div>Hello</div>
```
- 渲染为 VNode
- 有 `type` 和 `props`
- → 被排除，继续正常渲染

## 返回值类型检测顺序

```typescript
const result = await pageFn(context);

// 1. 检查重定向对象
if (isRedirectResult(result)) {
  return handleRedirect(result);
}

// 2. 检查 Response 对象
if (result instanceof Response) {
  return result;
}

// 3. 假设是 JSX 元素，渲染为 HTML
const html = renderJSX(result);
return new Response(html, {
  status: 200,
  headers: { "Content-Type": "text/html; charset=utf-8" }
});
```

## 测试验证

### 测试页面

**redirect_simple.tsx：**
```tsx
export default async function (_context: PageContext) {
  return { redirect: "/" };
}
```

**预期行为：**
- 返回 `{ redirect: "/" }`
- 检测为重定向对象
- 返回 302 响应，Location: /

### 测试结果

**修复前：**
```
测试请求: http://localhost:9001/redirect_simple.tsx
✓ 状态码: 200  ❌ 错误
```

**修复后：**
```
[重定向] www/redirect_simple.tsx -> / (302)
测试请求: http://localhost:9001/redirect_simple.tsx
✓ 状态码: 302  ✅ 正确
```

## 其他修复

### 添加类型保护函数（可选优化）

可以提取为独立的类型保护函数：

```typescript
/**
 * 检查是否是重定向结果
 * 排除 Preact VNode 和其他对象
 */
function isRedirectResult(value: unknown): value is RedirectResult {
  return (
    value !== null &&
    typeof value === "object" &&
    "redirect" in value &&
    !("type" in value) &&
    !("props" in value) &&
    !("key" in value) &&
    !("ref" in value)
  );
}

// 使用
if (isRedirectResult(result)) {
  const redirectResult = result;
  // ...
}
```

### 更严格的类型检查

也可以使用更精确的检查：

```typescript
function isPlainObject(value: unknown): boolean {
  if (value === null || typeof value !== "object") {
    return false;
  }

  // 检查原型链
  const proto = Object.getPrototypeOf(value);
  return proto === null || proto === Object.prototype;
}

// 使用
if (isPlainObject(result) && "redirect" in result) {
  // 确保是纯对象，不是 VNode 实例
}
```

## 相关代码修改

**文件：** src/main.ts

**第 162-183 行：**
```typescript
// 检查是否是重定向对象
if (result &&
    typeof result === "object" &&
    "redirect" in result &&
    !("type" in result) &&    // 新增：排除 VNode
    !("props" in result)) {   // 新增：排除 VNode

  const redirectResult = result as RedirectResult;
  const targetUrl = redirectResult.redirect;
  const status = redirectResult.status ?? 302;

  // 验证重定向状态码
  const validStatuses = [301, 302, 303, 307, 308];
  const finalStatus = validStatuses.includes(status) ? status : 302;

  if (config.dev) {  // 新增：开发模式日志
    console.log(`[重定向] ${filepath} -> ${targetUrl} (${finalStatus})`);
  }

  return new Response(null, {
    status: finalStatus,
    headers: {
      "Location": targetUrl,
    },
  });
}
```

## 验证测试

### 运行测试

```bash
# 类型检查
deno check src/main.ts

# 二进制测试
deno task test:binary

# 预期结果
✓ redirect_simple.tsx - 状态码: 302
✓ redirect.tsx - 状态码: 200
✓ redirect.tsx?to=home - 状态码: 302
✓ 所有测试通过
```

### 手动测试

```bash
# 启动服务器
deno run --allow-net --allow-read src/main.ts --dev

# 测试重定向
curl -I http://localhost:9000/redirect_simple.tsx

# 预期输出
# HTTP/1.1 302 Found
# Location: /
```

## 开发模式日志示例

**启动服务器：**
```bash
deno run --allow-net --allow-read src/main.ts --dev
```

**访问重定向页面：**
```
[重定向] www/redirect_simple.tsx -> / (302)
[重定向] www/redirect.tsx -> / (302)
```

**访问普通页面：**
```
（无重定向日志）
```

## 关键要点

1. **类型安全很重要**
   - 简单的 `in` 检查不够精确
   - 需要排除其他可能的类型

2. **理解框架的数据结构**
   - Preact VNode 有特定的属性
   - 需要区分 VNode 和普通对象

3. **使用排除法**
   - 通过排除已知属性来识别类型
   - 比检查所有可能的值更可靠

4. **添加调试日志**
   - 开发模式下输出关键信息
   - 便于排查问题

## 更新时间

2026-01-27
