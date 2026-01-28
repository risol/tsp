export default async function (context: PageContext) {
  const { method, query, url } = context;
  const name = query.name || "World";

  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <title>首页 - TSP-FPM 测试</title>
      </head>
      <body>
        <h1>首页</h1>
        <p>欢迎使用 TSP-FPM！</p>
        <p>当前路径: {url.pathname}</p>
        <p>请求方法: {method}</p>
        <p>URL 参数: {name}</p>
      </body>
    </html>
  );
}
