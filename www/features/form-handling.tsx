import { Layout } from "../components/Layout.tsx";

export default async function (context: PageContext) {
  const { method, body } = context;

  // 检测是否为htmx请求
  const isHtmx = context.headers.get("HX-Request") === "true";

  // 如果是htmx的POST请求，返回结果片段
  if (method === "POST" && body && isHtmx) {
    return (
      <div className="card bg-success bg-opacity-10 border-success border-2 mb-5">
        <div className="card-body">
          <h3 className="h4 text-success mb-3">
            ✅ 提交成功！（htmx无刷新）
          </h3>
          <div className="code-block bg-success bg-opacity-25 mt-3">
            {JSON.stringify(body, null, 2)}
          </div>
          <div className="alert alert-success mt-3 mb-0" role="alert">
            <small>
              ✓ 表单通过htmx提交，无需刷新页面<br />
              ✓ 服务器返回JSON数据显示<br />
              ✓ 用户体验更加流畅
            </small>
          </div>
        </div>
      </div>
    );
  }

  return (
    <Layout title="表单处理 - TSP" description="表单提交和数据处理演示（htmx支持）">
      {/* Page Header */}
      <div className="text-center mb-5">
        <h1 className="display-5 fw-bold text-dark mb-3">📝 表单处理</h1>
        <p className="text-muted fs-5">
          支持多种表单提交方式（含htmx无刷新提交）
        </p>
      </div>

      {/* 加载指示器 */}
      <div id="loading-indicator" className="htmx-indicator text-center mb-3">
        <div className="spinner-border text-primary" role="status">
          <span className="visually-hidden">提交中...</span>
        </div>
      </div>

      {/* Result Display - 只在非htmx POST时显示 */}
      {method === "POST" && body && !isHtmx && (
        <div className="card bg-success bg-opacity-10 border-success border-2 mb-5">
          <div className="card-body">
            <h3 className="h4 text-success mb-3">
              ✅ 提交成功！（传统方式）
            </h3>
            <div className="code-block bg-success bg-opacity-25 mt-3">
              {JSON.stringify(body, null, 2)}
            </div>
          </div>
        </div>
      )}

      {/* htmx Result Display */}
      <div id="form-result">
        {method !== "POST" && (
          <div className="card bg-light bg-opacity-50 border mb-5">
            <div className="card-body text-center text-muted">
              <p className="mb-0">填写并提交表单，查看htmx无刷新交互效果</p>
            </div>
          </div>
        )}
      </div>

      {/* POST Form with htmx */}
      <div className="card shadow-sm mb-4">
        <div className="card-body">
          <h2 className="h4 card-title text-primary mb-4">
            POST 表单（htmx无刷新提交）
          </h2>
          <form
            hx-post="/features/form-handling"
            hx-target="#form-result"
            hx-swap="outerHTML"
            hx-indicator="#loading-indicator"
            style={{ maxWidth: "500px" }}
          >
            <div className="mb-3">
              <label htmlFor="username" className="form-label fw-semibold">
                用户名：
              </label>
              <input
                type="text"
                id="username"
                name="username"
                required
                className="form-control"
                placeholder="请输入用户名"
              />
            </div>
            <div className="mb-3">
              <label htmlFor="email" className="form-label fw-semibold">
                邮箱：
              </label>
              <input
                type="email"
                id="email"
                name="email"
                className="form-control"
                placeholder="请输入邮箱（可选）"
              />
            </div>
            <div className="mb-3">
              <label htmlFor="message" className="form-label fw-semibold">
                留言：
              </label>
              <textarea
                id="message"
                name="message"
                className="form-control"
                rows="3"
                placeholder="请输入留言（可选）"
              />
            </div>
            <button type="submit" className="btn btn-primary btn-lg">
              提交表单（htmx）
            </button>
          </form>
        </div>
      </div>

      {/* Info Card */}
      <div className="card shadow-sm mb-4">
        <div className="card-body">
          <h2 className="h4 card-title text-primary mb-3">💡 htmx工作原理</h2>
          <div className="alert alert-info mb-0" role="alert">
            <h5 className="alert-heading">无刷新表单提交</h5>
            <p className="mb-2 small">
              当你点击"提交表单"时，htmx会：
            </p>
            <ol className="mb-0 small">
              <li>拦截表单提交事件，阻止默认的页面跳转</li>
              <li>发送AJAX POST请求到服务器</li>
              <li>显示加载指示器（旋转的圆圈）</li>
              <li>将服务器返回的HTML插入到指定区域（#form-result）</li>
              <li>用户无需等待页面刷新，体验更加流畅</li>
            </ol>
          </div>
        </div>
      </div>

      {/* Traditional Form for Comparison */}
      <div className="card shadow-sm mb-4">
        <div className="card-body">
          <h2 className="h4 card-title text-primary mb-4">
            传统表单提交（对比）
          </h2>
          <p className="text-muted mb-3">
            下面的表单使用传统的提交方式（会刷新页面）：
          </p>
          <form method="POST" style={{ maxWidth: "500px" }}>
            <div className="mb-3">
              <label htmlFor="username2" className="form-label fw-semibold">
                用户名：
              </label>
              <input
                type="text"
                id="username2"
                name="username"
                required
                className="form-control"
              />
            </div>
            <button type="submit" className="btn btn-secondary">
              提交表单（传统方式）
            </button>
          </form>
        </div>
      </div>

      {/* Back Button */}
      <div className="mt-4">
        <a href="/features" className="btn btn-outline-primary">
          ← 返回功能列表
        </a>
      </div>
    </Layout>
  );
}
