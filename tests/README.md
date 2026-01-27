# TSP-FPM 端到端测试

这是 TSP-FPM (类 PHP-FPM 模板执行引擎) 的端到端测试套件。

## 测试文件说明

### 1. `basic_test.ts` - 基础功能测试
测试 TSP-FPM 的核心基础功能：
- 首页渲染
- 查询参数处理
- GET/POST 请求处理
- 表单提交
- Cookies 解析
- 请求头处理

### 2. `routing_test.ts` - 路由和安全测试
测试 URL 路由映射和安全防护功能：
- URL 到文件系统的映射
- 自动添加 .tsx 扩展名
- 404 错误处理
- 路径穿越攻击防护
- 文件类型验证
- URL 编码处理
- 大小写敏感性

### 3. `redirect_test.ts` - 重定向功能测试
测试重定向功能：
- 302 临时重定向
- 301 永久重定向
- 条件重定向（基于 cookies）
- 重定向链
- Location 头处理
- 重定向状态码验证

### 4. `error_test.ts` - 错误处理测试
测试各种错误场景：
- 生产模式错误处理（隐藏详情）
- 开发模式错误处理（显示详情和堆栈）
- 模块加载错误
- 语法错误
- HTTP 方法支持
- 边缘情况处理

### 5. `custom_response_test.ts` - 自定义 Response 测试
测试返回自定义 Response 对象的功能：
- JSON API 响应
- 自定义状态码
- 自定义 Headers
- 不同内容类型（JSON、XML、纯文本）
- 空响应体（204 No Content）

## 运行测试

### 运行所有测试
```bash
deno test --allow-net --allow-read --allow-run tests/e2e/
```

### 运行单个测试文件
```bash
deno test --allow-net --allow-read --allow-run tests/e2e/basic_test.ts
```

### 运行特定测试
```bash
deno test --allow-net --allow-read --allow-run tests/e2e/basic_test.ts --filter "首页"
```

## 测试端口分配

为了并行运行测试，每个测试文件使用不同的端口：
- `basic_test.ts`: 9100
- `routing_test.ts`: 9101
- `redirect_test.ts`: 9102
- `error_test.ts` (生产模式): 9103
- `error_test.ts` (开发模式): 9104
- `custom_response_test.ts`: 9105

## 测试依赖

测试使用 Deno 内置的测试框架和断言库：
- `@std/assert` - 断言函数
- `@std/http` - HTTP 工具（如果需要）
- `Deno.test` - 测试框架

## 测试权限

测试需要以下权限：
- `--allow-net` - 启动测试服务器和发送 HTTP 请求
- `--allow-read` - 读取测试文件和创建临时测试页面
- `--allow-run` - 启动 Deno 子进程作为测试服务器

## 编写新的测试

1. 在 `tests/e2e/` 目录下创建新的测试文件
2. 使用 `Deno.test` 定义测试用例
3. 使用独立的端口号（从 9106 开始递增）
4. 包含适当的设置和清理逻辑
5. 使用 `assertEquals`、`assertStringIncludes` 等断言函数

## 测试最佳实践

1. **独立性**: 每个测试应该独立运行，不依赖其他测试
2. **清理**: 使用 `try...finally` 确保资源被正确清理
3. **等待时间**: 给服务器足够的启动时间（当前设置为 2 秒）
4. **临时文件**: 创建临时测试页面后，务必在清理阶段删除
5. **断言**: 使用明确的断言消息便于调试
6. **端口管理**: 避免端口冲突，每个测试套件使用独立端口

## 常见问题

### 测试失败：连接被拒绝
确保没有其他进程占用了测试端口（9100-9105）。

### 测试超时
增加服务器启动等待时间（修改 `setTimeout` 的延迟）。

### 权限错误
确保运行测试时包含所有必要的权限标志。

## 贡献

添加新的测试用例时，请：
1. 遵循现有的测试文件结构
2. 添加清晰的测试描述
3. 包含正常情况和错误情况的测试
4. 更新此 README 文档
