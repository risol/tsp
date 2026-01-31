# Cookie 管理 - 快速参考

## 基本用法

### 设置 Cookie
```tsx
export default Page(, async function(ctx, { cookies }) {
  cookies.set('name', 'value');
  return <div>Done!</div>;
});
```

### 读取 Cookie
```tsx
export default Page(async function(ctx) {
  const value = ctx.cookies.cookieName || 'default';
  return <div>{value}</div>;
});
```

### 删除 Cookie
```tsx
cookies.delete('cookieName', { path: '/' });
```

## Cookie 选项

```tsx
cookies.set('session', 'abc123', {
  expires: new Date('2025-12-31'),  // 或 maxAge
  maxAge: 3600,                      // 秒（优先级更高）
  domain: '.example.com',            // 可选
  path: '/',                         // 可选
  secure: true,                      // 仅 HTTPS
  httpOnly: true,                    // 禁止 JS 访问
  sameSite: 'Strict',                // 'Strict' | 'Lax' | 'None'
});
```

## 批量操作

### 批量设置
```tsx
cookies.setMultiple({
  'theme': { value: 'dark', options: { maxAge: 31536000 } },
  'lang': { value: 'en', options: { maxAge: 31536000 } },
});
```

### 批量删除
```tsx
cookies.deleteMultiple(['cookie1', 'cookie2'], { path: '/' });
```

## 安全清单

- ✅ 对 session cookie 使用 `httpOnly: true`
- ✅ 在 HTTPS 网站上使用 `secure: true`
- ✅ 使用 `sameSite: 'Strict'` 或 `'Lax'` 进行 CSRF 防护
- ✅ 优先使用 `maxAge` 而非 `expires`
- ✅ 设置显式的 `path`（通常为 `'/'`）
- ✅ 除非需要，否则避免使用 `domain`

## 示例

### 认证 Cookie
```tsx
cookies.set('sessionId', userId, {
  httpOnly: true,
  secure: true,
  sameSite: 'Strict',
  maxAge: 3600,
  path: '/',
});
```

### 用户偏好
```tsx
cookies.set('theme', 'dark', { maxAge: 31536000, path: '/' });
```

### Cookie 同意
```tsx
cookies.set('consent', 'accepted', { maxAge: 31536000, path: '/' });
```

## 测试

运行测试：
```bash
# 单元测试
deno test --allow-all tests/unit/cookie_test.ts

# E2E 测试（先启动服务器）
deno task dev
# 访问: http://localhost:9000/cookie_test.tsx

# 快速验证
deno run --allow-all tests/verify_cookies.ts
```

## 文件

- **实现**：`src/cookies.ts`
- **类型**：`types.d.ts`（AppDeps 接口）
- **集成**：`src/main.ts`（注册 + 响应处理）
- **测试**：`tests/unit/cookie_test.ts`
- **E2E**：`tests/test_www/cookie_test.tsx`
- **演示**：`www/cookie_demo.tsx`
- **文档**：`docs/features/cookies.md`

## 常见问题

**Cookie 未被设置？**
- 检查 domain/path 是否与站点匹配
- 检查 `secure` 标志（需要 HTTPS）
- 检查浏览器控制台是否有错误
- 在开发工具中检查响应头

**Cookie 未被读取？**
- 使用 `ctx.cookies.cookieName`（而非 cookies 管理器）
- 检查确切的名称（区分大小写）
- 验证 domain/path 是否匹配

**删除无效？**
- 必须与设置时的 domain/path 匹配
- 检查响应头中是否有带有 `Max-Age=0` 的 Set-Cookie

## 参见

- [完整文档](./cookies.md)
- [实现总结](./cookies-implementation-summary.md)
- [依赖注入](../injection.md)
