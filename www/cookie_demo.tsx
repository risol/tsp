import { Layout } from "./components/Layout.tsx";

/**
 * Cookie Demo Page
 *
 * A simple demonstration of TSP's cookie management capabilities.
 */

export default Page(async function (ctx, { cookies, nanoid }) {
  const action = ctx.query.action || "view";

  // View mode: show current cookies
  if (action === "view") {
    const visitCount = parseInt(ctx.cookies.visits || "0") + 1;

    return (
      <Layout title="Cookie 演示 - TSP" description="TSP Cookie 管理功能演示">
        {/* Page Header */}
        <div className="text-center mb-5">
          <h1 className="display-5 fw-bold text-white mb-3">🍪 Cookie 管理演示</h1>
          <p className="text-white-50 fs-5">
            完整的 Cookie 操作功能展示
          </p>
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
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">当前 Cookies</h2>
            {Object.keys(ctx.cookies).length > 0
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
                      {Object.entries(ctx.cookies).map(([name, value]) => (
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

        {/* Actions Card */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">试试看</h2>
            <div className="d-flex flex-wrap gap-2">
              <form method="POST" action="?action=set" className="d-inline">
                <button type="submit" className="btn btn-primary">
                  设置访问计数器 Cookie
                </button>
              </form>
              <form method="POST" action="?action=set_preferences" className="d-inline">
                <button type="submit" className="btn btn-success">
                  设置偏好设置 Cookie
                </button>
              </form>
              <form method="POST" action="?action=set_secure" className="d-inline">
                <button type="submit" className="btn btn-info">
                  设置安全会话 Cookie
                </button>
              </form>
              <form method="POST" action="?action=clear" className="d-inline">
                <button type="submit" className="btn btn-secondary">
                  清除所有 Cookie
                </button>
              </form>
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
    return { redirect: "?action=view", status: 302 };
  }

  // Set preference cookies
  if (action === "set_preferences") {
    cookies.setMultiple({
      "theme": { value: "dark", options: { maxAge: 31536000 } },
      "language": { value: "zh-CN", options: { maxAge: 31536000 } },
      "fontSize": { value: "14px", options: { maxAge: 31536000 } },
    });
    return { redirect: "?action=view", status: 302 };
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
    return { redirect: "?action=view", status: 302 };
  }

  // Clear all cookies
  if (action === "clear") {
    for (const name of Object.keys(ctx.cookies)) {
      cookies.delete(name, { path: "/" });
    }
    return { redirect: "?action=view", status: 302 };
  }

  return <div>Unknown action</div>;
});
