# 日志归档功能说明

TSP 的日志系统支持自动归档，可以根据文件大小或日期自动归档日志文件。

## 功能特性

### 1. 按大小归档

当日志文件达到指定大小时，自动创建新的归档文件：

```json
{
  "logger": {
    "file": "./logs/app.log",
    "rotation": {
      "maxSize": 10485760,  // 10MB
      "maxFiles": 5,        // 保留最近 5 个归档
      "compress": true      // 启用 gzip 压缩
    }
  }
}
```

**归档规则：**
- `app.log` → 当前日志文件
- `app.log.1` → 第一个归档
- `app.log.2` → 第二个归档
- `app.log.3.gz` → 压缩归档（如果启用 compress）

当 `app.log` 达到 10MB 时：
1. 删除最旧的归档（如果有 5 个归档）
2. 重命名现有归档：`.1` → `.2`, `.2` → `.3` ...
3. 重命名 `app.log` → `app.log.1`
4. 如果启用压缩，`app.log.1` 会被压缩为 `app.log.1.gz`
5. 创建新的 `app.log` 文件

### 2. 按日期归档

每天自动创建新的日志文件：

```json
{
  "logger": {
    "file": "./logs/app.log",
    "rotation": {
      "daily": true,
      "maxFiles": 30  // 保留最近 30 天的日志
    }
  }
}
```

**文件命名：**
- `app.log` → 今天的日志
- `app.log.2025-01-15` → 2025年1月15日的日志
- `app.log.2025-01-14.gz` → 前一天的日志（如果启用压缩）

### 3. 配置选项

| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `maxSize` | number | 10485760 (10MB) | 单个日志文件最大大小（字节） |
| `maxFiles` | number | 5 | 保留的归档文件数量 |
| `compress` | boolean | false | 是否压缩归档文件（gzip） |
| `daily` | boolean | false | 是否按日期归档 |

## 使用示例

### 示例 1：开发环境（不归档）

```json
{
  "dev": true,
  "logger": {
    "level": "DEBUG",
    "file": "./logs/dev.log"
    // 不配置 rotation，日志文件会无限增长
  }
}
```

### 示例 2：生产环境（按大小归档 + 压缩）

```json
{
  "dev": false,
  "logger": {
    "level": "INFO",
    "file": "./logs/app.log",
    "format": "json",
    "rotation": {
      "maxSize": 52428800,  // 50MB
      "maxFiles": 10,
      "compress": true
    }
  }
}
```

### 示例 3：审计日志（按日期归档）

```json
{
  "logger": {
    "level": "INFO",
    "file": "./logs/audit.log",
    "rotation": {
      "daily": true,
      "maxFiles": 365,  // 保留 1 年
      "compress": true
    }
  }
}
```

### 示例 4：访问日志（只保留最近 7 天）

```json
{
  "accessLogPath": "./logs/access.log",
  "logger": {
    "file": "./logs/app.log",
    "rotation": {
      "daily": true,
      "maxFiles": 7
    }
  }
}
```

## 性能考虑

1. **压缩开销**：启用压缩会增加 CPU 使用，但能节省 70-90% 的磁盘空间
2. **文件大小**：较大的 `maxSize` 会减少归档频率，但单个文件处理时间更长
3. **归档数量**：`maxFiles` 越大，磁盘占用越多

## 监控建议

使用 `tspinfo` 查看日志配置：

```bash
# 访问 http://localhost:9000/tspinfo
# 查看 📝 日志配置 部分
```

定期检查日志目录大小：

```bash
# Linux/Mac
du -sh ./logs

# Windows
dir .\logs
```

## 故障排查

### 日志文件没有被归档

**可能原因：**
- 配置文件中的 `rotation` 配置有误
- 日志文件还未达到 `maxSize`
- 磁盘权限不足

**解决方法：**
```bash
# 检查配置文件
cat config.json

# 检查日志文件权限
ls -la ./logs

# 查看当前日志文件大小
ls -lh ./logs/app.log
```

### 归档文件丢失

**可能原因：**
- `maxFiles` 设置太小
- 手动删除了归档文件

**解决方法：**
- 增加 `maxFiles` 值
- 使用备份策略定期备份日志到远程存储

## 高级用法

### 程序化配置

如果需要动态配置，可以在代码中创建 Logger：

```tsx
import { createProductionLogger } from "../src/logger.ts";

const logger = createProductionLogger({
  level: "INFO",
  file: "./logs/custom.log",
  rotation: {
    maxSize: 20 * 1024 * 1024,  // 20MB
    maxFiles: 15,
    compress: true,
    daily: false
  }
});

logger.info("Logger initialized");
```

### 分级日志

不同级别的日志写入不同文件：

```json
{
  "logger": {
    "level": "DEBUG",
    "file": "./logs/all.log",
    "rotation": { "maxSize": 10485760, "maxFiles": 5 }
  }
}
```

然后在代码中根据需要记录不同级别。
