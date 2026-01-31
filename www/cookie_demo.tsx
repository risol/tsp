import { Layout } from "./components/Layout.tsx";

/**
 * Cookie Demo Page with htmx support
 *
 * A demonstration of TSP's cookie management with htmx for seamless interactions.
 */

export default Page(async function (ctx, { cookies, nanoid }) {
  const action = ctx.query.action || "view";

  // 检测是否为htmx请求
  const isHtmx = ctx.headers.get("HX-Request") === "true";

  // View mode: show current cookies
  if (action === "view") {
    const visitCount = parseInt(ctx.cookies.visits || "0") + 1;

    return (
      <Layout title="Cookie 演示 - TSP" description="TSP Cookie 管理功能演示">
        {/* Page Header */}
        <div className="text-center mb-5">
          <h1 className="display-5 fw-bold text-white mb-3">🍪 Cookie 管理演示</h1>
          <p className="text-white-50 fs-5">
            完整的 Cookie 操作功能展示（支持htmx无刷新交互）
          </p>
        </div>

        {/* 加载指示器 */}
        <div id="loading-indicator" className="htmx-indicator text-center mb-3">
          <div className="spinner-border text-primary" role="status">
            <span className="visually-hidden">加载中...</span>
          </div>
        </div>

        {/* Welcome Card */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">欢迎！</h2>
            <p className="card-text">
              这是您的访问次数：<strong>{visitCount}</strong>
            </p>
            <p className="card-text text-muted">
              TSP 的 cookie 系统让您能够轻松管理 HTTP cookies，
              并提供完整的 TypeScript 支持。
            </p>
          </div>
        </div>

        {/* Current Cookies Card */}
        <div id="cookies-table">
          <CookiesTable cookies={ctx.cookies} />
        </div>

        {/* Actions Card */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">试试看（htmx无刷新）</h2>
            <div className="d-flex flex-wrap gap-2">
              <button
                hx-post="/cookie_demo?action=set"
                hx-target="#cookies-table"
                hx-swap="outerHTML"
                hx-indicator="#loading-indicator"
                className="btn btn-primary"
              >
                设置访问计数器 Cookie
              </button>
              <button
                hx-post="/cookie_demo?action=set_preferences"
                hx-target="#cookies-table"
                hx-swap="outerHTML"
                hx-indicator="#loading-indicator"
                className="btn btn-success"
              >
                设置偏好设置 Cookie
              </button>
              <button
                hx-post="/cookie_demo?action=set_secure"
                hx-target="#cookies-table"
                hx-swap="outerHTML"
                hx-indicator="#loading-indicator"
                className="btn btn-info"
              >
                设置安全会话 Cookie
              </button>
              <button
                hx-post="/cookie_demo?action=clear"
                hx-target="#cookies-table"
                hx-swap="outerHTML"
                hx-indicator="#loading-indicator"
                className="btn btn-secondary"
              >
                清除所有 Cookie
              </button>
            </div>

            <div className="alert alert-success mt-4 mb-0" role="alert">
              <h5 className="alert-heading">✨ htmx已启用</h5>
              <p className="mb-2 small">
                所有Cookie操作都会通过htmx无刷新更新表格，体验更加流畅！
              </p>
              <ul className="mb-0 small">
                <li>设置Cookie实时更新显示</li>
                <li>无需页面刷新即可看到变化</li>
                <li>操作流畅，用户体验更好</li>
              </ul>
            </div>
          </div>
        </div>

        {/* Code Examples Card */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">工作原理</h2>
            <p className="mb-3">在 TSP 中设置 Cookie 很简单：</p>
            <div className="bg-light p-3 rounded-3 mb-3">
              <pre className="mb-0 small"><code>{`export default Page(async function(ctx, { cookies }) {
  // Set a simple cookie
  cookies.set('username', 'john_doe');

  // Set with options
  cookies.set('sessionId', 'abc123', {
    httpOnly: true,
    secure: true,
    sameSite: 'Strict',
    maxAge: 3600,
  });

  return <div>Done!</div>;
});`}</code></pre>
            </div>
            <p className="mb-3">读取 Cookie 更简单 - 它们就在您的上下文中：</p>
            <div className="bg-light p-3 rounded-3">
              <pre className="mb-0 small"><code>{`const username = ctx.cookies.username || 'Guest';`}</code></pre>
            </div>
          </div>
        </div>
      </Layout>
    );
  }

  // Set visit counter cookie
  if (action === "set") {
    const visitCount = parseInt(ctx.cookies.visits || "0") + 1;
    cookies.set("visits", visitCount.toString(), { maxAge: 31536000 });

    if (isHtmx) {
      // htmx请求：返回更新后的Cookie表格
      return <CookiesTable cookies={ctx.cookies} />;
    } else {
      // 普通请求：重定向
      return { redirect: "/cookie_demo?action=view", status: 302 };
    }
  }

  // Set preference cookies
  if (action === "set_preferences") {
    cookies.setMultiple({
      "theme": { value: "dark", options: { maxAge: 31536000 } },
      "language": { value: "zh-CN", options: { maxAge: 31536000 } },
      "fontSize": { value: "14px", options: { maxAge: 31536000 } },
    });

    if (isHtmx) {
      // htmx请求：返回更新后的Cookie表格
      return <CookiesTable cookies={ctx.cookies} />;
    } else {
      // 普通请求：重定向
      return { redirect: "/cookie_demo?action=view", status: 302 };
    }
  }

  // Set secure session cookie
  if (action === "set_secure") {
    // 使用 nanoid 生成唯一的 session ID
    cookies.set("sessionId", nanoid(), {
      httpOnly: true,
      secure: true,
      sameSite: "Strict",
      maxAge: 3600,
      path: "/",
    });

    if (isHtmx) {
      // htmx请求：返回更新后的Cookie表格
      return <CookiesTable cookies={ctx.cookies} />;
    } else {
      // 普通请求：重定向
      return { redirect: "/cookie_demo?action=view", status: 302 };
    }
  }

  // Clear all cookies
  if (action === "clear") {
    for (const name of Object.keys(ctx.cookies)) {
      cookies.delete(name, { path: "/" });
    }

    if (isHtmx) {
      // htmx请求：返回更新后的Cookie表格
      return <CookiesTable cookies={ctx.cookies} />;
    } else {
      // 普通请求：重定向
      return { redirect: "/cookie_demo?action=view", status: 302 };
    }
  }

  return <div>Unknown action</div>;
});

// Cookie表格组件
function CookiesTable({ cookies }: { cookies: Record<string, string> }) {
  return (
    <div className="card shadow-sm mb-4 fade-in">
      <div className="card-body">
        <h2 className="h4 card-title text-primary mb-3">当前 Cookies</h2>
        {Object.keys(cookies).length > 0
          ? (
            <div className="table-responsive">
              <table className="table table-striped table-hover">
                <thead className="table-primary">
                  <tr>
                    <th>Name</th>
                    <th>Value</th>
                  </tr>
                </thead>
                <tbody>
                  {Object.entries(cookies).map(([name, value]) => (
                    <tr key={name}>
                      <td>
                        <code className="bg-light p-1 rounded">{name}</code>
                      </td>
                      <td>
                        <code className="bg-light p-1 rounded small">{value}</code>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )
          : (
            <p className="text-muted fst-italic">
              尚未设置任何 Cookie。试试下面的按钮！
            </p>
          )}
      </div>
    </div>
  );
}
