export default async function (context: PageContext) {
  const { method, url, query, body, cookies, file, root } = context;
  const name = query.name ?? "World";
  const queryParams = Object.keys(query).length > 0
    ? JSON.stringify(query, null, 2)
    : "暂无查询参数";

  const hasPostData = method === "POST" && body;
  const postData = hasPostData ? JSON.stringify(body, null, 2) : "暂无 POST 数据";

  const cookiesData = Object.keys(cookies).length > 0
    ? JSON.stringify(cookies, null, 2)
    : "暂无 Cookies";

  const style = `
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      min-height: 100vh;
      padding: 20px;
    }
    .container {
      max-width: 800px;
      margin: 0 auto;
      background: white;
      border-radius: 12px;
      padding: 40px;
      box-shadow: 0 20px 60px rgba(0,0,0,0.3);
    }
    h1 {
      color: #333;
      margin-bottom: 10px;
    }
    .subtitle {
      color: #666;
      margin-bottom: 30px;
      font-size: 14px;
    }
    .section {
      margin: 30px 0;
      padding: 20px;
      background: #f8f9fa;
      border-radius: 8px;
    }
    .section h2 {
      color: #444;
      font-size: 18px;
      margin-bottom: 15px;
    }
    .info-item {
      margin: 10px 0;
      padding: 10px;
      background: white;
      border-radius: 4px;
      border-left: 3px solid #667eea;
    }
    .label {
      font-weight: bold;
      color: #667eea;
    }
    pre {
      background: #2d3748;
      color: #e2e8f0;
      padding: 15px;
      border-radius: 6px;
      overflow-x: auto;
      font-size: 13px;
      line-height: 1.5;
    }
    form {
      margin-top: 15px;
    }
    input[type="text"] {
      padding: 10px 15px;
      border: 2px solid #e2e8f0;
      border-radius: 6px;
      font-size: 14px;
      width: 300px;
    }
    button {
      padding: 10px 20px;
      background: #667eea;
      color: white;
      border: none;
      border-radius: 6px;
      cursor: pointer;
      font-size: 14px;
      margin-left: 10px;
    }
    button:hover {
      background: #5568d3;
    }
  `;

  return (
    <>
      <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>TSP-FPM 示例</title>
        <style>{`
          * { margin: 0; padding: 0; box-sizing: border-box; }
          body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
          }
          .container {
            max-width: 800px;
            margin: 0 auto;
            background: white;
            border-radius: 12px;
            padding: 40px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
          }
          h1 {
            color: #333;
            margin-bottom: 10px;
          }
          .subtitle {
            color: #666;
            margin-bottom: 30px;
            font-size: 14px;
          }
          .section {
            margin: 30px 0;
            padding: 20px;
            background: #f8f9fa;
            border-radius: 8px;
          }
          .section h2 {
            color: #444;
            font-size: 18px;
            margin-bottom: 15px;
          }
          .info-item {
            margin: 10px 0;
            padding: 10px;
            background: white;
            border-radius: 4px;
            border-left: 3px solid #667eea;
          }
          .label {
            font-weight: bold;
            color: #667eea;
          }
          pre {
            background: #2d3748;
            color: #e2e8f0;
            padding: 15px;
            border-radius: 6px;
            overflow-x: auto;
            font-size: 13px;
            line-height: 1.5;
          }
          form {
            margin-top: 15px;
          }
          input[type="text"] {
            padding: 10px 15px;
            border: 2px solid #e2e8f0;
            border-radius: 6px;
            font-size: 14px;
            width: 300px;
          }
          button {
            padding: 10px 20px;
            background: #667eea;
            color: white;
            border: none;
            border-radius: 6px;
            cursor: pointer;
            font-size: 14px;
            margin-left: 10px;
          }
          button:hover {
            background: #5568d3;
          }
        `}</style>
      </head>
      <body>
        <div class="container">
          <h1>🎉 TSP-FPM 运行成功！</h1>
          <p class="subtitle">类 PHP-FPM 模板执行引擎 (TSX + Preact 版本)</p>

          <div class="section">
            <h2>👋 欢迎使用</h2>
            <p>Hello <strong>{name}</strong>!</p>
            <p style={{ marginTop: "10px" }}>
              <a href="?name=用户">点击这里</a> 设置你的名字
            </p>
          </div>

          <div class="section">
            <h2>📋 请求信息</h2>
            <div class="info-item">
              <span class="label">请求方法:</span> {method}
            </div>
            <div class="info-item">
              <span class="label">请求路径:</span> {url.pathname}
            </div>
            <div class="info-item">
              <span class="label">模板文件:</span> {file}
            </div>
            <div class="info-item">
              <span class="label">文档根目录:</span> {root}
            </div>
          </div>

          <div class="section">
            <h2>🔍 查询参数</h2>
            <pre>{queryParams}</pre>
            <form method="GET">
              <input type="text" name="custom" placeholder="输入自定义参数" />
              <button type="submit">提交 GET 请求</button>
            </form>
          </div>

          <div class="section">
            <h2>📝 POST 数据测试</h2>
            <pre>{postData}</pre>
            <form method="POST" style={{ marginTop: "15px" }}>
              <input type="text" name="username" placeholder="用户名" required />
              <button type="submit">提交 POST 请求</button>
            </form>
          </div>

          <div class="section">
            <h2>🍪 Cookies</h2>
            <pre>{cookiesData}</pre>
          </div>

          <div class="section">
            <h2>🚀 下一步</h2>
            <ul style={{ marginLeft: "20px", lineHeight: "1.8" }}>
              <li>编辑 <code>www/index.tsx</code> 自定义你的页面</li>
              <li>在 <code>www/</code> 目录下创建更多 .tsx 文件</li>
              <li>使用 <code>export default async function()</code> 导出处理函数</li>
              <li>使用 TypeScript + Preact 生成 HTML</li>
            </ul>
          </div>
        </div>
      </body>
      </html>
    </>
  );
}
