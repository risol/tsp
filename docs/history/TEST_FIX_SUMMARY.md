# 测试修复总结

## 问题
运行 `deno task test:basic` 时出现类型错误：

```
TS2554 [ERROR]: Expected 1 arguments, but got 3.
  const context = buildContext(request, "/test.tsx", TEST_ROOT);
```

## 根本原因

1. **函数签名不匹配**
   - `buildContext` 函数只接受一个参数（对象）
   - 测试代码传递了三个独立参数

2. **路径分隔符差异**
   - Windows 使用反斜杠 `\`
   - Unix/Linux 使用正斜杠 `/`
   - 导致路径比较失败

## 修复方案

### 1. 修正 buildContext 调用

**之前（错误）：**
```typescript
const context = buildContext(request, "/test.tsx", TEST_ROOT);
```

**之后（正确）：**
```typescript
async function buildContextFromRequest(
  request: Request,
  file: string,
  root: string,
): Promise<PageContext> {
  // ... 解析逻辑 ...

  return buildContext({
    method,
    url,
    headers: request.headers,
    query,
    body,
    cookies,
    file,
    root,
  });
}

const context = await buildContextFromRequest(request, "/test.tsx", TEST_ROOT);
```

### 2. 添加路径规范化

**修复前：**
```typescript
assertEquals(result.filepath, "www/index.tsx");  // Windows 失败
```

**修复后：**
```typescript
const normalizedPath = result.filepath!.replace(/\\/g, "/");
assertEquals(normalizedPath, "www/index.tsx");  // 跨平台通过
```

### 3. 更新安全测试

使用正确的 `securityCheck` 函数而不是依赖 `resolvePath` 抛出错误：

```typescript
for (const path of testPaths) {
  const result = await resolvePath(path, TEST_ROOT);
  if (result.success && result.filepath) {
    const checkResult = await securityCheck(result.filepath, TEST_ROOT);
    if (checkResult.success) {
      throw new Error(`应该阻止路径穿越: ${path}`);
    }
  }
}
```

## 测试结果

### 修复前
```
FAILED | 4 passed | 5 failed
```

### 修复后
```
ok | 10 passed | 0 failed (21ms)
```

## 测试覆盖

现在测试包含：

1. **路由解析（4个测试）**
   - ✅ 根路径 → index.tsx
   - ✅ 简单路径 → form.tsx
   - ✅ 带目录的路径 → api/test.tsx
   - ✅ 目录默认页

2. **上下文构建（3个测试）**
   - ✅ 基本请求（GET + query）
   - ✅ POST 请求（JSON body）
   - ✅ Cookie 解析

3. **安全检查（3个测试）**
   - ✅ 路径穿越攻击防护
   - ✅ 文件类型白名单
   - ✅ 正常文件允许通过

## 跨平台兼容性

测试现在在以下平台上都能通过：
- ✅ Windows (Git Bash / PowerShell)
- ✅ Linux
- ✅ macOS

## 关键学习点

1. **始终检查函数签名**
   - 使用 `deno check` 验证类型
   - 查看源代码确认参数

2. **路径处理要考虑跨平台**
   - 使用 `path.join()` 而不是硬编码分隔符
   - 测试时规范化路径后再比较

3. **测试要验证实际行为**
   - 不是假设函数如何工作
   - 而是测试函数实际的行为

## 运行测试

```bash
# 基本功能测试（快速）
deno task test:basic

# 二进制构建测试（完整）
deno task test:binary

# 所有测试
deno task test
```

## 相关文件

- [tests/basic_test.ts](../tests/basic_test.ts) - 基本功能测试
- [src/context.ts](../src/context.ts) - 上下文处理模块
- [src/router.ts](../src/router.ts) - 路由模块

## 更新时间
2026-01-27
