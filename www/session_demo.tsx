import { Layout } from "./components/Layout.tsx";

/**
 * Session Demo Page
 *
 * Demonstrates the session management functionality including:
 * - Login/logout
 * - Session data storage
 * - User information
 * - Session regeneration
 * - Session status
 */

export default Page(async function (context, { session }) {
  const action = context.query.action || "view";

  // Handle actions
  if (action === "login") {
    const userId = `user-${Date.now()}`;
    await session.login(userId, {
      name: "Demo User",
      email: "demo@example.com",
      role: "guest",
    });

    return {
      redirect: "/session_demo.tsx?action=view",
      status: 302,
    };
  }

  if (action === "logout") {
    await session.logout();

    return {
      redirect: "/session_demo.tsx?action=view",
      status: 302,
    };
  }

  if (action === "set-data") {
    await session.set("lastVisit", new Date().toISOString());
    await session.set(
      "visits",
      ((await session.get<number>("visits")) || 0) + 1,
    );
    await session.set("cart", ["item1", "item2", "item3"]);

    return {
      redirect: "/session_demo.tsx?action=view",
      status: 302,
    };
  }

  if (action === "delete-data") {
    await session.delete("cart");

    return {
      redirect: "/session_demo.tsx?action=view",
      status: 302,
    };
  }

  if (action === "regenerate") {
    await session.regenerateId();

    return {
      redirect: "/session_demo.tsx?action=view",
      status: 302,
    };
  }

  // Get session info
  const user = await session.getUser();
  const visits = await session.get<number>("visits") || 0;
  const lastVisit = await session.get<string>("lastVisit");
  const cart = await session.get<string[]>("cart");
  const isValid = await session.isValid();
  const sessionId = session.getId();

  return (
    <Layout title="Session 演示 - TSP" description="TSP Session 管理功能演示">
      {/* Page Header */}
      <div className="text-center mb-5">
        <h1 className="display-5 fw-bold text-white mb-3">🔐 Session 演示</h1>
        <p className="text-white-50 fs-5">
          完整的 Session 管理功能展示
        </p>
      </div>

      {/* Session Status Card */}
      <div className="card shadow-sm mb-4">
        <div className="card-body">
          <h2 className="h4 card-title text-primary mb-3">Session 状态</h2>
          <div className="d-flex align-items-center gap-2 mb-3">
            <span className={`badge ${isValid ? "bg-success" : "bg-danger"} fs-6`}>
              {isValid ? "✓ 有效" : "✗ 无效"}
            </span>
            <span className="text-muted ms-auto small">
              最大存活时间：1 天（86400秒）
            </span>
          </div>

          <div className="row border-bottom pb-2 mb-2">
            <div className="col-md-3 fw-bold text-muted">Session ID:</div>
            <div className="col-md-9">
              {sessionId || <span className="text-muted fst-italic">没有活动的会话</span>}
            </div>
          </div>

          <div className="row">
            <div className="col-md-3 fw-bold text-muted">有效期至：</div>
            <div className="col-md-9">
              {sessionId
                ? (
                  <span className={isValid ? "text-success" : "text-danger"}>
                    {isValid ? "活跃" : "已过期"}
                  </span>
                )
                : <span className="text-muted fst-italic">不适用</span>}
            </div>
          </div>
        </div>
      </div>

      {/* User Info Card */}
      <div className="card shadow-sm mb-4">
        <div className="card-body">
          <h2 className="h4 card-title text-primary mb-3">用户信息</h2>
          {user
            ? (
              <div>
                <div className="bg-gradient-primary text-white p-4 rounded-3 mb-4">
                  <div className="fs-4 fw-bold mb-1">{user.name}</div>
                  <div className="opacity-75 small mb-2">{user.email || "没有邮箱"}</div>
                  <div className="small opacity-75">
                    ID: {user.id} | 角色: {user.role || "未设置"}
                  </div>
                </div>

                <div className="row border-bottom pb-2 mb-2">
                  <div className="col-md-3 fw-bold text-muted">用户ID：</div>
                  <div className="col-md-9">{user.id}</div>
                </div>
                <div className="row border-bottom pb-2 mb-2">
                  <div className="col-md-3 fw-bold text-muted">姓名：</div>
                  <div className="col-md-9">{user.name}</div>
                </div>
                <div className="row border-bottom pb-2 mb-2">
                  <div className="col-md-3 fw-bold text-muted">邮箱：</div>
                  <div className="col-md-9">
                    {user.email || <span className="text-muted fst-italic">未设置</span>}
                  </div>
                </div>
                <div className="row">
                  <div className="col-md-3 fw-bold text-muted">角色：</div>
                  <div className="col-md-9">
                    {user.role || <span className="text-muted fst-italic">未设置</span>}
                  </div>
                </div>
              </div>
            )
            : (
              <div className="text-center py-5 text-muted">
                <p className="fs-5 mb-2">没有用户登录</p>
                <p className="small">
                  点击下面的"登录"按钮创建会话
                </p>
              </div>
            )}
        </div>
      </div>

      {/* Session Data Card */}
      <div className="card shadow-sm mb-4">
        <div className="card-body">
          <h2 className="h4 card-title text-primary mb-3">Session 数据</h2>
          <div className="row border-bottom pb-2 mb-2">
            <div className="col-md-3 fw-bold text-muted">访问次数：</div>
            <div className="col-md-9">{visits}</div>
          </div>
          <div className="row border-bottom pb-2 mb-2">
            <div className="col-md-3 fw-bold text-muted">最后访问：</div>
            <div className="col-md-9">
              {lastVisit || <span className="text-muted fst-italic">未设置</span>}
            </div>
          </div>
          <div className="row">
            <div className="col-md-3 fw-bold text-muted">购物车项目：</div>
            <div className="col-md-9">
              {cart
                ? (
                  <div className="bg-light p-3 rounded-3 mt-2">
                    <pre className="mb-0 small text-muted" style={{ fontFamily: 'monospace' }}>
                      {JSON.stringify(cart, null, 2)}
                    </pre>
                  </div>
                )
                : <span className="text-muted fst-italic">购物车为空</span>}
            </div>
          </div>
        </div>
      </div>

      {/* Actions Card */}
      <div className="card shadow-sm mb-4">
        <div className="card-body">
          <h2 className="h4 card-title text-primary mb-3">操作</h2>
          <div className="d-flex flex-wrap gap-2 mb-4">
            {!user
              ? (
                <a
                  href="/session_demo.tsx?action=login"
                  className="btn btn-success"
                >
                  登录
                </a>
              )
              : (
                <>
                  <a
                    href="/session_demo.tsx?action=set-data"
                    className="btn btn-primary"
                  >
                    设置演示数据
                  </a>
                  {cart && (
                    <a
                      href="/session_demo.tsx?action=delete-data"
                      className="btn btn-warning"
                    >
                      清空购物车
                    </a>
                  )}
                  <a
                    href="/session_demo.tsx?action=regenerate"
                    className="btn btn-info"
                  >
                    重新生成ID
                  </a>
                  <a
                    href="/session_demo.tsx?action=logout"
                    className="btn btn-danger"
                  >
                    退出登录
                  </a>
                </>
              )}
            <a href="/session_demo.tsx" className="btn btn-secondary">
              刷新页面
            </a>
          </div>

          <div className="alert alert-warning mb-0" role="alert">
            <h5 className="alert-heading">💡 提示：</h5>
            <ul className="mb-0 small">
              <li>登录创建一个包含用户数据的新会话</li>
              <li>"设置演示数据"向会话添加示例数据</li>
              <li>"重新生成ID"创建一个新的会话ID（安全功能）</li>
              <li>会话数据在页面刷新后保持不变</li>
              <li>会话在1天不活动后过期</li>
              <li>检查浏览器开发者工具 → 应用 → Cookies 查看会话 Cookie</li>
            </ul>
          </div>
        </div>
      </div>
    </Layout>
  );
});
