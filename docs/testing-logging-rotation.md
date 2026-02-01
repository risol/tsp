# 日志归档功能测试文档

本文档说明如何运行和验证日志归档功能的测试。

## 📋 测试概述

日志归档功能包含两类测试：

1. **单元测试** - 测试 `LogRotator` 类的核心功能
2. **E2E 测试** - 测试完整的日志归档流程集成

## 🧪 单元测试

### 运行单元测试

```bash
# 只运行日志归档单元测试
deno test --allow-all tests/unit/logger_rotation_test.ts

# 运行所有单元测试（包括日志归档）
deno task test:unit
```

### 单元测试覆盖

测试文件：`tests/unit/logger_rotation_test.ts`

| 测试用例 | 说明 |
|---------|------|
| LogRotator 创建实例成功 | 验证实例化 |
| LogRotator 写入基本日志 | 测试文件写入 |
| LogRotator 按大小自动归档 | 测试达到 maxSize 时归档 |
| LogRotator 限制归档文件数量 | 验证 maxFiles 限制 |
| LogRotator 按日期归档 | 测试 daily: true 模式 |
| LogRotator 生成正确的归档文件路径 | 验证路径命名 |
| LogRotator 清理超过 maxFiles 的旧归档 | 测试自动清理 |
| LogRotator 归档文件命名正确 | 测试文件名格式 |
| LogRotator 支持并发写入 | 测试并发安全性 |
| LogRotator 处理空字符串 | 边界测试 |
| LogRotator 处理特殊字符 | Unicode 和特殊字符 |
| LogRotator 处理长消息 | 大消息测试 |
| LogRotator 使用默认配置 | 默认配置验证 |
| LogRotator 自动创建日志目录 | 目录自动创建 |
| LogRotator 多次写入累积内容 | 写入累积测试 |

**总计：17 个测试用例**

## 🌐 E2E 测试

### 访问 E2E 测试页面

启动服务器后访问：
```
http://localhost:9000/logger_rotation_e2e
```

### E2E 测试场景

| 测试 | 说明 | 验证方法 |
|------|------|---------|
| 基本归档测试 | 写入 10 条日志 | 检查 ./logs/ 目录 |
| 大文件归档测试 | 写入 50KB 数据 | 触发归档 |
| 并发写入测试 | 20 个并发写入 | 数据完整性 |
| 日期归档测试 | 按日期命名 | 文件名包含日期 |
| 压缩归档测试 | gzip 压缩 | .gz 文件生成 |
| 归档清理测试 | 写入 5 轮数据 | 不超过 maxFiles |
| 性能测试 | 200 条日志 | 响应时间 |
| 错误处理测试 | 边界情况 | 不崩溃 |

### 运行 E2E 测试

```bash
# 1. 启动服务器（带日志配置）
deno task dev

# 2. 访问测试页面
http://localhost:9000/logger_rotation_e2e

# 3. 点击各个测试链接

# 4. 检查日志目录
ls -lh ./logs/

# 5. 验证归档文件
ls -lh ./logs/app.log.*
```

## 🔍 验证归档功能

### 按大小归档验证

```bash
# 配置示例
{
  "logger": {
    "file": "./logs/app.log",
    "rotation": {
      "maxSize": 1024,  # 1KB
      "maxFiles": 3,
      "compress": false
    }
  }
}

# 运行测试
curl http://localhost:9000/logger_rotation_e2e?action=large-file-rotation

# 检查结果
ls -lh ./logs/
# 应该看到：
# app.log        (当前日志)
# app.log.1      (第一个归档)
# app.log.2      (第二个归档)
```

### 按日期归档验证

```bash
# 配置示例
{
  "logger": {
    "file": "./logs/app.log",
    "rotation": {
      "daily": true,
      "maxFiles": 7
    }
  }
}

# 运行测试
curl http://localhost:9000/logger_rotation_e2e?action=daily-rotation

# 检查结果
ls -lh ./logs/
# 应该看到：
# app.log                      (当前日志)
# app.log.2026-02-01           (今天的日志)
# app.log.2026-01-31           (昨天的日志)
```

### 压缩归档验证

```bash
# 配置示例
{
  "logger": {
    "file": "./logs/app.log",
    "rotation": {
      "maxSize": 1024,
      "maxFiles": 3,
      "compress": true
    }
  }
}

# 运行测试
curl http://localhost:9000/logger_rotation_e2e?action=compress-rotation

# 检查结果
ls -lh ./logs/
# 应该看到：
# app.log         (未压缩)
# app.log.1.gz    (gzip 压缩)
# app.log.2.gz    (gzip 压缩)
```

## 📊 性能基准

### 单元测试性能

```
17 个测试用例
总耗时：~800ms
平均每测试：~47ms
```

### E2E 性能测试结果

访问 `http://localhost:9000/logger_rotation_e2e?action=performance`

预期结果：
```
{
  "elapsedMs": 100-300,
  "avgMsPerLog": "0.50-1.50",
  "logsPerSecond": "700-2000"
}
```

## 🐛 故障排查

### 测试失败

1. **单元测试失败**
   ```bash
   # 查看详细错误
   deno test --allow-all tests/unit/logger_rotation_test.ts --filter "测试名称"
   ```

2. **归档文件未生成**
   ```bash
   # 检查配置
   cat config.json | grep rotation

   # 检查日志文件大小
   ls -lh ./logs/app.log

   # 检查文件权限
   ls -la ./logs/
   ```

3. **路径错误**
   ```bash
   # 确认当前目录
   pwd

   # 检查日志目录是否存在
   ls -la ./logs/
   ```

### 清理测试文件

```bash
# 清理测试日志
rm -rf .test_logs/

# 清理实际日志
rm -rf ./logs/

# 清理所有
rm -rf .test_logs/ ./logs/
```

## 📝 测试最佳实践

1. **每次修改代码后运行单元测试**
   ```bash
   deno test --allow-all tests/unit/logger_rotation_test.ts
   ```

2. **修改归档逻辑后运行 E2E 测试**
   ```bash
   # 启动服务器
   deno task dev

   # 访问测试页面并运行所有测试
   # http://localhost:9000/logger_rotation_e2e
   ```

3. **性能测试对比**
   ```bash
   # 修改前
   curl http://localhost:9000/logger_rotation_e2e?action=performance

   # 修改后
   curl http://localhost:9000/logger_rotation_e2e?action=performance

   # 对比结果
   ```

4. **集成到 CI/CD**
   ```yaml
   # GitHub Actions 示例
   - name: Run logger rotation tests
     run: |
       deno test --allow-all tests/unit/logger_rotation_test.ts
   ```

## ✅ 测试检查清单

在发布前确保：

- [ ] 所有单元测试通过（17/17）
- [ ] 所有 E2E 测试场景验证
- [ ] 性能测试在可接受范围
- [ ] 归档文件命名正确
- [ ] 压缩功能正常（如果启用）
- [ ] 日期归档正确（如果启用）
- [ ] 旧归档清理正常
- [ ] 并发写入无数据损坏

## 📚 相关文档

- [日志归档使用文档](./logging-rotation.md)
- [配置示例](../config.rotation.example.jsonc)
- [Logger API 文档](../../src/logger.ts)
