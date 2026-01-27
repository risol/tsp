import type { PageContext } from "../src/cache.ts";

export default async function (context: PageContext) {
  const { method, url, query, body, cookies, file, root, headers } = context;

  // 处理 headers
  const headersObj: Record<string, string> = {};
  for (const [key, value] of headers.entries()) {
    headersObj[key] = value;
  }

  const queryParams = Object.keys(query).length > 0
    ? JSON.stringify(query, null, 2)
    : "暂无查询参数";

  const bodyData = body ? JSON.stringify(body, null, 2) : "暂无 POST 数据";

  const cookiesData = Object.keys(cookies).length > 0
    ? JSON.stringify(cookies, null, 2)
    : "暂无 Cookies";

  const style = `
    body {
      font-family: "Courier New", monospace;
      max-width: 1000px;
      margin: 0 auto;
      padding: 20px;
      background: #1e1e1e;
      color: #d4d4d4;
    }
    h1 {
      color: #4ec9b0;
      border-bottom: 2px solid #4ec9b0;
      padding-bottom: 10px;
    }
    .section {
      margin: 20px 0;
      padding: 15px;
      background: #252526;
      border-radius: 4px;
      border-left: 3px solid #007acc;
    }
    .section h2 {
      color: #569cd6;
      margin-top: 0;
    }
    pre {
      background: #1e1e1e;
      padding: 15px;
      border-radius: 4px;
      overflow-x: auto;
      border: 1px solid #3e3e42;
    }
    .key { color: #9cdcfe; }
    .string { color: #ce9178; }
    .number { color: #b5cea8; }
    .boolean { color: #569cd6; }
    a { color: #4ec9b0; }
    ul { padding-left: 20px; }
  ` as unknown as Record<string, string>;

  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>API 信息 - TSP-FPM</title>
        <style>{style}</style>
      </head>
      <body>
        <h1>🔧 API 请求信息</h1>

        <div class="section">
          <h2>请求方法</h2>
          <pre><span class="string">{method}</span></pre>
        </div>

        <div class="section">
          <h2>URL 信息</h2>
          <pre>{JSON.stringify({
            href: url.href,
            origin: url.origin,
            pathname: url.pathname,
            search: url.search
          }, null, 2)}</pre>
        </div>

        <div class="section">
          <h2>查询参数 (Query)</h2>
          <pre>{queryParams}</pre>
        </div>

        <div class="section">
          <h2>请求头 (Headers)</h2>
          <pre>{JSON.stringify(headersObj, null, 2)}</pre>
        </div>

        <div class="section">
          <h2>POST 数据 (Body)</h2>
          <pre>{bodyData}</pre>
        </div>

        <div class="section">
          <h2>Cookies</h2>
          <pre>{cookiesData}</pre>
        </div>

        <div class="section">
          <h2>文件路径</h2>
          <pre>{JSON.stringify({
            template: file,
            root
          }, null, 2)}</pre>
        </div>

        <div class="section">
          <h2>测试链接</h2>
          <ul>
            <li><a href="?name=test&lang=zh">?name=test&lang=zh</a></li>
            <li><a href="?debug=true&verbose=1">?debug=true&verbose=1</a></li>
            <li><a href="../index.tsx">返回首页</a></li>
          </ul>
        </div>
      </body>
    </html>
  );
}
