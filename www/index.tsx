import { Layout } from "./components/Layout.tsx";

export default async function (context: PageContext) {
  return (
    <Layout
      title="TSP - TypeScript Server Page"
      description="使用 Deno + TSX 实现的高性能模板服务器"
    >
      {/* Hero Section */}
      <div className="text-center p-5 mb-5 bg-gradient-brand rounded-3 text-white">
        <h1 className="display-3 fw-bold mb-3">🚀 TSP</h1>
        <p className="fs-5 mb-2">TypeScript Server Page</p>
        <p className="fs-6 opacity-75 mb-4">
          使用 Deno + TSX + Preact 构建的高性能模板服务器
        </p>
        <div className="d-flex gap-3 justify-content-center">
          <a href="/features" className="btn btn-light btn-lg px-4">
            查看功能
          </a>
          <a
            href="https://github.com/your-repo/tsp"
            className="btn btn-outline-light btn-lg px-4"
            target="_blank"
          >
            GitHub
          </a>
        </div>
      </div>

      {/* Features Overview */}
      <div className="mb-5">
        <h2 className="mb-4">✨ 核心特性</h2>
        <div className="row g-4">
          <div className="col-md-6 col-lg-3">
            <div className="card h-100 shadow-sm">
              <div className="card-body">
                <h3 className="h5 card-title text-primary">⚡ 高性能</h3>
                <p className="card-text">基于 Deno 运行时，提供原生 TypeScript 支持和出色的性能表现</p>
              </div>
            </div>
          </div>

          <div className="col-md-6 col-lg-3">
            <div className="card h-100 shadow-sm">
              <div className="card-body">
                <h3 className="h5 card-title text-primary">🎨 JSX/TSX</h3>
                <p className="card-text">使用 Preact 和 JSX 语法，享受现代化的前端开发体验</p>
              </div>
            </div>
          </div>

          <div className="col-md-6 col-lg-3">
            <div className="card h-100 shadow-sm">
              <div className="card-body">
                <h3 className="h5 card-title text-primary">🔒 类型安全</h3>
                <p className="card-text">完整的 TypeScript 支持，依赖注入功能，类型安全的开发体验</p>
              </div>
            </div>
          </div>

          <div className="col-md-6 col-lg-3">
            <div className="card h-100 shadow-sm">
              <div className="card-body">
                <h3 className="h5 card-title text-primary">📦 模块缓存</h3>
                <p className="card-text">智能的文件监听和模块缓存机制，自动热重载</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Quick Links */}
      <div className="mb-5">
        <h2 className="mb-4">📚 功能演示</h2>
        <div className="card shadow-sm">
          <div className="card-body">
            <h3 className="h4 mb-3 text-primary">🔧 核心功能</h3>
            <div className="table-responsive mb-4">
              <table className="table table-striped table-hover">
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
                    <td><a href="/features/request-info" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>表单处理</strong></td>
                    <td>GET/POST 表单提交和数据接收</td>
                    <td><a href="/features/form-handling" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>API 演示</strong></td>
                    <td>返回 JSON 格式的 API 响应示例</td>
                    <td><a href="/api" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>重定向</strong></td>
                    <td>HTTP 重定向功能演示</td>
                    <td><a href="/features/redirect" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                </tbody>
              </table>
            </div>

            <h3 className="h4 mb-3 text-primary">🍪 状态管理</h3>
            <div className="table-responsive mb-4">
              <table className="table table-striped table-hover">
                <thead>
                  <tr>
                    <th>功能</th>
                    <th>描述</th>
                    <th>链接</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td><strong>Session 管理</strong></td>
                    <td>用户会话管理，支持登录/登出、数据存储</td>
                    <td><a href="/session_demo" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>Cookie 管理</strong></td>
                    <td>HTTP Cookie 的设置、读取、删除等功能</td>
                    <td><a href="/cookie_demo" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                </tbody>
              </table>
            </div>

            <h3 className="h4 mb-3 text-primary">🗄️ MySQL 数据库</h3>
            <div className="table-responsive mb-4">
              <table className="table table-striped table-hover">
                <thead>
                  <tr>
                    <th>功能</th>
                    <th>描述</th>
                    <th>链接</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td><strong>MySQL 测试中心</strong></td>
                    <td>所有 MySQL 功能的导航中心</td>
                    <td><a href="/mysql-tests" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>MySQL 演示仪表板</strong></td>
                    <td>交互式 MySQL 功能演示和测试</td>
                    <td><a href="/mysql-demo" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>高级功能测试</strong></td>
                    <td>12 种 MySQL 高级功能测试</td>
                    <td><a href="/mysql-advanced" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>性能测试</strong></td>
                    <td>MySQL 客户端性能基准测试</td>
                    <td><a href="/mysql-performance" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>基本查询 API</strong></td>
                    <td>简单的 JSON API 查询示例</td>
                    <td><a href="/test-mysql" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>插入操作 API</strong></td>
                    <td>数据插入操作示例</td>
                    <td><a href="/test-mysql-insert" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>更新操作 API</strong></td>
                    <td>数据更新操作示例</td>
                    <td><a href="/test-mysql-update" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>事务操作 API</strong></td>
                    <td>数据库事务操作示例</td>
                    <td><a href="/test-mysql-transaction" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                </tbody>
              </table>
            </div>

            <h3 className="h4 mb-3 text-primary">🔴 Redis 缓存</h3>
            <div className="table-responsive mb-4">
              <table className="table table-striped table-hover">
                <thead>
                  <tr>
                    <th>功能</th>
                    <th>描述</th>
                    <th>链接</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td><strong>Redis 演示仪表板</strong></td>
                    <td>交互式 Redis 功能演示和测试</td>
                    <td><a href="/redis-demo" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>高级功能测试</strong></td>
                    <td>10 种 Redis 高级功能测试（9 种数据结构）</td>
                    <td><a href="/redis-advanced" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>性能测试</strong></td>
                    <td>Redis 客户端性能基准测试</td>
                    <td><a href="/redis-performance" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>基本操作 API</strong></td>
                    <td>简单的 JSON API 操作示例</td>
                    <td><a href="/test-redis" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                </tbody>
              </table>
            </div>

            <h3 className="h4 mb-3 text-primary">📦 其他功能</h3>
            <div className="table-responsive">
              <table className="table table-striped table-hover">
                <thead>
                  <tr>
                    <th>功能</th>
                    <th>描述</th>
                    <th>链接</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td><strong>文件上传</strong></td>
                    <td>多文件上传功能演示</td>
                    <td><a href="/features/file-upload" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>Nanoid ID 生成</strong></td>
                    <td>生成唯一 ID 的功能演示</td>
                    <td><a href="/features/nanoid-demo" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>静态文件服务</strong></td>
                    <td>静态文件的缓存和服务</td>
                    <td><a href="/static-demo" className="btn btn-sm btn-outline-primary">查看演示 →</a></td>
                  </tr>
                  <tr>
                    <td><strong>依赖注入</strong></td>
                    <td>类型安全的依赖注入功能</td>
                    <td><span className="badge bg-success">✓ 单元测试验证</span></td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>

      {/* Tech Stack */}
      <div className="mb-5">
        <h2 className="mb-4">🛠️ 技术栈</h2>
        <div className="row g-4">
          <div className="col-md-3 col-sm-6">
            <div className="info-item">
              <div className="info-label">运行时</div>
              <div className="info-value">Deno</div>
            </div>
          </div>
          <div className="col-md-3 col-sm-6">
            <div className="info-item">
              <div className="info-label">UI 框架</div>
              <div className="info-value">Preact</div>
            </div>
          </div>
          <div className="col-md-3 col-sm-6">
            <div className="info-item">
              <div className="info-label">模板语法</div>
              <div className="info-value">JSX/TSX</div>
            </div>
          </div>
          <div className="col-md-3 col-sm-6">
            <div className="info-item">
              <div className="info-label">语言</div>
              <div className="info-value">TypeScript</div>
            </div>
          </div>
        </div>
      </div>

      {/* Usage Example */}
      <div className="mb-5">
        <h2 className="mb-4">💻 快速开始</h2>
        <div className="card shadow-sm">
          <div className="card-body">
            <h3 className="h5 mb-3">创建一个页面</h3>
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

      {/* Footer CTA */}
      <div className="text-center p-5 bg-light rounded-3 mb-5">
        <h3 className="mb-3">开始使用 TSP</h3>
        <p className="text-muted mb-4">
          探索各种功能演示，了解如何构建强大的 Web 应用
        </p>
        <a href="/features" className="btn btn-primary btn-lg">
          探索功能 →
        </a>
      </div>

      {/* All Pages Navigation */}
      <div className="mb-5">
        <h2 className="mb-4">🔗 所有页面导航</h2>
        <div className="card shadow-sm">
          <div className="card-body">
            <div className="row g-3">
              {/* Core Pages */}
              <div className="col-lg-4 col-md-6">
                <div className="p-3 bg-light border rounded-3">
                  <h5 className="text-primary mb-2">📄 核心页面</h5>
                  <ul className="list-unstyled mb-0">
                    <li><a href="/" className="text-decoration-none">• 首页</a></li>
                    <li><a href="/features" className="text-decoration-none">• 功能总览</a></li>
                    <li><a href="/demos" className="text-decoration-none">• 演示页面</a></li>
                    <li><a href="/api" className="text-decoration-none">• API 演示</a></li>
                    <li><a href="/form" className="text-decoration-none">• 表单测试</a></li>
                    <li><a href="/redirect" className="text-decoration-none">• 重定向测试</a></li>
                  </ul>
                </div>
              </div>

              {/* Features */}
              <div className="col-lg-4 col-md-6">
                <div className="p-3 bg-warning bg-opacity-10 border rounded-3">
                  <h5 className="text-warning mb-2">✨ 功能演示</h5>
                  <ul className="list-unstyled mb-0">
                    <li><a href="/features/request-info" className="text-decoration-none">• 请求信息</a></li>
                    <li><a href="/features/form-handling" className="text-decoration-none">• 表单处理</a></li>
                    <li><a href="/features/redirect" className="text-decoration-none">• 重定向功能</a></li>
                    <li><a href="/features/file-upload" className="text-decoration-none">• 文件上传</a></li>
                    <li><a href="/features/nanoid-demo" className="text-decoration-none">• Nanoid 演示</a></li>
                  </ul>
                </div>
              </div>

              {/* Session & Cookie */}
              <div className="col-lg-4 col-md-6">
                <div className="p-3 bg-success bg-opacity-10 border rounded-3">
                  <h5 className="text-success mb-2">🍪 状态管理</h5>
                  <ul className="list-unstyled mb-0">
                    <li><a href="/session_demo" className="text-decoration-none">• Session 演示</a></li>
                    <li><a href="/cookie_demo" className="text-decoration-none">• Cookie 演示</a></li>
                    <li><a href="/cookie_test" className="text-decoration-none">• Cookie 测试</a></li>
                  </ul>
                </div>
              </div>

              {/* MySQL Database */}
              <div className="col-lg-4 col-md-6">
                <div className="p-3 bg-info bg-opacity-10 border rounded-3">
                  <h5 className="text-info mb-2">🗄️ MySQL 数据库</h5>
                  <ul className="list-unstyled mb-0 small">
                    <li><a href="/mysql-tests" className="text-decoration-none">• 📊 测试中心</a></li>
                    <li><a href="/mysql-demo" className="text-decoration-none">• 📈 演示仪表板</a></li>
                    <li><a href="/mysql-advanced" className="text-decoration-none">• 🔬 高级功能</a></li>
                    <li><a href="/mysql-performance" className="text-decoration-none">• ⚡ 性能测试</a></li>
                    <li><a href="/test-mysql" className="text-decoration-none">• 查询 API</a></li>
                    <li><a href="/test-mysql-insert" className="text-decoration-none">• 插入 API</a></li>
                    <li><a href="/test-mysql-update" className="text-decoration-none">• 更新 API</a></li>
                    <li><a href="/test-mysql-transaction" className="text-decoration-none">• 事务 API</a></li>
                  </ul>
                </div>
              </div>

              {/* Redis Cache */}
              <div className="col-lg-4 col-md-6">
                <div className="p-3 bg-danger bg-opacity-10 border rounded-3">
                  <h5 className="text-danger mb-2">🔴 Redis 缓存</h5>
                  <ul className="list-unstyled mb-0">
                    <li><a href="/redis-demo" className="text-decoration-none">• 📈 演示仪表板</a></li>
                    <li><a href="/redis-advanced" className="text-decoration-none">• 🔬 高级功能</a></li>
                    <li><a href="/redis-performance" className="text-decoration-none">• ⚡ 性能测试</a></li>
                    <li><a href="/test-redis" className="text-decoration-none">• 基本 API</a></li>
                  </ul>
                </div>
              </div>

              {/* Test Pages */}
              <div className="col-lg-4 col-md-6">
                <div className="p-3 bg-secondary bg-opacity-10 border rounded-3">
                  <h5 className="text-secondary mb-2">🧪 测试页面</h5>
                  <ul className="list-unstyled mb-0">
                    <li><a href="/test_page" className="text-decoration-none">• 异步测试页</a></li>
                    <li><a href="/test_page_sync" className="text-decoration-none">• 同步测试页</a></li>
                    <li><a href="/static-demo" className="text-decoration-none">• 静态文件测试</a></li>
                  </ul>
                </div>
              </div>
            </div>

            <div className="alert alert-warning mt-4 mb-0" role="alert">
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
