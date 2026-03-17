---
name: tsp-session
description: Session management in TSP. Use when implementing login/logout, storing user data in sessions, or session-based authentication.
---

# TSP Session

Use this skill for session management in TSP.

## Session Usage

```typescript
export default Page(async function(ctx, { session, response }) {
  // Check if session exists
  if (!session.isLoggedIn) {
    return response.redirect('/login');
  }

  // Get session data
  const userId = session.get('userId');
  const username = session.get('username');
  const role = session.get('role');

  // Set session data
  session.set('userId', 123);
  session.set('username', 'john');
  session.set('role', 'admin');

  // Set with options
  session.set('key', 'value', {
    maxAge: 3600,  // 1 hour
    httpOnly: true,
    secure: false,
    sameSite: 'Strict'
  });

  // Remove session data
  session.remove('username');

  // Check if key exists
  const hasKey = session.has('userId');

  // Get all session data
  const allData = session.all();

  // Check if logged in (convenience method)
  const isLoggedIn = session.isLoggedIn;

  // Get session ID
  const sessionId = session.id;

  return response.json({
    userId,
    username,
    role,
    isLoggedIn,
    sessionId
  });
});
```

## Login/Logout Example

```typescript
export default Page(async function(ctx, { body, session, response, logger }) {
  const z = await createZod();

  // Validate login request
  const data = body(z.object({
    username: z.string().min(1),
    password: z.string().min(1)
  }));

  // Verify credentials (example - use bcrypt in production)
  const user = await verifyLogin(data.username, data.password);
  if (!user) {
    return response.json({ error: 'Invalid credentials' }, 401);
  }

  // Set session
  session.set('userId', user.id);
  session.set('username', user.name);
  session.set('role', user.role);

  logger.info(`User ${user.name} logged in`);

  return response.json({ success: true, redirect: '/dashboard' });
});

// Logout handler
export async function POSTLogout(ctx: PageContext, { session, response, logger }) {
  const username = session.get('username');

  // Clear session
  session.clear();

  logger.info(`User ${username} logged out`);

  return response.redirect('/login');
}
```

## Protected Route

```typescript
export default Page(async function(ctx, { session, response }) {
  // Check authentication
  if (!session.isLoggedIn) {
    return response.redirect('/login?redirect=' + encodeURIComponent(ctx.url.pathname));
  }

  // Get current user
  const userId = session.get('userId');

  // Your logic here
  return response.html('<h1>Dashboard</h1>');
});
```

## Session Options

Configure in `config.jsonc`:

```jsonc
{
  "session": {
    "cookieName": "tsp_session",
    "maxAge": 86400,        // 24 hours in seconds
    "secure": false,        // true for HTTPS
    "httpOnly": true,       // JavaScript cannot access
    "sameSite": "Strict",   // CSRF protection
    "path": "/",
    "rolling": true         // Reset expiry on each request
  }
}
```

## Session with Redis

For multi-server deployments, configure Redis for session sharing:

```jsonc
{
  "redis": {
    "host": "127.0.0.1",
    "port": 6379,
    "password": "your_password",
    "db": 0
  }
}
```

## Key Methods

| Method | Description |
|--------|-------------|
| `get()` | Get session value |
| `set()` | Set session value |
| `remove()` | Remove session value |
| `has()` | Check if key exists |
| `all()` | Get all session data |
| `clear()` | Clear all session data |
| `id` | Get session ID |
| `isLoggedIn` | Check if logged in |

## Best Practices

- Always check `session.isLoggedIn` for protected routes
- Use `session.clear()` for logout
- Configure appropriate `maxAge` based on security requirements
- Use Redis for session sharing in multi-worker deployments
- Set `httpOnly: true` to prevent XSS attacks
- Use `secure: true` (HTTPS) in production
