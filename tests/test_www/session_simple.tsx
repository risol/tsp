/**
 * Session简单测试页面
 */

export default Page(async function (ctx, { session }) {
  // 简单测试：只检查session对象
  const sessionType = typeof session;
  const hasGetUser = typeof session?.getUser === "function";

  return (
    <html>
      <head>
        <title>Session Test</title>
      </head>
      <body>
        <h1>Session Test</h1>
        <p>session type: {sessionType}</p>
        <p>has getUser: {hasGetUser ? "yes" : "no"}</p>
      </body>
    </html>
  );
});
