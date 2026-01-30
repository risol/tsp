# Cookie Management - Quick Reference

## Basic Usage

### Set a Cookie
```tsx
export default Page(['cookies'], async function(ctx, { cookies }) {
  cookies.set('name', 'value');
  return <div>Done!</div>;
});
```

### Read Cookies
```tsx
export default Page(async function(ctx) {
  const value = ctx.cookies.cookieName || 'default';
  return <div>{value}</div>;
});
```

### Delete a Cookie
```tsx
cookies.delete('cookieName', { path: '/' });
```

## Cookie Options

```tsx
cookies.set('session', 'abc123', {
  expires: new Date('2025-12-31'),  // or maxAge
  maxAge: 3600,                      // seconds (takes precedence)
  domain: '.example.com',            // optional
  path: '/',                         // optional
  secure: true,                      // HTTPS only
  httpOnly: true,                    // no JS access
  sameSite: 'Strict',                // 'Strict' | 'Lax' | 'None'
});
```

## Batch Operations

### Set Multiple
```tsx
cookies.setMultiple({
  'theme': { value: 'dark', options: { maxAge: 31536000 } },
  'lang': { value: 'en', options: { maxAge: 31536000 } },
});
```

### Delete Multiple
```tsx
cookies.deleteMultiple(['cookie1', 'cookie2'], { path: '/' });
```

## Security Checklist

- ✅ Use `httpOnly: true` for session cookies
- ✅ Use `secure: true` on HTTPS sites
- ✅ Use `sameSite: 'Strict'` or `'Lax'` for CSRF protection
- ✅ Prefer `maxAge` over `expires`
- ✅ Set explicit `path` (usually `'/'`)
- ✅ Avoid `domain` unless needed

## Examples

### Authentication Cookie
```tsx
cookies.set('sessionId', userId, {
  httpOnly: true,
  secure: true,
  sameSite: 'Strict',
  maxAge: 3600,
  path: '/',
});
```

### User Preferences
```tsx
cookies.set('theme', 'dark', { maxAge: 31536000, path: '/' });
```

### Cookie Consent
```tsx
cookies.set('consent', 'accepted', { maxAge: 31536000, path: '/' });
```

## Testing

Run tests:
```bash
# Unit tests
deno test --allow-all tests/unit/cookie_test.ts

# E2E tests (start server first)
deno task dev
# Visit: http://localhost:9000/cookie_test.tsx

# Quick verification
deno run --allow-all tests/verify_cookies.ts
```

## Files

- **Implementation**: `src/cookies.ts`
- **Types**: `types.d.ts` (AppDeps interface)
- **Integration**: `src/main.ts` (registration + response handling)
- **Tests**: `tests/unit/cookie_test.ts`
- **E2E**: `tests/test_www/cookie_test.tsx`
- **Demo**: `www/cookie_demo.tsx`
- **Docs**: `docs/features/cookies.md`

## Common Issues

**Cookie not being set?**
- Check domain/path match your site
- Check `secure` flag (requires HTTPS)
- Check browser console for errors
- Inspect response headers in dev tools

**Cookie not being read?**
- Use `ctx.cookies.cookieName` (not the cookies manager)
- Check exact name (case-sensitive)
- Verify domain/path match

**Delete not working?**
- Must match domain/path from when it was set
- Check for `Set-Cookie` with `Max-Age=0` in response

## See Also

- [Full Documentation](./cookies.md)
- [Implementation Summary](./cookies-implementation-summary.md)
- [Dependency Injection](../injection.md)
