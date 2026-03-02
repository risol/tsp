# TSP Development Documentation

TSP is a Deno-based TypeScript full-stack framework using `.tsp`/`.tsx` file-based routing, supporting in-process hot reload, type-safe dependency injection, and compilable deployment.

## Table of Contents

- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [PageContext Details](#pagecontext-details)
- [Page Development](#page-development)
- [Component Development](#component-development)
- [Routing Rules](#routing-rules)
- [Request Handling](#request-handling)
- [Configuration Options](#configuration-options)
- [API Reference](#api-reference)

---

## Quick Start

### Installation and Running

```bash
# Clone project
git clone <repository-url>
cd tsp

# Start server
./tsp.sh dev
```

### First Page

Create `hello.tsx` in the `www` directory:

```tsx
export default async function (context: PageContext) {
  const { method, query } = context;
  const name = query.name || "World";

  return (
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <title>Hello</title>
      </head>
      <body>
        <h1>Hello, {name}!</h1>
        <p>Request Method: {method}</p>
      </body>
    </html>
  );
}
```

Access `http://localhost:9000/hello` to see the result.

> **Note**: The `PageContext` type is globally declared in `types.d.ts`, no import needed.

---

## Core Concepts

TSP uses the following core designs:

- **PageFunction**: The default function exported by each `.tsx` file, responsible for generating page content
- **PageContext**: The context object passed to the page function, containing all request information
- **Module Cache**: Smart caching based on file modification time, no restart needed to see code changes
- **JSX Rendering**: Use React to render JSX to HTML string

---

## PageContext Details

`PageContext` is the core object passed to each page function, containing all HTTP request information.

### Type Definition

```typescript
type PageContext = Readonly<{
  method: HttpMethod;
  url: URL;
  headers: Headers;
  query: Record<string, string>;
  body: unknown;
  cookies: Record<string, string>;
  file: string;
  root: string;
}>;
```

### Property Description

| Property | Type | Description |
|------|------|------|
| `method` | `HttpMethod` | HTTP request method |
| `url` | `URL` | Complete request URL object |
| `headers` | `Headers` | Request headers object |
| `query` | `Record<string, string>` | URL query parameters |
| `body` | `unknown` | Request body data |
| `cookies` | `Record<string, string>` | Cookie data |
| `file` | `string` | Current page file path |
| `root` | `string` | Document root directory path |

### HttpMethod Type

```typescript
type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";
```

---

### Usage Examples

#### 1. Get Request Method

```tsx
export default async function (context: PageContext) {
  const { method } = context;

  if (method === "POST") {
    // Handle POST request
  }

  return <div>Method: {method}</div>;
}
```

#### 2. Parse Query Parameters

```tsx
export default async function (context: PageContext) {
  const { query } = context;

  // URL: /page?name=John&age=25
  const name = query.name || "Anonymous";
  const age = parseInt(query.age || "0", 10);

  return (
    <div>
      <p>Name: {name}</p>
      <p>Age: {age}</p>
    </div>
  );
}
```

#### 3. Handle POST Data

The `body` value is automatically parsed based on `Content-Type`:

| Content-Type | Body Type |
|--------------|-----------|
| `application/json` | Parsed object |
| `application/x-www-form-urlencoded` | Key-value pair object |
| Other | Raw text string |
| GET/HEAD/DELETE | `null` |

```tsx
export default async function (context: PageContext) {
  const { method, body } = context;

  if (method === "POST" && body) {
    // JSON POST
    if (typeof body === "object" && body !== null) {
      const data = body as Record<string, unknown>;
      const username = data.username as string;
      // ...
    }
  }

  return <form method="POST">
    <input name="username" type="text" />
    <button type="submit">Submit</button>
  </form>;
}
```

#### 4. Read Cookies

```tsx
export default async function (context: PageContext) {
  const { cookies } = context;

  const sessionId = cookies.sessionId || "Not logged in";
  const theme = cookies.theme || "light";

  return <div>
    <p>Session: {sessionId}</p>
    <p>Theme: {theme}</p>
  </div>;
}
```

#### 5. Get Request Headers

```tsx
export default async function (context: PageContext) {
  const { headers } = context;

  const userAgent = headers.get("user-agent") || "Unknown";
  const accept = headers.get("accept") || "*/*";

  return <div>
    <p>User-Agent: {userAgent}</p>
    <p>Accept: {accept}</p>
  </div>;
}
```

#### 6. Use URL Object

```tsx
export default async function (context: PageContext) {
  const { url } = context;

  return <div>
    <p>Full URL: {url.href}</p>
    <p>Hostname: {url.hostname}</p>
    <p>Path: {url.pathname}</p>
    <p>Protocol: {url.protocol}</p>
    <p>Port: {url.port}</p>
  </div>;
}
```

#### 7. Get File Information

```tsx
export default async function (context: PageContext) {
  const { file, root } = context;

  return <div>
    <p>Current File: {file}</p>
    <p>Root Directory: {root}</p>
  </div>;
}
```

---

## Page Development

### Basic Structure

Each `.tsx` page must export a default function:

```tsx
export default async function (context: PageContext) {
  // Handle logic
  return <div>Page Content</div>;
}
```

### Return JSX

```tsx
export default async function (context: PageContext) {
  return (
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Page Title</title>
      </head>
      <body>
        <div id="app">
          <h1>Title</h1>
          <p>Content</p>
        </div>
      </body>
    </html>
  );
}
```

### Use Styles

```tsx
export default async function (context: PageContext) {
  const style = `
    body { font-family: sans-serif; }
    .container { max-width: 800px; margin: 0 auto; }
  `;

  return (
    <html>
      <head>
        <style>{style}</style>
      </head>
      <body>
        <div class="container">
          {/* Content */}
        </div>
      </body>
    </html>
  );
}
```

### Conditional Rendering

```tsx
export default async function (context: PageContext) {
  const { method, query } = context;
  const isDebug = query.debug === "true";

  return (
    <div>
      <h1>Page</h1>
      {isDebug && <div>Debug Info</div>}
      {method === "POST" ? <div>Submitted</div> : <div>Please Submit</div>}
    </div>
  );
}
```

### List Rendering

```tsx
export default async function (context: PageContext) {
  const items = ["Apple", "Banana", "Orange"];

  return (
    <ul>
      {items.map((item, index) => (
        <li key={index}>{item}</li>
      ))}
    </ul>
  );
}
```

### Redirect

Page functions can return a redirect object to trigger HTTP redirect:

```tsx
export default async function (context: PageContext) {
  return { redirect: "/login" };
}
```

Specify redirect status code:

```tsx
export default async function (context: PageContext) {
  return {
    redirect: "/new-location",
    status: 301  // Permanent redirect
  };
}
```

Available status codes:
- `301` - Moved Permanently
- `302` - Found (temporary redirect, default)
- `303` - See Other
- `307` - Temporary Redirect
- `308` - Permanent Redirect

Conditional redirect example:

```tsx
export default async function (context: PageContext) {
  const { cookies, query } = context;

  // Redirect to login if not logged in
  if (!cookies.sessionId) {
    return {
      redirect: `/login?redirect=${query.redirect || context.url.pathname}`
    };
  }

  return <div>Logged in content</div>;
}
```

### Return Custom Response

Page functions can also directly return Response object:

```tsx
export default async function (context: PageContext) {
  const { query } = context;

  // JSON response
  if (query.format === "json") {
    return new Response(JSON.stringify({ message: "Hello" }), {
      status: 200,
      headers: { "Content-Type": "application/json" }
    });
  }

  return <div>HTML Response</div>;
}
```

---

## Component Development

### Create Component

Create components in `www/components` directory:

```tsx
// www/components/Header.tsx
interface HeaderProps {
  title: string;
  subtitle?: string;
}

export function Header({ title, subtitle }: HeaderProps) {
  return (
    <header>
      <h1>{title}</h1>
      {subtitle && <p>{subtitle}</p>}
    </header>
  );
}

export default Header;
```

### Use Component

```tsx
import Header from "./components/Header.tsx";
import Footer from "./components/Footer.tsx";

export default async function (context: PageContext) {
  return (
    <div>
      <Header title="My Page" subtitle="Subtitle" />
      {/* Page content */}
      <Footer />
    </div>
  );
}
```

### Pass PageContext

```tsx
// www/components/Layout.tsx
interface LayoutProps {
  title: string;
  context: PageContext;
  children: unknown;
}

export function Layout({ title, context, children }: LayoutProps) {
  return (
    <html>
      <head>
        <title>{title}</title>
      </head>
      <body>
        <nav>
          <a href="/">Home</a>
          <a href="/about">About</a>
        </nav>
        {children}
        <footer>
          <p>Current Path: {context.url.pathname}</p>
        </footer>
      </body>
    </html>
  );
}

export default Layout;
```

---

## Routing Rules

### URL to File Mapping

| Request Path | Mapped File |
|---------|---------|
| `/` | `www/index.tsx` |
| `/about` | `www/about.tsx` |
| `/user/profile` | `www/user/profile.tsx` |
| `/user/` | `www/user/index.tsx` |

### Example

```
Request Path          File Path
────────────────────────────────
/               → www/index.tsx
/about          → www/about.tsx
/blog           → www/blog.tsx
/blog/post/123  → www/blog/post/123.tsx
/admin/         → www/admin/index.tsx
```

### 404 Handling

If file doesn't exist, return 404 status code:

```html
404 Not Found
```

### Security Protection

- Deny access to files other than `.tsx`
- Deny path traversal attacks (`../`)
- Can only access files under `root` directory

---

## Request Handling

### Request Flow

Workflow:
```
HTTP Request
   ↓
Route Resolution → Map to .tsx file
   ↓
Security Check
   ↓
Parse Request Parameters (query, body, cookies)
   ↓
Build PageContext
   ↓
Get Page Function (cache or reload)
   ↓
Execute Page Function → Return JSX
   ↓
Render JSX → HTML String
   ↓
Return HTTP Response
```

### Error Handling

Development mode (`--dev`) shows detailed errors:

```html
<h1>500 Internal Server Error</h1>
<pre>Error Message</pre>
<pre>Stack Trace</pre>
```

Production mode only shows generic error message.

---

## Configuration Options

### Command Line Arguments

```bash
./tsp.sh dev [options]
```

| Argument | Short | Description | Default |
|------|-------|------|--------|
| `--root` | `-r` | Document root directory | `./www` |
| `--port` | `-p` | Listening port | `9000` |
| `--dev` | `-d` | Development mode | `false` |
| `--help` | `-h` | Show help | - |

### Configuration Examples

```bash
# Use custom root directory
./tsp.sh dev --root ./site

# Specify port
./tsp.sh dev --port 8080

# Development mode (show detailed errors)
./tsp.sh dev --dev

# Combined
./tsp.sh dev --root ./site --port 8080 --dev

# Production mode
./tsp.sh start
```

---

## API Reference

### PageContext

```typescript
type PageContext = Readonly<{
  method: HttpMethod;
  url: URL;
  headers: Headers;
  query: Record<string, string>;
  body: unknown;
  cookies: Record<string, string>;
  file: string;
  root: string;
}>;
```

### PageFunction

```typescript
type PageFunction = (
  context: PageContext
) => Promise<PageResult> | PageResult;
```

### RedirectResult

Used to trigger HTTP redirect:

```typescript
interface RedirectResult {
  /** Redirect target URL */
  redirect: string;
  /** Redirect status code, default 302 */
  status?: 301 | 302 | 303 | 307 | 308;
}
```

### Exported Functions

Exported from `src/cache.ts`:

| Function | Description |
|------|------|
| `getPage(filepath)` | Get page function (with caching) |
| `renderJSX(jsx)` | Render JSX to HTML |
| `clearCache()` | Clear all cache |
| `getCacheSize()` | Get cache count |
| `type PageContext` | Page context type |
| `type RedirectResult` | Redirect result type |

---

## Best Practices

### 1. Always Export Default Function

```tsx
// Correct
export default async function (context: PageContext) { ... }

// Wrong
export async function render(context: PageContext) { ... }
```

### 2. Use Destructuring

```tsx
export default async function (context: PageContext) {
  const { method, query, url } = context;
  // ...
}
```

### 3. Type Safety

```tsx
export default async function (context: PageContext) {
  const { body } = context;

  if (typeof body === "object" && body !== null) {
    const data = body as { username?: string };
    const username = data.username || "Unknown";
  }
}
```

### 4. Component Reuse

Extract common parts as components to reduce code duplication.

### 5. Use TSX Instead of Template

TSX files are pages, not templates - naming should reflect this.

---

## FAQ

### Q: How to redirect?

A: Return redirect object:

```tsx
// Simple redirect (302)
return { redirect: "/target" };

// Permanent redirect (301)
return { redirect: "/target", status: 301 };

// Redirect with query parameter
return { redirect: `/login?redirect=${context.url.pathname}` };
```

### Q: How to return JSON?

A: Directly return Response object:

```tsx
return new Response(JSON.stringify({ data: "value" }), {
  status: 200,
  headers: { "Content-Type": "application/json" }
});
```

### Q: How to set custom response headers?

A: By returning Response object:

```tsx
return new Response(html, {
  status: 200,
  headers: {
    "Content-Type": "text/html; charset=utf-8",
    "X-Custom-Header": "value"
  }
});
```

### Q: How to handle static resources?

A: Static resources (images, CSS, JS) should be served by another server, or add corresponding route handling.

### Q: How does caching work?

A: The framework caches page functions based on file modification time. Files are automatically reloaded after modification, no server restart needed.

### Q: What return value types are supported?

A: Supports the following return types:
- JSX element: Rendered as HTML
- `{ redirect: string }` object: Triggers redirect
- `Response` object: Returned directly

---

## License

MIT License

## Related Documentation

- [Architecture](./architecture.md) - System architecture and design principles
- [Features](./features/README.md) - TSP feature documentation
- [Testing](./testing/README.md) - Testing related documentation
- [Getting Started](./getting-started.md) - Quick start in 5 minutes

---

[← Back to Documentation Center](./README.md)
