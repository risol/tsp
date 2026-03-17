---
name: tsp-response
description: HTTP response helpers in TSP. Use when sending JSON, HTML, redirects, or file responses.
---

# TSP Response

Use this skill for HTTP response operations in TSP.

## Basic Responses

```typescript
export default Page(async function(ctx, { response }) {
  // JSON response
  return response.json({ message: 'Hello' });
  return response.json({ users: [] }, 201);  // with status code

  // HTML response
  return response.html('<h1>Hello World</h1>');

  // Text response
  return response.text('Plain text response');

  // Redirect
  return response.redirect('/other-page');
  return response.redirect('/login', 302);  // with status code
});
```

## File Responses

```typescript
export default Page(async function(ctx, { response, logger }) {
  // Send file (auto-detect content type)
  return response.file('./public/document.pdf');

  // Send file with custom name
  return response.file('./data.csv', 'export.csv');

  // Send from stream
  const file = await Deno.open('./large-file.zip');
  return response.send(file.readable, 'application/zip', 'archive.zip');
});
```

## Response Headers

```typescript
export default Page(async function(ctx, { response }) {
  // JSON with custom headers
  return response.json({
    data: { token: 'abc123' }
  }, 200, {
    'X-Total-Count': '100',
    'X-Page': '1'
  });

  // Cache control
  return response.json({ data: 'static' }, 200, {
    'Cache-Control': 'public, max-age=3600'
  });

  // CORS headers
  return response.json({ data: 'api' }, 200, {
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type'
  });
});
```

## Response Types

| Method | Content-Type | Description |
|--------|--------------|-------------|
| `json()` | application/json | JSON response |
| `html()` | text/html | HTML response |
| `text()` | text/plain | Plain text |
| `redirect()` | - | HTTP redirect |
| `file()` | auto | File download |
| `send()` | custom | Raw response |

## Status Codes

| Code | Meaning |
|------|---------|
| 200 | OK |
| 201 | Created |
| 204 | No Content |
| 301 | Moved Permanently |
| 302 | Found (Redirect) |
| 400 | Bad Request |
| 401 | Unauthorized |
| 403 | Forbidden |
| 404 | Not Found |
| 500 | Internal Server Error |

## Best Practices

- Use appropriate status codes (200 for OK, 201 for created, etc.)
- Set Cache-Control for static resources
- Use CORS headers for API endpoints
- Return errors with meaningful messages
- Use `response.redirect()` for navigation
