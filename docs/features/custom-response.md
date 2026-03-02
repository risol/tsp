# Custom Response Object Test

## New Tests Added

The `custom_response.tsx` page has been added to test the direct return functionality of custom Response objects.

## Test Page Features

### custom_response.tsx

**Location**: `tests/test_www/custom_response.tsx`

**Features**:
- Default returns HTML response (custom Content-Type)
- Supports `?format=json` parameter to return JSON response
- Tests custom response headers

### Code Example

```tsx
export default async function (context: PageContext) {
  const { query } = context;
  const format = query.format;

  // JSON format
  if (format === "json") {
    return new Response(
      JSON.stringify({
        message: "Custom JSON Response",
        timestamp: new Date().toISOString(),
      }),
      {
        status: 200,
        headers: {
          "Content-Type": "application/json",
          "X-Custom-Header": "test-value",
        },
      }
    );
  }

  // HTML format (default)
  return new Response(
    "<!DOCTYPE html>...",
    {
      status: 200,
      headers: {
        "Content-Type": "text/html; charset=utf-8",
        "X-Response-Type": "custom",
      },
    }
  );
}
```

## Test Cases

### Test 1: HTML Response

```bash
curl http://localhost:9001/custom_response.tsx
```

**Expected Result**:
- Status Code: 200
- Content-Type: text/html; charset=utf-8
- Contains custom response header: X-Response-Type: custom

### Test 2: JSON Response

```bash
curl http://localhost:9001/custom_response.tsx?format=json
```

**Expected Result**:
- Status Code: 200
- Content-Type: application/json
- Returns JSON data
- Contains custom response header: X-Custom-Header: test-value

## Test Code

```typescript
await testHttpRequest(`http://localhost:${TEST_PORT}/custom_response.tsx`, 200);
await testHttpRequest(
  `http://localhost:${TEST_PORT}/custom_response.tsx?format=json`,
  200,
  "application/json"
);
```

## Advantages of Response Object

### 1. Complete Control

```tsx
return new Response(body, {
  status: 200,
  headers: {
    "Content-Type": "text/html",
    "Cache-Control": "no-cache",
    "Set-Cookie": "name=value",
  },
});
```

### 2. Support Various Content Types

**HTML**:
```tsx
return new Response("<h1>Hello</h1>", {
  headers: { "Content-Type": "text/html" }
});
```

**JSON**:
```tsx
return new Response(
  JSON.stringify({ message: "Hello" }),
  {
    headers: { "Content-Type": "application/json" }
  }
);
```

**Plain Text**:
```tsx
return new Response("Plain text", {
  headers: { "Content-Type": "text/plain" }
});
```

**Binary**:
```tsx
const data = await Deno.readFile("./image.png");
return new Response(data, {
  headers: { "Content-Type": "image/png" }
});
```

### 3. Flexible Status Codes

```tsx
// 200 OK
return new Response("Success", { status: 200 });

// 201 Created
return new Response("Created", { status: 201 });

// 204 No Content
return new Response(null, { status: 204 });

// 404 Not Found
return new Response("Not Found", { status: 404 });

// 500 Internal Server Error
return new Response("Server Error", { status: 500 });
```

### 4. Streaming Response

```tsx
// Create readable stream
const file = await Deno.open("./large-file.txt");

return new Response(file.readable, {
  headers: {
    "Content-Type": "text/plain",
    "Content-Disposition": "attachment; filename=file.txt",
  },
});
```

## Comparison with JSX Return

### JSX Return (Default)

```tsx
export default async function (context: PageContext) {
  // Automatically rendered as HTML
  return <h1>Hello</h1>;
}
```

**Processing Flow**:
1. Page function returns JSX
2. Call `renderJSX()` to render as HTML
3. Return 200 HTML response

### Response Return (Custom)

```tsx
export default async function (context: PageContext) {
  // Return Response directly
  return new Response("<h1>Hello</h1>", {
    headers: { "Content-Type": "text/html" }
  });
}
```

**Processing Flow**:
1. Page function returns Response object
2. Detected as `instanceof Response`
3. Return directly without any processing

## Test Coverage

### Current Tests Include

| Test | URL | Verification |
|------|-----|---------|
| HTML Response | `/custom_response.tsx` | Default HTML response |
| JSON Response | `/custom_response.tsx?format=json` | JSON format |
| Custom Headers | Both | Custom response headers |

### Verification Points

- ✅ Response objects are correctly identified
- ✅ Custom status codes work correctly
- ✅ Custom response headers can be set correctly
- ✅ Different Content-Types return correctly

## Code Changes

### File: tests/test_www/custom_response.tsx

**New file**, includes:
- HTML response example
- JSON response example
- Custom response headers example

### File: tests/binary_build_test.ts

**Modified**:
1. Added `expectedContentType` parameter to `testHttpRequest()` function
2. Added two custom response tests:
   - HTML response test
   - JSON response test (with Content-Type verification)

## Run Tests

```bash
# Run complete test
./tsp.sh test

# Expected Output
✓ custom_response.tsx - Status Code: 200
✓ custom_response.tsx?format=json - Status Code: 200
✓ Content-Type: application/json
✓ All tests passed
```

## Manual Testing

```bash
# Start server (using test directory)
./tsp.sh dev

# Test HTML response
curl http://localhost:9000/custom_response.tsx

# Test JSON response
curl http://localhost:9000/custom_response.tsx?format=json

# Test custom headers
curl -I http://localhost:9000/custom_response.tsx
```

## Summary

Custom Response objects provide:
- ✅ Complete HTTP response control
- ✅ Flexible content type support
- ✅ Custom status codes and response headers
- ✅ No need to go through JSX rendering

This is an advanced feature of TSP, suitable for scenarios requiring precise control over HTTP responses.

## Update Date

2026-01-27

## Related Documentation

- [Features Home](./README.md) - View other features
- [Redirect Feature](./redirect.md) - HTTP redirect support
- [Error Handling](./error-handling.md) - Error handling mechanism
- [Architecture](../architecture.md) - Learn about response handling implementation

---

[← Back to Features](./README.md) | [← Back to Documentation Center](../README.md)
