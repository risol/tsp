/**
 * E2E Test Page for Cookie Management
 *
 * This page tests various cookie functionality scenarios:
 * - Basic cookie setting
 * - Cookie with various options
 * - Cookie deletion
 * - Batch operations
 * - Reading request cookies and setting new ones
 */

export default Page(async function (ctx, { cookies }) {
  const action = ctx.query.action || "demo";

  // Demo mode: show all test links
  if (action === "demo") {
    return (
      <html>
        <head>
          <title>Cookie E2E 测试</title>
          <style>
            {`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            h1 { color: #333; }
            .test-section { margin: 30px 0; padding: 20px; border: 1px solid #ddd; border-radius: 8px; }
            .test-section h2 { margin-top: 0; color: #555; }
            a { display: inline-block; margin: 10px 10px 10px 0; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
            a:hover { background: #0051cc; }
            .result { margin-top: 20px; padding: 15px; background: #f0f0f0; border-radius: 5px; }
            code { background: #f5f5f5; padding: 2px 6px; border-radius: 3px; }
          `}
          </style>
        </head>
        <body>
          <h1>🍪 Cookie 管理 E2E 测试</h1>

          <div class="test-section">
            <h2>测试 1：基础 Cookie 设置</h2>
            <p>
              Sets a simple cookie named <code>username</code>
            </p>
            <a href="?action=set-basic">Test Basic Cookie</a>
          </div>

          <div class="test-section">
            <h2>测试 2：带选项的 Cookie</h2>
            <p>
              Sets a cookie with httpOnly, secure, sameSite, and maxAge options
            </p>
            <a href="?action=set-with-options">Test Cookie with Options</a>
          </div>

          <div class="test-section">
            <h2>测试 3：Cookie 删除</h2>
            <p>First sets a cookie, then deletes it</p>
            <a href="?action=delete">Test Cookie Deletion</a>
          </div>

          <div class="test-section">
            <h2>测试 4：批量操作</h2>
            <p>
              Sets multiple cookies at once using <code>setMultiple</code>
            </p>
            <a href="?action=batch-set">Test Batch Set</a>
          </div>

          <div class="test-section">
            <h2>测试 5：读取请求 Cookie</h2>
            <p>Reads existing cookies and displays them</p>
            <a href="?action=read">Test Read Cookies</a>
          </div>

          <div class="test-section">
            <h2>测试 6：带重定向的 Cookie</h2>
            <p>Sets a cookie and redirects to another page</p>
            <a href="?action=redirect">Test Cookie with Redirect</a>
          </div>

          <div class="test-section">
            <h2>测试 7：特殊字符</h2>
            <p>Tests cookies with special characters and Unicode</p>
            <a href="?action=special-chars">Test Special Characters</a>
          </div>

          <div class="test-section">
            <h2>当前 Cookies</h2>
            <div class="result">
              <strong>请求 Cookies：</strong>
              <pre>{JSON.stringify(ctx.cookies, null, 2)}</pre>
            </div>
          </div>
        </body>
      </html>
    );
  }

  // Test 1: Basic cookie setting
  if (action === "set-basic") {
    cookies.set("username", "john_doe");
    cookies.set("theme", "light");

    return (
      <html>
        <head>
          <title>基础 Cookie 测试</title>
          <style>
            {`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}
          </style>
        </head>
        <body>
          <div class="success">
            <h1>✅ 基础 Cookie 测试通过</h1>
            <p>设置的 Cookie：</p>
            <ul>
              <li>
                <code>username</code> = <code>john_doe</code>
              </li>
              <li>
                <code>theme</code> = <code>light</code>
              </li>
            </ul>
            <p>
              <strong>
                检查浏览器的开发者工具（应用 → Cookies）以
                验证！
              </strong>
            </p>
            <a href="?action=demo">← 返回测试</a>
          </div>
        </body>
      </html>
    );
  }

  // Test 2: Cookie with options
  if (action === "set-with-options") {
    cookies.set("sessionId", "abc123xyz", {
      httpOnly: true,
      secure: true,
      sameSite: "Strict",
      maxAge: 3600, // 1 hour
      path: "/",
    });

    return (
      <html>
        <head>
          <title>Cookie 选项测试</title>
          <style>
            {`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}
          </style>
        </head>
        <body>
          <div class="success">
            <h1>✅ Cookie 选项测试通过</h1>
            <p>设置带选项的 Cookie：</p>
            <ul>
              <li>
                <code>sessionId</code> = <code>abc123xyz</code>
              </li>
              <li>
                <code>httpOnly</code> = <code>true</code>
              </li>
              <li>
                <code>secure</code> = <code>true</code>
              </li>
              <li>
                <code>sameSite</code> = <code>Strict</code>
              </li>
              <li>
                <code>maxAge</code> = <code>3600</code> (1 hour)
              </li>
              <li>
                <code>path</code> = <code>/</code>
              </li>
            </ul>
            <p>
              <strong>
                检查浏览器开发者工具验证所有选项都已设置！
              </strong>
            </p>
            <a href="?action=demo">← 返回测试</a>
          </div>
        </body>
      </html>
    );
  }

  // Test 3: Cookie deletion
  if (action === "delete") {
    // First set a cookie
    if (!ctx.cookies.tempCookie) {
      cookies.set("tempCookie", "will_be_deleted");
      return (
        <html>
          <head>
            <title>Cookie 删除测试 - 步骤 1</title>
            <style>
              {`
              body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
              .info { background: #fff3cd; color: #856404; padding: 20px; border-radius: 8px; }
              a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
            `}
            </style>
          </head>
          <body>
            <div class="info">
              <h1>步骤 1：设置 Cookie</h1>
              <p>
                Set cookie <code>tempCookie</code> ={" "}
                <code>will_be_deleted</code>
              </p>
              <a href="?action=delete">步骤 2：删除 Cookie →</a>
            </div>
          </body>
        </html>
      );
    }

    // Now delete it
    cookies.delete("tempCookie");

    return (
      <html>
        <head>
          <title>Cookie 删除测试 - 步骤 2</title>
          <style>
            {`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}
          </style>
        </head>
        <body>
          <div class="success">
            <h1>✅ Cookie 删除测试通过</h1>
            <p>
              Deleted cookie <code>tempCookie</code>
            </p>
            <p>
              <strong>
              该 Cookie 现在应该已经从您的浏览器中移除！
              </strong>
            </p>
            <p>
              刷新此页面 - Cookie 不应该出现在请求
              cookies 中。
            </p>
            <a href="?action=demo">← 返回测试</a>
          </div>
        </body>
      </html>
    );
  }

  // Test 4: Batch operations
  if (action === "batch-set") {
    cookies.setMultiple({
      "theme": { value: "dark", options: { maxAge: 31536000 } },
      "language": { value: "zh-CN", options: { maxAge: 31536000 } },
      "fontSize": { value: "14px", options: { maxAge: 31536000 } },
    });

    return (
      <html>
        <head>
          <title>批量操作测试</title>
          <style>
            {`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}
          </style>
        </head>
        <body>
          <div class="success">
            <h1>✅ 批量操作测试通过</h1>
            <p>
              使用 <code>setMultiple</code> 设置多个 Cookie：
            </p>
            <ul>
              <li>
                <code>theme</code> = <code>dark</code> (maxAge: 1 year)
              </li>
              <li>
                <code>language</code> = <code>zh-CN</code> (maxAge: 1 year)
              </li>
              <li>
                <code>fontSize</code> = <code>14px</code> (maxAge: 1 year)
              </li>
            </ul>
            <p>
              <strong>
                检查浏览器开发者工具验证所有 3 个 Cookie 都已设置！
              </strong>
            </p>
            <a href="?action=demo">← 返回测试</a>
          </div>
        </body>
      </html>
    );
  }

  // Test 5: Read request cookies
  if (action === "read") {
    const username = ctx.cookies.username || "Not set";
    const theme = ctx.cookies.theme || "Not set";
    const sessionId = ctx.cookies.sessionId || "Not set";

    // Set a "lastVisit" cookie
    cookies.set("lastVisit", new Date().toISOString());

    return (
      <html>
        <head>
          <title>读取请求 Cookie 测试</title>
          <style>
            {`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}
          </style>
        </head>
        <body>
          <div class="success">
            <h1>✅ 读取请求 Cookie 测试</h1>
            <p>当前请求的 Cookie：</p>
            <ul>
              <li>
                <code>username</code>: {username}
              </li>
              <li>
                <code>theme</code>: {theme}
              </li>
              <li>
                <code>sessionId</code>: {sessionId}
              </li>
            </ul>
            <p>
              设置 <code>lastVisit</code> Cookie：{" "}
              <code>{new Date().toISOString()}</code>
            </p>
            <p>
              <strong>此页面的所有 Cookie：</strong>
            </p>
            <pre>{JSON.stringify(ctx.cookies, null, 2)}</pre>
            <a href="?action=demo">← 返回测试</a>
          </div>
        </body>
      </html>
    );
  }

  // Test 6: Cookie with redirect
  if (action === "redirect") {
    cookies.set("redirectTest", "successful", {
      maxAge: 3600,
    });

    return {
      redirect: "?action=read",
      status: 302,
    };
  }

  // Test 7: Special characters
  if (action === "special-chars") {
    cookies.set("user name", "John Doe");
    cookies.set("email", "test@example.com");
    cookies.set("chinese", "张三李四");
    cookies.set("emoji", "😀🎉");

    return (
      <html>
        <head>
          <title>特殊字符测试</title>
          <style>
            {`
            body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
            .success { background: #d4edda; color: #155724; padding: 20px; border-radius: 8px; }
            a { display: inline-block; margin-top: 20px; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          `}
          </style>
        </head>
        <body>
          <div class="success">
            <h1>✅ 特殊字符测试通过</h1>
            <p>设置带有特殊字符和 Unicode 的 Cookie：</p>
            <ul>
              <li>
                <code>user name</code> = <code>John Doe</code>
              </li>
              <li>
                <code>email</code> = <code>test@example.com</code>
              </li>
              <li>
                <code>chinese</code> = <code>张三李四</code>
              </li>
              <li>
                <code>emoji</code> = <code>😀🎉</code>
              </li>
            </ul>
            <p>
              <strong>所有值都会自动进行 URL 编码！</strong>
            </p>
            <p>
              检查浏览器开发者工具验证 Cookie 是否正确编码和
              存储。
            </p>
            <a href="?action=demo">← 返回测试</a>
          </div>
        </body>
      </html>
    );
  }

  // Fallback
  return <div>未知操作</div>;
});
