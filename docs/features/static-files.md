# Static File Service

TSP has built-in static file service functionality, can directly serve CSS, JavaScript, images and other static resources without additional web server.

## Features

- ✅ Automatic MIME type recognition
- ✅ HTTP cache support (ETag, Last-Modified)
- ✅ Disable cache in development mode
- ✅ Configurable file extension whitelist
- ✅ High-performance file reading
- ✅ 304 Not Modified response support

## Configuration

### Default Supported File Types

If not configured, TSP supports the following static file types by default:

| Extension | MIME Type | Description |
|--------|-----------|------|
| `.html` | `text/html` | HTML document |
| `.htm` | `text/html` | HTML document |
| `.css` | `text/css` | Stylesheet |
| `.js` | `application/javascript` | JavaScript |
| `.json` | `application/json` | JSON data |
| `.png` | `image/png` | PNG image |
| `.jpg`, `.jpeg` | `image/jpeg` | JPEG image |
| `.gif` | `image/gif` | GIF image |
| `.svg` | `image/svg+xml` | SVG vector image |
| `.ico` | `image/x-icon` | Icon |
| `.webp` | `image/webp` | WebP image |
| `.woff` | `font/woff` | WOFF font |
| `.woff2` | `font/woff2` | WOFF2 font |
| `.ttf` | `font/ttf` | TTF font |
| `.eot` | `application/vnd.ms-fontobject` | EOT font |
| `.mp3` | `audio/mpeg` | MP3 audio |
| `.mp4` | `video/mp4` | MP4 video |
| `.webm` | `video/webm` | WebM video |
| `.txt` | `text/plain` | Text file |
| `.md` | `text/markdown` | Markdown document |
| `.xml` | `application/xml` | XML document |

### Custom File Types

Customize allowed static file extensions via configuration file:

**config.jsonc**:
```jsonc
{
  "root": "./www",
  "port": 9000,
  "dev": false,

  // Only allow specific static file types
  "staticExtensions": [".css", ".js", ".png", ".jpg", ".svg"]
}
```

**Command line arguments** (not supported in current version, config file only):
```bash
# Future support
./tspserver --static-extensions .css,.js,.png
```

### Disable Static File Service

If you don't want to serve static files, set empty array:

```jsonc
{
  "staticExtensions": []
}
```

## Usage Examples

### Basic Usage

Assume you have the following project structure:

```
www/
├── index.tsx          # TSX page
├── styles/
│   └── main.css       # Static CSS
├── js/
│   └── app.js         # Static JS
└── images/
    └── logo.png       # Static image
```

Reference static files in `index.tsx`:

```tsx
export default async function(ctx: PageContext) {
  return (
    <html>
      <head>
        <title>Static File Example</title>
        <link rel="stylesheet" href="/styles/main.css" />
      </head>
      <body>
        <h1>Welcome to TSP</h1>
        <img src="/images/logo.png" alt="Logo" />
        <script src="/js/app.js"></script>
      </body>
    </html>
  );
}
```

### References in TSX Pages

```tsx
// www/index.tsx
export default async function() {
  return (
    <html>
      <head>
        <link rel="stylesheet" href="/static/style.css" />
        <link rel="icon" href="/favicon.ico" />
      </head>
      <body>
        <img src="/static/banner.jpg" alt="Banner" />
        <script src="/static/app.js"></script>
      </body>
    </html>
  );
}
```

## HTTP Cache

### Production Mode

In production mode, static files automatically get cache headers:

```http
HTTP/1.1 200 OK
Content-Type: image/png
ETag: "a1b2c3d4-1706543210"
Last-Modified: Mon, 29 Jan 2026 12:34:56 GMT
Cache-Control: public, max-age=86400
```

- **ETag**: Hash of file content
- **Last-Modified**: File last modification time
- **Cache-Control**: Public cache, max 1 day (86400 seconds)

Clients use these headers for cache validation:

**First Request**:
```http
GET /images/logo.png HTTP/1.1
Host: localhost:9000
```

**Subsequent Request (Cache Validation)**:
```http
GET /images/logo.png HTTP/1.1
Host: localhost:9000
If-None-Match: "a1b2c3d4-1706543210"
```

**Server Response (304 Not Modified)**:
```http
HTTP/1.1 304 Not Modified
ETag: "a1b2c3d4-1706543210"
Cache-Control: public, max-age=86400
```

### Development Mode

In development mode (`--dev`), static files don't use cache:

```http
HTTP/1.1 200 OK
Content-Type: text/css
Cache-Control: no-cache, no-store, must-revalidate
Pragma: no-cache
Expires: 0
```

This ensures latest file content is always loaded during development.

## Performance Optimization

### ETag Generation

TSP uses SHA-256 hash algorithm to generate ETag:

```typescript
// Pseudo code
const hash = await crypto.subtle.digest("SHA-256", fileContent);
const etag = `"${hash.slice(0, 16)}-${mtime}"`;
```

ETag includes:
- First 16 bytes of file content hash
- File modification timestamp

### Memory Usage

Static files are read directly from file system, not cached in memory. Each request reads file (leveraging OS file cache).

### Concurrency

Static file service supports high concurrency requests, each request is handled independently.

## Security Considerations

### File Extension Whitelist

Only extensions configured in `staticExtensions` will be served. This prevents:

- Accessing sensitive files (like `.env`, `.json` config files)
- Source code leakage (like `.ts`, `.tsx` files)
- Directory traversal attacks

### Path Security

Static file service inherits TSP's security features:

- ✅ Path traversal attack prevention (`../`)
- ✅ Path normalization
- ✅ Root directory boundary check

### Example: Secure Configuration

```jsonc
{
  // Only allow frontend resources, deny config files
  "staticExtensions": [
    ".css", ".js", ".png", ".jpg", ".jpeg",
    ".gif", ".svg", ".ico", ".woff", ".woff2"
  ]
}
```

## Priority with TSX Files

Processing priority (high to low):

1. **TSX Pages** - `.tsx` files always take priority, executed as pages
2. **Static Files** - Files with whitelisted extensions are served directly
3. **404** - Other requests return 404

Example:

```
Request: /about
1. Try loading www/about.tsx → Execute as page
2. If not exists, try loading www/about (static file) → Serve file
3. Neither exists → Return 404

Request: /styles/main.css
1. Try loading www/styles/main.css.tsx → Not exists
2. Try loading www/styles/main.css → Serve static file
3. Neither exists → Return 404
```

## Troubleshooting

### Static File Returns 404

**Problem**: Access `/style.css` returns 404

**Possible Causes**:
1. File extension not in whitelist
2. File doesn't exist
3. Path is incorrect

**Solution**:
```jsonc
{
  // Ensure extension is in list
  "staticExtensions": [".css", ".js", ".png"]
}
```

### Cache Issues

**Problem**: Modified static file, but browser shows old version

**Solution**:
1. Development mode: Start with `--dev`
2. Production mode: Clear browser cache or use forced refresh (Ctrl+F5)
3. Add version number: `/style.css?v=1.0.0`

### MIME Type Error

**Problem**: File downloads instead of displaying in browser

**Solution**:
Check if file extension is correct, TSP automatically recognizes MIME types.

If need custom MIME type, can modify `MIME_TYPES` mapping in `src/static.ts`.

## Advanced Usage

### Custom MIME Types

Edit `src/static.ts`:

```typescript
const MIME_TYPES: Record<string, string> = {
  // Add custom types
  ".wasm": "application/wasm",
  ".pdf": "application/pdf",
  // ... other types
};
```

### Custom Cache Policy

Edit `src/static.ts`, modify `Cache-Control` header:

```typescript
// Production mode: cache for 7 days
headers["Cache-Control"] = "public, max-age=604800";
```

## Best Practices

1. **Separate Static Resources**
   ```
   www/
   ├── pages/          # TSX pages
   ├── static/         # Static files
   │   ├── css/
   │   ├── js/
   │   └── images/
   └── components/     # Shared components
   ```

2. **Use Version Numbers**
   ```tsx
   <link rel="stylesheet" href="/static/style.css?v=1.0.0" />
   ```

3. **Production Optimization**
   - Compress static files (CSS, JS)
   - Use modern image formats (WebP)
   - Configure reasonable cache times

4. **Security Configuration**
   ```jsonc
   {
     // Only allow necessary static file types
     "staticExtensions": [
       ".css", ".js", ".png", ".jpg",
       ".jpeg", ".gif", ".svg", ".woff", ".woff2"
     ]
   }
   ```

## Related Documentation

- [Configuration Documentation](../configuration.md) - Configuration options details
- [Architecture Documentation](../architecture.md) - Learn static file service implementation
- [Security Documentation](../security.md) - Security related features

---

[← Back to Features](./README.md) | [← Back to Documentation Center](../README.md)
