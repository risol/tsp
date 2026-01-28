import { withDeps } from "../src/injection.ts";

export default withDeps(async function(context, { testFunc }) {
  // 调用注入的 testFunc
  testFunc();

  return (
    <>
      <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>依赖注入测试</title>
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
            margin-bottom: 20px;
          }
          .success {
            color: #10b981;
            font-size: 18px;
            font-weight: bold;
          }
          .info {
            margin: 20px 0;
            padding: 15px;
            background: #f0f9ff;
            border-left: 4px solid #3b82f6;
            border-radius: 4px;
          }
          code {
            background: #f3f4f6;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Courier New', monospace;
          }
        `}</style>
      </head>
      <body>
        <div class="container">
          <h1>🎯 依赖注入测试</h1>
          <p class="success">✓ testFunc 已成功注入并执行！</p>
          <p style={{ marginTop: '20px' }}>
            请查看控制台，应该能看到输出 "testFunc"
          </p>

          <div class="info">
            <strong>使用方式：</strong>
            <pre style={{ marginTop: '10px' }}><code>{`import { withDeps } from "../src/injection.ts";

export default withDeps(async function(context, { testFunc }) {
  testFunc();  // 调用注入的函数

  return <div>Hello</div>;
});`}</code></pre>
          </div>
        </div>
      </body>
      </html>
    </>
  );
});
