
export default async function (context: PageContext) {
  const { method, body } = context;

  if (method === "POST" && typeof body === "object" && body !== null) {
    const data = body as Record<string, unknown>;
    const username = data.username as string;

    return (
      <html lang="zh-CN">
        <head>
          <meta charset="UTF-8" />
          <title>表单测试 - TSP-FPM</title>
        </head>
        <body>
          <h1>表单提交成功</h1>
          <p>欢迎, {username || "匿名用户"}!</p>
          <p>请求方法: {method}</p>
        </body>
      </html>
    );
  }

  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <title>表单测试 - TSP-FPM</title>
      </head>
      <body>
        <h1>表单测试</h1>
        <form method="POST">
          <label>
            用户名:
            <input type="text" name="username" />
          </label>
          <button type="submit">提交</button>
        </form>
        <p>请求方法: {method}</p>
      </body>
    </html>
  );
}
