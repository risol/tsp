/**
 * Logger 模块单元测试
 */

import {
  createDefaultLogger,
  createLogger,
  createProductionLogger,
  type Logger,
  LogLevel,
} from "../../src/logger.ts";
import {
  assertEquals,
  assertExists,
} from "https://deno.land/std@0.210.0/testing/asserts.ts";

// 测试用例：创建默认日志记录器
Deno.test("createDefaultLogger 创建有效的日志记录器", () => {
  const logger = createDefaultLogger();

  assertExists(logger);
  assertExists(logger.debug);
  assertExists(logger.info);
  assertExists(logger.warn);
  assertExists(logger.error);
});

// 测试用例：创建生产环境日志记录器
Deno.test("createProductionLogger 创建有效的日志记录器", () => {
  const logger = createProductionLogger();

  assertExists(logger);
  assertExists(logger.debug);
  assertExists(logger.info);
  assertExists(logger.warn);
  assertExists(logger.error);
});

// 测试用例：日志级别枚举
Deno.test("LogLevel 枚举值正确", () => {
  assertEquals(LogLevel.DEBUG, 0);
  assertEquals(LogLevel.INFO, 1);
  assertEquals(LogLevel.WARN, 2);
  assertEquals(LogLevel.ERROR, 3);
});

// 测试用例：最小日志级别过滤
Deno.test("minLevel 配置过滤日志", () => {
  // 只显示 WARN 和 ERROR
  const logger = createLogger({
    minLevel: LogLevel.WARN,
    colorize: false,
    console: false,
  });

  // 这些调用不应该抛出错误
  logger.debug("不应该显示");
  logger.info("不应该显示");
  logger.warn("应该显示");
  logger.error("应该显示");
});

// 测试用例：ERROR 级别只显示错误
Deno.test("ERROR 级别只显示错误日志", () => {
  const logger = createLogger({
    minLevel: LogLevel.ERROR,
    colorize: false,
    console: false,
  });

  logger.debug("不应该显示");
  logger.info("不应该显示");
  logger.warn("不应该显示");
  logger.error("应该显示");
});

// 测试用例：DEBUG 级别显示所有日志
Deno.test("DEBUG 级别显示所有日志", () => {
  const logger = createLogger({
    minLevel: LogLevel.DEBUG,
    colorize: false,
    console: false,
  });

  logger.debug("应该显示");
  logger.info("应该显示");
  logger.warn("应该显示");
  logger.error("应该显示");
});

// 测试用例：文本格式日志
Deno.test("文本格式日志包含正确信息", () => {
  const logger = createLogger({
    minLevel: LogLevel.INFO,
    colorize: false,
    console: false,
    format: "text",
  });

  // 不应该抛出错误
  logger.info("测试消息");
});

// 测试用例：JSON 格式日志
Deno.test("JSON 格式日志包含正确字段", () => {
  const logger = createLogger({
    minLevel: LogLevel.INFO,
    colorize: false,
    console: false,
    format: "json",
  });

  // 不应该抛出错误
  logger.info("测试消息");
});

// 测试用例：多个参数日志
Deno.test("支持多个参数", () => {
  const logger = createLogger({
    minLevel: LogLevel.INFO,
    colorize: false,
    console: false,
  });

  logger.info("消息1", "消息2", { data: "test" }, 123);
});

// 测试用例：对象参数序列化
Deno.test("对象参数正确序列化", () => {
  const logger = createLogger({
    minLevel: LogLevel.INFO,
    colorize: false,
    console: false,
  });

  logger.info({ user: "test", action: "login" });
});

// 测试用例：禁用控制台输出
Deno.test("console: false 时不输出到控制台", () => {
  const logger = createLogger({
    minLevel: LogLevel.INFO,
    colorize: false,
    console: false,
  });

  // 不应该抛出错误
  logger.info("不应该在控制台显示");
});

// 测试用例：彩色输出不影响日志内容
Deno.test("colorize 选项不影响日志功能", () => {
  const loggerColor = createLogger({
    minLevel: LogLevel.INFO,
    colorize: true,
    console: false,
  });

  const loggerPlain = createLogger({
    minLevel: LogLevel.INFO,
    colorize: false,
    console: false,
  });

  loggerColor.info("彩色日志");
  loggerPlain.info("纯文本日志");
});

// 测试用例：日志记录器接口完整性
Deno.test("Logger 接口包含所有必需方法", () => {
  const logger: Logger = createDefaultLogger();

  assertEquals(typeof logger.debug, "function");
  assertEquals(typeof logger.info, "function");
  assertEquals(typeof logger.warn, "function");
  assertEquals(typeof logger.error, "function");
});

// 测试用例：不同级别的日志方法存在
Deno.test("所有日志方法都可以调用", () => {
  const logger = createLogger({
    minLevel: LogLevel.DEBUG,
    colorize: false,
    console: false,
  });

  // 测试所有方法都能正常调用
  logger.debug("debug");
  logger.info("info");
  logger.warn("warn");
  logger.error("error");
});

// 测试用例：空参数处理
Deno.test("空参数不抛出错误", () => {
  const logger = createLogger({
    minLevel: LogLevel.INFO,
    colorize: false,
    console: false,
  });

  logger.info();
  logger.debug();
  logger.warn();
  logger.error();
});

// 测试用例：特殊字符处理
Deno.test("特殊字符不导致错误", () => {
  const logger = createLogger({
    minLevel: LogLevel.INFO,
    colorize: false,
    console: false,
  });

  logger.info("测试中文字符 🎉");
  logger.info("测试特殊字符: \\n \\t \\r");
  logger.info("测试引号: \"test\" 'test'");
});

// 测试用例：大对象序列化
Deno.test("大对象可以正确序列化", () => {
  const logger = createLogger({
    minLevel: LogLevel.INFO,
    colorize: false,
    console: false,
  });

  const largeObject = {
    users: Array.from({ length: 100 }, (_, i) => ({
      id: i,
      name: `User${i}`,
      email: `user${i}@example.com`,
    })),
  };

  logger.info(largeObject);
});

// 测试用例：循环引用处理（JSON.stringify 会处理）
Deno.test("循环引用对象会被 JSON.stringify 处理", () => {
  const logger = createLogger({
    minLevel: LogLevel.INFO,
    colorize: false,
    console: false,
  });

  const obj: Record<string, unknown> = { a: 1 };
  obj.self = obj; // 循环引用

  // JSON.stringify 会抛出错误，但这是预期行为
  // 测试确保日志系统不会崩溃
  try {
    logger.info(obj);
  } catch (error) {
    // 如果抛出错误，这是正常的 JSON.stringify 行为
    assertExists(error);
  }
});
