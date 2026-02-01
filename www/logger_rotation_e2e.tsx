/**
 * 日志归档功能 E2E 测试页面
 * 测试日志归档的各种场景
 */

import { Layout } from "./components/Layout.tsx";

export default Page(async function (ctx, { logger, response }) {
  const action = ctx.query.action || "demo";

  // 测试 1：基本归档测试
  if (action === "basic-rotation") {
    logger.info("=== 基本归档测试开始 ===");
    logger.info("这条消息测试基本的日志归档功能");

    // 写入多条日志以测试归档
    for (let i = 0; i < 10; i++) {
      logger.info(`测试消息 ${i + 1}`, {
        timestamp: new Date().toISOString(),
        iteration: i + 1,
      });
    }

    logger.info("=== 基本归档测试结束 ===");

    return response.json({
      success: true,
      test: "基本归档测试",
      message: "已写入 10 条日志，请检查日志文件和归档文件",
      hint: "查看 ./logs/ 目录",
    });
  }

  // 测试 2：大文件归档测试
  if (action === "large-file-rotation") {
    logger.info("=== 大文件归档测试开始 ===");

    // 写入大量数据以触发归档
    const messageSize = 500; // 每条消息约 500 字节
    const messageCount = 100; // 总共约 50KB

    for (let i = 0; i < messageCount; i++) {
      const data = {
        id: i + 1,
        message: "这是一条很长的测试消息，用于增加日志文件大小",
        data: "x".repeat(messageSize - 100), // 填充数据
        timestamp: new Date().toISOString(),
      };
      logger.info(`大文件测试消息 ${i + 1}`, data);
    }

    logger.info("=== 大文件归档测试结束 ===");

    return response.json({
      success: true,
      test: "大文件归档测试",
      message: `已写入 ${messageCount} 条大消息`,
      totalSize: messageSize * messageCount,
      hint: "如果配置了归档，应该会触发归档",
    });
  }

  // 测试 3：并发写入测试
  if (action === "concurrent-write") {
    logger.info("=== 并发写入测试开始 ===");

    const concurrentWrites = 20;
    const promises = Array.from({ length: concurrentWrites }, async (_, i) => {
      logger.info(`并发消息 ${i + 1}`, {
        thread: i,
        timestamp: Date.now(),
      });

      // 模拟异步操作
      await new Promise((resolve) => setTimeout(resolve, Math.random() * 10));

      logger.info(`并发消息 ${i + 1} 完成`, {
        thread: i,
        timestamp: Date.now(),
      });
    });

    await Promise.all(promises);

    logger.info("=== 并发写入测试结束 ===");

    return response.json({
      success: true,
      test: "并发写入测试",
      message: `已完成 ${concurrentWrites} 个并发写入`,
      hint: "检查日志文件是否完整且无数据损坏",
    });
  }

  // 测试 4：日期归档测试
  if (action === "daily-rotation") {
    logger.info("=== 日期归档测试 ===");
    logger.info("当前时间:", new Date().toISOString());
    logger.info("如果启用了 daily: true，日志会按日期命名");
    logger.info("文件名格式: app.log.2025-01-15");

    return response.json({
      success: true,
      test: "日期归档测试",
      message: "已写入日志",
      hint: "检查日志文件名是否包含日期",
      config: {
        feature: "daily rotation",
        enabled: "检查 config.json 中的 logger.rotation.daily",
      },
    });
  }

  // 测试 5：压缩归档测试
  if (action === "compress-rotation") {
    logger.info("=== 压缩归档测试 ===");
    logger.info("这条日志用于测试压缩归档功能");

    // 写入较多数据
    for (let i = 0; i < 50; i++) {
      logger.info(`压缩测试消息 ${i + 1}`, {
        data: "测试数据 " + "x".repeat(100),
        index: i,
      });
    }

    logger.info("=== 压缩归档测试结束 ===");

    return response.json({
      success: true,
      test: "压缩归档测试",
      message: "已写入 50 条消息",
      hint: "如果启用了 compress: true，归档文件应该是 .gz 格式",
      expected: "app.log.1.gz 而不是 app.log.1",
    });
  }

  // 测试 6：查看日志配置
  if (action === "config") {
    return response.json({
      success: true,
      test: "日志配置",
      message: "当前日志配置信息",
      hint: "这些是建议的配置，实际配置请查看 config.json",
      recommended: {
        production: {
          level: "INFO",
          file: "./logs/app.log",
          format: "text",
          rotation: {
            maxSize: 10485760, // 10MB
            maxFiles: 5,
            compress: true,
            daily: false,
          },
        },
        audit: {
          level: "INFO",
          file: "./logs/audit.log",
          format: "json",
          rotation: {
            daily: true,
            maxFiles: 365,
            compress: true,
          },
        },
      },
    });
  }

  // 测试 7：性能测试
  if (action === "performance") {
    logger.info("=== 归档性能测试开始 ===");

    const start = Date.now();
    const messageCount = 200;

    for (let i = 0; i < messageCount; i++) {
      logger.info(`性能测试消息 ${i + 1}`, {
        iteration: i + 1,
        data: { value: Math.random() },
      });
    }

    const elapsed = Date.now() - start;
    const avgTime = elapsed / messageCount;

    logger.info("=== 归档性能测试结束 ===", {
      totalMessages: messageCount,
      totalTime: elapsed,
      avgTime,
    });

    return response.json({
      success: true,
      test: "归档性能测试",
      message: `写入 ${messageCount} 条日志`,
      elapsedMs: elapsed,
      avgMsPerLog: avgTime.toFixed(2),
      logsPerSecond: (1000 / avgTime).toFixed(0),
    });
  }

  // 测试 8：错误处理
  if (action === "error-handling") {
    logger.info("=== 错误处理测试 ===");

    try {
      // 测试各种可能导致错误的情况
      logger.info("测试空参数", {});
      logger.info("测试特殊字符", "\n\t\r");
      logger.info("测试 Unicode", "🎉 🔥 ⚡ 💻");
      logger.info("测试超长消息", "x".repeat(10000));
      logger.info("测试深度嵌套对象", {
        level1: { level2: { level3: { level4: "deep" } } },
      });
    } catch (error) {
      logger.error("错误处理测试捕获异常", {
        error: error instanceof Error ? error.message : String(error),
      });
    }

    logger.info("=== 错误处理测试结束 ===");

    return response.json({
      success: true,
      test: "错误处理测试",
      message: "已测试各种边界情况",
      hint: "检查日志确认没有崩溃或数据损坏",
    });
  }

  // 测试 9：归档文件清理测试
  if (action === "cleanup") {
    logger.info("=== 归档文件清理测试 ===");
    logger.info("这个测试会写入大量数据以触发归档");

    // 写入足够多的数据以创建多个归档
    for (let round = 0; round < 5; round++) {
      logger.info(`=== 第 ${round + 1} 轮写入 ===`);
      for (let i = 0; i < 20; i++) {
        logger.info(`第 ${round + 1} 轮消息 ${i + 1}`, {
          round: round + 1,
          message: i + 1,
          data: "x".repeat(100),
        });
      }
    }

    logger.info("=== 归档文件清理测试结束 ===");

    return response.json({
      success: true,
      test: "归档文件清理测试",
      message: "已写入 5 轮数据，每轮 20 条",
      hint: "检查归档文件数量，应该不超过 maxFiles 配置",
      check: "在 ./logs/ 目录中运行 ls -la 查看归档文件",
    });
  }

  // 默认：展示测试说明
  return (
    <Layout title="日志归档 E2E 测试" description="测试日志归档功能">
      <div className="container py-5">
        {/* Hero Section */}
        <div className="text-center mb-5">
          <h1 className="display-5 fw-bold text-dark mb-3">🗄️ 日志归档 E2E 测试</h1>
          <p className="text-muted fs-5">
            测试日志归档功能的完整流程
          </p>
        </div>

        {/* Info Box */}
        <div className="alert alert-info mb-4" role="alert">
          <h5 className="alert-heading">💡 测试说明</h5>
          <p className="mb-2">
            这些测试会向日志文件写入大量数据，用于验证归档功能是否正常工作。
          </p>
          <p className="mb-0">
            <strong>提示：</strong>测试完成后，请检查 <code>./logs/</code> 目录查看归档文件。
          </p>
        </div>

        {/* Basic Tests */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">基础功能测试</h2>
            <div className="list-group list-group-flush">
              <a
                href="?action=basic-rotation"
                className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0"
              >
                <div>
                  <strong>基本归档测试</strong>
                  <div className="text-muted small">写入 10 条测试日志</div>
                </div>
                <span className="text-muted">→</span>
              </a>
              <a
                href="?action=large-file-rotation"
                className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0"
              >
                <div>
                  <strong>大文件归档测试</strong>
                  <div className="text-muted small">写入 50KB 数据触发归档</div>
                </div>
                <span className="text-muted">→</span>
              </a>
              <a
                href="?action=concurrent-write"
                className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0"
              >
                <div>
                  <strong>并发写入测试</strong>
                  <div className="text-muted small">20 个并发日志写入</div>
                </div>
                <span className="text-muted">→</span>
              </a>
            </div>
          </div>
        </div>

        {/* Advanced Tests */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">高级功能测试</h2>
            <div className="list-group list-group-flush">
              <a
                href="?action=daily-rotation"
                className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0"
              >
                <div>
                  <strong>日期归档测试</strong>
                  <div className="text-muted small">测试按日期归档功能</div>
                </div>
                <span className="text-muted">→</span>
              </a>
              <a
                href="?action=compress-rotation"
                className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0"
              >
                <div>
                  <strong>压缩归档测试</strong>
                  <div className="text-muted small">测试 gzip 压缩归档</div>
                </div>
                <span className="text-muted">→</span>
              </a>
              <a
                href="?action=cleanup"
                className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0"
              >
                <div>
                  <strong>归档清理测试</strong>
                  <div className="text-muted small">测试旧归档文件自动清理</div>
                </div>
                <span className="text-muted">→</span>
              </a>
            </div>
          </div>
        </div>

        {/* Performance and Error Tests */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">性能和错误测试</h2>
            <div className="list-group list-group-flush">
              <a
                href="?action=performance"
                className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0"
              >
                <div>
                  <strong>性能测试</strong>
                  <div className="text-muted small">写入 200 条日志测量性能</div>
                </div>
                <span className="text-muted">→</span>
              </a>
              <a
                href="?action=error-handling"
                className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0"
              >
                <div>
                  <strong>错误处理测试</strong>
                  <div className="text-muted small">测试边界情况和错误恢复</div>
                </div>
                <span className="text-muted">→</span>
              </a>
              <a
                href="?action=config"
                className="list-group-item list-group-item-action d-flex justify-content-between align-items-center border-0 px-0"
              >
                <div>
                  <strong>查看配置建议</strong>
                  <div className="text-muted small">推荐的日志归档配置</div>
                </div>
                <span className="text-muted">→</span>
              </a>
            </div>
          </div>
        </div>

        {/* Configuration Example */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">📝 配置示例</h2>
            <div className="code-block bg-light p-3 rounded mb-3">
              <pre className="mb-0 small text-muted">
{`{
  "logger": {
    "file": "./logs/app.log",
    "level": "INFO",
    "format": "text",
    "rotation": {
      "maxSize": 10485760,  // 10MB
      "maxFiles": 5,        // 保留 5 个归档
      "compress": true,     // 启用 gzip 压缩
      "daily": false        // 按大小归档
    }
  }
}`}
              </pre>
            </div>
            <h6 className="fw-bold mt-3">归档文件命名：</h6>
            <ul className="list-unstyled small text-muted">
              <li>• 按大小归档：app.log, app.log.1, app.log.2.gz</li>
              <li>• 按日期归档：app.log, app.log.2025-01-15, app.log.2025-01-14.gz</li>
            </ul>
          </div>
        </div>

        {/* Check Results */}
        <div className="card shadow-sm mb-4">
          <div className="card-body">
            <h2 className="h4 card-title text-primary mb-3">🔍 检查测试结果</h2>
            <p className="text-muted mb-3">
              运行测试后，使用以下命令检查归档文件：
            </p>
            <div className="code-block bg-dark text-light p-3 rounded">
              <pre className="mb-0 small">
{`# 查看日志目录
ls -lh ./logs/

# 查看归档文件详情
ls -lh ./logs/app.log.*

# 统计归档文件数量
ls -1 ./logs/app.log.* | wc -l

# 查看当前日志文件大小
du -h ./logs/app.log

# 查看所有日志文件总大小
du -sh ./logs/`}
              </pre>
            </div>
          </div>
        </div>

        {/* Back Link */}
        <div className="mt-4">
          <a href="/logger_e2e" className="btn btn-outline-primary">
            ← 返回 Logger 测试
          </a>
        </div>
      </div>
    </Layout>
  );
});
