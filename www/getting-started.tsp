import { Sidebar } from "./components/Sidebar.tsx";
import { Footer } from "./components/Footer.tsx";

export default Page(async function(ctx) {
  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>快速开始 - TSP</title>
        <style dangerouslySetInnerHTML={{ __html: `
          * { margin: 0; padding: 0; box-sizing: border-box; }
          :root {
            --primary: #3178c6;
            --primary-dark: #235a97;
            --text: #24292e;
            --text-light: #586069;
            --bg: #ffffff;
            --bg-secondary: #f6f8fa;
            --border: #e1e4e8;
            --code-bg: #f6f8fa;
            --sidebar-width: 240px;
          }
          body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: var(--text);
            background: var(--bg);
          }
          .layout { display: flex; min-height: 100vh; }
          .sidebar {
            width: var(--sidebar-width);
            background: var(--bg-secondary);
            border-right: 1px solid var(--border);
            position: fixed;
            height: 100vh;
            overflow-y: auto;
            padding: 20px;
          }
          .main { margin-left: var(--sidebar-width); flex: 1; }
          .header { border-bottom: 1px solid var(--border); padding: 16px 32px; background: var(--bg); }
          .header-content { max-width: 1200px; margin: 0 auto; display: flex; justify-content: space-between; align-items: center; }
          .logo { font-size: 20px; font-weight: 600; color: var(--primary); text-decoration: none; }
          .nav-links { display: flex; gap: 24px; }
          .nav-links a { color: var(--text); text-decoration: none; font-size: 14px; }
          .nav-links a:hover { color: var(--primary); }
          .content { max-width: 900px; margin: 0 auto; padding: 40px 32px; }
          h1 { font-size: 36px; margin-bottom: 16px; font-weight: 600; }
          h2 { font-size: 24px; margin: 40px 0 16px; padding-bottom: 8px; border-bottom: 1px solid var(--border); }
          h3 { font-size: 18px; margin: 24px 0 12px; }
          p { margin-bottom: 16px; color: var(--text-light); }
          ul, ol { margin: 16px 0; padding-left: 24px; }
          li { margin-bottom: 8px; color: var(--text-light); }
          code { background: var(--code-bg); padding: 2px 6px; border-radius: 4px; font-size: 14px; }
          pre { background: var(--code-bg); padding: 16px; border-radius: 8px; overflow-x: auto; margin: 16px 0; }
          pre code { background: none; padding: 0; }
          .code-block { position: relative; }
          .code-block .lang { position: absolute; top: 8px; right: 12px; font-size: 12px; color: var(--text-light); }
          .btn { display: inline-block; background: var(--primary); color: white; padding: 10px 20px; border-radius: 6px; text-decoration: none; font-size: 14px; }
          .btn:hover { background: var(--primary-dark); }
          .btn-secondary { background: var(--bg-secondary); color: var(--text); border: 1px solid var(--border); }
          .btn-secondary:hover { background: var(--border); }
          .step { margin-bottom: 32px; }
          .step-number { display: inline-block; width: 28px; height: 28px; background: var(--primary); color: white; border-radius: 50%; text-align: center; line-height: 28px; margin-right: 8px; font-weight: 600; font-size: 14px; }
          .note { background: #fff8e1; border-left: 4px solid #ffc107; padding: 12px 16px; margin: 16px 0; border-radius: 0 4px 4px 0; }
          footer { background: var(--bg-secondary); border-top: 1px solid var(--border); padding: 24px 32px; text-align: center; color: var(--text-light); font-size: 14px; }
          .warning { background: #ffebee; border-left: 4px solid #f44336; padding: 12px 16px; margin: 16px 0; border-radius: 0 4px 4px 0; }
        `}} />
      </head>
      <body>
        <div class="layout">
          <Sidebar />
          <div class="main">
            <header class="header">
              <div class="header-content">
                <a href="/" class="logo">TSP</a>
                <nav class="nav-links">
                  <a href="/getting-started">快速开始</a>
                  <a href="/features">功能</a>
                  <a href="/examples">示例</a>
                </nav>
              </div>
            </header>

            <div class="content">
              <h1>快速开始</h1>
              <p>本指南将帮助你快速搭建 TSP 开发环境并创建第一个页面。</p>

              <div class="step">
                <h2><span class="step-number">1</span>编译 deno-tsp</h2>
                <p>先编译项目内置的 deno-tsp（无需依赖官方 Deno 可执行文件）：</p>
                <div class="code-block">
                  <span class="lang">bash</span>
                  <pre><code>sh ./tsp.sh build:denort
sh ./tsp.sh build:deno</code></pre>
                </div>
              </div>

              <div class="step">
                <h2><span class="step-number">2</span>克隆项目</h2>
                <div class="code-block">
                  <span class="lang">bash</span>
                  <pre><code>git clone https://github.com/your-repo/tsp.git
cd tsp</code></pre>
                </div>
              </div>

              <div class="step">
                <h2><span class="step-number">3</span>启动服务器</h2>
                <p>使用 sh 脚本启动开发服务器：</p>
                <div class="code-block">
                  <span class="lang">bash</span>
                  <pre><code>sh ./tsp.sh dev</code></pre>
                </div>
                <p>服务器将在 <code>http://localhost:9000</code> 启动。</p>
                <div class="note">
                  <strong>提示：</strong>开发模式下启用热重载，修改文件后会自动重新加载。
                </div>
              </div>

              <div class="step">
                <h2><span class="step-number">4</span>创建第一个页面</h2>
                <p>在 <code>www/</code> 目录下创建 <code>hello.tsp</code>：</p>
                <div class="code-block">
                  <span class="lang">tsx</span>
                  <pre><code>{`export default Page(async function(ctx, { logger }) {
  return (
    <html>
      <head>
        <title>Hello TSP</title>
      </head>
      <body>
        <h1>你好，TSP!</h1>
        <p>这是我的第一个页面。</p>
      </body>
    </html>
  );
});`}</code></pre>
                </div>
                <p>访问 <code>http://localhost:9000/hello</code> 即可看到页面。</p>
              </div>

              <div class="step">
                <h2><span class="step-number">5</span>使用组件</h2>
                <p>创建可复用的布局组件 <code>www/components/Layout.tsx</code>：</p>
                <div class="code-block">
                  <span class="lang">tsx</span>
                  <pre><code>{`export function Layout({ title, children }: {
  title: string;
  children?: unknown;
}) {
  return (
    <html>
      <head>
        <title>{title}</title>
        <meta charset="UTF-8" />
      </head>
      <body>
        <nav style={{ padding: "16px", background: "#f6f8fa" }}>
          <a href="/" style={{ marginRight: "16px" }}>首页</a>
          <a href="/about">关于</a>
        </nav>
        <main style={{ padding: "24px" }}>
          {children}
        </main>
      </body>
    </html>
  );
}`}</code></pre>
                </div>
                <p>在页面中导入使用：</p>
                <div class="code-block">
                  <span class="lang">tsx</span>
                  <pre><code>{`import { Layout } from "./components/Layout.tsx";

export default Page(async function(ctx) {
  return (
    <Layout title="关于">
      <h1>关于 TSP</h1>
      <p>这是一个基于 Deno 的 TypeScript 全栈框架。</p>
    </Layout>
  );
});`}</code></pre>
                </div>
              </div>

              <h2>下一步</h2>
              <ul>
                <li><a href="/features" style={{ color: "#3178c6" }}>了解完整功能</a> - 查看所有功能特性</li>
                <li><a href="/examples" style={{ color: "#3178c6" }}>代码示例</a> - 学习常见场景的实现</li>
                <li><a href="https://github.com/your-repo/tsp" style={{ color: "#3178c6" }}>GitHub</a> - 查看源码和文档</li>
              </ul>
            </div>

            <Footer />
          </div>
        </div>
      </body>
    </html>
  );
});

