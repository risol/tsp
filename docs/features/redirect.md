# Redirect Test Fix

## Problem Description

When running binary tests, redirect tests fail:

```
Test Request: http://localhost:9001/redirect.tsx
✓ Status Code: 200

Error: Request failed: Values are not equal: Expected status code 302
-   200
+   302
```

## Root Cause

The `/redirect.tsx` page triggers redirect only with query parameters:

```tsx
// redirect.tsx logic
export default async function (context: PageContext) {
  const { query } = context;

  // Only redirect with specific query parameter
  if (query.to === "home") {
    return { redirect: "/" };  // 302
  }

  // Default: show page (200)
  return <html>...</html>;
}
```

The test code directly accesses `/redirect.tsx` (without parameters), expecting 302, but actually returns 200.

## Fix Solutions

### Solution 1: Create Simple Redirect Test Page

Create `www/redirect_simple.tsx`, always redirects:

```tsx
export default async function (_context: PageContext) {
  // Simple redirect test page
  // Always redirects to home page
  return { redirect: "/" };
}
```

### Solution 2: Use Correct Query Parameters

Modify test code, use URL that triggers redirect:

```typescript
// Before fix
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx`, 302); // ❌ Fails

// After fix
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx`, 200);  // Default page
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx?to=home`, 302); // Triggers redirect
```

### Solution 3: Combined Testing (Final Solution)

```typescript
// 4. Test various HTTP requests
await testHttpRequest(`http://localhost:${TEST_PORT}/`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/index.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/form.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/api.tsx`, 200);

// Redirect tests - multiple scenarios
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect_simple.tsx`, 302); // Simple redirect
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx`, 200);  // Redirect example page
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx?to=home`, 302); // Redirect with parameter

await testHttpRequest(`http://localhost:${TEST_PORT}/nonexistent.tsx`, 404); // 404
```

## Complete redirect.tsx Behavior

| URL | Behavior | Status Code |
|-----|------|--------|
| `/redirect.tsx` | Show redirect example page | 200 |
| `/redirect.tsx?to=home` | Redirect to home page | 302 |
| `/redirect.tsx?to=new-home` | Permanent redirect to home page | 301 |
| `/redirect.tsx?to=protected` (not logged in) | Redirect to login page | 302 |
| `/redirect.tsx?to=protected` (logged in) | Show protected page | 200 |

## Test Coverage

Now tests include three redirect scenarios:

1. **Simple Redirect**
   ```typescript
   await testHttpRequest(`/redirect_simple.tsx`, 302);
   ```
   - Always returns `{ redirect: "/" }`
   - Verify basic redirect functionality

2. **Redirect Example Page**
   ```typescript
   await testHttpRequest(`/redirect.tsx`, 200);
   ```
   - Default shows redirect example page
   - Verify page renders correctly

3. **Redirect with Parameters**
   ```typescript
   await testHttpRequest(`/redirect.tsx?to=home`, 302);
   ```
   - Triggers conditional redirect
   - Verify query parameter handling

## Code Changes

### New File: www/redirect_simple.tsx

```tsx
export default async function (_context: PageContext) {
  return { redirect: "/" };
}
```

### Modified File: tests/binary_build_test.ts

**Lines 225-233:**
```typescript
await testHttpRequest(`http://localhost:${TEST_PORT}/`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/index.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/form.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/api.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect_simple.tsx`, 302); // New
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx?to=home`, 302); // Modified
await testHttpRequest(`http://localhost:${TEST_PORT}/nonexistent.tsx`, 404);
```

## Verification Results

### Before Fix

```
✗ Expected status code 302
- Actual: 200
```

### After Fix (Expected)

```
✓ redirect_simple.tsx - Status Code: 302
✓ redirect.tsx - Status Code: 200
✓ redirect.tsx?to=home - Status Code: 302
✓ All tests passed
```

## Redirect Feature Description

### Return Value Types

TSP supports three return value types:

1. **JSX Element** - Rendered as HTML (200)
   ```tsx
   return <div>Hello</div>;
   ```

2. **Redirect Object** - Triggers HTTP redirect (301/302/303/307/308)
   ```tsx
   return { redirect: "/" };
   return { redirect: "/", status: 301 };
   ```

3. **Response Object** - Custom response
   ```tsx
   return new Response("...", { status: 200, headers: {...} });
   ```

### Redirect Status Codes

| Status Code | Meaning | Use Case |
|-----------|------|---------|
| 301 | Moved Permanently | Permanent redirect |
| 302 | Found | Temporary redirect (default) |
| 303 | See Other | Redirect POST to GET |
| 307 | Temporary Redirect | Keep request method temporary redirect |
| 308 | Permanent Redirect | Keep request method permanent redirect |

### Code Examples

**Basic Redirect:**
```tsx
export default async function (context: PageContext) {
  return { redirect: "/target" };
}
```

**Permanent Redirect:**
```tsx
export default async function (context: PageContext) {
  return { redirect: "/target", status: 301 };
}
```

**Conditional Redirect:**
```tsx
export default async function (context: PageContext) {
  const { cookies } = context;

  if (!cookies.sessionId) {
    return { redirect: "/login" };
  }

  return <div>Logged in content</div>;
}
```

## Run Tests

```bash
# Complete binary test
./tsp.sh test

# Manual test redirect
./tsp.sh dev

# Access:
# http://localhost:9000/redirect_simple.tsx  -> Redirect to home
# http://localhost:9000/redirect.tsx?to=home -> Redirect to home
```

## Other Notes

### Redirect Loops

Avoid creating redirect loops:

```tsx
// ❌ Wrong: Infinite loop
// index.tsx redirects to /home
// home.tsx redirects to /index

// ✅ Correct: Conditional redirect
export default async function (context: PageContext) {
  const { query } = context;

  if (query.redirect !== "done") {
    return { redirect: "/?redirect=done" };
  }

  return <div>Home Page</div>;
}
```

### Relative vs Absolute Paths

Absolute paths are recommended for redirects:

```tsx
// ✅ Recommended
return { redirect: "/login" };
return { redirect: "https://example.com/page" };

// ⚠️ May have issues
return { redirect: "login" };  // Relative path
```

### Testing Redirects

Use `curl` to test redirects:

```bash
# Don't follow redirect
curl -I http://localhost:9000/redirect_simple.tsx

# Output:
# HTTP/1.1 302 Found
# Location: /

# Follow redirect
curl -L http://localhost:9000/redirect_simple.tsx
```

## Update Date

2026-01-27

## Related Documentation

- [Features Home](./README.md) - View other features
- [Custom Response](./custom-response.md) - Another way to control responses
- [Architecture](../architecture.md) - Learn about redirect implementation

---

[← Back to Features](./README.md) | [← Back to Documentation Center](../README.md)
