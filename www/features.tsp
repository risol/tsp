import { Sidebar } from "../components/Sidebar.tsx";
import { Footer } from "../components/Footer.tsx";

export default Page(async function(ctx) {
  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>功能特性 - TSP</title>
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
          .feature-section { margin-bottom: 48px; }
          .feature-icon { font-size: 32px; margin-bottom: 12px; }
          .feature-card {
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 24px;
            margin: 16px 0;
          }
          .feature-card h3 { margin-top: 0; }
          .badge { display: inline-block; background: #e8f5e9; color: #2e7d32; padding: 2px 8px; border-radius: 4px; font-size: 12px; margin-left: 8px; }
          .badge.new { background: #e3f2fd; color: #1976d2; }
          footer { background: var(--bg-secondary); border-top: 1px solid var(--border); padding: 24px 32px; text-align: center; color: var(--text-light); font-size: 14px; }
          table { width: 100%; border-collapse: collapse; margin: 16px 0; }
          th, td { border: 1px solid var(--border); padding: 12px; text-align: left; }
          th { background: var(--bg-secondary); }
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
              <h1>功能特性</h1>
              <p>TSP 是基于 Deno 的 TypeScript 全栈框架，提供同进程热更新、类型安全依赖注入与工程化部署能力。</p>

              <div class="feature-section">
                <h2><span class="feature-icon">🔥</span>热重载</h2>
                <p>修改文件后立即生效，无需手动重启服务器。</p>
                <ul>
                  <li>支持任意层级的嵌套依赖</li>
                  <li>智能缓存，只重新编译修改的文件</li>
                  <li>并发请求下保证缓存一致性</li>
                </ul>
                <div class="feature-card">
                  <h3>工作原理</h3>
                  <p>TSP 使用基于文件修改时间 (mtime) 的缓存机制。当文件发生变化时，会自动清除缓存并重新编译。所有依赖该文件的模块也会被递归清除缓存。</p>
                </div>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">💎</span>类型安全</h2>
                <p>完整的 TypeScript 支持，从数据验证到数据库查询，全程类型安全。</p>
                <ul>
                  <li>全局类型声明，无需手动导入</li>
                  <li>依赖注入系统提供完整的类型推断</li>
                  <li>Schema-first 数据验证</li>
                </ul>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">🗄️</span>数据库集成</h2>
                <p>内置 MySQL、Redis、LDAP 支持，所有 API 都是类型安全的。</p>

                <h3>MySQL Schema-first API</h3>
                <table>
                  <tr>
                    <th>方法</th>
                    <th>说明</th>
                  </tr>
                  <tr>
                    <td><code>query</code></td>
                    <td>多行查询</td>
                  </tr>
                  <tr>
                    <td><code>queryOne</code></td>
                    <td>严格单行（0行或多行报错）</td>
                  </tr>
                  <tr>
                    <td><code>queryMaybe</code></td>
                    <td>可选单行（0行返回 null）</td>
                  </tr>
                  <tr>
                    <td><code>scalar</code></td>
                    <td>单值查询</td>
                  </tr>
                  <tr>
                    <td><code>execute</code></td>
                    <td>写操作（INSERT/UPDATE/DELETE）</td>
                  </tr>
                  <tr>
                    <td><code>tx</code></td>
                    <td>事务操作</td>
                  </tr>
                  <tr>
                    <td><code>queryPage</code></td>
                    <td>分页查询</td>
                  </tr>
                </table>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">⚙️</span>依赖注入</h2>
                <p>类型安全的依赖注入系统，按需加载，提升性能。</p>
                <div class="code-block">
                  <span class="lang">tsx</span>
                  <pre><code>{`// 在页面函数中直接使用依赖
export default Page(async function(ctx, { session, createMySQL, logger }) {
  // logger - 日志记录
  logger.info('页面加载');

  // session - 会话管理
  await session.init();
  await session.set('userId', '123');

  // createMySQL - 数据库连接
  const db = await createMySQL({ host: 'localhost', user: 'root', password: '123456', database: 'test' });

  const users = await db.query('SELECT * FROM users');
  return <div>用户数: {users.length}</div>;
});`}</code></pre>
                </div>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">📦</span>开箱即用</h2>
                <p>内置多种功能，无需额外安装：</p>
                <ul>
                  <li><strong>Session 管理</strong> - 安全可靠的会话存储</li>
                  <li><strong>Cookies</strong> - 便捷的 Cookie 操作</li>
                  <li><strong>文件上传</strong> - 支持多文件、大文件</li>
                  <li><strong>静态文件</strong> - HTML、CSS、JS、图片等</li>
                  <li><strong>文件管理器</strong> <span class="badge new">NEW</span> - Web 界面的文件管理</li>
                  <li><strong>访问日志</strong> - 可配置的访问日志</li>
                </ul>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">🔧</span>配置灵活</h2>
                <p>支持 JSON/JSONC 配置文件，修改后自动生效。</p>
                <div class="code-block">
                  <span class="lang">jsonc</span>
                  <pre><code>{`{
  // 基础配置
  "root": "./www",
  "port": 9000,

  // 热重载配置
  "hotReload": true,

  // Session 配置
  "session": {
    "secure": false,
    "maxAge": 86400,
    "sameSite": "Strict"
  },

  // 日志配置
  "logger": {
    "level": "INFO",
    "file": ".logs/app.log"
  },

  // 文件管理器
  "fileManager": {
    "enabled": true,
    "path": "/__filemanager",
    "password": "your_password"
  }
}`}</code></pre>
                </div>
              </div>

              <div class="feature-section">
                <h2><span class="feature-icon">🌐</span>跨平台编译</h2>
                <p>支持编译为独立的可执行文件，可在 Windows、Linux、macOS 上运行。</p>
                <div class="code-block">
                  <span class="lang">bash</span>
                  <pre><code># 构建发布包（支持多平台）
deno task build

# 构建产物在 release/ 目录
cd release
./tspserver --root ./www --port 9000</code></pre>
                </div>
              </div>

              <h2>更多功能</h2>
              <ul>
                <li><a href="/examples" style={{ color: "#3178c6" }}>代码示例</a> - 查看更多使用场景</li>
                <li><a href="https://github.com/your-repo/tsp" style={{ color: "#3178c6" }}>GitHub</a> - 查看完整文档</li>
              </ul>
            </div>

            <Footer />
          </div>
        </div>
      </body>
    </html>
  );
});
