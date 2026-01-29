import { Layout } from "./components/Layout.tsx";

export default async function(context: PageContext) {
  return (
    <Layout
      title="功能特性 - TSP"
      description="TSP 提供的完整功能特性列表"
    >
      <h1 style={{ fontSize: '36px', marginBottom: '24px', color: '#1e293b' }}>
        📚 功能特性
      </h1>
      <p style={{ fontSize: '18px', color: '#64748b', marginBottom: '40px' }}>
        探索 TSP 提供的强大功能，每个功能都配有详细的演示和代码示例
      </p>

      <div className="info-grid">
        {/* Request Info */}
        <div className="card">
          <h3 style={{ fontSize: '20px', marginBottom: '12px', color: '#667eea' }}>
            📡 请求信息
          </h3>
          <p style={{ marginBottom: '16px', color: '#64748b' }}>
            查看完整的 HTTP 请求信息，包括方法、URL、Headers、Cookies 等
          </p>
          <ul style={{ marginLeft: '20px', color: '#64748b' }}>
            <li>HTTP 方法（GET, POST, PUT, DELETE 等）</li>
            <li>请求 URL 和路径</li>
            <li>请求头信息</li>
            <li>查询参数</li>
            <li>Cookies</li>
          </ul>
          <a href="/features/request-info" className="btn btn-primary" style={{ marginTop: '16px' }}>
            查看演示 →
          </a>
        </div>

        {/* Form Handling */}
        <div className="card">
          <h3 style={{ fontSize: '20px', marginBottom: '12px', color: '#667eea' }}>
            📝 表单处理
          </h3>
          <p style={{ marginBottom: '16px', color: '#64748b' }}>
            完整的表单提交和数据处理支持
          </p>
          <ul style={{ marginLeft: '20px', color: '#64748b' }}>
            <li>GET 表单提交（URL 参数）</li>
            <li>POST 表单提交（application/x-www-form-urlencoded）</li>
            <li>JSON 数据提交</li>
            <li>文件上传支持</li>
          </ul>
          <a href="/features/form-handling" className="btn btn-primary" style={{ marginTop: '16px' }}>
            查看演示 →
          </a>
        </div>

        {/* API Demo */}
        <div className="card">
          <h3 style={{ fontSize: '20px', marginBottom: '12px', color: '#667eea' }}>
            🔌 API 演示
          </h3>
          <p style={{ marginBottom: '16px', color: '#64748b' }}>
            RESTful API 开发支持
          </p>
          <ul style={{ marginLeft: '20px', color: '#64748b' }}>
            <li>JSON 格式响应</li>
            <li>自定义状态码</li>
            <li>响应头设置</li>
            <li>直接返回 Response 对象</li>
          </ul>
          <a href="/api" className="btn btn-primary" style={{ marginTop: '16px' }}>
            查看演示 →
          </a>
        </div>

        {/* Redirect */}
        <div className="card">
          <h3 style={{ fontSize: '20px', marginBottom: '12px', color: '#667eea' }}>
            ➡️ 重定向
          </h3>
          <p style={{ marginBottom: '16px', color: '#64748b' }}>
            HTTP 重定向功能
          </p>
          <ul style={{ marginLeft: '20px', color: '#64748b' }}>
            <li>301 Moved Permanently</li>
            <li>302 Found</li>
            <li>303 See Other</li>
            <li>307 Temporary Redirect</li>
            <li>308 Permanent Redirect</li>
          </ul>
          <a href="/features/redirect" className="btn btn-primary" style={{ marginTop: '16px' }}>
            查看演示 →
          </a>
        </div>

        {/* Dependency Injection */}
        <div className="card">
          <h3 style={{ fontSize: '20px', marginBottom: '12px', color: '#667eea' }}>
            💉 依赖注入
          </h3>
          <p style={{ marginBottom: '16px', color: '#64748b' }}>
            类型安全的依赖注入功能
          </p>
          <ul style={{ marginLeft: '20px', color: '#64748b' }}>
            <li>完整的 TypeScript 类型提示</li>
            <li>自动依赖注入</li>
            <li>支持异步依赖</li>
            <li>访问 Context 的依赖</li>
          </ul>
          <div style={{ marginTop: '16px' }}>
            <span className="badge badge-success">已集成</span>
            <span style={{ marginLeft: '8px', fontSize: '14px', color: '#64748b' }}>
              通过单元测试验证
            </span>
          </div>
        </div>

        {/* Error Handling */}
        <div className="card">
          <h3 style={{ fontSize: '20px', marginBottom: '12px', color: '#667eea' }}>
            ⚠️ 错误处理
          </h3>
          <p style={{ marginBottom: '16px', color: '#64748b' }}>
            完善的错误处理机制
          </p>
          <ul style={{ marginLeft: '20px', color: '#64748b' }}>
            <li>自动错误捕获</li>
            <li>开发模式显示详细错误</li>
            <li>生产模式隐藏敏感信息</li>
            <li>自定义错误页面</li>
          </ul>
          <div style={{ marginTop: '16px' }}>
            <span className="badge badge-success">已集成</span>
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
          <table style={{ width: '100%' }}>
            <tbody>
              <tr>
                <td><a href="/">返回首页</a></td>
                <td style={{ textAlign: 'right', color: '#64748b' }}>TSP 主页</td>
              </tr>
              <tr>
                <td><a href="/features/request-info">请求信息演示</a></td>
                <td style={{ textAlign: 'right', color: '#64748b' }}>查看 HTTP 请求详情</td>
              </tr>
              <tr>
                <td><a href="/api">API 演示</a></td>
                <td style={{ textAlign: 'right', color: '#64748b' }}>查看 API 响应示例</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </Layout>
  );
}
