import { Layout } from "./components/Layout.tsx";

/**
 * Session Demo Page with htmx support
 *
 * Demonstrates the session management functionality with htmx for seamless interactions:
 * - Login/logout without page refresh
 * - Session data storage
 * - User information
 * - Session regeneration
 * - Session status
 */

export default Page(async function (context, { session }) {
  const action = context.query.action || "view";

  // 检测是否为htmx请求
  const isHtmx = context.headers.get("HX-Request") === "true";

  // Handle actions
  if (action === "login") {
    const userId = `user-${Date.now()}`;
    await session.login(userId, {
      name: "Demo User",
      email: "demo@example.com",
      role: "guest",
    });

    if (isHtmx) {
      // htmx请求：返回HTML片段
      const user = await session.getUser();
      return <UserCard user={user} />;
    } else {
      // 普通请求：重定向
      return {
        redirect: "/session_demo",
        status: 302,
      };
    }
  }

  if (action === "logout") {
    await session.logout();

    if (isHtmx) {
      // htmx请求：返回未登录状态
      return <NoUserCard />;
    } else {
      // 普通请求：重定向
      return {
        redirect: "/session_demo",
        status: 302,
      };
    }
  }

  if (action === "set-data") {
    await session.set("lastVisit", new Date().toISOString());
    await session.set(
      "visits",
      ((await session.get<number>("visits")) || 0) + 1,
    );
    await session.set("cart", ["item1", "item2", "item3"]);

    if (isHtmx) {
      // htmx请求：返回更新后的Session数据卡片
      const visits = await session.get<number>("visits") || 0;
      const lastVisit = await session.get<string>("lastVisit");
      const cart = await session.get<string[]>("cart");
      return <SessionDataCard visits={visits} lastVisit={lastVisit} cart={cart} />;
    } else {
      // 普通请求：重定向
      return {
        redirect: "/session_demo",
        status: 302,
      };
    }
  }

  if (action === "delete-data") {
    await session.delete("cart");

    if (isHtmx) {
      // htmx请求：返回更新后的Session数据卡片
      const visits = await session.get<number>("visits") || 0;
      const lastVisit = await session.get<string>("lastVisit");
      const cart = await session.get<string[]>("cart");
      return <SessionDataCard visits={visits} lastVisit={lastVisit} cart={cart} />;
    } else {
      // 普通请求：重定向
      return {
        redirect: "/session_demo",
        status: 302,
      };
    }
  }

  if (action === "regenerate") {
    await session.regenerateId();

    if (isHtmx) {
      // htmx请求：返回更新后的Session状态
      const sessionId = session.getId();
      const isValid = await session.isValid();
      return <SessionStatusCard sessionId={sessionId} isValid={isValid} />;
    } else {
      // 普通请求：重定向
      return {
        redirect: "/session_demo",
        status: 302,
      };
    }
  }

  // Get session info
  const user = await session.getUser();
  const visits = await session.get<number>("visits") || 0;
  const lastVisit = await session.get<string>("lastVisit");
  const cart = await session.get<string[]>("cart");
  const isValid = await session.isValid();
  const sessionId = session.getId();

  // 如果是htmx请求返回特定组件，则返回完整页面
  return (
    <Layout title="Session 演示 - TSP" description="TSP Session 管理功能演示">
      {/* Page Header */}
      <div className="text-center mb-5">
        <h1 className="display-5 fw-bold text-dark mb-3">🔐 Session 演示</h1>
        <p className="text-muted fs-5">
          完整的 Session 管理功能展示（支持htmx无刷新交互）
        </p>
      </div>

      {/* 加载指示器 */}
      <div id="loading-indicator" className="htmx-indicator text-center mb-3">
        <div className="spinner-border text-primary" role="status">
          <span className="visually-hidden">加载中...</span>
        </div>
      </div>

      {/* Session Status Card */}
      <div id="session-status">
        <SessionStatusCard sessionId={sessionId} isValid={isValid} />
      </div>

      {/* User Info Card */}
      <div id="user-info">
        {user ? <UserCard user={user} /> : <NoUserCard />}
      </div>

      {/* Session Data Card */}
      <div id="session-data">
        <SessionDataCard visits={visits} lastVisit={lastVisit} cart={cart} />
      </div>

      {/* Actions Card */}
      <div className="card shadow-sm mb-4">
        <div className="card-body">
          <h2 className="h4 card-title text-primary mb-3">操作（htmx无刷新）</h2>
          <div className="d-flex flex-wrap gap-2 mb-4">
            {!user
              ? (
                <button
                  hx-get="/session_demo?action=login"
                  hx-target="#user-info"
                  hx-swap="outerHTML"
                  hx-indicator="#loading-indicator"
                  className="btn btn-success"
                >
                  登录
                </button>
              )
              : (
                <>
                  <button
                    hx-get="/session_demo?action=set-data"
                    hx-target="#session-data"
                    hx-swap="outerHTML"
                    hx-indicator="#loading-indicator"
                    className="btn btn-primary"
                  >
                    设置演示数据
                  </button>
                  {cart && (
                    <button
                      hx-get="/session_demo?action=delete-data"
                      hx-target="#session-data"
                      hx-swap="outerHTML"
                      hx-indicator="#loading-indicator"
                      className="btn btn-warning"
                    >
                      清空购物车
                    </button>
                  )}
                  <button
                    hx-get="/session_demo?action=regenerate"
                    hx-target="#session-status"
                    hx-swap="outerHTML"
                    hx-indicator="#loading-indicator"
                    className="btn btn-info"
                  >
                    重新生成ID
                  </button>
                  <button
                    hx-get="/session_demo?action=logout"
                    hx-target="#user-info"
                    hx-swap="outerHTML"
                    hx-indicator="#loading-indicator"
                    className="btn btn-danger"
                  >
                    退出登录
                  </button>
                </>
              )}
            <a href="/session_demo" className="btn btn-secondary">
              刷新页面
            </a>
          </div>

          <div className="alert alert-success mb-0" role="alert">
            <h5 className="alert-heading">✨ htmx已启用</h5>
            <p className="mb-2 small">
              所有操作都会通过htmx无刷新更新页面，体验更加流畅！
            </p>
            <ul className="mb-0 small">
              <li>登录/退出无需页面刷新</li>
              <li>设置数据实时更新显示</li>
              <li>重新生成ID即时生效</li>
              <li>所有操作都有加载指示器</li>
            </ul>
          </div>
        </div>
      </div>
    </Layout>
  );
});

// Session状态卡片组件
function SessionStatusCard({ sessionId, isValid }: { sessionId: string; isValid: boolean }) {
  return (
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
  );
}

// 用户信息卡片组件（已登录）
function UserCard({ user }: { user: any }) {
  return (
    <div className="card shadow-sm mb-4">
      <div className="card-body">
        <h2 className="h4 card-title text-primary mb-3">用户信息</h2>
        <div className="bg-primary text-white p-4 rounded-3 mb-4">
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
    </div>
  );
}

// 用户信息卡片组件（未登录）
function NoUserCard() {
  return (
    <div className="card shadow-sm mb-4">
      <div className="card-body">
        <h2 className="h4 card-title text-primary mb-3">用户信息</h2>
        <div className="text-center py-5 text-muted">
          <p className="fs-5 mb-2">没有用户登录</p>
          <p className="small">
            点击下面的"登录"按钮创建会话
          </p>
        </div>
      </div>
    </div>
  );
}

// Session数据卡片组件
function SessionDataCard({ visits, lastVisit, cart }: { visits: number; lastVisit?: string; cart?: string[] }) {
  return (
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
  );
}
