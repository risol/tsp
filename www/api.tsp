<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>API 信息 - TSP-FPM</title>
  <style>
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
  </style>
</head>
<body>
  <h1>🔧 API 请求信息</h1>

  <div class="section">
    <h2>请求方法</h2>
    <pre><span class="string"><%= it.method %></span></pre>
  </div>

  <div class="section">
    <h2>URL 信息</h2>
    <pre>{
  <span class="key">"href"</span>: <span class="string">"<%= it.url.href %>"</span>,
  <span class="key">"origin"</span>: <span class="string">"<%= it.url.origin %>"</span>,
  <span class="key">"pathname"</span>: <span class="string">"<%= it.url.pathname %>"</span>,
  <span class="key">"search"</span>: <span class="string">"<%= it.url.search %>"</span>
}</pre>
  </div>

  <div class="section">
    <h2>查询参数 (Query)</h2>
    <% if (Object.keys(it.query).length > 0) { %>
      <pre><%= JSON.stringify(it.query, null, 2) %></pre>
    <% } else { %>
      <pre><span class="string">暂无查询参数</span></pre>
    <% } %>
  </div>

  <div class="section">
    <h2>请求头 (Headers)</h2>
    <pre><%
      const headers = {};
      for (const [key, value] of it.headers.entries()) {
        headers[key] = value;
      }
    %><%= JSON.stringify(headers, null, 2) %></pre>
  </div>

  <div class="section">
    <h2>POST 数据 (Body)</h2>
    <% if (it.body) { %>
      <pre><%= JSON.stringify(it.body, null, 2) %></pre>
    <% } else { %>
      <pre><span class="string">暂无 POST 数据</span></pre>
    <% } %>
  </div>

  <div class="section">
    <h2>Cookies</h2>
    <% if (Object.keys(it.cookies).length > 0) { %>
      <pre><%= JSON.stringify(it.cookies, null, 2) %></pre>
    <% } else { %>
      <pre><span class="string">暂无 Cookies</span></pre>
    <% } %>
  </div>

  <div class="section">
    <h2>文件路径</h2>
    <pre>{
  <span class="key">"template"</span>: <span class="string">"<%= it.file %>"</span>,
  <span class="key">"root"</span>: <span class="string">"<%= it.root %>"</span>
}</pre>
  </div>

  <div class="section">
    <h2>测试链接</h2>
    <ul>
      <li><a href="?name=test&lang=zh" style="color: #4ec9b0;">?name=test&lang=zh</a></li>
      <li><a href="?debug=true&verbose=1" style="color: #4ec9b0;">?debug=true&verbose=1</a></li>
      <li><a href="../index.tsp" style="color: #4ec9b0;">返回首页</a></li>
    </ul>
  </div>
</body>
</html>
