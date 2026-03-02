import { Nav } from "./components/Nav.tsx";
import { Sidebar } from "./components/Sidebar.tsx";
import { Footer } from "./components/Footer.tsx";

export default Page(async function(ctx) {
  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>TSP - TypeScript Full-Stack Framework</title>
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
          .feature-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 24px; margin: 24px 0; }
          .feature-card {
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 20px;
            transition: box-shadow 0.2s;
          }
          .feature-card:hover { box-shadow: 0 4px 12px rgba(0,0,0,0.1); }
          .feature-card h3 { margin-top: 0; color: var(--text); }
          .feature-card p { margin-bottom: 0; font-size: 14px; }
          .badge { display: inline-block; background: #e8f5e9; color: #2e7d32; padding: 2px 8px; border-radius: 4px; font-size: 12px; margin-left: 8px; }
          .btn { display: inline-block; background: var(--primary); color: white; padding: 10px 20px; border-radius: 6px; text-decoration: none; font-size: 14px; }
          .btn:hover { background: var(--primary-dark); }
          .btn-secondary { background: var(--bg-secondary); color: var(--text); border: 1px solid var(--border); }
          .btn-secondary:hover { background: var(--border); }
          .cta { text-align: center; padding: 40px 0; }
          .cta .btn { margin: 0 8px; }
          .code-block { position: relative; }
          .code-block .lang { position: absolute; top: 8px; right: 12px; font-size: 12px; color: var(--text-light); }
          footer { background: var(--bg-secondary); border-top: 1px solid var(--border); padding: 24px 32px; text-align: center; color: var(--text-light); font-size: 14px; }
          .highlight { background: #fff3cd; padding: 2px 4px; border-radius: 2px; }
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
              <h1>TSP: TypeScript Full-Stack Framework</h1>
              <p style={{ fontSize: "18px" }}>
                基于 Deno 的 TypeScript 全栈框架，使用 .tsp/.tsx 文件路由构建现代 Web 应用
              </p>

              <div class="cta">
                <a href="/getting-started" class="btn">快速开始</a>
                <a href="/features" class="btn btn-secondary">了解更多</a>
              </div>

              <h2>为什么选择 TSP?</h2>

              <div class="feature-grid">
                <div class="feature-card">
                  <h3>🚀 简单易用</h3>
                  <p>像 PHP 一样直接执行 .tsp 文件，无需复杂的构建配置，写完即可运行。</p>
                </div>
                <div class="feature-card">
                  <h3>🔥 热重载</h3>
                  <p>修改文件后立即生效，支持任意层级的嵌套依赖，开发体验极佳。</p>
                </div>
                <div class="feature-card">
                  <h3>💎 类型安全</h3>
                  <p>完整的 TypeScript 支持，依赖注入系统提供完整的类型推断。</p>
                </div>
                <div class="feature-card">
                  <h3>🗄️ 数据库集成</h3>
                  <p>Schema-first MySQL/Redis/LDAP 支持，类型安全的数据库查询 API。</p>
                </div>
                <div class="feature-card">
                  <h3>📦 功能完整</h3>
                  <p>Session、Cookies、文件上传、静态文件服务等，开箱即用。</p>
                </div>
                <div class="feature-card">
                  <h3>🔧 配置灵活</h3>
                  <p>配置文件支持 JSON/JSONC，修改后自动生效，无需重启服务器。</p>
                </div>
              </div>

              <h2>安装使用</h2>

              <p>使用项目内置的 deno-tsp 运行时，无需依赖官方 Deno 可执行文件：</p>

              <div class="code-block">
                <span class="lang">bash</span>
                <pre><code># 编译 deno-tsp
sh ./tsp.sh build:denort
sh ./tsp.sh build:deno

# 启动开发服务器
sh ./tsp.sh dev</code></pre>
              </div>

              <p>常用命令：</p>

              <div class="code-block">
                <span class="lang">bash</span>
                <pre><code># 生产模式
sh ./tsp.sh start

# 编译二进制
sh ./tsp.sh build:tspserver
sh ./tsp.sh build:tspserver:rel</code></pre>
              </div>

              <h2>创建你的第一个页面</h2>

              <p>在 <code>www/</code> 目录下创建 <code>index.tsp</code>：</p>

              <div class="code-block">
                <span class="lang">tsx</span>
                <pre><code>{`export default Page(async function(ctx, { logger }) {
  logger.info('首页被访问');

  return (
    <html>
      <head>
        <title>我的 TSP 网站</title>
      </head>
      <body>
        <h1>欢迎来到 TSP!</h1>
        <p>当前时间: {new Date().toLocaleString()}</p>
      </body>
    </html>
  );
});`}</code></pre>
              </div>

              <p>然后访问 <code>http://localhost:9000</code> 即可看到页面。</p>

              <h2>下一步</h2>

              <ul>
                <li><a href="/getting-started">快速开始</a> - 完整的安装和配置指南</li>
                <li><a href="/features">功能详情</a> - 了解更多特性</li>
                <li><a href="/examples">代码示例</a> - 常见场景的代码示例</li>
              </ul>
            </div>

            <Footer />
          </div>
        </div>
      </body>
    </html>
  );
});

