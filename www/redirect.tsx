export default async function (context: PageContext) {
  const { query, cookies } = context;

  // 示例 1: 简单重定向
  if (query.to === "home") {
    return { redirect: "/" };
  }

  // 示例 2: 永久重定向 (301)
  if (query.to === "new-home") {
    return { redirect: "/", status: 301 };
  }

  // 示例 3: 未登录重定向
  if (query.to === "protected" && !cookies.sessionId) {
    return {
      redirect: `/login?redirect=/protected`,
    };
  }

  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <title>重定向示例</title>
      </head>
      <body>
        <h1>重定向示例</h1>

        <section className="mb-4">
          <h2>简单重定向 (302)</h2>
          <p>点击链接跳转到首页：</p>
          <a href="/redirect?to=home" className="btn btn-primary">跳转到首页</a>
        </section>

        <section className="mb-4">
          <h2>永久重定向 (301)</h2>
          <p>点击链接永久跳转到首页：</p>
          <a href="/redirect?to=new-home" className="btn btn-primary">永久跳转到首页</a>
        </section>

        <section className="mb-4">
          <h2>条件重定向</h2>
          <p>如果没有 session cookie，将跳转到登录页：</p>
          <a href="/redirect?to=protected" className="btn btn-primary">访问受保护页面</a>
        </section>

        <section className="mb-4">
          <h2>带参数的重定向</h2>
          <a href="/redirect?to=protected" className="btn btn-primary">访问受保护页面（未登录会重定向）</a>
        </section>
      </body>
    </html>
  );
}
