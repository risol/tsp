/**
 * 静态文件服务演示页面
 * 展示如何在 TSX 页面中使用静态文件
 */

export default async function (context: PageContext) {
  return (
    <html>
      <head>
        <title>静态文件服务演示</title>
        {/* 引用静态 CSS 文件 */}
        <link rel="stylesheet" href="/static/example.css" />
      </head>
      <body>
        <h1>静态文件服务演示</h1>

        <div class="container">
          <h2>功能特性</h2>
          <ul>
            <li>✅ 自动 MIME 类型识别</li>
            <li>✅ HTTP 缓存支持（ETag、Last-Modified）</li>
            <li>✅ 开发模式禁用缓存</li>
            <li>✅ 可配置的文件扩展名白名单</li>
            <li>✅ 高性能文件读取</li>
          </ul>
        </div>

        <div class="container">
          <h2>支持的文件类型</h2>
          <p>
            默认支持：CSS, JavaScript, JSON, PNG, JPG, GIF, SVG, ICO,
            WebP, WOFF, WOFF2, TTF, EOT, MP3, MP4, WebM, TXT, MD, XML
          </p>
          <p>
            可通过 <code>config.json</code> 中的{" "}
            <code>staticExtensions</code> 字段自定义。
          </p>
        </div>

        <div class="container">
          <h2>HTTP 缓存</h2>
          <p>
            生产模式下，静态文件会自动添加 ETag 和 Last-Modified
            头，支持 304 Not Modified 响应。
          </p>
          <p>
            开发模式下（<code>--dev</code>
           ），缓存被禁用，确保始终加载最新文件。
          </p>
        </div>

        <div class="container">
          <h2>配置示例</h2>
          <pre>
            {`{
  "staticExtensions": [
    ".css",
    ".js",
    ".png",
    ".jpg",
    ".svg"
  ]
}`}
          </pre>
        </div>

        <div class="container">
          <h2>测试</h2>
          <p>
            打开浏览器开发者工具，查看网络请求，可以看到样式表被正确加载。
          </p>
          <a href="/" class="button">
            返回首页
          </a>
        </div>

        <div class="container">
          <h2>当前请求信息</h2>
          <p>
            <strong>Method:</strong> {context.method}
          </p>
          <p>
            <strong>URL:</strong> {context.url.pathname}
          </p>
          <p>
            <strong>User-Agent:</strong>{" "}
            {context.headers.get("user-agent") || "Unknown"}
          </p>
        </div>
      </body>
    </html>
  );
}
