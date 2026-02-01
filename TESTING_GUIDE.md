# 如何访问和运行测试

## 🧪 E2E 测试（通过浏览器）

### 1. 启动服务器

```bash
deno task dev
```

### 2. 访问测试页面

在浏览器中打开以下链接：

#### Logger 测试
```
http://localhost:9000/logger_e2e
```

测试内容：
- 基本日志输出
- 多参数日志
- 对象序列化
- 日志级别过滤
- 大数据量日志
- 错误堆栈日志
- 特殊字符处理
- 性能测试

#### 日志归档测试
```
http://localhost:9000/logger_rotation_e2e
```

测试内容：
- 基本归档测试
- 大文件归档测试
- 并发写入测试
- 日期归档测试
- 压缩归档测试
- 归档清理测试
- 性能测试
- 错误处理测试

### 3. 运行测试

在测试页面上：
1. 点击各个测试链接
2. 查看返回的 JSON 结果
3. 检查服务器控制台的日志输出
4. 检查 `./logs/` 目录的日志文件

### 4. 验证结果

```bash
# 查看日志目录
ls -lh ./logs/

# 查看归档文件
ls -lh ./logs/app.log.*

# 实时监控日志
tail -f ./logs/app.log
```

## 🔬 单元测试（命令行）

### 运行所有单元测试

```bash
deno task test:unit
```

### 只运行日志归档单元测试

```bash
deno test --allow-all tests/unit/logger_rotation_test.ts
```

### 只运行 Logger 单元测试

```bash
deno test --allow-all tests/unit/logger_test.ts
```

### 查看测试详情

```bash
# 详细输出
deno test --allow-all tests/unit/logger_rotation_test.ts

# 过滤特定测试
deno test --allow-all tests/unit/logger_rotation_test.ts --filter "按大小"
```

## 📋 测试结果

### 单元测试示例输出

```
running 17 tests from ./tests/unit/logger_rotation_test.ts
日志归档测试设置 ... ok (2ms)
LogRotator 创建实例成功 ... ok (0ms)
LogRotator 写入基本日志 ... ok (2ms)
...
ok | 17 passed | 0 failed (775ms)
```

### E2E 测试示例输出

访问测试链接后，会返回 JSON：

```json
{
  "success": true,
  "test": "基本归档测试",
  "message": "已写入 10 条日志，请检查日志文件和归档文件",
  "hint": "查看 ./logs/ 目录"
}
```

## 🏠 从首页访问

启动服务器后，访问首页：
```
http://localhost:9000
```

在首页的 **"🧪 测试工具"** 卡片中点击：
- **Logger 测试**
- **日志归档测试**

## 📝 配置日志归档

创建 `config.json` 文件：

```json
{
  "root": "./www",
  "port": 9000,
  "dev": false,
  "logger": {
    "file": "./logs/app.log",
    "level": "INFO",
    "rotation": {
      "maxSize": 10485760,
      "maxFiles": 5,
      "compress": true
    }
  }
}
```

然后启动服务器：
```bash
deno run --allow-net --allow-read --allow-write src/main.ts --config config.json
```

## 🔍 故障排查

### 找不到测试页面

确认文件已复制到 www 目录：
```bash
ls -lh www/*e2e.tsx
```

应该看到：
- www/logger_e2e.tsx
- www/logger_rotation_e2e.tsx

### 测试页面 404

确保服务器正在运行：
```bash
deno task dev
```

然后访问：
```
http://localhost:9000/logger_e2e
```

### 日志文件未生成

检查配置文件中的 `logger.file` 路径是否正确。

## ✅ 快速验证

1. **启动服务器**
   ```bash
   deno task dev
   ```

2. **访问首页**
   ```
   http://localhost:9000
   ```

3. **点击"Logger 测试"**

4. **点击"基本日志输出"测试**

5. **查看服务器控制台**
   应该看到彩色日志输出

6. **访问日志归档测试**
   ```
   http://localhost:9000/logger_rotation_e2e
   ```

7. **点击"大文件归档测试"**

8. **检查日志文件**
   ```bash
   ls -lh ./logs/
   ```

## 📚 相关文档

- [日志归档使用文档](docs/logging-rotation.md)
- [日志归档测试文档](docs/testing-logging-rotation.md)
- [配置示例](config.rotation.example.jsonc)
