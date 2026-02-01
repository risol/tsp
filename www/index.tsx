import { Layout } from "./components/Layout.tsx";

export default async function (context: PageContext) {
  return (
    <Layout
      title="TSP - TypeScript Server Page"
      description="使用 Deno + TSX 实现的高性能模板服务器"
    >
      <div className="container py-5">
        {/* Hero Section */}
        <div className="text-center mb-5">
          <h1 className="display-4 fw-bold mb-3 text-dark">TSP</h1>
          <p className="fs-4 mb-4 text-muted">TypeScript Server Page</p>
          <p className="lead mb-4 text-muted" style={{ maxWidth: "600px", margin: "0 auto" }}>
            基于 Deno + TSX + Preact 构建的高性能模板服务器，支持热重载、类型安全和依赖注入
          </p>
          <div className="d-flex gap-3 justify-content-center">
            <a href="/features" className="btn btn-primary btn-lg">查看功能</a>
            <a href="/demos" className="btn btn-outline-primary btn-lg">功能演示</a>
            <a href="/tspinfo" className="btn btn-outline-secondary btn-lg">服务器信息</a>
          </div>
        </div>

        {/* 核心特性 */}
        <div className="row g-4 mb-5">
          <div className="col-md-3">
            <div className="card h-100 border-0 shadow-sm bg-white">
              <div className="card-body text-center">
                <div className="mb-2" style={{ fontSize: "2rem" }}>⚡</div>
                <h5 className="card-title">高性能</h5>
                <p className="card-text text-muted small">基于 Deno 运行时</p>
              </div>
            </div>
          </div>
          <div className="col-md-3">
            <div className="card h-100 border-0 shadow-sm bg-white">
              <div className="card-body text-center">
                <div className="text-primary mb-2" style={{ fontSize: "2rem" }}>🎨</div>
                <h5 className="card-title">TSX 语法</h5>
                <p className="card-text text-muted small">Preact + JSX</p>
              </div>
            </div>
          </div>
          <div className="col-md-3">
            <div className="card h-100 border-0 shadow-sm bg-white">
              <div className="card-body text-center">
                <div className="text-primary mb-2" style={{ fontSize: "2rem" }}>🔥</div>
                <h5 className="card-title">热重载</h5>
                <p className="card-text text-muted small">自动刷新页面</p>
              </div>
            </div>
          </div>
          <div className="col-md-3">
            <div className="card h-100 border-0 shadow-sm bg-white">
              <div className="card-body text-center">
                <div className="text-primary mb-2" style={{ fontSize: "2rem" }}>🔒</div>
                <h5 className="card-title">类型安全</h5>
                <p className="card-text text-muted small">完整 TypeScript</p>
              </div>
            </div>
          </div>
        </div>

        {/* 功能导航 */}
        <div className="row g-4 mb-5">
          <div className="col-lg-6">
            <div className="card border-0 shadow-sm h-100 bg-white">
              <div className="card-header bg-white border-0 pt-3">
                <h5 className="mb-0">🔧 核心功能</h5>
              </div>
              <div className="card-body">
                <div className="list-group list-group-flush">
                  <a href="/features/request-info" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>请求信息</span>
                    <span className="text-muted">→</span>
                  </a>
                  <a href="/features/form-handling" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>表单处理</span>
                    <span className="text-muted">→</span>
                  </a>
                  <a href="/api" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>API 演示</span>
                    <span className="text-muted">→</span>
                  </a>
                  <a href="/features/file-upload" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>文件上传</span>
                    <span className="text-muted">→</span>
                  </a>
                </div>
              </div>
            </div>
          </div>

          <div className="col-lg-6">
            <div className="card border-0 shadow-sm h-100 bg-white">
              <div className="card-header bg-transparent border-0 pt-3">
                <h5 className="mb-0">🍪 状态管理</h5>
              </div>
              <div className="card-body">
                <div className="list-group list-group-flush">
                  <a href="/session_demo" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>Session 管理</span>
                    <span className="text-muted">→</span>
                  </a>
                  <a href="/cookie_demo" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>Cookie 管理</span>
                    <span className="text-muted">→</span>
                  </a>
                </div>
              </div>
            </div>
          </div>

          <div className="col-lg-6">
            <div className="card border-0 shadow-sm h-100 bg-white">
              <div className="card-header bg-transparent border-0 pt-3">
                <h5 className="mb-0">🧪 测试工具</h5>
              </div>
              <div className="card-body">
                <div className="list-group list-group-flush">
                  <a href="/logger_e2e" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>Logger 测试</span>
                    <span className="badge bg-primary rounded-pill">E2E</span>
                  </a>
                  <a href="/logger_rotation_e2e" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>日志归档测试</span>
                    <span className="badge bg-primary rounded-pill">E2E</span>
                  </a>
                </div>
              </div>
            </div>
          </div>

          <div className="col-lg-6">
            <div className="card border-0 shadow-sm h-100 bg-white">
              <div className="card-header bg-transparent border-0 pt-3">
                <h5 className="mb-0">🗄️ MySQL 数据库</h5>
              </div>
              <div className="card-body">
                <div className="list-group list-group-flush">
                  <a href="/mysql-demo" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>演示仪表板</span>
                    <span className="text-muted">→</span>
                  </a>
                  <a href="/mysql-advanced" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>高级功能测试</span>
                    <span className="text-muted">→</span>
                  </a>
                  <a href="/mysql-performance" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>性能测试</span>
                    <span className="text-muted">→</span>
                  </a>
                </div>
              </div>
            </div>
          </div>

          <div className="col-lg-6">
            <div className="card border-0 shadow-sm h-100 bg-white">
              <div className="card-header bg-white border-0 pt-3">
                <h5 className="mb-0">🔴 Redis 缓存</h5>
              </div>
              <div className="card-body">
                <div className="list-group list-group-flush">
                  <a href="/redis-demo" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>演示仪表板</span>
                    <span className="text-muted">→</span>
                  </a>
                  <a href="/redis-advanced" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>高级功能测试</span>
                    <span className="text-muted">→</span>
                  </a>
                  <a href="/redis-performance" className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0">
                    <span>性能测试</span>
                    <span className="text-muted">→</span>
                  </a>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* 技术栈 */}
        <div className="card border-0 shadow-sm mb-5 bg-white">
          <div className="card-body py-4">
            <div className="row text-center g-4">
              <div className="col-6 col-md-3">
                <div className="text-muted small mb-1">运行时</div>
                <div className="fw-bold">Deno</div>
              </div>
              <div className="col-6 col-md-3">
                <div className="text-muted small mb-1">UI 框架</div>
                <div className="fw-bold">Preact</div>
              </div>
              <div className="col-6 col-md-3">
                <div className="text-muted small mb-1">模板语法</div>
                <div className="fw-bold">JSX/TSX</div>
              </div>
              <div className="col-6 col-md-3">
                <div className="text-muted small mb-1">语言</div>
                <div className="fw-bold">TypeScript</div>
              </div>
            </div>
          </div>
        </div>

        {/* 快速开始 */}
        <div className="card border-0 shadow-sm mb-5 bg-white">
          <div className="card-body p-4">
            <h5 className="card-title mb-3">💻 快速开始</h5>
            <pre className="bg-light p-3 rounded mb-0" style={{ fontSize: "0.875rem" }}>
{`// www/index.tsx
export default async function(context: PageContext) {
  return (
    <html>
      <head><title>Hello World</title></head>
      <body>
        <h1>Hello from TSP!</h1>
        <p>Request method: {context.method}</p>
      </body>
    </html>
  );
}`}
            </pre>
          </div>
        </div>

        {/* 提示信息 */}
        <div className="alert alert-info border-0 shadow-sm" role="alert">
          <h6 className="alert-heading">💡 提示</h6>
          <p className="mb-2 small">
            • MySQL/Redis 功能需要先启动 Docker 容器：<code>.\docker-start.ps1</code>
          </p>
          <p className="mb-0 small">
            • 所有页面支持热重载，修改文件后自动刷新
          </p>
        </div>
      </div>
    </Layout>
  );
}
