export default Page(async function(ctx, { response }) {
  const action = ctx.query.action || "demo";

  if (action === "json") {
    return response.json({
      message: "Hello from API",
      timestamp: new Date().toISOString(),
    });
  }

  if (action === "json-201") {
    return response.json({ id: 123, created: true }, 201);
  }

  if (action === "error") {
    return response.error("Test error message", 400);
  }

  if (action === "error-500") {
    return response.error("Server error", 500);
  }

  if (action === "redirect") {
    return response.redirect("/?action=demo");
  }

  if (action === "redirect-301") {
    return response.redirect("/?action=demo", 301);
  }

  if (action === "text") {
    return response.text("Plain text response");
  }

  if (action === "html") {
    return response.html("<h1>HTML Response</h1>");
  }

  if (action === "file") {
    return response.file("Hello, World!", "test.txt");
  }

  if (action === "nocontent") {
    return response.noContent();
  }

  // 默认：展示测试链接
  return (
    <html>
      <head>
        <title>Response Helper 测试</title>
        <meta charset="UTF-8" />
        <style>
          {`
          body { font-family: system-ui, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }
          h1 { color: #333; }
          ul { list-style: none; padding: 0; }
          li { margin: 10px 0; }
          a { display: inline-block; padding: 10px 20px; background: #0070f3; color: white; text-decoration: none; border-radius: 5px; }
          a:hover { background: #0051cc; }
        `}
        </style>
      </head>
      <body>
        <h1>📦 Response Helper 测试</h1>
        <ul>
          <li><a href="?action=json">Test JSON (200)</a></li>
          <li><a href="?action=json-201">Test JSON (201 Created)</a></li>
          <li><a href="?action=error">Test Error (400)</a></li>
          <li><a href="?action=error-500">Test Error (500)</a></li>
          <li><a href="?action=redirect">Test Redirect (302)</a></li>
          <li><a href="?action=redirect-301">Test Redirect (301)</a></li>
          <li><a href="?action=text">Test Text</a></li>
          <li><a href="?action=html">Test HTML</a></li>
          <li><a href="?action=file">Test File Download</a></li>
          <li><a href="?action=nocontent">Test No Content (204)</a></li>
        </ul>
      </body>
    </html>
  );
});
