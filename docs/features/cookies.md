# Cookie Management in TSP

TSP provides a powerful, type-safe cookie management system through dependency injection. This feature allows you to set, delete, and manage HTTP cookies with full TypeScript support.

## Overview

The cookie management system is built on top of TSP's dependency injection framework. It provides:

- **Type-safe API** with full TypeScript IntelliSense
- **Standard cookie options** support (expires, maxAge, domain, path, secure, httpOnly, sameSite)
- **Automatic URL encoding** for cookie names and values
- **Batch operations** for setting/deleting multiple cookies at once
- **Seamless integration** with redirects and responses

## Basic Usage

### Setting Cookies

```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  // Set a simple cookie
  cookies.set('username', 'john_doe');

  return <div>Cookie set!</div>;
});
```

### Reading Cookies

Request cookies are automatically parsed and available in `ctx.cookies`:

```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  // Read existing cookie
  const username = ctx.cookies.username || 'Guest';

  return <div>Hello, {username}!</div>;
});
```

### Deleting Cookies

```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  // Delete a cookie
  cookies.delete('username');

  return <div>Cookie deleted!</div>;
});
```

## Cookie Options

You can specify various options when setting cookies:

```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  cookies.set('sessionId', 'abc123', {
    // Cookie expiration (maxAge takes precedence)
    maxAge: 3600,           // Expires in 1 hour (in seconds)
    expires: new Date(),    // Or specify exact expiration date

    // Scope
    domain: '.example.com', // Cookie domain
    path: '/',              // Cookie path

    // Security
    secure: true,           // Only send over HTTPS
    httpOnly: true,         // No JavaScript access
    sameSite: 'Strict',     // CSRF protection: 'Strict' | 'Lax' | 'None'
  });

  return <div>Secure cookie set!</div>;
});
```

### Option Reference

| Option | Type | Description |
|--------|------|-------------|
| `expires` | `string \| Date` | Absolute expiration date (GMT string or Date object) |
| `maxAge` | `number` | Max age in seconds (takes precedence over `expires`) |
| `domain` | `string` | Domain restriction (e.g., `.example.com`) |
| `path` | `string` | Path restriction (e.g., `/`) |
| `secure` | `boolean` | Only send cookie over HTTPS |
| `httpOnly` | `boolean` | Prevent JavaScript access (XSS protection) |
| `sameSite` | `'Strict' \| 'Lax' \| 'None'` | CSRF protection |

## Advanced Usage

### Batch Operations

Set multiple cookies at once:

```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  cookies.setMultiple({
    'theme': { value: 'dark', options: { maxAge: 31536000 } },
    'language': { value: 'zh-CN', options: { maxAge: 31536000 } },
    'fontSize': { value: '14px', options: { maxAge: 31536000 } },
  });

  return <div>Preferences saved!</div>;
});
```

Delete multiple cookies:

```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  cookies.deleteMultiple(['temp1', 'temp2', 'temp3']);

  return <div>Temporary cookies cleared!</div>;
});
```

### Cookies with Redirects

Cookies are automatically added to redirect responses:

```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  // Set cookie before redirect
  cookies.set('justLoggedIn', 'true', { maxAge: 10 });

  // Cookie will be sent with this redirect
  return {
    redirect: '/dashboard',
    status: 302,
  };
});
```

### Special Characters and Unicode

Cookie names and values are automatically URL-encoded:

```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  // Special characters are encoded automatically
  cookies.set('user name', 'John Doe');
  cookies.set('email', 'test@example.com');
  cookies.set('chinese', '张三');
  cookies.set('emoji', '😀');

  return <div>Special cookies set!</div>;
});
```

## Security Best Practices

### 1. Use HttpOnly for Sensitive Cookies

Prevents XSS attacks from reading cookies via JavaScript:

```tsx
cookies.set('sessionId', 'abc123', {
  httpOnly: true,  // ✅ Recommended for session cookies
});
```

### 2. Use Secure for HTTPS-Only Sites

Ensures cookies are only sent over HTTPS:

```tsx
cookies.set('sessionId', 'abc123', {
  secure: true,  // ✅ Required for SameSite=None
});
```

### 3. Set SameSite for CSRF Protection

```tsx
cookies.set('sessionId', 'abc123', {
  sameSite: 'Strict',  // ✅ Best for CSRF protection
  // or 'Lax' for better usability
});
```

**SameSite modes:**
- `Strict`: Strongest protection, cookie not sent on external site links
- `Lax`: Balanced, cookie sent on top-level navigations
- `None`: No restriction, requires `Secure: true`

### 4. Use Max-Age Instead of Expires

`maxAge` is more reliable as it's relative to the current time:

```tsx
cookies.set('sessionId', 'abc123', {
  maxAge: 3600,  // ✅ Preferred (relative time)
  // expires: new Date(Date.now() + 3600 * 1000),  // ❌ Avoid
});
```

### 5. Set Explicit Path and Domain

Prevents cookie leakage to other paths/subdomains:

```tsx
cookies.set('sessionId', 'abc123', {
  path: '/',              // ✅ Explicit path
  domain: undefined,      // ✅ Current host only (recommended)
  // domain: '.example.com',  // ❌ Avoid unless needed
});
```

## Real-World Examples

### User Authentication

```tsx
export default Page(['cookies'], async function(ctx, { cookies, db }) {
  const { username, password } = ctx.body as { username: string; password: string };

  // Validate credentials
  const user = await db.query(
    'SELECT * FROM users WHERE username = ? AND password = ?',
    [username, password]
  );

  if (user.length > 0) {
    // Set secure session cookie
    cookies.set('sessionId', user[0].id, {
      httpOnly: true,
      secure: true,
      sameSite: 'Strict',
      maxAge: 3600,  // 1 hour
      path: '/',
    });

    return { redirect: '/dashboard', status: 302 };
  }

  return <div>Invalid credentials</div>;
});
```

### User Preferences

```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  const { theme, language } = ctx.body as { theme: string; language: string };

  // Set long-lived preference cookies
  cookies.setMultiple({
    'theme': { value: theme, options: { maxAge: 31536000, path: '/' } },
    'language': { value: language, options: { maxAge: 31536000, path: '/' } },
  });

  return <div>Preferences saved!</div>;
});
```

### Cookie Consent

```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  const consent = ctx.query.consent;

  if (consent === 'accept') {
    cookies.set('cookieConsent', 'accepted', {
      maxAge: 31536000,  // 1 year
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

### Tracking Last Visit

```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  const lastVisit = ctx.cookies.lastVisit;
  const now = new Date().toISOString();

  // Update last visit cookie
  cookies.set('lastVisit', now, {
    maxAge: 31536000,  // 1 year
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

## How It Works

### WeakMap-Based Storage

The cookie system uses a `WeakMap<PageContext, RequestContext>` to store cookie operations per request:

- **Key**: PageContext object
- **Value**: Array of Set-Cookie header strings

This design ensures:
- **Automatic cleanup**: When PageContext is garbage collected, cookie context is automatically freed
- **Request isolation**: Different requests have independent cookie operations
- **No memory leaks**: WeakMap doesn't prevent garbage collection

### Response Integration

After your page function executes, TSP:

1. Extracts cookie operations from the WeakMap
2. Serializes them to Set-Cookie headers
3. Adds them to the HTTP response (before sending)
4. Works with all response types (redirects, Response objects, JSX)

### URL Encoding

Cookie names and values are automatically `encodeURIComponent()`-encoded:

```tsx
cookies.set('user name', 'John Doe');
// Becomes: "user%20name=John%20Doe"
```

Request cookies in `ctx.cookies` are automatically decoded:

```tsx
const value = ctx.cookies['user name'];  // "John Doe"
```

## Testing

TSP includes comprehensive cookie tests:

```bash
# Run unit tests
deno test --allow-all tests/unit/cookie_test.ts

# Run E2E tests (requires server)
deno task dev
# Visit: http://localhost:9000/cookie_test.tsx
```

### Manual Testing Checklist

- [ ] Basic cookie setting
- [ ] Cookie with all options
- [ ] Cookie deletion
- [ ] Batch operations
- [ ] Cookie with redirect
- [ ] Special characters and Unicode
- [ ] HttpOnly cookie (verify with dev tools)
- [ ] Secure cookie (requires HTTPS)
- [ ] SameSite behavior

## Troubleshooting

### Cookies Not Being Set

1. **Check browser console** for errors
2. **Verify domain/path** match your site URL
3. **Check Secure flag** (won't work on HTTP)
4. **Inspect response headers** in browser dev tools
5. **Verify SameSite settings** (SameSite=None requires Secure)

### Cookies Not Being Read

1. **Check `ctx.cookies`** (request cookies, not response cookies)
2. **Verify cookie name** (case-sensitive)
3. **Check domain/path** match
4. **Verify cookie hasn't expired**

### Delete Not Working

1. **Match domain/path** from when cookie was set
2. **Check response headers** for Set-Cookie with `Max-Age=0`
3. **Verify browser honors deletion** (some browsers have bugs)

## API Reference

### CookieManager Interface

```typescript
interface CookieManager {
  /** Set a single cookie */
  set: (name: string, value: string, options?: CookieOptions) => void;

  /** Delete a single cookie */
  delete: (name: string, options?: Pick<CookieOptions, 'domain' | 'path'>) => void;

  /** Set multiple cookies at once */
  setMultiple: (cookies: Record<string, { value: string; options?: CookieOptions }>) => void;

  /** Delete multiple cookies at once */
  deleteMultiple: (names: string[], options?: Pick<CookieOptions, 'domain' | 'path'>) => void;
}
```

### CookieOptions Interface

```typescript
interface CookieOptions {
  /** Expiration date (GMT string or Date object) */
  expires?: string | Date;

  /** Max age in seconds */
  maxAge?: number;

  /** Domain restriction */
  domain?: string;

  /** Path restriction */
  path?: string;

  /** HTTPS only */
  secure?: boolean;

  /** No JavaScript access */
  httpOnly?: boolean;

  /** Same-site policy */
  sameSite?: 'Strict' | 'Lax' | 'None';
}
```

## See Also

- [Dependency Injection](../injection.md)
- [Page Context](../page-context.md)
- [Security Best Practices](../security.md)
- [E2E Tests](../../tests/test_www/cookie_test.tsx)
