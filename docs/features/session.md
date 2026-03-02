# Session Management

TSP provides a secure, type-safe session management system, built on top of the cookie functionality. Sessions allow you to maintain user state across multiple requests with built-in security features.

## Features

- **Secure**: HMAC-SHA256 signed session IDs prevent forgery
- **Cookie-based**: Uses httpOnly + secure + sameSite for protection
- **In-memory storage**: Fast session data access, automatic cleanup
- **Sliding expiration**: Optional automatic refresh of session timeout
- **Session fixation protection**: Regenerate session ID on login
- **Type-safe**: Complete TypeScript support and dependency injection

## Quick Start

### 1. Enable Session Dependency

Session is automatically registered in `src/main.ts`, no additional setup needed.

### 2. Use Session in Pages

```tsx
export default Page(async function(ctx, { session }) {
  const user = await session.getUser();

  if (!user) {
    // If not authenticated, redirect to login page
    return { redirect: '/login.tsx', status: 302 };
  }

  return <div>Welcome, {user.name}!</div>;
});
```

## SessionManager API

`SessionManager` provides the following methods:

### User Management

#### `getUser(): Promise<SessionUser | null>`

Get current user from session.

```tsx
const user = await session.getUser();
if (user) {
  console.log(`User: ${user.name} (${user.email})`);
}
```

#### `login(userId: string, userData?: Partial<SessionUser>): Promise<void>`

Create or update session for user.

```tsx
await session.login('user-123', {
  name: 'John Doe',
  email: 'john@example.com',
  role: 'admin',
});
```

#### `logout(): Promise<void>`

Destroy current session.

```tsx
await session.logout();
```

### Data Storage

#### `set(key: string, value: unknown): Promise<void>`

Store data in session.

```tsx
await session.set('cart', ['Item 1', 'Item 2']);
await session.set('preferences', { theme: 'dark' });
await session.set('visits', 5);
```

#### `get<T>(key: string): Promise<T | null>`

Retrieve data from session.

```tsx
const cart = await session.get<string[]>('cart');
const visits = await session.get<number>('visits');
```

#### `delete(key: string): Promise<void>`

Delete data from session.

```tsx
await session.delete('cart');
```

### Session Management

#### `(): Promise<void>regenerateId`

Generate new session ID (prevent session fixation attacks).

```tsx
// After sensitive operations like login
await session.regenerateId();
```

#### `touch(): Promise<void>`

Refresh session expiration time.

```tsx
await session.touch();
```

#### `isValid(): Promise<boolean>`

Check if current session is valid.

```tsx
const valid = await session.isValid();
```

#### `getId(): string`

Get current session ID.

```tsx
const sessionId = session.getId();
```

## Common Patterns

### Authentication Flow

#### Login Page

```tsx
export default Page(async function(ctx, { session }) {
  if (ctx.method === 'POST') {
    const { username, password } = ctx.body as { username: string; password: string };

    // Validate credentials (e.g., from database)
    if (username === 'admin' && password === 'password') {
      // Create session
      await session.login('user-123', {
        name: 'Administrator',
        email: 'admin@example.com',
        role: 'admin',
      });

      // Regenerate ID to prevent session fixation
      await session.regenerateId();

      return { redirect: '/dashboard.tsx', status: 302 };
    }

    return <div>Invalid credentials</div>;
  }

  return (
    <form method="POST">
      <input type="text" name="username" placeholder="Username" />
      <input type="password" name="password" placeholder="Password" />
      <button type="submit">Login</button>
    </form>
  );
});
```

#### Protected Page

```tsx
export default Page(async function(ctx, { session }) {
  // Check authentication
  const user = await session.getUser();
  if (!user) {
    return { redirect: '/login.tsx', status: 302 };
  }

  // Update visit count
  const visits = await session.get<number>('visits') || 0;
  await session.set('visits', visits + 1);

  return (
    <div>
      <h1>Welcome, {user.name}!</h1>
      <p>Visit Count: {visits + 1}</p>
      <a href="/logout.tsx">Logout</a>
    </div>
  );
});
```

#### Logout Page

```tsx
export default Page(async function(ctx, { session }) {
  await session.logout();
  return { redirect: '/login.tsx', status: 302 };
});
```

### Shopping Cart

```tsx
export default Page(async function(ctx, { session }) {
  // Get current shopping cart
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
      <h1>Shopping Cart</h1>
      <ul>
        {cart.map(item => <li key={item}>{item}</li>)}
      </ul>

      <form method="POST">
        <input type="text" name="item" placeholder="Item name" />
        <input type="hidden" name="action" value="add" />
        <button type="submit">Add to Cart</button>
      </form>
    </div>
  );
});
```

### Flash Messages

```tsx
export default Page(async function(ctx, { session }) {
  // Get and clear flash message
  const flash = await session.get<string>('flash');
  if (flash) {
    await session.delete('flash');
  }

  return (
    <div>
      {flash && <div class="flash">{flash}</div>}
      <h1>Page Content</h1>
    </div>
  );
});

// Set flash message elsewhere
await session.set('flash', 'Operation successful!');
```

## Configuration

### Environment Variables

```bash
# .env
TSP_SESSION_SECRET=your-random-secret-key-min-32-chars
TSP_SESSION_MAX_AGE=86400
```

### Programmatic Configuration

You can modify session options in `src/main.ts`:

```typescript
const options = {
  ...getDefaultOptions(),
  maxAge: 3600,  // 1 hour (seconds)
  secret: new TextEncoder().encode(Deno.env.get('TSP_SESSION_SECRET')),
  cookieName: 'myapp_session',
  secure: true,
  httpOnly: true,
  sameSite: 'Strict',
};

sessionStore = new SessionStore(options);
```

### Session Options

| Option | Type | Default | Description |
|------|------|--------|------|
| `cookieName` | string | `'tsp_session'` | Cookie name for session ID |
| `maxAge` | number | `86400` | Session max lifetime (seconds, 1 day) |
| `cleanupInterval` | number | `300000` | Cleanup interval (ms, 5 minutes) |
| `secret` | Uint8Array | Auto-generated | Key for HMAC signature |
| `secure` | boolean | `true` | Set cookie secure flag |
| `httpOnly` | boolean | `true` | Set cookie httpOnly flag |
| `sameSite` | string | `'Strict'` | SameSite attribute ('Strict', 'Lax', 'None') |
| `path` | string | `'/'` | Cookie path |
| `rolling` | boolean | `true` | Enable rolling sessions (refresh on access) |
| `autoTouch` | boolean | `true` | Auto refresh session on access |

## Security Best Practices

### 1. Always Use HTTPS in Production

```typescript
const options = {
  ...getDefaultOptions(),
  secure: true,  // Requires HTTPS
};
```

### 2. Set Strong Session Secret

```bash
# Generate random key
openssl rand -base64 32
```

```bash
# .env
TSP_SESSION_SECRET=<generated-secret>
```

### 3. Regenerate Session ID After Login

```tsx
await session.login(userId, userData);
await session.regenerateId();  // Prevent session fixation
```

### 4. Use Appropriate Cookie Settings

- **httpOnly**: Prevent JavaScript access (prevent XSS theft)
- **secure**: Only send cookie over HTTPS
- **sameSite**: Prevent CSRF attacks

### 5. Set Reasonable Expiration Time

```typescript
const options = {
  ...getDefaultOptions(),
  maxAge: 3600,  // 1 hour for sensitive apps
  // maxAge: 86400,  // 1 day for general apps
  // maxAge: 604800,  // 1 week for "remember me"
};
```

### 6. Implement Proper Logout

```tsx
export default Page(async function(ctx, { session }) {
  await session.logout();  // Destroy session
  return { redirect: '/login.tsx', status: 302 };
});
```

## Session Storage

### Current Limitations

- Sessions are stored in memory
- Sessions are lost on server restart
- Not suitable for distributed/cluster deployments

### Future Enhancements

Planned support:
- Redis storage
- Database storage
- Distributed session storage
- Session persistence across restarts

## Troubleshooting

### Session Not Persisting

**Problem**: Session data is lost on page refresh.

**Solutions**: Check:
1. Cookies are enabled in browser
2. HTTP development environment sets `secure` flag to `false`
3. Browser is not blocking third-party cookies

### Session ID Not Changing

**Problem**: `regenerateId()` doesn't seem to work.

**Solutions**: Make sure to check `session.getId()` after calling `regenerateId()`.

### Production Deployment

**Problem**: Sessions fail in production.

**Solutions**:
1. Set `TSP_SESSION_SECRET` environment variable
2. Use HTTPS (set `secure: true`)
3. If using subdomains, check cookie domain settings

### Memory Issues

**Problem**: Too many sessions consuming memory.

**Solutions**:
1. Reduce `maxAge` to make sessions expire faster
2. Adjust `cleanupInterval` to run more frequently
3. Monitor session count in production

## Demo

When running the development server, access the complete demo at `/session_demo.tsx`:

```bash
./tsp.sh dev
# Access http://localhost:9000/session_demo.tsx
```

Demo includes:
- Login/logout
- Session data storage
- Session regeneration
- User information
- Session status

## Type Definitions

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

## See Also

- [Cookie Management](./cookies.md)
- [Dependency Injection](./injection.md)
- [AppDeps Guide](./appdeps.md)

---

[← Back to Features](./README.md) | [← Back to Documentation Center](../README.md)
