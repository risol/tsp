import type { PageContext } from "../../src/cache.ts";

export default async function (context: PageContext) {
  const { method, url, headers } = context;
  const userAgent = headers.get("user-agent") || "Unknown";

  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <title>API 信息 - TSP-FPM</title>
      </head>
      <body>
        <h1>API 信息</h1>
        <h2>请求信息</h2>
        <p>请求方法: <strong>{method}</strong></p>
        <p>请求路径: <strong>{url.pathname}</strong></p>
        <p>User-Agent: <strong>{userAgent}</strong></p>

        <h2>上下文数据</h2>
        <ul>
          <li>method: {method}</li>
          <li>url: {url.href}</li>
          <li>pathname: {url.pathname}</li>
        </ul>
      </body>
    </html>
  );
}
