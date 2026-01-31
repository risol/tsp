# Session Management

TSP provides a secure, type-safe session management system built on top of the cookie functionality. Sessions allow you to maintain user state across requests with built-in security features.

## Features

- **Secure**: HMAC-SHA256 signed session IDs prevent forgery
- **Cookie-based**: Uses httpOnly + secure + sameSite for protection
- **In-memory storage**: Fast session data access with automatic cleanup
- **Sliding expiration**: Optional automatic refresh of session timeout
- **Session fixation protection**: Regenerate session IDs on login
- **Type-safe**: Full TypeScript support with dependency injection

## Quick Start

### 1. Enable Session Dependency

Session is automatically registered in `src/main.ts`. No additional setup needed.

### 2. Use Sessions in Your Pages

```tsx
export default Page(async function(ctx, { session }) {
  const user = await session.getUser();

  if (!user) {
    // Redirect to login if not authenticated
    return { redirect: '/login.tsx', status: 302 };
  }

  return <div>Welcome, {user.name}!</div>;
});
```

## SessionManager API

The `SessionManager` provides the following methods:

### User Management

#### `getUser(): Promise<SessionUser | null>`

Get the current user from the session.

```tsx
const user = await session.getUser();
if (user) {
  console.log(`User: ${user.name} (${user.email})`);
}
```

#### `login(userId: string, userData?: Partial<SessionUser>): Promise<void>`

Create or update a session for a user.

```tsx
await session.login('user-123', {
  name: 'John Doe',
  email: 'john@example.com',
  role: 'admin',
});
```

#### `logout(): Promise<void>`

Destroy the current session.

```tsx
await session.logout();
```

### Data Storage

#### `set(key: string, value: unknown): Promise<void>`

Store data in the session.

```tsx
await session.set('cart', ['item1', 'item2']);
await session.set('preferences', { theme: 'dark' });
await session.set('visits', 5);
```

#### `get<T>(key: string): Promise<T | null>`

Retrieve data from the session.

```tsx
const cart = await session.get<string[]>('cart');
const visits = await session.get<number>('visits');
```

#### `delete(key: string): Promise<void>`

Remove data from the session.

```tsx
await session.delete('cart');
```

### Session Management

#### `regenerateId(): Promise<void>`

Generate a new session ID (prevents session fixation attacks).

```tsx
// After sensitive operations like login
await session.regenerateId();
```

#### `touch(): Promise<void>`

Refresh the session expiration time.

```tsx
await session.touch();
```

#### `isValid(): Promise<boolean>`

Check if the current session is valid.

```tsx
const valid = await session.isValid();
```

#### `getId(): string`

Get the current session ID.

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

    // Verify credentials (e.g., against database)
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

  // Update visit counter
  const visits = await session.get<number>('visits') || 0;
  await session.set('visits', visits + 1);

  return (
    <div>
      <h1>Welcome, {user.name}!</h1>
      <p>Visits: {visits + 1}</p>
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
  // Get current cart
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

// Setting flash message elsewhere
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
  maxAge: 3600,  // 1 hour (in seconds)
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
|--------|------|---------|-------------|
| `cookieName` | string | `'tsp_session'` | Cookie name for session ID |
| `maxAge` | number | `86400` | Session max age in seconds (1 day) |
| `cleanupInterval` | number | `300000` | Cleanup interval in milliseconds (5 minutes) |
| `secret` | Uint8Array | auto-generated | Secret key for HMAC signing |
| `secure` | boolean | `true` | Set secure flag on cookie |
| `httpOnly` | boolean | `true` | Set httpOnly flag on cookie |
| `sameSite` | string | `'Strict'` | SameSite attribute ('Strict', 'Lax', 'None') |
| `path` | string | `'/'` | Cookie path |
| `rolling` | boolean | `true` | Enable rolling sessions (refresh on access) |
| `autoTouch` | boolean | `true` | Auto-touch session on access |

## Security Best Practices

### 1. Always Use HTTPS in Production

```typescript
const options = {
  ...getDefaultOptions(),
  secure: true,  // Requires HTTPS
};
```

### 2. Set a Strong Session Secret

```bash
# Generate a random secret
openssl rand -base64 32
```

```bash
# .env
TSP_SESSION_SECRET=<generated-secret>
```

### 3. Regenerate Session ID After Login

```tsx
await session.login(userId, userData);
await session.regenerateId();  // Prevents session fixation
```

### 4. Use Appropriate Cookie Settings

- **httpOnly**: Prevents JavaScript access to cookies (prevents XSS theft)
- **secure**: Only sends cookie over HTTPS
- **sameSite**: Prevents CSRF attacks

### 5. Set Reasonable Expiration Times

```typescript
const options = {
  ...getDefaultOptions(),
  maxAge: 3600,  // 1 hour for sensitive apps
  // maxAge: 86400,  // 1 day for general apps
  // maxAge: 604800,  // 1 week for "remember me" functionality
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
- Sessions are lost when the server restarts
- Not suitable for distributed/clustered deployments

### Future Enhancements

Planned support for:
- Redis storage
- Database storage
- Distributed session stores
- Session persistence across restarts

## Troubleshooting

### Session Not Persisting

**Problem**: Session data is lost on page refresh.

**Solution**: Check that:
1. Cookies are enabled in the browser
2. `secure` flag is set to `false` for HTTP development
3. Browser is not blocking third-party cookies

### Session ID Not Changing

**Problem**: `regenerateId()` doesn't seem to work.

**Solution**: Ensure you're checking `session.getId()` after calling `regenerateId()`.

### Production Deployment

**Problem**: Sessions fail in production.

**Solution**:
1. Set `TSP_SESSION_SECRET` environment variable
2. Use HTTPS (set `secure: true`)
3. Check cookie domain settings if using subdomains

### Memory Issues

**Problem**: Too many sessions consuming memory.

**Solution**:
1. Reduce `maxAge` to expire sessions sooner
2. Adjust `cleanupInterval` to run more frequently
3. Monitor session count in production

## Demo

A complete demo is available at `/session_demo.tsx` when running the dev server:

```bash
deno task dev
# Visit http://localhost:9000/session_demo.tsx
```

The demo demonstrates:
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
- [Dependency Injection](../architecture/injection.md)
- [Security Best Practices](./security.md)
