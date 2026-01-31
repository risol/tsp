/**
 * Logger E2E 测试页面
 * 用于测试日志记录功能
 */

export default Page(async function (ctx, { logger, response }) {
  const action = ctx.query.action || "demo";

  // 测试1: 基本日志输出
  if (action === "basic") {
    logger.debug("这是一条 DEBUG 日志");
    logger.info("这是一条 INFO 日志");
    logger.warn("这是一条 WARN 日志");
    logger.error("这是一条 ERROR 日志");

    return response.json({
      success: true,
      test: "基本日志输出",
      message: "已输出所有级别的日志，请查看控制台",
    });
  }

  // 测试2: 多参数日志
  if (action === "multi-arg") {
    logger.info("用户登录", { userId: 123, ip: "192.168.1.1" });
    logger.warn("数据库连接缓慢", { latency: 1500, threshold: 1000 });
    logger.error("API 调用失败", { url: "/api/users", status: 500 });

    return response.json({
      success: true,
      test: "多参数日志",
      message: "已输出多参数日志，请查看控制台",
    });
  }

  // 测试3: 对象序列化
  if (action === "object") {
    const user = {
      id: 1,
      name: "张三",
      email: "zhangsan@example.com",
      roles: ["admin", "user"],
      metadata: {
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      },
    };

    logger.info("用户信息", user);

    return response.json({
      success: true,
      test: "对象序列化",
      user,
    });
  }

  // 测试4: 日志级别过滤
  if (action === "level-filter") {
    logger.debug("这条 DEBUG 日志应该被过滤（默认 INFO 级别）");
    logger.info("这条 INFO 日志应该显示");
    logger.warn("这条 WARN 日志应该显示");
    logger.error("这条 ERROR 日志应该显示");

    return response.json({
      success: true,
      test: "日志级别过滤",
      message: "已测试日志级别过滤，请检查控制台输出",
    });
  }

  // 测试5: 大数据量日志
  if (action === "large-data") {
    const items = Array.from({ length: 50 }, (_, i) => ({
      id: i + 1,
      name: `项目 ${i + 1}`,
      value: Math.random() * 1000,
    }));

    logger.info("批量处理数据", { count: items.length, items });

    return response.json({
      success: true,
      test: "大数据量日志",
      count: items.length,
    });
  }

  // 测试6: 错误堆栈日志
  if (action === "error-stack") {
    try {
      throw new Error("模拟的错误");
    } catch (error) {
      logger.error("捕获到异常", {
        message: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined,
      });
    }

    return response.json({
      success: true,
      test: "错误堆栈日志",
      message: "已记录错误堆栈信息",
    });
  }

  // 测试7: 特殊字符处理
  if (action === "special-chars") {
    logger.info("测试中文字符：你好世界 🌍🎉");
    logger.info("测试特殊字符：\\n \\t \\r");
    logger.info("测试引号：\"双引号\" '单引号' `反引号`");
    logger.info("测试 Emoji：🔥 ⚡ 💻 🚀");

    return response.json({
      success: true,
      test: "特殊字符处理",
      message: "已输出特殊字符日志",
    });
  }

  // 测试8: 空参数
  if (action === "empty") {
    logger.info();
    logger.debug();
    logger.warn();
    logger.error();

    return response.json({
      success: true,
      test: "空参数",
      message: "已测试空参数调用",
    });
  }

  // 测试9: 性能测试（连续日志）
  if (action === "performance") {
    const start = Date.now();
    const iterations = 100;

    for (let i = 0; i < iterations; i++) {
      logger.info(`性能测试日志 ${i + 1}`, { iteration: i + 1 });
    }

    const elapsed = Date.now() - start;

    return response.json({
      success: true,
      test: "性能测试",
      iterations,
      elapsedMs: elapsed,
      avgMs: elapsed / iterations,
    });
  }

  // 默认：展示测试说明
  return (
    <html>
      <head>
        <title>Logger E2E 测试</title>
        <meta charset="UTF-8" />
        <style>
          {`
          body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 1000px;
            margin: 0 auto;
            padding: 40px 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
          }
          .container {
            background: white;
            border-radius: 16px;
            padding: 40px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
          }
          h1 {
            color: #333;
            margin-bottom: 10px;
            font-size: 2em;
          }
          .subtitle {
            color: #666;
            margin-bottom: 30px;
            font-size: 1.1em;
          }
          .test-section {
            margin: 30px 0;
            padding: 20px;
            background: #f8f9fa;
            border-radius: 12px;
            border-left: 4px solid #667eea;
          }
          .test-section h2 {
            margin-top: 0;
            color: #495057;
            font-size: 1.3em;
          }
          .test-list {
            list-style: none;
            padding: 0;
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
            gap: 15px;
          }
          .test-item {
            background: white;
            padding: 15px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
            transition: transform 0.2s, box-shadow 0.2s;
          }
          .test-item:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
          }
          .test-item a {
            display: block;
            text-decoration: none;
            color: #333;
          }
          .test-item h3 {
            margin: 0 0 8px 0;
            color: #667eea;
            font-size: 1.1em;
          }
          .test-item p {
            margin: 0;
            color: #666;
            font-size: 0.9em;
          }
          .info-box {
            background: #e7f3ff;
            border-left: 4px solid #2196f3;
            padding: 15px;
            border-radius: 8px;
            margin-bottom: 20px;
          }
          .info-box strong {
            color: #1976d2;
          }
          code {
            background: #f5f5f5;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Courier New', monospace;
            font-size: 0.9em;
          }
        `}
        </style>
      </head>
      <body>
        <div class="container">
          <h1>📝 Logger E2E 测试</h1>
          <p class="subtitle">测试日志记录器的各种功能和场景</p>

          <div class="info-box">
            <strong>💡 提示：</strong>
            点击下面的测试链接会触发相应的日志输出，请查看控制台（终端）来验证日志是否正确输出。
          </div>

          <div class="test-section">
            <h2>基础功能测试</h2>
            <ul class="test-list">
              <li class="test-item">
                <a href="?action=basic">
                  <h3>基本日志输出</h3>
                  <p>测试所有日志级别：DEBUG、INFO、WARN、ERROR</p>
                </a>
              </li>
              <li class="test-item">
                <a href="?action=multi-arg">
                  <h3>多参数日志</h3>
                  <p>测试多个参数的日志输出</p>
                </a>
              </li>
              <li class="test-item">
                <a href="?action=object">
                  <h3>对象序列化</h3>
                  <p>测试复杂对象的 JSON 序列化</p>
                </a>
              </li>
              <li class="test-item">
                <a href="?action=level-filter">
                  <h3>日志级别过滤</h3>
                  <p>测试日志级别过滤功能（默认 INFO）</p>
                </a>
              </li>
            </ul>
          </div>

          <div class="test-section">
            <h2>高级功能测试</h2>
            <ul class="test-list">
              <li class="test-item">
                <a href="?action=large-data">
                  <h3>大数据量日志</h3>
                  <p>测试大量数据的日志输出</p>
                </a>
              </li>
              <li class="test-item">
                <a href="?action=error-stack">
                  <h3>错误堆栈日志</h3>
                  <p>测试错误堆栈信息的记录</p>
                </a>
              </li>
              <li class="test-item">
                <a href="?action=special-chars">
                  <h3>特殊字符处理</h3>
                  <p>测试中文、Emoji、特殊字符</p>
                </a>
              </li>
              <li class="test-item">
                <a href="?action=empty">
                  <h3>空参数</h3>
                  <p>测试无参数调用是否正常</p>
                </a>
              </li>
              <li class="test-item">
                <a href="?action=performance">
                  <h3>性能测试</h3>
                  <p>测试连续输出 100 条日志的性能</p>
                </a>
              </li>
            </ul>
          </div>

          <div class="test-section">
            <h2>使用示例</h2>
            <pre><code>{`// 在 TSX 文件中使用 logger
export default Page(async function(ctx, { logger }) {
  logger.info("页面加载成功");
  logger.debug("调试信息", { userId: 123 });
  logger.warn("警告：操作未授权");
  logger.error("错误：数据库连接失败");

  return <div>Hello</div>;
});`}</code></pre>
          </div>

          <div class="test-section">
            <h2>配置说明</h2>
            <p>在 <code>config.jsonc</code> 或 <code>config.json</code> 中配置日志：</p>
            <pre><code>{`{
  "logger": {
    "level": "INFO",      // 日志级别：DEBUG, INFO, WARN, ERROR
    "file": ".logs/app.log",  // 日志文件路径（可选）
    "colorize": true,      // 是否启用彩色输出
    "format": "text"       // 格式：text 或 json
  }
}`}</code></pre>
            <ul style="margin-top: 15px;">
              <li><strong>level</strong>：只输出大于等于此级别的日志</li>
              <li><strong>file</strong>：不设置则只输出到控制台</li>
              <li><strong>colorize</strong>：开发模式默认 true，生产模式默认 false</li>
              <li><strong>format</strong>：text 为人类可读格式，json 为机器可解析格式</li>
            </ul>
          </div>
        </div>
      </body>
    </html>
  );
});
