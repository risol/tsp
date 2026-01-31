import { Layout } from "./components/Layout.tsx";

export default async function (context: PageContext) {
  return (
    <Layout
      title="TSP - TypeScript Server Page"
      description="使用 Deno + TSX 实现的高性能模板服务器"
    >
      {/* Hero Section - 专业版 */}
      <div className="hero-section">
        <div className="icon-float mb-3" style={{ fontSize: '4rem' }}>⚡</div>
        <h1 className="hero-title">TSP</h1>
        <p className="hero-subtitle">TypeScript Server Page</p>
        <p className="hero-description">
          使用 Deno + TSX + Preact 构建的高性能模板服务器
        </p>
        <div className="d-flex gap-3 justify-content-center flex-wrap">
          <a href="/features" className="btn btn-light btn-hero me-2">
            ✨ 查看功能
          </a>
          <a
            href="https://github.com/your-repo/tsp"
            className="btn btn-outline-light btn-hero"
            target="_blank"
          >
            📦 GitHub
          </a>
        </div>
      </div>

      {/* 统计数据卡片 */}
      <div className="mb-5">
        <div className="row g-4">
          <div className="col-md-3 col-sm-6">
            <div className="stat-card scale-in">
              <div className="stat-number">5+</div>
              <div className="stat-label">核心特性</div>
            </div>
          </div>
          <div className="col-md-3 col-sm-6">
            <div className="stat-card scale-in" style={{ animationDelay: '0.1s' }}>
              <div className="stat-number">25+</div>
              <div className="stat-label">功能演示</div>
            </div>
          </div>
          <div className="col-md-3 col-sm-6">
            <div className="stat-card scale-in" style={{ animationDelay: '0.2s' }}>
              <div className="stat-number">100%</div>
              <div className="stat-label">TypeScript</div>
            </div>
          </div>
          <div className="col-md-3 col-sm-6">
            <div className="stat-card scale-in" style={{ animationDelay: '0.3s' }}>
              <div className="stat-number">⚡</div>
              <div className="stat-label">热重载</div>
            </div>
          </div>
        </div>
      </div>

      {/* Features Overview - 专业版 */}
      <div className="section mb-5 fade-in">
        <h2 className="section-title text-center">✨ 核心特性</h2>
        <div className="info-grid">
          <div className="feature-card">
            <div className="display-6 mb-3 text-primary">⚡</div>
            <h3 className="h5 card-title text-primary mb-3">高性能</h3>
            <p className="card-text text-muted">
              基于 Deno 运行时，提供原生 TypeScript 支持和出色的性能表现
            </p>
          </div>

          <div className="feature-card">
            <div className="display-6 mb-3 text-primary">🎨</div>
            <h3 className="h5 card-title text-primary mb-3">JSX/TSX</h3>
            <p className="card-text text-muted">
              使用 Preact 和 JSX 语法，享受现代化的前端开发体验
            </p>
          </div>

          <div className="feature-card">
            <div className="display-6 mb-3 text-primary">🔒</div>
            <h3 className="h5 card-title text-primary mb-3">类型安全</h3>
            <p className="card-text text-muted">
              完整的 TypeScript 支持，依赖注入功能，类型安全的开发体验
            </p>
          </div>

          <div className="feature-card">
            <div className="display-6 mb-3 text-primary">📦</div>
            <h3 className="h5 card-title text-primary mb-3">模块缓存</h3>
            <p className="card-text text-muted">
              智能的文件监听和模块缓存机制，自动热重载
            </p>
          </div>
        </div>
      </div>

      {/* Quick Links - 表格增强版 */}
      <div className="section mb-5 fade-in">
        <h2 className="section-title text-center">📚 功能演示</h2>

        {/* 核心功能 */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h3 className="h4 mb-3 text-primary">🔧 核心功能</h3>
            <div className="table-responsive">
              <table className="table table-hover table-enhanced mb-0">
                <thead>
                  <tr>
                    <th>功能</th>
                    <th>描述</th>
                    <th className="text-end">操作</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td><strong>📡 请求信息</strong></td>
                    <td>查看 HTTP 请求的各种信息（方法、URL、Headers 等）</td>
                    <td className="text-end">
                      <a href="/features/request-info" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                  <tr>
                    <td><strong>📝 表单处理</strong></td>
                    <td>GET/POST 表单提交和数据接收</td>
                    <td className="text-end">
                      <a href="/features/form-handling" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                  <tr>
                    <td><strong>🔌 API 演示</strong></td>
                    <td>返回 JSON 格式的 API 响应示例</td>
                    <td className="text-end">
                      <a href="/api" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                  <tr>
                    <td><strong>➡️ 重定向</strong></td>
                    <td>HTTP 重定向功能演示</td>
                    <td className="text-end">
                      <a href="/features/redirect" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        {/* 状态管理 */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h3 className="h4 mb-3 text-success">🍪 状态管理</h3>
            <div className="table-responsive">
              <table className="table table-hover table-enhanced mb-0">
                <tbody>
                  <tr>
                    <td><strong>👤 Session 管理</strong></td>
                    <td>用户会话管理，支持登录/登出、数据存储</td>
                    <td className="text-end">
                      <a href="/session_demo" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                  <tr>
                    <td><strong>🍪 Cookie 管理</strong></td>
                    <td>HTTP Cookie 的设置、读取、删除等功能</td>
                    <td className="text-end">
                      <a href="/cookie_demo" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        {/* MySQL 数据库 */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h3 className="h4 mb-3 text-info">🗄️ MySQL 数据库</h3>
            <div className="table-responsive">
              <table className="table table-hover table-enhanced mb-0">
                <tbody>
                  <tr>
                    <td><strong>📊 MySQL 测试中心</strong></td>
                    <td>所有 MySQL 功能的导航中心</td>
                    <td className="text-end">
                      <a href="/mysql-tests" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                  <tr>
                    <td><strong>📈 MySQL 演示仪表板</strong></td>
                    <td>交互式 MySQL 功能演示和测试</td>
                    <td className="text-end">
                      <a href="/mysql-demo" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                  <tr>
                    <td><strong>🔬 高级功能测试</strong></td>
                    <td>12 种 MySQL 高级功能测试</td>
                    <td className="text-end">
                      <a href="/mysql-advanced" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                  <tr>
                    <td><strong>⚡ 性能测试</strong></td>
                    <td>MySQL 客户端性能基准测试</td>
                    <td className="text-end">
                      <a href="/mysql-performance" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        {/* Redis 缓存 */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h3 className="h4 mb-3 text-danger">🔴 Redis 缓存</h3>
            <div className="table-responsive">
              <table className="table table-hover table-enhanced mb-0">
                <tbody>
                  <tr>
                    <td><strong>📈 Redis 演示仪表板</strong></td>
                    <td>交互式 Redis 功能演示和测试</td>
                    <td className="text-end">
                      <a href="/redis-demo" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                  <tr>
                    <td><strong>🔬 高级功能测试</strong></td>
                    <td>10 种 Redis 高级功能测试（9 种数据结构）</td>
                    <td className="text-end">
                      <a href="/redis-advanced" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                  <tr>
                    <td><strong>⚡ 性能测试</strong></td>
                    <td>Redis 客户端性能基准测试</td>
                    <td className="text-end">
                      <a href="/redis-performance" className="btn btn-sm btn-gradient-primary">查看演示 →</a>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>

      {/* Tech Stack - 美化版 */}
      <div className="section mb-5 slide-in-left">
        <h2 className="section-title text-center">🛠️ 技术栈</h2>
        <div className="row g-4">
          <div className="col-md-3 col-sm-6">
            <div className="info-item">
              <div className="info-label">🦎 运行时</div>
              <div className="info-value fw-bold">Deno</div>
            </div>
          </div>
          <div className="col-md-3 col-sm-6">
            <div className="info-item">
              <div className="info-label">⚛️ UI 框架</div>
              <div className="info-value fw-bold">Preact</div>
            </div>
          </div>
          <div className="col-md-3 col-sm-6">
            <div className="info-item">
              <div className="info-label">📝 模板语法</div>
              <div className="info-value fw-bold">JSX/TSX</div>
            </div>
          </div>
          <div className="col-md-3 col-sm-6">
            <div className="info-item">
              <div className="info-label">💎 语言</div>
              <div className="info-value fw-bold">TypeScript</div>
            </div>
          </div>
        </div>
      </div>

      {/* Usage Example - 美化版 */}
      <div className="section mb-5 fade-in">
        <h2 className="section-title text-center">💻 快速开始</h2>
        <div className="card shadow-sm">
          <div className="card-body">
            <h3 className="h5 mb-3">📝 创建一个页面</h3>
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
      </div>

      {/* Footer CTA - 美化版 */}
      <div className="text-center p-5 bg-white bg-opacity-10 rounded-3 mb-5 fade-in">
        <div className="icon-float mb-3" style={{ fontSize: '3rem' }}>🚀</div>
        <h3 className="mb-3 text-white fw-bold">开始使用 TSP</h3>
        <p className="text-white-75 mb-4 fs-5">
          探索各种功能演示，了解如何构建强大的 Web 应用
        </p>
        <a href="/features" className="btn btn-light btn-hero">
          探索功能 →
        </a>
      </div>

      {/* All Pages Navigation - 简化版 */}
      <div className="section mb-5">
        <h2 className="section-title text-center">🔗 快速导航</h2>
        <div className="card shadow-sm">
          <div className="card-body">
            <div className="row g-3">
              {/* Core Pages */}
              <div className="col-lg-4 col-md-6">
                <div className="p-3 bg-white rounded-3 border border-2" style={{ borderColor: '#667eea' }}>
                  <h5 className="text-primary mb-2">📄 核心页面</h5>
                  <ul className="list-unstyled mb-0">
                    <li><a href="/" className="text-decoration-none d-block py-1">• 首页</a></li>
                    <li><a href="/features" className="text-decoration-none d-block py-1">• 功能总览</a></li>
                    <li><a href="/demos" className="text-decoration-none d-block py-1">• 演示页面</a></li>
                    <li><a href="/api" className="text-decoration-none d-block py-1">• API 演示</a></li>
                  </ul>
                </div>
              </div>

              {/* Features */}
              <div className="col-lg-4 col-md-6">
                <div className="p-3 bg-white rounded-3 border border-2" style={{ borderColor: '#f59e0b' }}>
                  <h5 className="text-warning mb-2">✨ 功能演示</h5>
                  <ul className="list-unstyled mb-0">
                    <li><a href="/features/request-info" className="text-decoration-none d-block py-1">• 请求信息</a></li>
                    <li><a href="/features/form-handling" className="text-decoration-none d-block py-1">• 表单处理</a></li>
                    <li><a href="/features/redirect" className="text-decoration-none d-block py-1">• 重定向功能</a></li>
                    <li><a href="/features/file-upload" className="text-decoration-none d-block py-1">• 文件上传</a></li>
                  </ul>
                </div>
              </div>

              {/* Session & Cookie */}
              <div className="col-lg-4 col-md-6">
                <div className="p-3 bg-white rounded-3 border border-2" style={{ borderColor: '#10b981' }}>
                  <h5 className="text-success mb-2">🍪 状态管理</h5>
                  <ul className="list-unstyled mb-0">
                    <li><a href="/session_demo" className="text-decoration-none d-block py-1">• Session 演示</a></li>
                    <li><a href="/cookie_demo" className="text-decoration-none d-block py-1">• Cookie 演示</a></li>
                    <li><a href="/cookie_test" className="text-decoration-none d-block py-1">• Cookie 测试</a></li>
                  </ul>
                </div>
              </div>
            </div>

            <div className="alert alert-info mt-4 mb-0" role="alert">
              <h5 className="alert-heading">💡 提示</h5>
              <p className="mb-0 small">
                • MySQL/Redis 功能需要先启动 Docker 容器：<code>.\docker-start.ps1</code><br />
                • 所有页面都支持热重载，修改文件后自动刷新<br />
                • 点击任意链接即可查看对应功能的详细演示
              </p>
            </div>
          </div>
        </div>
      </div>
    </Layout>
  );
}
