export default async function (context) {
  const { method, body } = context;

  const hasPostData = method === "POST" && body;
  const resultHtml = hasPostData
    ? `
    <div class="result">
      <h3>✅ 提交成功！</h3>
      <pre>${JSON.stringify(body, null, 2)}</pre>
    </div>`
    : "";

  return `
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>表单示例 - TSP-FPM</title>
  <style>
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      max-width: 800px;
      margin: 0 auto;
      padding: 20px;
      background: #f5f5f5;
    }
    .card {
      background: white;
      padding: 30px;
      border-radius: 8px;
      box-shadow: 0 2px 10px rgba(0,0,0,0.1);
    }
    h1 { color: #333; margin-bottom: 20px; }
    .form-group { margin-bottom: 15px; }
    label { display: block; margin-bottom: 5px; font-weight: bold; }
    input, textarea {
      width: 100%;
      padding: 10px;
      border: 1px solid #ddd;
      border-radius: 4px;
      font-size: 14px;
    }
    button {
      background: #007bff;
      color: white;
      border: none;
      padding: 10px 20px;
      border-radius: 4px;
      cursor: pointer;
      font-size: 14px;
    }
    button:hover { background: #0056b3; }
    .result {
      margin-top: 20px;
      padding: 15px;
      background: #d4edda;
      border: 1px solid #c3e6cb;
      border-radius: 4px;
    }
    .error {
      background: #f8d7da;
      border-color: #f5c6cb;
    }
  </style>
</head>
<body>
  <div class="card">
    <h1>📝 表单提交示例</h1>

    ${resultHtml}

    <form method="POST">
      <div class="form-group">
        <label>用户名：</label>
        <input type="text" name="username" required>
      </div>

      <div class="form-group">
        <label>邮箱：</label>
        <input type="email" name="email" required>
      </div>

      <div class="form-group">
        <label>年龄：</label>
        <input type="number" name="age" min="1" max="150">
      </div>

      <div class="form-group">
        <label>个人简介：</label>
        <textarea name="bio" rows="4"></textarea>
      </div>

      <button type="submit">提交</button>
    </form>
  </div>
</body>
</html>
  `.trim();
}
