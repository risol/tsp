# TSP 中的 Cookie 管理

TSP 通过依赖注入提供强大、类型安全的 cookie 管理系统。该功能允许你设置、删除和管理 HTTP cookie，并拥有完整的 TypeScript 支持。

## 概述

Cookie 管理系统构建在 TSP 的依赖注入框架之上，提供：

- **类型安全的 API**，完整的 TypeScript IntelliSense 支持
- **标准 cookie 选项**支持（expires、maxAge、domain、path、secure、httpOnly、sameSite）
- **自动 URL 编码**，用于 cookie 名称和值
- **批量操作**，一次性设置/删除多个 cookie
- **无缝集成**，与重定向和响应配合使用

## 基本用法

### 设置 Cookie

```tsx
export default Page(, async function(ctx, { cookies }) {
  // 设置简单的 cookie
  cookies.set('username', 'john_doe');

  return <div>Cookie set!</div>;
});
```

### 读取 Cookie

请求 cookie 会自动解析并在 `ctx.cookies` 中可用：

```tsx
export default Page(, async function(ctx, { cookies }) {
  // 读取现有 cookie
  const username = ctx.cookies.username || 'Guest';

  return <div>Hello, {username}!</div>;
});
```

### 删除 Cookie

```tsx
export default Page(, async function(ctx, { cookies }) {
  // 删除 cookie
  cookies.delete('username');

  return <div>Cookie deleted!</div>;
});
```

## Cookie 选项

你可以在设置 cookie 时指定各种选项：

```tsx
export default Page(, async function(ctx, { cookies }) {
  cookies.set('sessionId', 'abc123', {
    // Cookie 过期（maxAge 优先）
    maxAge: 3600,           // 1小时后过期（单位：秒）
    expires: new Date(),    // 或指定确切的过期日期

    // 作用域
    domain: '.example.com', // Cookie 域名
    path: '/',              // Cookie 路径

    // 安全性
    secure: true,           // 仅通过 HTTPS 发送
    httpOnly: true,         // 禁止 JavaScript 访问
    sameSite: 'Strict',     // CSRF 防护：'Strict' | 'Lax' | 'None'
  });

  return <div>Secure cookie set!</div>;
});
```

### 选项参考

| 选项 | 类型 | 说明 |
|--------|------|-------------|
| `expires` | `string \| Date` | 绝对过期日期（GMT 字符串或 Date 对象） |
| `maxAge` | `number` | 最大存活时间（秒），优先于 `expires` |
| `domain` | `string` | 域名限制（例如 `.example.com`） |
| `path` | `string` | 路径限制（例如 `/`） |
| `secure` | `boolean` | 仅通过 HTTPS 发送 cookie |
| `httpOnly` | `boolean` | 防止 JavaScript 访问（XSS 防护） |
| `sameSite` | `'Strict' \| 'Lax' \| 'None'` | CSRF 防护 |

## 高级用法

### 批量操作

一次性设置多个 cookie：

```tsx
export default Page(, async function(ctx, { cookies }) {
  cookies.setMultiple({
    'theme': { value: 'dark', options: { maxAge: 31536000 } },
    'language': { value: 'zh-CN', options: { maxAge: 31536000 } },
    'fontSize': { value: '14px', options: { maxAge: 31536000 } },
  });

  return <div>Preferences saved!</div>;
});
```

删除多个 cookie：

```tsx
export default Page(, async function(ctx, { cookies }) {
  cookies.deleteMultiple(['temp1', 'temp2', 'temp3']);

  return <div>Temporary cookies cleared!</div>;
});
```

### Cookie 与重定向

Cookie 会自动添加到重定向响应中：

```tsx
export default Page(, async function(ctx, { cookies }) {
  // 重定向前设置 cookie
  cookies.set('justLoggedIn', 'true', { maxAge: 10 });

  // Cookie 将随此重定向一起发送
  return {
    redirect: '/dashboard',
    status: 302,
  };
});
```

### 特殊字符和 Unicode

Cookie 名称和值会自动进行 URL 编码：

```tsx
export default Page(, async function(ctx, { cookies }) {
  // 特殊字符会自动编码
  cookies.set('user name', 'John Doe');
  cookies.set('email', 'test@example.com');
  cookies.set('chinese', '张三');
  cookies.set('emoji', '😀');

  return <div>Special cookies set!</div>;
});
```

## 安全最佳实践

### 1. 对敏感 Cookie 使用 HttpOnly

防止 XSS 攻击通过 JavaScript 读取 cookie：

```tsx
cookies.set('sessionId', 'abc123', {
  httpOnly: true,  // ✅ 推荐用于 session cookie
});
```

### 2. 在仅 HTTPS 网站使用 Secure

确保 cookie 仅通过 HTTPS 发送：

```tsx
cookies.set('sessionId', 'abc123', {
  secure: true,  // ✅ SameSite=None 所必需
});
```

### 3. 设置 SameSite 进行 CSRF 防护

```tsx
cookies.set('sessionId', 'abc123', {
  sameSite: 'Strict',  // ✅ CSRF 防护最佳选择
  // 或 'Lax' 以获得更好的可用性
});
```

**SameSite 模式：**
- `Strict`：最强保护，从外部站点链接时不发送 cookie
- `Lax`：平衡，在顶级导航时发送 cookie
- `None`：无限制，需要 `Secure: true`

### 4. 使用 Max-Age 而非 Expires

`maxAge` 更可靠，因为它是相对于当前时间的：

```tsx
cookies.set('sessionId', 'abc123', {
  maxAge: 3600,  // ✅ 推荐（相对时间）
  // expires: new Date(Date.now() + 3600 * 1000),  // ❌ 避免
});
```

### 5. 设置显式的 Path 和 Domain

防止 cookie 泄漏到其他路径/子域名：

```tsx
cookies.set('sessionId', 'abc123', {
  path: '/',              // ✅ 显式路径
  domain: undefined,      // ✅ 仅当前主机（推荐）
  // domain: '.example.com',  // ❌ 除非需要，否则避免
});
```

## 真实场景示例

### 用户认证

```tsx
export default Page(, async function(ctx, { cookies, db }) {
  const { username, password } = ctx.body as { username: string; password: string };

  // 验证凭据
  const user = await db.query(
    'SELECT * FROM users WHERE username = ? AND password = ?',
    [username, password]
  );

  if (user.length > 0) {
    // 设置安全的 session cookie
    cookies.set('sessionId', user[0].id, {
      httpOnly: true,
      secure: true,
      sameSite: 'Strict',
      maxAge: 3600,  // 1 小时
      path: '/',
    });

    return { redirect: '/dashboard', status: 302 };
  }

  return <div>Invalid credentials</div>;
});
```

### 用户偏好设置

```tsx
export default Page(, async function(ctx, { cookies }) {
  const { theme, language } = ctx.body as { theme: string; language: string };

  // 设置长期偏好设置 cookie
  cookies.setMultiple({
    'theme': { value: theme, options: { maxAge: 31536000, path: '/' } },
    'language': { value: language, options: { maxAge: 31536000, path: '/' } },
  });

  return <div>Preferences saved!</div>;
});
```

### Cookie 同意

```tsx
export default Page(, async function(ctx, { cookies }) {
  const consent = ctx.query.consent;

  if (consent === 'accept') {
    cookies.set('cookieConsent', 'accepted', {
      maxAge: 31536000,  // 1 年
      path: '/',
    });
  } else if (consent === 'decline') {
    cookies.set('cookieConsent', 'declined', {
      maxAge: 31536000,
      path: '/',
    });
  }

  return <div>Preference recorded</div>;
});
```

### 追踪最后访问时间

```tsx
export default Page(, async function(ctx, { cookies }) {
  const lastVisit = ctx.cookies.lastVisit;
  const now = new Date().toISOString();

  // 更新最后访问时间 cookie
  cookies.set('lastVisit', now, {
    maxAge: 31536000,  // 1 年
    path: '/',
  });

  return (
    <div>
      {lastVisit
        ? <div>Welcome back! Last visit: {lastVisit}</div>
        : <div>Welcome, first-time visitor!</div>
      }
    </div>
  );
});
```

## 工作原理

### 基于 WeakMap 的存储

Cookie 系统使用 `WeakMap<PageContext, RequestContext>` 来存储每个请求的 cookie 操作：

- **键**：PageContext 对象
- **值**：Set-Cookie 头字符串数组

这种设计确保：
- **自动清理**：当 PageContext 被垃圾回收时，cookie 上下文自动释放
- **请求隔离**：不同请求的 cookie 操作相互独立
- **无内存泄漏**：WeakMap 不会阻止垃圾回收

### 响应集成

页面函数执行后，TSP 会：

1. 从 WeakMap 提取 cookie 操作
2. 将它们序列化为 Set-Cookie 头
3. 添加到 HTTP 响应中（发送前）
4. 适用于所有响应类型（重定向、Response 对象、JSX）

### URL 编码

Cookie 名称和值会自动进行 `encodeURIComponent()` 编码：

```tsx
cookies.set('user name', 'John Doe');
// 变为: "user%20name=John%20Doe"
```

`ctx.cookies` 中的请求 cookie 会自动解码：

```tsx
const value = ctx.cookies['user name'];  // "John Doe"
```

## 测试

TSP 包含全面的 cookie 测试：

```bash
# 运行单元测试
deno test --allow-all tests/unit/cookie_test.ts

# 运行 E2E 测试（需要服务器）
deno task dev
# 访问: http://localhost:9000/cookie_test.tsx
```

### 手动测试清单

- [ ] 基础 cookie 设置
- [ ] 带所有选项的 cookie
- [ ] Cookie 删除
- [ ] 批量操作
- [ ] Cookie 与重定向
- [ ] 特殊字符和 Unicode
- [ ] HttpOnly cookie（使用开发工具验证）
- [ ] Secure cookie（需要 HTTPS）
- [ ] SameSite 行为

## 故障排除

### Cookie 未被设置

1. **检查浏览器控制台**是否有错误
2. **验证 domain/path** 是否与站点 URL 匹配
3. **检查 Secure 标志**（在 HTTP 上无效）
4. **在浏览器开发工具中检查响应头**
5. **验证 SameSite 设置**（SameSite=None 需要 Secure）

### Cookie 未被读取

1. **检查 `ctx.cookies`**（请求 cookie，非响应 cookie）
2. **验证 cookie 名称**（区分大小写）
3. **检查 domain/path** 是否匹配
4. **验证 cookie 未过期**

### 删除无效

1. **匹配设置时的 domain/path**
2. **检查响应头**中是否有带有 `Max-Age=0` 的 Set-Cookie
3. **验证浏览器遵守删除**（某些浏览器有 bug）

## API 参考

### CookieManager 接口

```typescript
interface CookieManager {
  /** 设置单个 cookie */
  set: (name: string, value: string, options?: CookieOptions) => void;

  /** 删除单个 cookie */
  delete: (name: string, options?: Pick<CookieOptions, 'domain' | 'path'>) => void;

  /** 一次性设置多个 cookie */
  setMultiple: (cookies: Record<string, { value: string; options?: CookieOptions }>) => void;

  /** 一次性删除多个 cookie */
  deleteMultiple: (names: string[], options?: Pick<CookieOptions, 'domain' | 'path'>) => void;
}
```

### CookieOptions 接口

```typescript
interface CookieOptions {
  /** 过期日期（GMT 字符串或 Date 对象） */
  expires?: string | Date;

  /** 最大存活时间（秒） */
  maxAge?: number;

  /** 域名限制 */
  domain?: string;

  /** 路径限制 */
  path?: string;

  /** 仅 HTTPS */
  secure?: boolean;

  /** 禁止 JavaScript 访问 */
  httpOnly?: boolean;

  /** Same-site 策略 */
  sameSite?: 'Strict' | 'Lax' | 'None';
}
```

## 参见

- [依赖注入](../injection.md)
- [页面上下文](../page-context.md)
- [安全最佳实践](../security.md)
- [E2E 测试](../../tests/test_www/cookie_test.tsx)
