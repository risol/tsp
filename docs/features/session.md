# Session 管理

TSP 提供了一个安全、类型安全的 session 管理系统，基于 cookie 功能构建。Session 让你可以在多个请求之间维护用户状态，并内置安全功能。

## 特性

- **安全**：HMAC-SHA256 签名的 session ID 防止伪造
- **基于 Cookie**：使用 httpOnly + secure + sameSite 提供保护
- **内存存储**：快速的 session 数据访问，自动清理
- **滑动过期**：可选的自动刷新 session 超时时间
- **Session 固定防护**：登录时重新生成 session ID
- **类型安全**：完整的 TypeScript 支持和依赖注入

## 快速开始

### 1. 启用 Session 依赖

Session 在 `src/main.ts` 中自动注册，无需额外设置。

### 2. 在页面中使用 Session

```tsx
export default Page(async function(ctx, { session }) {
  const user = await session.getUser();

  if (!user) {
    // 如果未认证，重定向到登录页
    return { redirect: '/login.tsx', status: 302 };
  }

  return <div>欢迎, {user.name}!</div>;
});
```

## SessionManager API

`SessionManager` 提供以下方法：

### 用户管理

#### `getUser(): Promise<SessionUser | null>`

从 session 中获取当前用户。

```tsx
const user = await session.getUser();
if (user) {
  console.log(`用户: ${user.name} (${user.email})`);
}
```

#### `login(userId: string, userData?: Partial<SessionUser>): Promise<void>`

为用户创建或更新 session。

```tsx
await session.login('user-123', {
  name: '张三',
  email: 'zhangsan@example.com',
  role: 'admin',
});
```

#### `logout(): Promise<void>`

销毁当前 session。

```tsx
await session.logout();
```

### 数据存储

#### `set(key: string, value: unknown): Promise<void>`

在 session 中存储数据。

```tsx
await session.set('cart', ['商品1', '商品2']);
await session.set('preferences', { theme: 'dark' });
await session.set('visits', 5);
```

#### `get<T>(key: string): Promise<T | null>`

从 session 中检索数据。

```tsx
const cart = await session.get<string[]>('cart');
const visits = await session.get<number>('visits');
```

#### `delete(key: string): Promise<void>`

从 session 中删除数据。

```tsx
await session.delete('cart');
```

### Session 管理

#### `regenerateId(): Promise<void>`

生成新的 session ID（防止 session 固定攻击）。

```tsx
// 在登录等敏感操作后
await session.regenerateId();
```

#### `touch(): Promise<void>`

刷新 session 过期时间。

```tsx
await session.touch();
```

#### `isValid(): Promise<boolean>`

检查当前 session 是否有效。

```tsx
const valid = await session.isValid();
```

#### `getId(): string`

获取当前 session ID。

```tsx
const sessionId = session.getId();
```

## 常见模式

### 认证流程

#### 登录页面

```tsx
export default Page(async function(ctx, { session }) {
  if (ctx.method === 'POST') {
    const { username, password } = ctx.body as { username: string; password: string };

    // 验证凭证（例如，从数据库）
    if (username === 'admin' && password === 'password') {
      // 创建 session
      await session.login('user-123', {
        name: '管理员',
        email: 'admin@example.com',
        role: 'admin',
      });

      // 重新生成 ID 防止 session 固定
      await session.regenerateId();

      return { redirect: '/dashboard.tsx', status: 302 };
    }

    return <div>凭证无效</div>;
  }

  return (
    <form method="POST">
      <input type="text" name="username" placeholder="用户名" />
      <input type="password" name="password" placeholder="密码" />
      <button type="submit">登录</button>
    </form>
  );
});
```

#### 受保护页面

```tsx
export default Page(async function(ctx, { session }) {
  // 检查认证
  const user = await session.getUser();
  if (!user) {
    return { redirect: '/login.tsx', status: 302 };
  }

  // 更新访问计数
  const visits = await session.get<number>('visits') || 0;
  await session.set('visits', visits + 1);

  return (
    <div>
      <h1>欢迎, {user.name}!</h1>
      <p>访问次数: {visits + 1}</p>
      <a href="/logout.tsx">登出</a>
    </div>
  );
});
```

#### 登出页面

```tsx
export default Page(async function(ctx, { session }) {
  await session.logout();
  return { redirect: '/login.tsx', status: 302 };
});
```

### 购物车

```tsx
export default Page(async function(ctx, { session }) {
  // 获取当前购物车
  const cart = await session.get<string[]>('cart') || [];

  if (ctx.method === 'POST') {
    const { action, item } = ctx.body as { action: string; item: string };

    if (action === 'add') {
      cart.push(item);
      await session.set('cart', cart);
    } else if (action === 'remove') {
      const newCart = cart.filter(i => i !== item);
      await session.set('cart', newCart);
    }

    return { redirect: '/cart.tsx', status: 302 };
  }

  return (
    <div>
      <h1>购物车</h1>
      <ul>
        {cart.map(item => <li key={item}>{item}</li>)}
      </ul>

      <form method="POST">
        <input type="text" name="item" placeholder="商品名称" />
        <input type="hidden" name="action" value="add" />
        <button type="submit">添加到购物车</button>
      </form>
    </div>
  );
});
```

### Flash 消息

```tsx
export default Page(async function(ctx, { session }) {
  // 获取并清除 flash 消息
  const flash = await session.get<string>('flash');
  if (flash) {
    await session.delete('flash');
  }

  return (
    <div>
      {flash && <div class="flash">{flash}</div>}
      <h1>页面内容</h1>
    </div>
  );
});

// 在其他地方设置 flash 消息
await session.set('flash', '操作成功！');
```

## 配置

### 环境变量

```bash
# .env
TSP_SESSION_SECRET=your-random-secret-key-min-32-chars
TSP_SESSION_MAX_AGE=86400
```

### 程序化配置

你可以在 `src/main.ts` 中修改 session 选项：

```typescript
const options = {
  ...getDefaultOptions(),
  maxAge: 3600,  // 1 小时（秒）
  secret: new TextEncoder().encode(Deno.env.get('TSP_SESSION_SECRET')),
  cookieName: 'myapp_session',
  secure: true,
  httpOnly: true,
  sameSite: 'Strict',
};

sessionStore = new SessionStore(options);
```

### Session 选项

| 选项 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `cookieName` | string | `'tsp_session'` | session ID 的 cookie 名称 |
| `maxAge` | number | `86400` | session 最长有效期（秒，1 天） |
| `cleanupInterval` | number | `300000` | 清理间隔（毫秒，5 分钟） |
| `secret` | Uint8Array | 自动生成 | HMAC 签名的密钥 |
| `secure` | boolean | `true` | 设置 cookie 的 secure 标志 |
| `httpOnly` | boolean | `true` | 设置 cookie 的 httpOnly 标志 |
| `sameSite` | string | `'Strict'` | SameSite 属性（'Strict'、'Lax'、'None'）|
| `path` | string | `'/'` | Cookie 路径 |
| `rolling` | boolean | `true` | 启用滚动 session（访问时刷新）|
| `autoTouch` | boolean | `true` | 访问时自动刷新 session |

## 安全最佳实践

### 1. 生产环境始终使用 HTTPS

```typescript
const options = {
  ...getDefaultOptions(),
  secure: true,  // 需要 HTTPS
};
```

### 2. 设置强 Session 密钥

```bash
# 生成随机密钥
openssl rand -base64 32
```

```bash
# .env
TSP_SESSION_SECRET=<generated-secret>
```

### 3. 登录后重新生成 Session ID

```tsx
await session.login(userId, userData);
await session.regenerateId();  // 防止 session 固定
```

### 4. 使用适当的 Cookie 设置

- **httpOnly**：防止 JavaScript 访问 cookie（防止 XSS 窃取）
- **secure**：仅通过 HTTPS 发送 cookie
- **sameSite**：防止 CSRF 攻击

### 5. 设置合理的过期时间

```typescript
const options = {
  ...getDefaultOptions(),
  maxAge: 3600,  // 敏感应用 1 小时
  // maxAge: 86400,  // 一般应用 1 天
  // maxAge: 604800,  // "记住我"功能 1 周
};
```

### 6. 实现正确的登出

```tsx
export default Page(async function(ctx, { session }) {
  await session.logout();  // 销毁 session
  return { redirect: '/login.tsx', status: 302 };
});
```

## Session 存储

### 当前限制

- Session 存储在内存中
- 服务器重启时 session 会丢失
- 不适合分布式/集群部署

### 未来增强

计划支持：
- Redis 存储
- 数据库存储
- 分布式 session 存储
- 跨重启的 session 持久化

## 故障排查

### Session 未持久化

**问题**：页面刷新时 session 数据丢失。

**解决方案**：检查：
1. 浏览器中启用了 cookie
2. HTTP 开发环境将 `secure` 标志设置为 `false`
3. 浏览器未阻止第三方 cookie

### Session ID 未更改

**问题**：`regenerateId()` 似乎不起作用。

**解决方案**：确保在调用 `regenerateId()` 后检查 `session.getId()`。

### 生产部署

**问题**：生产环境中 session 失败。

**解决方案**：
1. 设置 `TSP_SESSION_SECRET` 环境变量
2. 使用 HTTPS（设置 `secure: true`）
3. 如果使用子域名，检查 cookie 域名设置

### 内存问题

**问题**：太多 session 消耗内存。

**解决方案**：
1. 减少 `maxAge` 使 session 更快过期
2. 调整 `cleanupInterval` 更频繁地运行
3. 在生产环境中监控 session 数量

## 演示

运行开发服务器时，可以在 `/session_demo.tsx` 访问完整的演示：

```bash
deno task dev
# 访问 http://localhost:9000/session_demo.tsx
```

演示内容包括：
- 登录/登出
- Session 数据存储
- Session 重新生成
- 用户信息
- Session 状态

## 类型定义

```typescript
interface SessionUser {
  id: string;
  name: string;
  email?: string;
  role?: string;
  [key: string]: unknown;
}

interface SessionManager {
  getUser(): Promise<SessionUser | null>;
  login(userId: string, userData?: Partial<SessionUser>): Promise<void>;
  logout(): Promise<void>;
  set(key: string, value: unknown): Promise<void>;
  get<T = unknown>(key: string): Promise<T | null>;
  delete(key: string): Promise<void>;
  regenerateId(): Promise<void>;
  touch(): Promise<void>;
  isValid(): Promise<boolean>;
  getId(): string;
}
```

## 参见

- [Cookie 管理](./cookies.md)
- [依赖注入](./injection.md)
- [AppDeps 使用指南](./appdeps.md)

---

[← 返回功能特性](./README.md) | [← 返回文档中心](../README.md)
