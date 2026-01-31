import { Layout } from "./components/Layout.tsx";

export default async function (context: PageContext) {
  return (
    <Layout
      title="TSP - TypeScript Server Page"
      description="使用 Deno + TSX 实现的高性能模板服务器"
    >
      {/* Hero Section */}
      <div
        style={{
          textAlign: "center",
          padding: "60px 20px",
          background: "linear-gradient(135deg, #667eea 0%, #764ba2 100%)",
          borderRadius: "12px",
          color: "white",
          marginBottom: "40px",
        }}
      >
        <h1
          style={{ fontSize: "48px", fontWeight: "bold", marginBottom: "16px" }}
        >
          🚀 TSP
        </h1>
        <p style={{ fontSize: "20px", opacity: 0.9, marginBottom: "24px" }}>
          TypeScript Server Page
        </p>
        <p style={{ fontSize: "16px", opacity: 0.8, marginBottom: "32px" }}>
          使用 Deno + TSX + Preact 构建的高性能模板服务器
        </p>
        <div style={{ display: "flex", gap: "16px", justifyContent: "center" }}>
          <a
            href="/features"
            className="btn"
            style={{ background: "white", color: "#667eea" }}
          >
            查看功能
          </a>
          <a
            href="https://github.com/your-repo/tsp"
            className="btn"
            style={{ background: "rgba(255,255,255,0.2)", color: "white" }}
            target="_blank"
          >
            GitHub
          </a>
        </div>
      </div>

      {/* Features Overview */}
      <div className="section">
        <h2 className="section-title">✨ 核心特性</h2>
        <div className="info-grid">
          <div className="card">
            <h3
              style={{
                fontSize: "20px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              ⚡ 高性能
            </h3>
            <p>基于 Deno 运行时，提供原生 TypeScript 支持和出色的性能表现</p>
          </div>

          <div className="card">
            <h3
              style={{
                fontSize: "20px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              🎨 JSX/TSX
            </h3>
            <p>使用 Preact 和 JSX 语法，享受现代化的前端开发体验</p>
          </div>

          <div className="card">
            <h3
              style={{
                fontSize: "20px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              🔒 类型安全
            </h3>
            <p>完整的 TypeScript 支持，依赖注入功能，类型安全的开发体验</p>
          </div>

          <div className="card">
            <h3
              style={{
                fontSize: "20px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              📦 模块缓存
            </h3>
            <p>智能的文件监听和模块缓存机制，自动热重载</p>
          </div>
        </div>
      </div>

      {/* Quick Links */}
      <div className="section">
        <h2 className="section-title">📚 功能演示</h2>
        <div className="card">
          <h3 style={{ marginBottom: "16px", color: "#667eea" }}>🔧 核心功能</h3>
          <table style={{ width: "100%", marginBottom: "30px" }}>
            <thead>
              <tr>
                <th>功能</th>
                <th>描述</th>
                <th>链接</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <strong>请求信息</strong>
                </td>
                <td>查看 HTTP 请求的各种信息（方法、URL、Headers 等）</td>
                <td>
                  <a href="/features/request-info">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>表单处理</strong>
                </td>
                <td>GET/POST 表单提交和数据接收</td>
                <td>
                  <a href="/features/form-handling">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>API 演示</strong>
                </td>
                <td>返回 JSON 格式的 API 响应示例</td>
                <td>
                  <a href="/api">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>重定向</strong>
                </td>
                <td>HTTP 重定向功能演示</td>
                <td>
                  <a href="/features/redirect">查看演示 →</a>
                </td>
              </tr>
            </tbody>
          </table>

          <h3 style={{ marginBottom: "16px", color: "#667eea" }}>🍪 状态管理</h3>
          <table style={{ width: "100%", marginBottom: "30px" }}>
            <thead>
              <tr>
                <th>功能</th>
                <th>描述</th>
                <th>链接</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <strong>Session 管理</strong>
                </td>
                <td>用户会话管理，支持登录/登出、数据存储</td>
                <td>
                  <a href="/session_demo">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>Cookie 管理</strong>
                </td>
                <td>HTTP Cookie 的设置、读取、删除等功能</td>
                <td>
                  <a href="/cookie_demo">查看演示 →</a>
                </td>
              </tr>
            </tbody>
          </table>

          <h3 style={{ marginBottom: "16px", color: "#667eea" }}>🗄️ MySQL 数据库</h3>
          <table style={{ width: "100%", marginBottom: "30px" }}>
            <thead>
              <tr>
                <th>功能</th>
                <th>描述</th>
                <th>链接</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <strong>MySQL 测试中心</strong>
                </td>
                <td>所有 MySQL 功能的导航中心</td>
                <td>
                  <a href="/mysql-tests">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>MySQL 演示仪表板</strong>
                </td>
                <td>交互式 MySQL 功能演示和测试</td>
                <td>
                  <a href="/mysql-demo">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>高级功能测试</strong>
                </td>
                <td>12 种 MySQL 高级功能测试</td>
                <td>
                  <a href="/mysql-advanced">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>性能测试</strong>
                </td>
                <td>MySQL 客户端性能基准测试</td>
                <td>
                  <a href="/mysql-performance">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>基本查询 API</strong>
                </td>
                <td>简单的 JSON API 查询示例</td>
                <td>
                  <a href="/test-mysql">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>插入操作 API</strong>
                </td>
                <td>数据插入操作示例</td>
                <td>
                  <a href="/test-mysql-insert">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>更新操作 API</strong>
                </td>
                <td>数据更新操作示例</td>
                <td>
                  <a href="/test-mysql-update">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>事务操作 API</strong>
                </td>
                <td>数据库事务操作示例</td>
                <td>
                  <a href="/test-mysql-transaction">查看演示 →</a>
                </td>
              </tr>
            </tbody>
          </table>

          <h3 style={{ marginBottom: "16px", color: "#667eea" }}>🔴 Redis 缓存</h3>
          <table style={{ width: "100%", marginBottom: "30px" }}>
            <thead>
              <tr>
                <th>功能</th>
                <th>描述</th>
                <th>链接</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <strong>Redis 演示仪表板</strong>
                </td>
                <td>交互式 Redis 功能演示和测试</td>
                <td>
                  <a href="/redis-demo">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>高级功能测试</strong>
                </td>
                <td>10 种 Redis 高级功能测试（9 种数据结构）</td>
                <td>
                  <a href="/redis-advanced">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>性能测试</strong>
                </td>
                <td>Redis 客户端性能基准测试</td>
                <td>
                  <a href="/redis-performance">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>基本操作 API</strong>
                </td>
                <td>简单的 JSON API 操作示例</td>
                <td>
                  <a href="/test-redis">查看演示 →</a>
                </td>
              </tr>
            </tbody>
          </table>

          <h3 style={{ marginBottom: "16px", color: "#667eea" }}>📦 其他功能</h3>
          <table style={{ width: "100%" }}>
            <thead>
              <tr>
                <th>功能</th>
                <th>描述</th>
                <th>链接</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <strong>文件上传</strong>
                </td>
                <td>多文件上传功能演示</td>
                <td>
                  <a href="/features/file-upload">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>Nanoid ID 生成</strong>
                </td>
                <td>生成唯一 ID 的功能演示</td>
                <td>
                  <a href="/features/nanoid-demo">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>静态文件服务</strong>
                </td>
                <td>静态文件的缓存和服务</td>
                <td>
                  <a href="/static-demo">查看演示 →</a>
                </td>
              </tr>
              <tr>
                <td>
                  <strong>依赖注入</strong>
                </td>
                <td>类型安全的依赖注入功能</td>
                <td>
                  <span style={{ color: "#64748b", fontSize: "14px" }}>
                    ✓ 单元测试验证
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Tech Stack */}
      <div className="section">
        <h2 className="section-title">🛠️ 技术栈</h2>
        <div className="info-grid">
          <div className="info-item">
            <div className="info-label">运行时</div>
            <div className="info-value">Deno</div>
          </div>
          <div className="info-item">
            <div className="info-label">UI 框架</div>
            <div className="info-value">Preact</div>
          </div>
          <div className="info-item">
            <div className="info-label">模板语法</div>
            <div className="info-value">JSX/TSX</div>
          </div>
          <div className="info-item">
            <div className="info-label">语言</div>
            <div className="info-value">TypeScript</div>
          </div>
        </div>
      </div>

      {/* Usage Example */}
      <div className="section">
        <h2 className="section-title">💻 快速开始</h2>
        <div className="card">
          <h3 style={{ marginBottom: "16px" }}>创建一个页面</h3>
          <div className="code-block">
            {`// www/index.tsx
export default async function(context: PageContext) {
  const { method, url, query } = context;

  return (
    <html>
      <head>
        <title>Hello World</title>
      </head>
      <body>
        <h1>Hello from TSP!</h1>
        <p>Request method: {method}</p>
        <p>URL: {url.pathname}</p>
      </body>
    </html>
  );
}`}
          </div>
        </div>
      </div>

      {/* Footer CTA */}
      <div
        style={{
          textAlign: "center",
          padding: "40px",
          background: "#f8fafc",
          borderRadius: "12px",
          marginTop: "40px",
        }}
      >
        <h3 style={{ marginBottom: "16px", color: "#1e293b" }}>
          开始使用 TSP
        </h3>
        <p style={{ marginBottom: "24px", color: "#64748b" }}>
          探索各种功能演示，了解如何构建强大的 Web 应用
        </p>
        <a href="/features" className="btn btn-primary">
          探索功能 →
        </a>
      </div>

      {/* All Pages Navigation */}
      <div className="section">
        <h2 className="section-title">🔗 所有页面导航</h2>
        <div className="card">
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fill, minmax(250px, 1fr))",
              gap: "12px",
            }}
          >
            {/* Core Pages */}
            <div style={{ padding: "12px", background: "#f0f9ff", borderRadius: "8px" }}>
              <strong style={{ color: "#0369a1" }}>📄 核心页面</strong>
              <ul style={{ marginTop: "8px", marginBottom: "0", paddingLeft: "20px" }}>
                <li><a href="/">首页</a></li>
                <li><a href="/features">功能总览</a></li>
                <li><a href="/demos">演示页面</a></li>
                <li><a href="/api">API 演示</a></li>
                <li><a href="/form">表单测试</a></li>
                <li><a href="/redirect">重定向测试</a></li>
              </ul>
            </div>

            {/* Features */}
            <div style={{ padding: "12px", background: "#fef3c7", borderRadius: "8px" }}>
              <strong style={{ color: "#b45309" }}>✨ 功能演示</strong>
              <ul style={{ marginTop: "8px", marginBottom: "0", paddingLeft: "20px" }}>
                <li><a href="/features/request-info">请求信息</a></li>
                <li><a href="/features/form-handling">表单处理</a></li>
                <li><a href="/features/redirect">重定向功能</a></li>
                <li><a href="/features/file-upload">文件上传</a></li>
                <li><a href="/features/nanoid-demo">Nanoid 演示</a></li>
              </ul>
            </div>

            {/* Session & Cookie */}
            <div style={{ padding: "12px", background: "#dcfce7", borderRadius: "8px" }}>
              <strong style={{ color: "#15803d" }}>🍪 状态管理</strong>
              <ul style={{ marginTop: "8px", marginBottom: "0", paddingLeft: "20px" }}>
                <li><a href="/session_demo">Session 演示</a></li>
                <li><a href="/cookie_demo">Cookie 演示</a></li>
                <li><a href="/cookie_test">Cookie 测试</a></li>
              </ul>
            </div>

            {/* MySQL Database */}
            <div style={{ padding: "12px", background: "#fae8ff", borderRadius: "8px" }}>
              <strong style={{ color: "#86198f" }}>🗄️ MySQL 数据库</strong>
              <ul style={{ marginTop: "8px", marginBottom: "0", paddingLeft: "20px" }}>
                <li><a href="/mysql-tests">📊 测试中心</a></li>
                <li><a href="/mysql-demo">📈 演示仪表板</a></li>
                <li><a href="/mysql-advanced">🔬 高级功能</a></li>
                <li><a href="/mysql-performance">⚡ 性能测试</a></li>
                <li><a href="/test-mysql">查询 API</a></li>
                <li><a href="/test-mysql-insert">插入 API</a></li>
                <li><a href="/test-mysql-update">更新 API</a></li>
                <li><a href="/test-mysql-transaction">事务 API</a></li>
              </ul>
            </div>

            {/* Redis Cache */}
            <div style={{ padding: "12px", background: "#fee2e2", borderRadius: "8px" }}>
              <strong style={{ color: "#dc2626" }}>🔴 Redis 缓存</strong>
              <ul style={{ marginTop: "8px", marginBottom: "0", paddingLeft: "20px" }}>
                <li><a href="/redis-demo">📈 演示仪表板</a></li>
                <li><a href="/redis-advanced">🔬 高级功能</a></li>
                <li><a href="/redis-performance">⚡ 性能测试</a></li>
                <li><a href="/test-redis">基本 API</a></li>
              </ul>
            </div>

            {/* Test Pages */}
            <div style={{ padding: "12px", background: "#fce7f3", borderRadius: "8px" }}>
              <strong style={{ color: "#be185d" }}>🧪 测试页面</strong>
              <ul style={{ marginTop: "8px", marginBottom: "0", paddingLeft: "20px" }}>
                <li><a href="/test_page">异步测试页</a></li>
                <li><a href="/test_page_sync">同步测试页</a></li>
                <li><a href="/static-demo">静态文件测试</a></li>
              </ul>
            </div>
          </div>

          <div style={{ marginTop: "20px", padding: "16px", background: "#fffbeb", borderRadius: "8px", border: "2px solid #fbbf24" }}>
            <h4 style={{ margin: "0 0 8px 0", color: "#b45309" }}>💡 提示</h4>
            <p style={{ margin: "0", fontSize: "14px", color: "#78350f" }}>
              • MySQL/Redis 功能需要先启动 Docker 容器：<code>.\docker-start.ps1</code><br />
              • 所有页面都支持热重载，修改文件后自动刷新<br />
              • 点击任意链接即可查看对应功能的详细演示
            </p>
          </div>
        </div>
      </div>
    </Layout>
  );
}
