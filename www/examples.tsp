import { Sidebar } from "../components/Sidebar.tsx";
import { Footer } from "../components/Footer.tsx";

export default Page(async function(ctx) {
  return (
    <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>代码示例 - TSP</title>
        <style dangerouslySetInnerHTML={{ __html: `
          * { margin: 0; padding: 0; box-sizing: border-box; }
          :root {
            --primary: #3178c6;
            --primary-dark: #235a97;
            --text: #24292e;
            --text-light: #586069;
            --bg: #ffffff;
            --bg-secondary: #f6f8fa;
            --border: #e1e4e8;
            --code-bg: #f6f8fa;
            --sidebar-width: 240px;
          }
          body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: var(--text);
            background: var(--bg);
          }
          .layout { display: flex; min-height: 100vh; }
          .sidebar {
            width: var(--sidebar-width);
            background: var(--bg-secondary);
            border-right: 1px solid var(--border);
            position: fixed;
            height: 100vh;
            overflow-y: auto;
            padding: 20px;
          }
          .main { margin-left: var(--sidebar-width); flex: 1; }
          .header { border-bottom: 1px solid var(--border); padding: 16px 32px; background: var(--bg); }
          .header-content { max-width: 1200px; margin: 0 auto; display: flex; justify-content: space-between; align-items: center; }
          .logo { font-size: 20px; font-weight: 600; color: var(--primary); text-decoration: none; }
          .nav-links { display: flex; gap: 24px; }
          .nav-links a { color: var(--text); text-decoration: none; font-size: 14px; }
          .nav-links a:hover { color: var(--primary); }
          .content { max-width: 900px; margin: 0 auto; padding: 40px 32px; }
          h1 { font-size: 36px; margin-bottom: 16px; font-weight: 600; }
          h2 { font-size: 24px; margin: 40px 0 16px; padding-bottom: 8px; border-bottom: 1px solid var(--border); }
          h3 { font-size: 18px; margin: 24px 0 12px; }
          p { margin-bottom: 16px; color: var(--text-light); }
          ul, ol { margin: 16px 0; padding-left: 24px; }
          li { margin-bottom: 8px; color: var(--text-light); }
          code { background: var(--code-bg); padding: 2px 6px; border-radius: 4px; font-size: 14px; }
          pre { background: var(--code-bg); padding: 16px; border-radius: 8px; overflow-x: auto; margin: 16px 0; }
          pre code { background: none; padding: 0; }
          .code-block { position: relative; margin: 24px 0; }
          .code-block .lang { position: absolute; top: 8px; right: 12px; font-size: 12px; color: var(--text-light); }
          .example-card {
            border: 1px solid var(--border);
            border-radius: 8px;
            margin-bottom: 24px;
            overflow: hidden;
          }
          .example-header {
            background: var(--bg-secondary);
            padding: 12px 16px;
            border-bottom: 1px solid var(--border);
            font-weight: 600;
          }
          .example-body { padding: 16px; }
          footer { background: var(--bg-secondary); border-top: 1px solid var(--border); padding: 24px 32px; text-align: center; color: var(--text-light); font-size: 14px; }
          .toc { background: var(--bg-secondary); border-radius: 8px; padding: 16px; margin-bottom: 24px; }
          .toc ul { margin: 0; }
        `}} />
      </head>
      <body>
        <div class="layout">
          <Sidebar />
          <div class="main">
            <header class="header">
              <div class="header-content">
                <a href="/" class="logo">TSP</a>
                <nav class="nav-links">
                  <a href="/getting-started">快速开始</a>
                  <a href="/features">功能</a>
                  <a href="/examples">示例</a>
                </nav>
              </div>
            </header>

            <div class="content">
              <h1>代码示例</h1>
              <p>以下示例展示 TSP 作为 Deno + TypeScript 全栈框架在常见业务场景中的写法。</p>

              <div class="toc">
                <strong>目录</strong>
                <ul>
                  <li><a href="#basic" style={{ color: "#3178c6" }}>基础示例</a></li>
                  <li><a href="#database" style={{ color: "#3178c6" }}>数据库操作</a></li>
                  <li><a href="#session" style={{ color: "#3178c6" }}>Session 管理</a></li>
                  <li><a href="#api" style={{ color: "#3178c6" }}>RESTful API</a></li>
                  <li><a href="#upload" style={{ color: "#3178c6" }}>文件上传</a></li>
                </ul>
              </div>

              <h2 id="basic">基础示例</h2>

              <div class="example-card">
                <div class="example-body">
                  <h3>返回 HTML</h3>
                  <div class="code-block">
                    <span class="lang">tsx</span>
                    <pre><code>{`export default Page(async function(ctx) {
  return (
    <html>
      <head>
        <title>我的页面</title>
      </head>
      <body>
        <h1>Hello World!</h1>
        <p>当前时间: {new Date().toLocaleString()}</p>
      </body>
    </html>
  );
});`}</code></pre>
                  </div>
                </div>
              </div>

              <div class="example-card">
                <div class="example-body">
                  <h3>返回 JSON</h3>
                  <div class="code-block">
                    <span class="lang">tsx</span>
                    <pre><code>{`export default Page(async function(ctx, { response }) {
  return response.json({
    success: true,
    data: {
      name: "张三",
      age: 25
    }
  });
});`}</code></pre>
                  </div>
                </div>
              </div>

              <div class="example-card">
                <div class="example-body">
                  <h3>重定向</h3>
                  <div class="code-block">
                    <span class="lang">tsx</span>
                    <pre><code>{`export default Page(async function(ctx, { response }) {
  return response.redirect('/login');
});`}</code></pre>
                  </div>
                </div>
              </div>

              <h2 id="database">数据库操作</h2>

              <div class="example-card">
                <div class="example-body">
                  <h3>MySQL 查询</h3>
                  <div class="code-block">
                    <span class="lang">tsx</span>
                    <pre><code>{`export default Page(async function(ctx, { createMySQL, z, response }) {
  // 创建数据库连接
  const db = await createMySQL({
    host: '127.0.0.1',
    port: 3306,
    user: 'root',
    password: 'password',
    database: 'test_db'
  }, z);

  // 定义返回数据的 Schema
  const UserSchema = z.object({
    id: z.number(),
    name: z.string(),
    email: z.string()
  });

  // 查询用户列表
  const users = await db.query(UserSchema, 'SELECT * FROM users');

  // 关闭连接
  await db.close();

  return response.json({ users });
});`}</code></pre>
                  </div>
                </div>
              </div>

              <div class="example-card">
                <div class="example-body">
                  <h3>事务操作</h3>
                  <div class="code-block">
                    <span class="lang">tsx</span>
                    <pre><code>{`export default Page(async function(ctx, { createMySQL, z, response }) {
  const db = await createMySQL({ host: '127.0.0.1', user: 'root', password: 'pass', database: 'test' }, z);

  const ResultSchema = z.object({ affectedRows: z.number() });

  try {
    const result = await db.tx(async (tx) => {
      // 扣除转出账户
      await tx.execute(ResultSchema,
        'UPDATE accounts SET balance = balance - ? WHERE id = ?',
        [amount, fromId]
      );

      // 转入目标账户
      await tx.execute(ResultSchema,
        'UPDATE accounts SET balance = balance + ? WHERE id = ?',
        [amount, toId]
      );

      return { success: true };
    });

    return response.json(result);
  } catch (error) {
    return response.json({ success: false, error: error.message }, 500);
  }
});`}</code></pre>
                  </div>
                </div>
              </div>

              <h2 id="session">Session 管理</h2>

              <div class="example-card">
                <div class="example-body">
                  <h3>用户登录</h3>
                  <div class="code-block">
                    <span class="lang">tsx</span>
                    <pre><code>{`export default Page(async function(ctx, { session, response }) {
  if (ctx.method === 'POST') {
    const { email, password } = ctx.body as { email: string; password: string };

    // 验证用户（实际项目中应查询数据库）
    if (email === 'test@example.com' && password === '123456') {
      // 初始化 session
      await session.init();
      await session.set('userId', '1');
      await session.set('userName', 'Test User');

      return response.redirect('/dashboard');
    }

    return response.error('邮箱或密码错误', 401);
  }

  // 显示登录表单
  return (
    <html>
      <body>
        <form method="POST">
          <input type="email" name="email" placeholder="邮箱"<input type="password required />
          " name="password" placeholder="密码" required />
          <button type="submit">登录</button>
        </form>
      </body>
    </html>
  );
});`}</code></pre>
                  </div>
                </div>
              </div>

              <h2 id="api">RESTful API</h2>

              <div class="example-card">
                <div class="example-body">
                  <h3>用户管理 API</h3>
                  <div class="code-block">
                    <span class="lang">tsx</span>
                    <pre><code>{`// www/api/users.tsp
export default Page(async function(ctx, { createMySQL, response, z }) {
  const db = await createMySQL({ host: '127.0.0.1', user: 'root', password: 'pass', database: 'test' }, z);
  const UserSchema = z.object({ id: z.number(), name: z.string(), email: z.string() });

  // GET /api/users - 获取用户列表
  if (ctx.method === 'GET') {
    const users = await db.query(UserSchema, 'SELECT * FROM users');
    return response.json({ data: users });
  }

  // POST /api/users - 创建用户
  if (ctx.method === 'POST') {
    const { name, email } = ctx.body as { name: string; email: string };
    const ResultSchema = z.object({ affectedRows: z.number(), insertId: z.number() });

    const result = await db.execute(
      ResultSchema,
      'INSERT INTO users (name, email) VALUES (?, ?)',
      [name, email]
    );

    return response.json({ success: true, id: result.insertId }, 201);
  }

  return response.error('Method Not Allowed', 405);
});`}</code></pre>
                  </div>
                </div>
              </div>

              <h2 id="upload">文件上传</h2>

              <div class="example-card">
                <div class="example-body">
                  <h3>处理文件上传</h3>
                  <div class="code-block">
                    <span class="lang">tsx</span>
                    <pre><code>{`export default Page(async function(ctx, { response }) {
  if (ctx.method === 'POST') {
    const file = ctx.files.avatar as UploadedFile;

    if (file) {
      // 保存文件
      await file.save(\`./uploads/\${file.name}\`);

      return response.json({
        success: true,
        filename: file.name,
        size: file.size
      });
    }

    return response.error('没有上传文件', 400);
  }

  // 显示上传表单
  return (
    <html>
      <body>
        <form method="POST" enctype="multipart/form-data">
          <input type="file" name="avatar" />
          <button type="submit">上传</button>
        </form>
      </body>
    </html>
  );
});`}</code></pre>
                  </div>
                </div>
              </div>

              <h2>了解更多</h2>
              <ul>
                <li><a href="/features" style={{ color: "#3178c6" }}>功能详情</a> - 了解更多特性</li>
                <li><a href="/getting-started" style={{ color: "#3178c6" }}>快速开始</a> - 完整入门指南</li>
              </ul>
            </div>

            <Footer />
          </div>
        </div>
      </body>
    </html>
  );
});
