# TSP Architecture Documentation

## Overview

TSP is a Deno-based TypeScript full-stack framework using `.tsp`/`.tsx` file-based routing, supporting in-process hot reload, type-safe dependency injection, and compilable deployment.

## Core Features

- Use `.tsx` files as pages
- React-based JSX rendering
- Smart file modification time caching
- Support for dynamically loading TSX in compiled binaries
- Complete TypeScript type support

## Technology Stack

- **Runtime**: Deno
- **Language**: TypeScript/TSX
- **Rendering Engine**: React (18.3.1)
- **Module Loading**: Custom TSX loader with caching
- **HTTP Service**: Deno std/http

## Architecture Design

### File Structure

```
tsp/
├── src/
│   ├── main.ts       # Main entry point, HTTP server
│   ├── cache.ts      # Template caching and loading logic
│   ├── context.ts    # Request context building
│   └── router.ts     # URL to file mapping
├── www/              # Page root directory
│   ├── index.tsx     # Home page
│   ├── form.tsx      # Form example
│   ├── api.tsx       # API example
│   ├── redirect.tsx  # Redirect example
│   └── components/   # Reusable components
└── docs/
    └── documentation.md    # Detailed development documentation
```

### Workflow

```
HTTP Request
    ↓
Route Resolution (router.ts)
    ↓
Security Check
    ↓
Build Context (context.ts)
    ↓
Get Page Function (cache.ts)
    ├─ Cache Hit → Use directly
    └─ Cache Miss → Dynamically load TSX
        ├─ Source mode: Direct import
        └─ Binary mode: Custom loader
    ↓
Execute Page Function → Return JSX
    ↓
Render JSX → HTML (React)
    ↓
Return HTTP Response
```

### Core Modules

#### 1. main.ts

- Start HTTP server
- Parse command line arguments
- Request distribution and error handling

**Key Features**:
- Parse `--root`, `--port`, `--dev` arguments
- Listen on specified port
- Handle each request, call corresponding modules

#### 2. cache.ts

**Most Core Module**, responsible for:

- Page function caching (based on file modification time)
- Dynamic loading of TSX files
- Support for dynamic loading in all modes

**Cache Strategy**:
```typescript
type CacheEntry = {
  mtimeMs: number;      // File modification time
  pageFunction: PageFunction;  // Page function
};

const cache = new Map<string, CacheEntry>();
```

**Dynamic Loading Logic**:
```typescript
// Source mode
const module = await import(fileUrl);

// Binary mode
const loader = await getGlobalLoader();
const response = await loader.load(fileUrl, RequestedModuleType.Default);
const code = new TextDecoder().decode(response.code);
const dataUrl = `data:application/javascript,${encodeURIComponent(code)}`;
const module = await import(dataUrl);
```

#### 3. context.ts

Build request context object:

```typescript
type PageContext = {
  method: HttpMethod;
  url: URL;
  headers: Headers;
  query: Record<string, string>;
  body: unknown;
  cookies: Record<string, string>;
  file: string;
  root: string;
};
```

**Handles**:
- URL query parameter parsing
- Request body parsing (JSON / form / text)
- Cookie parsing
- Request header handling

#### 4. router.ts

URL to file mapping rules:

| URL Path | File Path |
|---------|---------|
| `/` | `www/index.tsx` |
| `/about` | `www/about.tsx` |
| `/user/profile` | `www/user/profile.tsx` |
| `/user/` | `www/user/index.tsx` |

**Security Features**:
- Path normalization
- Block path traversal (`../`)
- Only allow `.tsx` files
- Must be within root directory

## Page Development

### Page Structure

Each `.tsx` file exports a default function:

```tsx
export default async function (context: PageContext) {
  return <div>Page Content</div>;
}
```

### Supported Return Values

1. **JSX Element** - Rendered as HTML
2. **Redirect Object** - Trigger HTTP redirect
   ```tsx
   return { redirect: "/target" };
   ```
3. **Response Object** - Custom response
   ```tsx
   return new Response("...", { status: 200, headers: {...} });
   ```

### Component Development

Create reusable components in `www/components/`:

```tsx
// www/components/Header.tsx
interface HeaderProps {
  title: string;
}

export function Header({ title }: HeaderProps) {
  return <header><h1>{title}</h1></header>;
}
```

## Build and Deployment

### Development Mode

```bash
./tsp.sh dev
```

- See code changes in real-time
- Show detailed error messages

### Production Deployment

**Method 1: Using tsp.sh**
```bash
./tsp.sh start
```

**Method 2: Compile to Binary**
```bash
# Compile
./tsp.sh build:tspserver:rel

# Run (need to set DENO_DIR)
DENO_DIR=/path/to/.deno ./tspserver --root ./www --port 9000
```

**Advantages after Compilation**:
- No need to install Deno
- Faster startup speed
- Single file distribution

**Notes**:
- Need to set `DENO_DIR` environment variable

## Performance Optimization

### Cache Mechanism

1. **File Cache**: Smart cache based on `mtime`
   - File not modified → Reuse cache
   - File modified → Reload

2. **Module Cache**: Deno's module cache
   - Auto-cache remote dependencies
   - Speed up module loading

### Performance Metrics

| Operation | Time |
|------|-----|
| First load TSX (source mode) | ~20ms |
| First load TSX (compiled) | ~50ms |
| After cache load | ~5ms |
| React rendering | ~1-5ms |

## Security Design

### Path Security

```typescript
// Path normalization
const normalized = normalize(path);
const absolute = resolve(root, normalized);

// Check if within root
if (!absolute.startsWith(root)) {
  throw new Error("Path traversal detected");
}
```

### File Type Whitelist

```typescript
if (!filepath.endsWith(".tsx")) {
  throw new Error("Only .tsx files are allowed");
}
```

### Error Handling

- **Development Mode**: Show detailed errors and stack
- **Production Mode**: Hide sensitive information, show generic error page

## Extensibility

### Adding Middleware

Add to the request handling flow in `main.ts`:

```typescript
async function handleRequest(req: Request) {
  // 1. Logging middleware
  console.log(`${req.method} ${req.url}`);

  // 2. Authentication middleware
  const authHeader = req.headers.get("Authorization");
  if (!authHeader) {
    return new Response("Unauthorized", { status: 401 });
  }

  // 3. Route handling
  // ...
}
```

### Custom Types

Extend `PageContext`:

```typescript
// src/context.ts
export interface ExtendedPageContext extends PageContext {
  user?: User;
  session?: Session;
}

export function buildExtendedContext(req: Request, file: string, root: string): ExtendedPageContext {
  const base = buildContext(req, file, root);

  return {
    ...base,
    user: getCurrentUser(req),
  };
}
```

## Comparison with Other Solutions

### vs PHP-FPM

| Feature | PHP-FPM | TSP |
|------|---------|---------|
| Language | PHP | TypeScript |
| Template Syntax | PHP | TSX (JSX) |
| Type Safety | ❌ | ✅ |
| Component-based | ❌ | ✅ |
| Modern Toolchain | ❌ | ✅ |

### vs Next.js

| Feature | Next.js | TSP |
|------|---------|---------|
| Build Step | ✅ Required | ❌ Not Required |
| Server-Side Rendering | ✅ | ✅ |
| Simplicity | ❌ Complex | ✅ Simple |
| Learning Curve | Steep | Gentle |

## Best Practices

1. **Type Safety**: Always use `PageContext` type
2. **Component Reuse**: Extract common parts as components
3. **Error Handling**: Use try-catch to wrap async operations
4. **Performance**: Use caching mechanism, avoid duplicate calculations
5. **Security**: Validate all user input

## FAQ

### Q: Why choose TSX instead of template engine?

A: TSX provides:
- Complete type safety
- Component-based capabilities
- Modern development experience
- React ecosystem support

### Q: How to support hot reload?

A: Use `--dev` mode, files will automatically reload after modification

### Q: What platforms can I deploy to?

A:
- Any platform supporting Deno
- Compiled binaries can be deployed to any Linux/Windows/macOS server

## Future Plans

- [ ] Support more rendering modes (SSG, ISR)
- [x] Add database integration ✅ MySQL Schema-first API implemented
- [ ] WebSocket support
- [ ] Plugin system
- [ ] CLI tool enhancement
- [ ] More database support (PostgreSQL, MongoDB, etc.)

## Contributing

Welcome to contribute code, report issues, or make suggestions!

## License

MIT License

## Related Documentation

- [Development Guide](./development.md) - Development environment setup and best practices
- [Features](./features/README.md) - TSP feature documentation
- [Getting Started](./getting-started.md) - Quick start in 5 minutes
- [Testing Documentation](./testing/README.md) - Testing related documentation

---

[← Back to Documentation Center](./README.md)
