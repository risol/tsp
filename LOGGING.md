# TSP 日志系统

TSP 提供了完整的日志系统，支持三种不同类型的日志。

## 📊 三种日志类型

### 1. HTTP Access 日志

记录每个 HTTP 请求的访问信息。

**配置方式**：
```jsonc
{
  "accessLogPath": ".logs/access.log"
}
```

**日志格式**：
```
2024-01-31T12:00:00.000Z GET /index.html 200 "Mozilla/5.0"
```

**包含信息**：
- 时间戳（ISO 8601 格式）
- HTTP 方法（GET, POST, PUT, DELETE 等）
- 请求路径
- 响应状态码
- User-Agent

---

### 2. Application Logger（应用日志）

用于在 TSX 页面代码中记录自定义日志。

**配置方式**：
```jsonc
{
  "logger": {
    "level": "INFO",
    "file": ".logs/app.log",
    "colorize": true,
    "format": "text"
  }
}
```

**使用方式**：
```tsx
export default Page(async function(ctx, { logger }) {
  logger.info("用户登录", { userId: 123 });
  logger.warn("操作未授权", { ip: ctx.headers.get("x-forwarded-for") });
  logger.error("数据库错误", { error: err.message });

  return <div>Hello</div>;
});
```

**日志级别**：
- `DEBUG`: 调试信息（默认不显示）
- `INFO`: 一般信息（默认）
- `WARN`: 警告信息
- `ERROR`: 错误信息

**日志格式**：

**Text 格式**：
```
[2024-01-31T12:00:00.000Z] [INFO] 用户登录 {"userId":123}
[2024-01-31T12:00:01.000Z] [ERROR] 数据库错误 {"error":"Connection failed"}
```

**JSON 格式**：
```json
{"level":"INFO","timestamp":"2024-01-31T12:00:00.000Z","message":"用户登录","userId":123}
{"level":"ERROR","timestamp":"2024-01-31T12:00:01.000Z","message":"数据库错误","error":"Connection failed"}
```

---

### 3. Server 日志（服务器日志）

自动记录服务器系统事件，包括：

- ✅ 服务器启动/关闭
- ✅ 预编译开始/完成
- ✅ 缓存预热
- ✅ 请求处理错误
- ✅ 文件加载失败

**配置方式**：与 Application Logger 共用配置

**自动记录的日志示例**：
```
[2024-01-31T12:00:00.000Z] [INFO] TSP Server 启动中 {"root":"./www","port":9000,"mode":"Production"}
[2024-01-31T12:00:01.000Z] [INFO] 开始预编译 TSX 文件
[2024-01-31T12:00:02.000Z] [INFO] 预编译完成 {"count":42}
[2024-01-31T12:00:03.000Z] [INFO] 开始预热缓存
[2024-01-31T12:00:04.000Z] [INFO] 缓存预热完成 {"count":42}
[2024-01-31T12:00:05.000Z] [INFO] 服务器启动成功 {"url":"http://localhost:9000","port":9000}
[2024-01-31T12:00:10.000Z] [ERROR] 请求处理错误 {"error":"Unexpected token","url":"http://localhost:9000/test"}
```

---

## ⚙️ 配置选项

### Logger 配置

```typescript
interface LoggerConfig {
  // 日志级别：DEBUG, INFO, WARN, ERROR
  level?: "DEBUG" | "INFO" | "WARN" | "ERROR";

  // 日志文件路径（可选）
  // 不设置：只输出到控制台
  // 设置：同时输出到控制台和文件
  file?: string;

  // 是否启用彩色输出
  // 开发模式默认：true
  // 生产模式默认：false
  colorize?: boolean;

  // 日志格式：text 或 json
  // text: 人类可读的文本格式
  // json: 机器可解析的 JSON 格式
  format?: "text" | "json";
}
```

### 完整配置示例

```jsonc
{
  "root": "./www",
  "port": 9000,
  "dev": false,

  // HTTP Access 日志
  "accessLogPath": ".logs/access.log",

  // Application & Server 日志
  "logger": {
    "level": "INFO",
    "file": ".logs/app.log",
    "colorize": true,
    "format": "text"
  }
}
```

---

## 🎯 使用场景

### 开发环境

```jsonc
{
  "dev": true,
  "logger": {
    "level": "DEBUG",    // 显示所有日志
    "colorize": true,    // 彩色输出
    "format": "text"     // 可读格式
  }
}
```

### 生产环境

```jsonc
{
  "dev": false,
  "accessLogPath": ".logs/access.log",  // 记录所有请求
  "logger": {
    "level": "INFO",              // 只记录重要信息
    "file": ".logs/app.log",      // 写入文件
    "colorize": false,            // 无颜色
    "format": "json"              // JSON 格式便于分析
  }
}
```

### 调试模式

```jsonc
{
  "dev": true,
  "logger": {
    "level": "DEBUG",    // 显示所有级别
    "format": "text"     // 详细格式
  }
}
```

---

## 📁 日志文件管理

### 日志目录结构

```
project/
├── .logs/
│   ├── access.log    # HTTP Access 日志
│   └── app.log       # Application & Server 日志
├── config.jsonc
└── www/
```

### 日志轮转

TSP 不会自动轮转日志文件，建议使用外部工具：

**Linux/macOS**：
```bash
# 使用 logrotate
# /etc/logrotate.d/tsp
.logs/*.log {
  daily
  rotate 7
  compress
  delaycompress
  missingok
  notifempty
}
```

**Windows**：
使用 PowerShell 任务计划定期重命名或压缩日志文件。

---

## 🔍 日志分析

### 查看 Access 日志

```bash
# 查看最近的请求
tail -f .logs/access.log

# 统计状态码
awk '{print $4}' .logs/access.log | sort | uniq -c

# 查找 404 错误
grep " 404 " .logs/access.log
```

### 查看 Application 日志

```bash
# 查看错误日志
grep "ERROR" .logs/app.log

# 查看 JSON 格式日志
jq '.' .logs/app.log

# 统计日志级别
grep -oE '\[(DEBUG|INFO|WARN|ERROR)\]' .logs/app.log | sort | uniq -c
```

---

## 💡 最佳实践

1. **开发环境**：使用 `DEBUG` 级别和彩色输出
2. **生产环境**：使用 `INFO` 或 `WARN` 级别，写入文件
3. **日志分析**：生产环境使用 JSON 格式便于机器处理
4. **性能**：避免在循环中记录大量 DEBUG 日志
5. **安全**：不要在日志中记录敏感信息（密码、token 等）

---

## 📝 API 参考

### Logger 接口

```typescript
interface Logger {
  debug(...args: unknown[]): void;
  info(...args: unknown[]): void;
  warn(...args: unknown[]): void;
  error(...args: unknown[]): void;
}
```

### 在 TSX 中使用

```tsx
export default Page(async function(ctx, { logger }) {
  // 支持多个参数
  logger.info("用户登录", { userId: 123, ip: "192.168.1.1" });

  // 支持对象序列化
  logger.debug("请求数据", ctx.body);

  // 支持错误堆栈
  try {
    await doSomething();
  } catch (error) {
    logger.error("操作失败", {
      error: error.message,
      stack: error.stack
    });
  }

  return <div>Hello</div>;
});
```

---

## 🆚 与 console.log 的区别

| 特性 | console.log | logger |
|------|------------|--------|
| 日志级别 | ❌ 无 | ✅ DEBUG/INFO/WARN/ERROR |
| 文件输出 | ❌ 仅控制台 | ✅ 支持文件 |
| 格式化 | ❌ 基础 | ✅ text/json |
| 彩色输出 | ✅ 固定 | ✅ 可配置 |
| 结构化数据 | ❌ 弱 | ✅ 强（自动 JSON 序列化） |
| 级别过滤 | ❌ 无 | ✅ 支持 |
| 配置管理 | ❌ 无 | ✅ 统一配置 |

**推荐**：在 TSX 页面中使用 `logger`，在服务器启动代码中使用 `logger` 或 `console.log` 都可以。
