# 静态文件服务

TSP 内置了静态文件服务功能，可以直接提供 CSS、JavaScript、图片等静态资源，无需额外的 Web 服务器。

## 功能特性

- ✅ 自动 MIME 类型识别
- ✅ HTTP 缓存支持（ETag、Last-Modified）
- ✅ 开发模式禁用缓存
- ✅ 可配置的文件扩展名白名单
- ✅ 高性能文件读取
- ✅ 304 Not Modified 响应支持

## 配置

### 默认支持的文件类型

如果不配置，TSP 默认支持以下静态文件类型：

| 扩展名 | MIME 类型 | 说明 |
|--------|-----------|------|
| `.css` | `text/css` | 样式表 |
| `.js` | `application/javascript` | JavaScript |
| `.json` | `application/json` | JSON 数据 |
| `.png` | `image/png` | PNG 图片 |
| `.jpg`, `.jpeg` | `image/jpeg` | JPEG 图片 |
| `.gif` | `image/gif` | GIF 图片 |
| `.svg` | `image/svg+xml` | SVG 矢量图 |
| `.ico` | `image/x-icon` | 图标 |
| `.webp` | `image/webp` | WebP 图片 |
| `.woff` | `font/woff` | WOFF 字体 |
| `.woff2` | `font/woff2` | WOFF2 字体 |
| `.ttf` | `font/ttf` | TTF 字体 |
| `.eot` | `application/vnd.ms-fontobject` | EOT 字体 |
| `.mp3` | `audio/mpeg` | MP3 音频 |
| `.mp4` | `video/mp4` | MP4 视频 |
| `.webm` | `video/webm` | WebM 视频 |
| `.txt` | `text/plain` | 文本文件 |
| `.md` | `text/markdown` | Markdown 文档 |
| `.xml` | `application/xml` | XML 文档 |

### 自定义文件类型

通过配置文件自定义允许的静态文件扩展名：

**config.jsonc**:
```jsonc
{
  "root": "./www",
  "port": 9000,
  "dev": false,

  // 只允许特定的静态文件类型
  "staticExtensions": [".css", ".js", ".png", ".jpg", ".svg"]
}
```

**命令行参数**（当前版本不支持，仅支持配置文件）：
```bash
# 未来可能支持
./tspserver --static-extensions .css,.js,.png
```

### 禁用静态文件服务

如果不希望提供静态文件服务，设置空数组：

```jsonc
{
  "staticExtensions": []
}
```

## 使用示例

### 基本用法

假设你有以下项目结构：

```
www/
├── index.tsx          # TSX 页面
├── styles/
│   └── main.css       # 静态 CSS
├── js/
│   └── app.js         # 静态 JS
└── images/
    └── logo.png       # 静态图片
```

在 `index.tsx` 中引用静态文件：

```tsx
export default async function(ctx: PageContext) {
  return (
    <html>
      <head>
        <title>静态文件示例</title>
        <link rel="stylesheet" href="/styles/main.css" />
      </head>
      <body>
        <h1>欢迎使用 TSP</h1>
        <img src="/images/logo.png" alt="Logo" />
        <script src="/js/app.js"></script>
      </body>
    </html>
  );
}
```

### TSX 页面中的引用

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

## HTTP 缓存

### 生产模式

在生产模式下，静态文件会自动添加缓存头：

```http
HTTP/1.1 200 OK
Content-Type: image/png
ETag: "a1b2c3d4-1706543210"
Last-Modified: Mon, 29 Jan 2026 12:34:56 GMT
Cache-Control: public, max-age=86400
```

- **ETag**: 文件内容的哈希值
- **Last-Modified**: 文件最后修改时间
- **Cache-Control**: 公开缓存，最大时间 1 天（86400 秒）

客户端会使用这些头信息进行缓存验证：

**首次请求**:
```http
GET /images/logo.png HTTP/1.1
Host: localhost:9000
```

**再次请求（缓存验证）**:
```http
GET /images/logo.png HTTP/1.1
Host: localhost:9000
If-None-Match: "a1b2c3d4-1706543210"
```

**服务器响应（304 Not Modified）**:
```http
HTTP/1.1 304 Not Modified
ETag: "a1b2c3d4-1706543210"
Cache-Control: public, max-age=86400
```

### 开发模式

在开发模式下（`--dev`），静态文件不会使用缓存：

```http
HTTP/1.1 200 OK
Content-Type: text/css
Cache-Control: no-cache, no-store, must-revalidate
Pragma: no-cache
Expires: 0
```

这确保开发时始终加载最新的文件内容。

## 性能优化

### ETag 生成

TSP 使用 SHA-256 哈希算法生成 ETag：

```typescript
// 伪代码
const hash = await crypto.subtle.digest("SHA-256", fileContent);
const etag = `"${hash.slice(0, 16)}-${mtime}"`;
```

ETag 包含：
- 文件内容的前 16 字节哈希
- 文件修改时间戳

### 内存使用

静态文件直接从文件系统读取，不缓存在内存中。每次请求都会读取文件（利用操作系统的文件缓存）。

### 并发处理

静态文件服务支持高并发请求，每个请求独立处理。

## 安全考虑

### 文件扩展名白名单

只有配置在 `staticExtensions` 中的扩展名才会被提供。这可以防止：

- 访问敏感文件（如 `.env`, `.json` 配置文件）
- 源代码泄露（如 `.ts`, `.tsx` 文件）
- 目录遍历攻击

### 路径安全

静态文件服务继承了 TSP 的安全特性：

- ✅ 路径穿越攻击防护（`../`）
- ✅ 路径归一化
- ✅ 根目录边界检查

### 示例：安全的配置

```jsonc
{
  // 只允许前端资源，禁止访问配置文件
  "staticExtensions": [
    ".css", ".js", ".png", ".jpg", ".jpeg",
    ".gif", ".svg", ".ico", ".woff", ".woff2"
  ]
}
```

## 与 TSX 文件的优先级

处理优先级（从高到低）：

1. **TSX 页面** - `.tsx` 文件始终优先，会被执行为页面
2. **静态文件** - 扩展名在白名单中的文件会被直接提供
3. **404** - 其他请求返回 404

示例：

```
请求: /about
1. 尝试加载 www/about.tsx → 执行为页面
2. 如果不存在，尝试加载 www/about（静态文件）→ 提供文件
3. 都不存在 → 返回 404

请求: /styles/main.css
1. 尝试加载 www/styles/main.css.tsx → 不存在
2. 尝试加载 www/styles/main.css → 提供静态文件
3. 都不存在 → 返回 404
```

## 故障排查

### 静态文件返回 404

**问题**: 访问 `/style.css` 返回 404

**可能原因**:
1. 文件扩展名不在白名单中
2. 文件不存在
3. 路径错误

**解决方案**:
```jsonc
{
  // 确保扩展名在列表中
  "staticExtensions": [".css", ".js", ".png"]
}
```

### 缓存问题

**问题**: 修改了静态文件，但浏览器显示旧版本

**解决方案**:
1. 开发模式：使用 `--dev` 启动
2. 生产模式：清除浏览器缓存或使用强制刷新（Ctrl+F5）
3. 添加版本号：`/style.css?v=1.0.0`

### MIME 类型错误

**问题**: 文件下载而不是在浏览器中显示

**解决方案**:
检查文件扩展名是否正确，TSP 会自动识别 MIME 类型。

如果需要自定义 MIME 类型，可以修改 `src/static.ts` 中的 `MIME_TYPES` 映射表。

## 高级用法

### 自定义 MIME 类型

编辑 `src/static.ts`:

```typescript
const MIME_TYPES: Record<string, string> = {
  // 添加自定义类型
  ".wasm": "application/wasm",
  ".pdf": "application/pdf",
  // ... 其他类型
};
```

### 自定义缓存策略

编辑 `src/static.ts`，修改 `Cache-Control` 头：

```typescript
// 生产模式：缓存 7 天
headers["Cache-Control"] = "public, max-age=604800";
```

## 最佳实践

1. **分离静态资源**
   ```
   www/
   ├── pages/          # TSX 页面
   ├── static/         # 静态文件
   │   ├── css/
   │   ├── js/
   │   └── images/
   └── components/     # 共享组件
   ```

2. **使用版本号**
   ```tsx
   <link rel="stylesheet" href="/static/style.css?v=1.0.0" />
   ```

3. **生产环境优化**
   - 压缩静态文件（CSS、JS）
   - 使用现代图片格式（WebP）
   - 配置合理的缓存时间

4. **安全配置**
   ```jsonc
   {
     // 只允许必要的静态文件类型
     "staticExtensions": [
       ".css", ".js", ".png", ".jpg",
       ".jpeg", ".gif", ".svg", ".woff", ".woff2"
     ]
   }
   ```

## 相关文档

- [配置文件说明](../configuration.md) - 配置选项详解
- [架构设计](../architecture.md) - 了解静态文件服务的实现
- [安全特性](../security.md) - 安全相关功能

---

[← 返回功能特性](./README.md) | [← 返回文档中心](../README.md)
