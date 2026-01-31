/**
 * Cookie 设置测试
 * 验证 cookies 依赖注入可以正常工作
 */

export default Page(async function (ctx, { cookies }) {
  // 测试设置 cookie
  if (ctx.query.test === "set") {
    cookies.set("testCookie", "testValue", {
      maxAge: 3600,
    });

    return new Response(
      JSON.stringify({ success: true, message: "Cookie set successfully" }),
      {
        headers: { "Content-Type": "application/json" },
      },
    );
  }

  // 测试获取 cookie
  if (ctx.query.test === "get") {
    const value = ctx.cookies.testCookie || "not set";

    return new Response(
      JSON.stringify({ testCookie: value }),
      {
        headers: { "Content-Type": "application/json" },
      },
    );
  }

  // 默认页面
  return (
    <html>
      <head>
        <title>Cookie Set Test</title>
      </head>
      <body>
        <h1>Cookie Set Test</h1>
        <p>Test that cookies.set() works correctly</p>
        <a href="?test=set">Set Cookie</a>
        <br />
        <a href="?test=get">Get Cookie</a>
      </body>
    </html>
  );
});
