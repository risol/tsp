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
    <html>
      <head>
        <title>Session 演示</title>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <style>
          {`
          * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
          }

          body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
          }

          .container {
            max-width: 800px;
            margin: 0 auto;
          }

          h1 {
            color: white;
            text-align: center;
            margin-bottom: 30px;
            font-size: 2.5rem;
            text-shadow: 2px 2px 4px rgba(0,0,0,0.2);
          }

          .card {
            background: white;
            border-radius: 12px;
            padding: 30px;
            margin-bottom: 20px;
            box-shadow: 0 10px 30px rgba(0,0,0,0.2);
          }

          .card h2 {
            color: #667eea;
            margin-bottom: 20px;
            font-size: 1.5rem;
            border-bottom: 2px solid #f0f0f0;
            padding-bottom: 10px;
          }

          .status {
            display: flex;
            align-items: center;
            gap: 10px;
            margin-bottom: 20px;
          }

          .status-badge {
            padding: 6px 12px;
            border-radius: 20px;
            font-size: 0.9rem;
            font-weight: bold;
          }

          .status-valid {
            background: #10b981;
            color: white;
          }

          .status-invalid {
            background: #ef4444;
            color: white;
          }

          .info-row {
            display: flex;
            padding: 12px 0;
            border-bottom: 1px solid #f0f0f0;
          }

          .info-row:last-child {
            border-bottom: none;
          }

          .info-label {
            font-weight: bold;
            color: #666;
            width: 150px;
            flex-shrink: 0;
          }

          .info-value {
            color: #333;
            word-break: break-all;
          }

          .info-value.empty {
            color: #999;
            font-style: italic;
          }

          .actions {
            display: flex;
            gap: 10px;
            flex-wrap: wrap;
            margin-top: 20px;
          }

          .btn {
            padding: 12px 24px;
            border: none;
            border-radius: 8px;
            font-size: 1rem;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.3s ease;
            text-decoration: none;
            display: inline-block;
          }

          .btn:hover {
            transform: translateY(-2px);
            box-shadow: 0 5px 15px rgba(0,0,0,0.2);
          }

          .btn:active {
            transform: translateY(0);
          }

          .btn-primary {
            background: #667eea;
            color: white;
          }

          .btn-success {
            background: #10b981;
            color: white;
          }

          .btn-danger {
            background: #ef4444;
            color: white;
          }

          .btn-warning {
            background: #f59e0b;
            color: white;
          }

          .btn-info {
            background: #3b82f6;
            color: white;
          }

          .btn-secondary {
            background: #6b7280;
            color: white;
          }

          .user-card {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 20px;
            border-radius: 8px;
            margin-bottom: 20px;
          }

          .user-name {
            font-size: 1.5rem;
            font-weight: bold;
            margin-bottom: 5px;
          }

          .user-email {
            opacity: 0.9;
            font-size: 0.9rem;
          }

          .no-session {
            text-align: center;
            padding: 40px;
            color: #999;
          }

          .data-display {
            background: #f9fafb;
            padding: 15px;
            border-radius: 8px;
            margin-top: 10px;
            font-family: 'Courier New', monospace;
            font-size: 0.9rem;
            color: #374151;
          }

          .hint {
            background: #fef3c7;
            border-left: 4px solid #f59e0b;
            padding: 15px;
            margin-top: 20px;
            border-radius: 4px;
            font-size: 0.9rem;
            color: #92400e;
          }

          .hint strong {
            display: block;
            margin-bottom: 5px;
          }
        `}
        </style>
      </head>
      <body>
        <div class="container">
          <h1>🔐 Session 演示</h1>

          {/* Session Status Card */}
          <div class="card">
            <h2>Session 状态</h2>
            <div class="status">
              <span
                class={`status-badge ${
                  isValid ? "status-valid" : "status-invalid"
                }`}
              >
                {isValid ? "✓ 有效" : "✗ 无效"}
              </span>
              <span
                style={{
                  marginLeft: "auto",
                  fontSize: "0.9rem",
                  color: "#666",
                }}
              >
                最大存活时间：1 天（86400秒）
              </span>
            </div>

            <div class="info-row">
              <div class="info-label">Session ID:</div>
              <div class="info-value">
                {sessionId || <span class="empty">没有活动的会话</span>}
              </div>
            </div>

            <div class="info-row">
              <div class="info-label">有效期至：</div>
              <div class="info-value">
                {sessionId
                  ? (
                    <span>
                      {isValid ? "活跃" : "已过期"}
                    </span>
                  )
                  : <span class="empty">不适用</span>}
              </div>
            </div>
          </div>

          {/* User Info Card */}
          <div class="card">
            <h2>用户信息</h2>
            {user
              ? (
                <div>
                  <div class="user-card">
                    <div class="user-name">{user.name}</div>
                    <div class="user-email">{user.email || "没有邮箱"}</div>
                    <div
                      style={{
                        marginTop: "10px",
                        fontSize: "0.9rem",
                        opacity: 0.8,
                      }}
                    >
                      ID: {user.id} | 角色: {user.role || "未设置"}
                    </div>
                  </div>

                  <div class="info-row">
                    <div class="info-label">用户ID：</div>
                    <div class="info-value">{user.id}</div>
                  </div>
                  <div class="info-row">
                    <div class="info-label">姓名：</div>
                    <div class="info-value">{user.name}</div>
                  </div>
                  <div class="info-row">
                    <div class="info-label">邮箱：</div>
                    <div class="info-value">
                      {user.email || <span class="empty">未设置</span>}
                    </div>
                  </div>
                  <div class="info-row">
                    <div class="info-label">角色：</div>
                    <div class="info-value">
                      {user.role || <span class="empty">未设置</span>}
                    </div>
                  </div>
                </div>
              )
              : (
                <div class="no-session">
                  <p>没有用户登录</p>
                  <p style={{ fontSize: "0.9rem", marginTop: "10px" }}>
                    点击下面的"登录"按钮创建会话
                  </p>
                </div>
              )}
          </div>

          {/* Session Data Card */}
          <div class="card">
            <h2>Session 数据</h2>
            <div class="info-row">
              <div class="info-label">访问次数：</div>
              <div class="info-value">{visits}</div>
            </div>
            <div class="info-row">
              <div class="info-label">最后访问：</div>
              <div class="info-value">
                {lastVisit || <span class="empty">未设置</span>}
              </div>
            </div>
            <div class="info-row">
              <div class="info-label">购物车项目：</div>
              <div class="info-value">
                {cart
                  ? (
                    <div class="data-display">
                      {JSON.stringify(cart, null, 2)}
                    </div>
                  )
                  : <span class="empty">购物车为空</span>}
              </div>
            </div>
          </div>

          {/* Actions Card */}
          <div class="card">
            <h2>操作</h2>
            <div class="actions">
              {!user
                ? (
                  <a
                    href="/session_demo.tsx?action=login"
                    class="btn btn-success"
                  >
                    登录
                  </a>
                )
                : (
                  <>
                    <a
                      href="/session_demo.tsx?action=set-data"
                      class="btn btn-primary"
                    >
                      设置演示数据
                    </a>
                    {cart && (
                      <a
                        href="/session_demo.tsx?action=delete-data"
                        class="btn btn-warning"
                      >
                        清空购物车
                      </a>
                    )}
                    <a
                      href="/session_demo.tsx?action=regenerate"
                      class="btn btn-info"
                    >
                      重新生成ID
                    </a>
                    <a
                      href="/session_demo.tsx?action=logout"
                      class="btn btn-danger"
                    >
                      退出登录
                    </a>
                  </>
                )}
              <a href="/session_demo.tsx" class="btn btn-secondary">
                刷新页面
              </a>
            </div>

            <div class="hint">
              <strong>💡 提示：</strong>
              <ul style={{ marginLeft: "20px", marginTop: "10px" }}>
                <li>登录创建一个包含用户数据的新会话</li>
                <li>"设置演示数据"向会话添加示例数据</li>
                <li>
                  "重新生成ID"创建一个新的会话ID（安全功能）
                </li>
                <li>会话数据在页面刷新后保持不变</li>
                <li>会话在1天不活动后过期</li>
                <li>
                  检查浏览器开发者工具 → 应用 → Cookies 查看
                  会话 Cookie
                </li>
              </ul>
            </div>
          </div>
        </div>
      </body>
    </html>
  );
});
