# TSP Error Handling Test

## New Tests Added

The `error.tsx` page has been added to test error handling in production and development modes.

## Test Page

### error.tsx

**Location**: `tests/test_www/error.tsx`

**Features**:
- Intentionally throw errors
- Verify error catching mechanism
- Test error display differences between development and production modes

```tsx
export default async function (_context: PageContext) {
  // Intentionally throw error
  throw new Error("This is a test error for verifying error handling mechanism");
}
```

## Error Handling Mechanism

### Development Mode (--dev)

```bash
./tsp.sh dev --dev
```

**Behavior**:
- Shows detailed error information
- Shows complete stack trace
- Shows error file and line number
- Easy to debug

**Example Output**:
```html
<h1>500 Internal Server Error</h1>
<pre>Error Message: This is a test error for verifying error handling mechanism</pre>
<pre>Stack Trace:
    at error.tsx:3:15
    at getPage (cache.ts:45:20)
    ...
</pre>
```

### Production Mode (Default)

```bash
./tsp.sh start
```

**Behavior**:
- Hides sensitive stack information
- Shows friendly error page
- Only shows error message
- Doesn't expose internal implementation

**Example Output**:
```html
<h1>500 Internal Server Error</h1>
<pre>Error Message: This is a test error for verifying error handling mechanism</pre>
```

## Test Cases

### Test Error Catching

```typescript
await testHttpRequest(`http://localhost:9001/error.tsx`, 500, undefined, true);
```

**Verification Points**:
- ✅ Returns 500 status code
- ✅ Contains error information
- ✅ Production mode doesn't include stack trace

## Implementation Code

### Error Handling in src/main.ts

```typescript
try {
  // Execute page function
  const result = await pageFn(context);

  // Handle result...
} catch (error) {
  // Error handling
  const errorMessage = error instanceof Error ? error.message : String(error);
  const stackTrace = error instanceof Error ? error.stack : "";

  console.error("Request error:", errorMessage);

  if (config.dev) {
    // Development mode: show detailed errors
    const html = `
<!DOCTYPE html>
<html>
<head><title>Error</title></head>
<body>
  <h1>500 Internal Server Error</h1>
  <pre>${errorMessage}</pre>
  <pre>${stackTrace}</pre>
</body>
</html>
    `;
    return new Response(html, {
      status: 500,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  } else {
    // Production mode: hide stack
    const html = `
<!DOCTYPE html>
<html>
<head><title>Error</title></head>
<body>
  <h1>500 Internal Server Error</h1>
  <pre>${errorMessage}</pre>
</body>
</html>
    `;
    return new Response(html, {
      status: 500,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }
}
```

## Test Scenarios

### Scenario 1: Runtime Error

```tsx
export default async function (context: PageContext) {
  // Access undefined variable
  const obj = null as unknown;
  console.log((obj as any).value);
}
```

**Test**: Access `/error.tsx`
**Expected**: 500 error page

### Scenario 2: Async Error

```tsx
export default async function (context: PageContext) {
  // Promise rejection
  await Promise.reject("Async error");
}
```

**Test**: Access `/error.tsx`
**Expected**: 500 error page

### Scenario 3: Type Error

```tsx
export default async function (context: PageContext) {
  // Type error
  const str: string = null as unknown;
  console.log(str.toUpperCase());
}
```

**Test**: Access `/error.tsx`
**Expected**: 500 error page

## Security Features

### Development Mode

- ✅ Shows complete error information
- ✅ Shows stack trace
- ✅ Shows file path
- ✅ Shows line number

### Production Mode

- ✅ Hides sensitive information
- ✅ Only shows error message
- ✅ Doesn't expose code paths
- ✅ Doesn't expose stack trace

## Test Verification

### Run Tests

```bash
# Development mode test
./tsp.sh dev
curl http://localhost:9000/error.tsx

# Production mode test
./tsp.sh start
curl http://localhost:9000/error.tsx
```

### Binary Test

```bash
./tsp.sh test
```

**Tests Include**:
- ✅ Access `/error.tsx`
- ✅ Verify 500 status code
- ✅ Verify error message display
- ✅ Verify stack is hidden in production mode

## Error Handling Best Practices

### 1. Throw Meaningful Errors

```tsx
export default async function (context: PageContext) {
  const userId = context.query.userId;

  if (!userId) {
    throw new Error("Missing required parameter: userId");
  }

  // ...
}
```

### 2. Catch and Handle Errors

```tsx
export default async function (context: PageContext) {
  try {
    // Operations that may fail
    const data = await fetchData();
    return <div>{data}</div>;
  } catch (error) {
    // Handle error, return friendly page
    console.error("Data loading failed:", error);
    return <div>Loading failed, please try again later</div>;
  }
}
```

### 3. Log Error Details

```tsx
export default async function (context: PageContext) {
  try {
    // ...
  } catch (error) {
    // Log error details
    console.error("[ERROR]", {
      file: context.file,
      error: error instanceof Error ? error.message : String(error),
      timestamp: new Date().toISOString(),
    });

    throw error; // Re-throw, let framework handle
  }
}
```

## Test Directory Update

```
tests/test_www/
├── index.tsx            # Home page test
├── form.tsx             # Form/POST test
├── api.tsx              # API/context test
├── custom_response.tsx  # Custom Response test
├── error.tsx            # Error handling test ✨ New
├── redirect.tsx         # Redirect test (temporarily disabled)
└── README.md
```

## Run Tests

```bash
./tsp.sh test
```

**Expected Result**:
```
✓ error.tsx - Status Code: 500
✓ Error message displays correctly
✓ Stack hidden in production mode
✓ All tests passed
```

## Summary

Error handling tests verify:
- ✅ Development mode shows detailed errors
- ✅ Production mode hides sensitive information
- ✅ 500 error page displays correctly
- ✅ Errors are correctly caught and handled
- ✅ Stack trace shown in dev mode, hidden in prod mode

This is an important test to ensure application security in production environments.

## Update Date

2026-01-27

## Related Documentation

- [Features Home](./README.md) - View other features
- [Custom Response](./custom-response.md) - Custom HTTP response
- [Development Guide](../development.md) - How to debug errors
- [Architecture](../architecture.md) - Learn about error handling implementation

---

[← Back to Features](./README.md) | [← Back to Documentation Center](../README.md)
