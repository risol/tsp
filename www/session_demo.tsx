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
        <title>Session Demo</title>
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
          <h1>🔐 Session Demo</h1>

          {/* Session Status Card */}
          <div class="card">
            <h2>Session Status</h2>
            <div class="status">
              <span
                class={`status-badge ${
                  isValid ? "status-valid" : "status-invalid"
                }`}
              >
                {isValid ? "✓ Valid" : "✗ Invalid"}
              </span>
              <span
                style={{
                  marginLeft: "auto",
                  fontSize: "0.9rem",
                  color: "#666",
                }}
              >
                Max Age: 1 day (86400s)
              </span>
            </div>

            <div class="info-row">
              <div class="info-label">Session ID:</div>
              <div class="info-value">
                {sessionId || <span class="empty">No active session</span>}
              </div>
            </div>

            <div class="info-row">
              <div class="info-label">Valid Until:</div>
              <div class="info-value">
                {sessionId
                  ? (
                    <span>
                      {isValid ? "Active" : "Expired"}
                    </span>
                  )
                  : <span class="empty">N/A</span>}
              </div>
            </div>
          </div>

          {/* User Info Card */}
          <div class="card">
            <h2>User Information</h2>
            {user
              ? (
                <div>
                  <div class="user-card">
                    <div class="user-name">{user.name}</div>
                    <div class="user-email">{user.email || "No email"}</div>
                    <div
                      style={{
                        marginTop: "10px",
                        fontSize: "0.9rem",
                        opacity: 0.8,
                      }}
                    >
                      ID: {user.id} | Role: {user.role || "N/A"}
                    </div>
                  </div>

                  <div class="info-row">
                    <div class="info-label">User ID:</div>
                    <div class="info-value">{user.id}</div>
                  </div>
                  <div class="info-row">
                    <div class="info-label">Name:</div>
                    <div class="info-value">{user.name}</div>
                  </div>
                  <div class="info-row">
                    <div class="info-label">Email:</div>
                    <div class="info-value">
                      {user.email || <span class="empty">Not set</span>}
                    </div>
                  </div>
                  <div class="info-row">
                    <div class="info-label">Role:</div>
                    <div class="info-value">
                      {user.role || <span class="empty">Not set</span>}
                    </div>
                  </div>
                </div>
              )
              : (
                <div class="no-session">
                  <p>No user logged in</p>
                  <p style={{ fontSize: "0.9rem", marginTop: "10px" }}>
                    Click "Login" below to create a session
                  </p>
                </div>
              )}
          </div>

          {/* Session Data Card */}
          <div class="card">
            <h2>Session Data</h2>
            <div class="info-row">
              <div class="info-label">Visits:</div>
              <div class="info-value">{visits}</div>
            </div>
            <div class="info-row">
              <div class="info-label">Last Visit:</div>
              <div class="info-value">
                {lastVisit || <span class="empty">Not set</span>}
              </div>
            </div>
            <div class="info-row">
              <div class="info-label">Cart Items:</div>
              <div class="info-value">
                {cart
                  ? (
                    <div class="data-display">
                      {JSON.stringify(cart, null, 2)}
                    </div>
                  )
                  : <span class="empty">Cart is empty</span>}
              </div>
            </div>
          </div>

          {/* Actions Card */}
          <div class="card">
            <h2>Actions</h2>
            <div class="actions">
              {!user
                ? (
                  <a
                    href="/session_demo.tsx?action=login"
                    class="btn btn-success"
                  >
                    Login
                  </a>
                )
                : (
                  <>
                    <a
                      href="/session_demo.tsx?action=set-data"
                      class="btn btn-primary"
                    >
                      Set Demo Data
                    </a>
                    {cart && (
                      <a
                        href="/session_demo.tsx?action=delete-data"
                        class="btn btn-warning"
                      >
                        Clear Cart
                      </a>
                    )}
                    <a
                      href="/session_demo.tsx?action=regenerate"
                      class="btn btn-info"
                    >
                      Regenerate ID
                    </a>
                    <a
                      href="/session_demo.tsx?action=logout"
                      class="btn btn-danger"
                    >
                      Logout
                    </a>
                  </>
                )}
              <a href="/session_demo.tsx" class="btn btn-secondary">
                Refresh Page
              </a>
            </div>

            <div class="hint">
              <strong>💡 Tips:</strong>
              <ul style={{ marginLeft: "20px", marginTop: "10px" }}>
                <li>Login creates a new session with user data</li>
                <li>"Set Demo Data" adds sample data to the session</li>
                <li>
                  "Regenerate ID" creates a new session ID (security feature)
                </li>
                <li>Session data persists across page refreshes</li>
                <li>Session expires after 1 day of inactivity</li>
                <li>
                  Check browser DevTools → Application → Cookies to see the
                  session cookie
                </li>
              </ul>
            </div>
          </div>
        </div>
      </body>
    </html>
  );
});
