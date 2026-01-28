
export default async function (context: PageContext) {
  const { query } = context;
  const format = query.format;

  // 如果请求 JSON 格式，返回 JSON Response
  if (format === "json") {
    return new Response(
      JSON.stringify({
        message: "这是一个自定义 JSON 响应",
        timestamp: new Date().toISOString(),
        method: context.method,
        path: context.url.pathname,
      }),
      {
        status: 200,
        headers: {
          "Content-Type": "application/json",
          "X-Custom-Header": "test-value",
        },
      }
    );
  }

  // 默认返回 HTML
  return new Response(
    "<!DOCTYPE html><html><head><title>自定义响应测试</title></head><body><h1>自定义 Response 对象测试</h1><p>添加 ?format=json 参数获取 JSON 响应</p></body></html>",
    {
      status: 200,
      headers: {
        "Content-Type": "text/html; charset=utf-8",
        "X-Response-Type": "custom",
      },
    }
  );
}
