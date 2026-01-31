/**
 * Page 包装器演示页面
 *
 * 展示 Page 包装器的使用方式（作为可选方案）
 * 推荐使用全局函数 session() 和 cookies()
 */

export default Page(async function(ctx, { session, cookies, logger, testFunc }) {
  // 获取当前用户
  const user = await session.getUser();

  return (
    <html>
      <head>
        <title>Page 包装器演示</title>
        <meta charset="UTF-8" />
        <style>
          {`
          body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 800px;
            margin: 40px auto;
            padding: 20px;
            line-height: 1.6;
          }
          h1 {
            color: #333;
            border-bottom: 2px solid #0070f3;
            padding-bottom: 10px;
          }
          .card {
            background: #f8f9fa;
            border: 1px solid #dee2e6;
            border-radius: 8px;
            padding: 20px;
            margin: 20px 0;
          }
          .info {
            background: #e7f3ff;
            border-left: 4px solid #0070f3;
            padding: 15px;
            margin: 20px 0;
            border-radius: 4px;
          }
          code {
            background: #e9ecef;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Courier New', monospace;
          }
          pre {
            background: #282c34;
            color: #abb2bf;
            padding: 15px;
            border-radius: 8px;
            overflow-x: auto;
          }
          pre code {
            background: transparent;
            padding: 0;
            color: #abb2bf;
          }
          a {
            display: inline-block;
            padding: 10px 20px;
            margin: 5px;
            background: #0070f3;
            color: white;
            text-decoration: none;
            border-radius: 5px;
            border: none;
            cursor: pointer;
          }
          a:hover {
            background: #0051cc;
          }
          `}
        </style>
      </head>
      <body>
        <h1>📦 Page 包装器演示</h1>

        <div class="card">
          <h2>当前状态</h2>
          <p><strong>当前用户：</strong> {user?.name || "未登录"}</p>
          <p><strong>测试函数：</strong> {testFunc()}</p>
        </div>

        <div class="info">
          <strong>💡 推荐方式：</strong>
          <p>使用全局函数更简洁：</p>
          <pre><code>{`
export default async function(ctx) {
  const user = await session.getUser();
  cookies.set('theme', 'dark');
  return <div>欢迎, {user?.name}</div>;
};
          `.trim()}</code></pre>
        </div>

        <div class="card">
          <h2>Page 包装器方式（可选）</h2>
          <p>本页面使用的是 Page 包装器方式：</p>
          <pre><code>{`
export default Page(async function(ctx, { session, cookies, logger }) {
  const user = await session.getUser();
  logger('用户访问');
  cookies.set('theme', 'dark');
  return <div>欢迎, {user?.name}</div>;
});
          `.trim()}</code></pre>
          <p><strong>特点：</strong></p>
          <ul>
            <li>依赖通过函数参数注入</li>
            <li>需要显式声明依赖</li>
            <li>适合需要多种依赖的场景</li>
            <li>提供完整的类型提示</li>
          </ul>
        </div>

        <div class="card">
          <h2>对比</h2>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <tr style={{ borderBottom: "1px solid #dee2e6" }}>
              <th style={{ padding: "10px", textAlign: "left" }}>特性</th>
              <th style={{ padding: "10px", textAlign: "left" }}>全局函数</th>
              <th style={{ padding: "10px", textAlign: "left" }}>Page 包装器</th>
            </tr>
            <tr style={{ borderBottom: "1px solid #dee2e6" }}>
              <td style={{ padding: "10px" }}>语法简洁性</td>
              <td style={{ padding: "10px", color: "green" }}>✓ 更简洁</td>
              <td style={{ padding: "10px" }}>需要包装器</td>
            </tr>
            <tr style={{ borderBottom: "1px solid #dee2e6" }}>
              <td style={{ padding: "10px" }}>类型提示</td>
              <td style={{ padding: "10px", color: "green" }}>✓ 完整</td>
              <td style={{ padding: "10px", color: "green" }}>✓ 完整</td>
            </tr>
            <tr style={{ borderBottom: "1px solid #dee2e6" }}>
              <td style={{ padding: "10px" }}>依赖注入</td>
              <td style={{ padding: "10px" }}>仅 session/cookies</td>
              <td style={{ padding: "10px", color: "green" }}>✓ 支持自定义依赖</td>
            </tr>
            <tr>
              <td style={{ padding: "10px" }}>推荐场景</td>
              <td style={{ padding: "10px", color: "green" }}>✓ 大多数页面</td>
              <td style={{ padding: "10px" }}>复杂依赖场景</td>
            </tr>
          </table>
        </div>

        <div style={{ marginTop: "20px" }}>
          <a href="/">返回首页</a>
          <a href="/session_demo.tsx">Session 演示</a>
          <a href="/cookie_demo.tsx">Cookie 演示</a>
        </div>
      </body>
    </html>
  );
});
