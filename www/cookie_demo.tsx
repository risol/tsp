/**
 * Cookie Demo Page
 *
 * A simple demonstration of TSP's cookie management capabilities.
 */

export default Page(async function (ctx, { cookies }) {
  const action = ctx.query.action || "view";

  // View mode: show current cookies
  if (action === "view") {
    const visitCount = parseInt(ctx.cookies.visits || "0") + 1;

    return (
      <html>
        <head>
          <title>Cookie Demo - TSP</title>
          <meta charset="UTF-8" />
          <style>
            {`
            body {
              font-family: system-ui, -apple-system, sans-serif;
              max-width: 800px;
              margin: 40px auto;
              padding: 20px;
              line-height: 1.6;
            }
            h1 { color: #333; border-bottom: 2px solid #0070f3; padding-bottom: 10px; }
            .card {
              background: #f8f9fa;
              border: 1px solid #dee2e6;
              border-radius: 8px;
              padding: 20px;
              margin: 20px 0;
            }
            .btn {
              display: inline-block;
              padding: 10px 20px;
              margin: 5px;
              background: #0070f3;
              color: white;
              text-decoration: none;
              border-radius: 5px;
              border: none;
              cursor: pointer;
              font-size: 14px;
            }
            .btn:hover { background: #0051cc; }
            .btn.secondary { background: #6c757d; }
            .btn.secondary:hover { background: #545b62; }
            table {
              width: 100%;
              border-collapse: collapse;
              margin: 20px 0;
            }
            th, td {
              text-align: left;
              padding: 12px;
              border-bottom: 1px solid #dee2e6;
            }
            th { background: #0070f3; color: white; }
            code {
              background: #e9ecef;
              padding: 2px 6px;
              border-radius: 3px;
              font-family: 'Courier New', monospace;
            }
          `}
          </style>
        </head>
        <body>
          <h1>🍪 Cookie Management Demo</h1>

          <div class="card">
            <h2>Welcome!</h2>
            <p>
              This is your visit number: <strong>{visitCount}</strong>
            </p>
            <p>
              TSP's cookie system makes it easy to manage HTTP cookies with full
              TypeScript support.
            </p>
          </div>

          <div class="card">
            <h2>Current Cookies</h2>
            {Object.keys(ctx.cookies).length > 0
              ? (
                <table>
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Value</th>
                    </tr>
                  </thead>
                  <tbody>
                    {Object.entries(ctx.cookies).map(([name, value]) => (
                      <tr>
                        <td>
                          <code>{name}</code>
                        </td>
                        <td>
                          <code>{value}</code>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )
              : (
                <p>
                  <em>No cookies set yet. Try the buttons below!</em>
                </p>
              )}
          </div>

          <div class="card">
            <h2>Try It Out</h2>
            <div>
              <form method="POST" action="?action=set">
                <button type="submit" class="btn">
                  Set Visit Counter Cookie
                </button>
              </form>
              <form method="POST" action="?action=set_preferences">
                <button type="submit" class="btn">
                  Set Preference Cookies
                </button>
              </form>
              <form method="POST" action="?action=set_secure">
                <button type="submit" class="btn">
                  Set Secure Session Cookie
                </button>
              </form>
              <form method="POST" action="?action=clear">
                <button type="submit" class="btn secondary">
                  Clear All Cookies
                </button>
              </form>
            </div>
          </div>

          <div class="card">
            <h2>How It Works</h2>
            <p>Setting cookies in TSP is simple:</p>
            <pre><code>{`
export default Page(async function(ctx, { cookies }) {
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
});
            `.trim()}</code></pre>
            <p>Reading cookies is even easier - they're in your context:</p>
            <pre><code>{`
const username = ctx.cookies.username || 'Guest';
            `.trim()}</code></pre>
          </div>
        </body>
      </html>
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
    cookies.set("sessionId", crypto.randomUUID(), {
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
