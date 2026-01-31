import { Layout } from "./components/Layout.tsx";

export default async function (context: PageContext) {
  return (
    <Layout
      title="功能演示 - TSP"
      description="TSP 所有功能演示的完整列表"
    >
      <h1 className="display-5 mb-4 text-dark">🎯 功能演示</h1>
      <p className="fs-5 text-muted mb-5">
        探索 TSP 提供的所有功能演示，每个演示都包含完整的使用示例和代码
      </p>

      {/* Session & Cookie Demos */}
      <div className="section">
        <h2 className="section-title">🔐 会话与状态管理</h2>
        <div className="info-grid">
          {/* Session Demo */}
          <div className="card">
            <h3 className="h5 mb-3 text-primary">
              👤 Session 管理
            </h3>
            <p className="mb-3 text-muted small">
              完整的用户会话管理系统演示
            </p>
            <ul className="mb-3 text-muted small ps-3">
              <li>用户登录/登出</li>
              <li>Session 数据存储</li>
              <li>HMAC-SHA256 签名保护</li>
              <li>自动过期和清理</li>
            </ul>
            <div className="mt-auto">
              <a href="/session_demo" className="btn btn-primary">查看演示 →</a>
            </div>
          </div>

          {/* Cookie Demo */}
          <div className="card">
            <h3
              style={{
                fontSize: "18px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              🍪 Cookie 管理
            </h3>
            <p
              style={{
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "14px",
              }}
            >
              HTTP Cookie 的完整管理功能
            </p>
            <ul
              style={{
                marginLeft: "18px",
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "13px",
              }}
            >
              <li>设置和读取 Cookie</li>
              <li>支持多种安全选项</li>
              <li>批量操作</li>
              <li>URL 编码处理</li>
            </ul>
            <div style={{ marginTop: "auto" }}>
              <a href="/cookie_demo" className="btn btn-primary">查看演示 →</a>
            </div>
          </div>

          {/* Cookie Test */}
          <div className="card">
            <h3
              style={{
                fontSize: "18px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              🧪 Cookie 测试
            </h3>
            <p
              style={{
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "14px",
              }}
            >
              Cookie 功能的完整 E2E 测试
            </p>
            <ul
              style={{
                marginLeft: "18px",
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "13px",
              }}
            >
              <li>基础 Cookie 设置</li>
              <li>Cookie 选项测试</li>
              <li>批量操作测试</li>
              <li>特殊字符测试</li>
            </ul>
            <div style={{ marginTop: "auto" }}>
              <a href="/cookie_test" className="btn btn-primary">查看测试 →</a>
            </div>
          </div>
        </div>
      </div>

      {/* HTTP Features */}
      <div className="section">
        <h2 className="section-title">📡 HTTP 功能</h2>
        <div className="info-grid">
          {/* Request Info */}
          <div className="card">
            <h3
              style={{
                fontSize: "18px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              📋 请求信息
            </h3>
            <p
              style={{
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "14px",
              }}
            >
              查看完整的 HTTP 请求信息
            </p>
            <ul
              style={{
                marginLeft: "18px",
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "13px",
              }}
            >
              <li>HTTP 方法和 URL</li>
              <li>请求头信息</li>
              <li>查询参数</li>
              <li>Cookies</li>
            </ul>
            <div style={{ marginTop: "auto" }}>
              <a href="/features/request-info" className="btn btn-primary">
                查看演示 →
              </a>
            </div>
          </div>

          {/* Form Handling */}
          <div className="card">
            <h3
              style={{
                fontSize: "18px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              📝 表单处理
            </h3>
            <p
              style={{
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "14px",
              }}
            >
              表单提交和数据处理
            </p>
            <ul
              style={{
                marginLeft: "18px",
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "13px",
              }}
            >
              <li>GET 表单提交</li>
              <li>POST 表单提交</li>
              <li>JSON 数据提交</li>
              <li>文件上传</li>
            </ul>
            <div style={{ marginTop: "auto" }}>
              <a href="/features/form-handling" className="btn btn-primary">
                查看演示 →
              </a>
            </div>
          </div>

          {/* API Demo */}
          <div className="card">
            <h3
              style={{
                fontSize: "18px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              🔌 API 响应
            </h3>
            <p
              style={{
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "14px",
              }}
            >
              RESTful API 开发演示
            </p>
            <ul
              style={{
                marginLeft: "18px",
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "13px",
              }}
            >
              <li>JSON 格式响应</li>
              <li>自定义状态码</li>
              <li>响应头设置</li>
              <li>Response 对象</li>
            </ul>
            <div style={{ marginTop: "auto" }}>
              <a href="/api" className="btn btn-primary">查看演示 →</a>
            </div>
          </div>

          {/* Redirect */}
          <div className="card">
            <h3
              style={{
                fontSize: "18px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              ➡️ 重定向
            </h3>
            <p
              style={{
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "14px",
              }}
            >
              HTTP 重定向功能演示
            </p>
            <ul
              style={{
                marginLeft: "18px",
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "13px",
              }}
            >
              <li>301/302 重定向</li>
              <li>303/307 重定向</li>
              <li>308 永久重定向</li>
              <li>自定义状态码</li>
            </ul>
            <div style={{ marginTop: "auto" }}>
              <a href="/features/redirect" className="btn btn-primary">
                查看演示 →
              </a>
            </div>
          </div>
        </div>
      </div>

      {/* Static Files */}
      <div className="section">
        <h2 className="section-title">📁 静态文件</h2>
        <div className="info-grid">
          <div className="card">
            <h3
              style={{
                fontSize: "18px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              📦 静态资源
            </h3>
            <p
              style={{
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "14px",
              }}
            >
              静态文件服务演示
            </p>
            <ul
              style={{
                marginLeft: "18px",
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "13px",
              }}
            >
              <li>自动 MIME 类型检测</li>
              <li>ETag 缓存支持</li>
              <li>范围请求支持</li>
              <li>缓存控制</li>
            </ul>
            <div style={{ marginTop: "auto" }}>
              <a href="/static-demo" className="btn btn-primary">查看演示 →</a>
            </div>
          </div>

          <div className="card">
            <h3
              style={{
                fontSize: "18px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              🖼️ 图片资源
            </h3>
            <p
              style={{
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "14px",
              }}
            >
              访问静态图片文件
            </p>
            <ul
              style={{
                marginLeft: "18px",
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "13px",
              }}
            >
              <li>JPG/PNG/GIF 支持</li>
              <li>自动压缩优化</li>
              <li>浏览器缓存</li>
            </ul>
            <div style={{ marginTop: "auto" }}>
              <a href="/static/logo.png" className="btn btn-primary">
                查看图片 →
              </a>
            </div>
          </div>
        </div>
      </div>

      {/* Advanced Features */}
      <div className="section">
        <h2 className="section-title">⚙️ 高级功能</h2>
        <div className="info-grid">
          <div className="card">
            <h3
              style={{
                fontSize: "18px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              💉 依赖注入
            </h3>
            <p
              style={{
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "14px",
              }}
            >
              类型安全的依赖注入系统
            </p>
            <ul
              style={{
                marginLeft: "18px",
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "13px",
              }}
            >
              <li>完整的类型提示</li>
              <li>自动依赖注入</li>
              <li>支持异步依赖</li>
              <li>Context 访问</li>
            </ul>
            <div style={{ marginTop: "auto" }}>
              <span className="badge badge-success">已集成</span>
              <span
                style={{
                  marginLeft: "8px",
                  fontSize: "13px",
                  color: "#64748b",
                }}
              >
                单元测试验证
              </span>
            </div>
          </div>

          <div className="card">
            <h3
              style={{
                fontSize: "18px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              🔥 热重载
            </h3>
            <p
              style={{
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "14px",
              }}
            >
              文件修改自动重新加载
            </p>
            <ul
              style={{
                marginLeft: "18px",
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "13px",
              }}
            >
              <li>自动监听文件变化</li>
              <li>依赖级联更新</li>
              <li>零停机时间</li>
              <li>支持嵌套组件</li>
            </ul>
            <div style={{ marginTop: "auto" }}>
              <span className="badge badge-info">开发模式</span>
              <span
                style={{
                  marginLeft: "8px",
                  fontSize: "13px",
                  color: "#64748b",
                }}
              >
                使用 --dev 启动
              </span>
            </div>
          </div>

          <div className="card">
            <h3
              style={{
                fontSize: "18px",
                marginBottom: "12px",
                color: "#667eea",
              }}
            >
              ⚠️ 错误处理
            </h3>
            <p
              style={{
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "14px",
              }}
            >
              完善的错误处理机制
            </p>
            <ul
              style={{
                marginLeft: "18px",
                marginBottom: "16px",
                color: "#64748b",
                fontSize: "13px",
              }}
            >
              <li>自动错误捕获</li>
              <li>开发模式详细错误</li>
              <li>生产模式安全提示</li>
              <li>自定义错误页面</li>
            </ul>
            <div style={{ marginTop: "auto" }}>
              <span className="badge badge-success">已集成</span>
            </div>
          </div>
        </div>
      </div>

      {/* Quick Start */}
      <div className="section">
        <h2 className="section-title">🚀 快速开始</h2>
        <div className="card">
          <h3 style={{ marginBottom: "16px" }}>创建你的第一个页面</h3>
          <div className="code-block">
            {`// www/my-page.tsx
export default async function(context: PageContext) {
  const { method, url, query } = context;

  return (
    <html>
      <head>
        <title>My Page</title>
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
          <p style={{ marginTop: "16px", color: "#64748b" }}>
            访问 <code>http://localhost:9000/my-page</code> 查看结果
          </p>
        </div>
      </div>

      {/* Links */}
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
          开始探索 TSP
        </h3>
        <p style={{ marginBottom: "24px", color: "#64748b" }}>
          选择一个演示开始体验 TSP 的强大功能
        </p>
        <div
          style={{
            display: "flex",
            gap: "12px",
            justifyContent: "center",
            flexWrap: "wrap",
          }}
        >
          <a href="/session_demo" className="btn btn-primary">Session 演示</a>
          <a href="/cookie_demo" className="btn btn-primary">Cookie 演示</a>
          <a href="/features/request-info" className="btn btn-secondary">
            请求信息
          </a>
          <a href="/features/form-handling" className="btn btn-secondary">
            表单处理
          </a>
        </div>
      </div>
    </Layout>
  );
}
