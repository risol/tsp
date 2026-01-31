import { Layout } from "./components/Layout.tsx";

export default async function (context: PageContext) {
  return (
    <Layout
      title="功能特性 - TSP"
      description="TSP 提供的完整功能特性列表"
    >
      <h1 className="display-5 mb-4 text-dark">📚 功能特性</h1>
      <p className="fs-5 text-muted mb-5">
        探索 TSP 提供的强大功能，每个功能都配有详细的演示和代码示例
      </p>

      <div className="info-grid">
        {/* Request Info */}
        <div className="card">
          <h3 className="h5 mb-3 text-primary">
            📡 请求信息
          </h3>
          <p className="mb-3 text-muted">
            查看完整的 HTTP 请求信息，包括方法、URL、Headers、Cookies 等
          </p>
          <ul className="mb-3 text-muted ps-3">
            <li>HTTP 方法（GET, POST, PUT, DELETE 等）</li>
            <li>请求 URL 和路径</li>
            <li>请求头信息</li>
            <li>查询参数</li>
            <li>Cookies</li>
          </ul>
          <a
            href="/features/request-info"
            className="btn btn-primary mt-3"
          >
            查看演示 →
          </a>
        </div>

        {/* Form Handling */}
        <div className="card">
          <h3 className="h5 mb-3 text-primary">
            📝 表单处理
          </h3>
          <p className="mb-3 text-muted">
            完整的表单提交和数据处理支持
          </p>
          <ul className="mb-3 text-muted ps-3">
            <li>GET 表单提交（URL 参数）</li>
            <li>POST 表单提交（application/x-www-form-urlencoded）</li>
            <li>JSON 数据提交</li>
            <li>文件上传支持</li>
          </ul>
          <a
            href="/features/form-handling"
            className="btn btn-primary mt-3"
          >
            查看演示 →
          </a>
        </div>

        {/* File Upload */}
        <div className="card">
          <h3 className="h5 mb-3 text-primary">
            📁 文件上传（nanoid）
          </h3>
          <p className="mb-3 text-muted">
            使用 nanoid 生成唯一文件名的文件上传
          </p>
          <ul className="mb-3 text-muted ps-3">
            <li>multipart/form-data 解析</li>
            <li>使用 nanoid 生成唯一文件名</li>
            <li>文件大小和类型限制</li>
            <li>多文件上传支持</li>
          </ul>
          <a
            href="/features/file-upload"
            className="btn btn-primary mt-3"
          >
            查看演示 →
          </a>
        </div>

        {/* nanoid Demo */}
        <div className="card">
          <h3 className="h5 mb-3 text-primary">
            🔐 nanoid 唯一 ID
          </h3>
          <p className="mb-3 text-muted">
            轻量级、URL-safe 的唯一 ID 生成器
          </p>
          <ul className="mb-3 text-muted ps-3">
            <li>可直接在 TSX 中使用（无需导入）</li>
            <li>默认 21 字符，比 UUID 短 40%</li>
            <li>URL-safe 字符集</li>
            <li>可自定义长度</li>
          </ul>
          <a
            href="/features/nanoid-demo"
            className="btn btn-primary mt-3"
          >
            查看演示 →
          </a>
        </div>

        {/* API Demo */}
        <div className="card">
          <h3 className="h5 mb-3 text-primary">
            🔌 API 演示
          </h3>
          <p className="mb-3 text-muted">
            RESTful API 开发支持
          </p>
          <ul className="mb-3 text-muted ps-3">
            <li>JSON 格式响应</li>
            <li>自定义状态码</li>
            <li>响应头设置</li>
            <li>直接返回 Response 对象</li>
          </ul>
          <a
            href="/api"
            className="btn btn-primary mt-3"
          >
            查看演示 →
          </a>
        </div>

        {/* Redirect */}
        <div className="card">
          <h3 className="h5 mb-3 text-primary">
            ➡️ 重定向
          </h3>
          <p className="mb-3 text-muted">
            HTTP 重定向功能
          </p>
          <ul className="mb-3 text-muted ps-3">
            <li>301 Moved Permanently</li>
            <li>302 Found</li>
            <li>303 See Other</li>
            <li>307 Temporary Redirect</li>
            <li>308 Permanent Redirect</li>
          </ul>
          <a
            href="/features/redirect"
            className="btn btn-primary mt-3"
          >
            查看演示 →
          </a>
        </div>

        {/* Session Management */}
        <div className="card">
          <h3 className="h5 mb-3 text-primary">
            👤 Session 管理
          </h3>
          <p className="mb-3 text-muted">
            完整的用户会话管理系统
          </p>
          <ul className="mb-3 text-muted ps-3">
            <li>用户登录/登出</li>
            <li>Session 数据存储</li>
            <li>HMAC-SHA256 签名保护</li>
            <li>自动过期和清理</li>
            <li>防止 Session 固定攻击</li>
          </ul>
          <a
            href="/session_demo"
            className="btn btn-primary mt-3"
          >
            查看演示 →
          </a>
        </div>

        {/* Cookie Management */}
        <div className="card">
          <h3 className="h5 mb-3 text-primary">
            🍪 Cookie 管理
          </h3>
          <p className="mb-3 text-muted">
            HTTP Cookie 的完整管理功能
          </p>
          <ul className="mb-3 text-muted ps-3">
            <li>设置和读取 Cookie</li>
            <li>支持多种选项（httpOnly, secure, sameSite）</li>
            <li>批量操作（setMultiple）</li>
            <li>Cookie 删除</li>
            <li>URL 编码自动处理</li>
          </ul>
          <a
            href="/cookie_demo"
            className="btn btn-primary mt-3"
          >
            查看演示 →
          </a>
        </div>

        {/* File Upload */}
        <div className="card">
          <h3 className="h5 mb-3 text-primary">
            📁 文件上传
          </h3>
          <p className="mb-3 text-muted">
            完整的文件上传处理功能
          </p>
          <ul className="mb-3 text-muted ps-3">
            <li>multipart/form-data 解析</li>
            <li>单文件上传</li>
            <li>多文件上传</li>
            <li>文件大小限制</li>
            <li>自动保存到服务器</li>
          </ul>
          <a
            href="/features/file-upload"
            className="btn btn-primary mt-3"
          >
            查看演示 →
          </a>
        </div>

        {/* Dependency Injection */}
        <div className="card">
          <h3 className="h5 mb-3 text-primary">
            💉 依赖注入
          </h3>
          <p className="mb-3 text-muted">
            类型安全的依赖注入功能
          </p>
          <ul className="mb-3 text-muted ps-3">
            <li>完整的 TypeScript 类型提示</li>
            <li>自动依赖注入</li>
            <li>支持异步依赖</li>
            <li>访问 Context 的依赖</li>
          </ul>
          <div className="mt-3">
            <span className="badge bg-success">已集成</span>
            <span className="ms-2 fs-6 text-muted">
              通过单元测试验证
            </span>
          </div>
        </div>

        {/* Error Handling */}
        <div className="card">
          <h3 className="h5 mb-3 text-primary">
            ⚠️ 错误处理
          </h3>
          <p className="mb-3 text-muted">
            完善的错误处理机制
          </p>
          <ul className="mb-3 text-muted ps-3">
            <li>自动错误捕获</li>
            <li>开发模式显示详细错误</li>
            <li>生产模式隐藏敏感信息</li>
            <li>自定义错误页面</li>
          </ul>
          <div className="mt-3">
            <span className="badge bg-success">已集成</span>
          </div>
        </div>
      </div>

      {/* Usage Stats */}
      <div className="section">
        <h2 className="section-title">📊 使用统计</h2>
        <div className="card">
          <div className="info-grid">
            <div className="info-item">
              <div className="info-label">运行时</div>
              <div className="info-value">Deno</div>
            </div>
            <div className="info-item">
              <div className="info-label">UI 框架</div>
              <div className="info-value">Preact 10.25.4</div>
            </div>
            <div className="info-item">
              <div className="info-label">类型检查</div>
              <div className="info-value">TypeScript</div>
            </div>
            <div className="info-item">
              <div className="info-label">模块系统</div>
              <div className="info-value">ES Modules</div>
            </div>
          </div>
        </div>
      </div>

      {/* Quick Links */}
      <div className="section">
        <h2 className="section-title">🔗 快速链接</h2>
        <div className="card">
          <table className="table table-bordered">
            <tbody>
              <tr>
                <td>
                  <a href="/" className="text-decoration-none">返回首页</a>
                </td>
                <td className="text-end text-muted">
                  TSP 主页
                </td>
              </tr>
              <tr>
                <td>
                  <a href="/demos" className="text-decoration-none">所有演示</a>
                </td>
                <td className="text-end text-muted">
                  查看所有功能演示
                </td>
              </tr>
              <tr>
                <td>
                  <a href="/session_demo" className="text-decoration-none">Session 演示</a>
                </td>
                <td className="text-end text-muted">
                  用户会话管理
                </td>
              </tr>
              <tr>
                <td>
                  <a href="/cookie_demo" className="text-decoration-none">Cookie 演示</a>
                </td>
                <td className="text-end text-muted">
                  Cookie 管理功能
                </td>
              </tr>
              <tr>
                <td>
                  <a href="/features/request-info" className="text-decoration-none">请求信息演示</a>
                </td>
                <td className="text-end text-muted">
                  查看 HTTP 请求详情
                </td>
              </tr>
              <tr>
                <td>
                  <a href="/api" className="text-decoration-none">API 演示</a>
                </td>
                <td className="text-end text-muted">
                  查看 API 响应示例
                </td>
              </tr>
              <tr>
                <td>
                  <a href="/features/file-upload" className="text-decoration-none">文件上传演示</a>
                </td>
                <td className="text-end text-muted">
                  文件上传功能
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </Layout>
  );
}
