import type { PageContext } from "../src/cache.ts";
import { Layout } from "./components/Layout.tsx";

export default async function(context: PageContext) {
  return (
    <Layout
      title="TSP - TypeScript Server Page"
      description="使用 Deno + TSX 实现的高性能模板服务器"
    >
      {/* Hero Section */}
      <div style={{
        textAlign: 'center',
        padding: '60px 20px',
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        borderRadius: '12px',
        color: 'white',
        marginBottom: '40px',
      }}>
        <h1 style={{ fontSize: '48px', fontWeight: 'bold', marginBottom: '16px' }}>
          🚀 TSP
        </h1>
        <p style={{ fontSize: '20px', opacity: 0.9, marginBottom: '24px' }}>
          TypeScript Server Page
        </p>
        <p style={{ fontSize: '16px', opacity: 0.8, marginBottom: '32px' }}>
          使用 Deno + TSX + Preact 构建的高性能模板服务器
        </p>
        <div style={{ display: 'flex', gap: '16px', justifyContent: 'center' }}>
          <a href="/features" className="btn" style={{ background: 'white', color: '#667eea' }}>
            查看功能
          </a>
          <a
            href="https://github.com/your-repo/tsp"
            className="btn"
            style={{ background: 'rgba(255,255,255,0.2)', color: 'white' }}
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
            <h3 style={{ fontSize: '20px', marginBottom: '12px', color: '#667eea' }}>
              ⚡ 高性能
            </h3>
            <p>基于 Deno 运行时，提供原生 TypeScript 支持和出色的性能表现</p>
          </div>

          <div className="card">
            <h3 style={{ fontSize: '20px', marginBottom: '12px', color: '#667eea' }}>
              🎨 JSX/TSX
            </h3>
            <p>使用 Preact 和 JSX 语法，享受现代化的前端开发体验</p>
          </div>

          <div className="card">
            <h3 style={{ fontSize: '20px', marginBottom: '12px', color: '#667eea' }}>
              🔒 类型安全
            </h3>
            <p>完整的 TypeScript 支持，依赖注入功能，类型安全的开发体验</p>
          </div>

          <div className="card">
            <h3 style={{ fontSize: '20px', marginBottom: '12px', color: '#667eea' }}>
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
          <table style={{ width: '100%' }}>
            <thead>
              <tr>
                <th>功能</th>
                <th>描述</th>
                <th>链接</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td><strong>请求信息</strong></td>
                <td>查看 HTTP 请求的各种信息（方法、URL、Headers 等）</td>
                <td><a href="/features/request-info">查看演示 →</a></td>
              </tr>
              <tr>
                <td><strong>表单处理</strong></td>
                <td>GET/POST 表单提交和数据接收</td>
                <td><a href="/features/form-handling">查看演示 →</a></td>
              </tr>
              <tr>
                <td><strong>API 演示</strong></td>
                <td>返回 JSON 格式的 API 响应示例</td>
                <td><a href="/features/api-demo">查看演示 →</a></td>
              </tr>
              <tr>
                <td><strong>重定向</strong></td>
                <td>HTTP 重定向功能演示</td>
                <td><a href="/features/redirect">查看演示 →</a></td>
              </tr>
              <tr>
                <td><strong>依赖注入</strong></td>
                <td>类型安全的依赖注入功能</td>
                <td><a href="/features/dependency-injection">查看演示 →</a></td>
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
          <h3 style={{ marginBottom: '16px' }}>创建一个页面</h3>
          <div className="code-block">
{`// www/index.tsx
import type { PageContext } from "../src/cache.ts";

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
      <div style={{
        textAlign: 'center',
        padding: '40px',
        background: '#f8fafc',
        borderRadius: '12px',
        marginTop: '40px',
      }}>
        <h3 style={{ marginBottom: '16px', color: '#1e293b' }}>
          开始使用 TSP
        </h3>
        <p style={{ marginBottom: '24px', color: '#64748b' }}>
          探索各种功能演示，了解如何构建强大的 Web 应用
        </p>
        <a href="/features" className="btn btn-primary">
          探索功能 →
        </a>
      </div>
    </Layout>
  );
}
