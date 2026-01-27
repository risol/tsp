# TSP-FPM 测试使用指南

## 快速开始

### 运行所有测试
```bash
deno task test
```

### 运行特定测试
```bash
# 基础功能测试
deno task test:basic

# 路由测试
deno task test:routing

# 重定向测试
deno task test:redirect

# 错误处理测试
deno task test:error

# 自定义响应测试
deno task test:custom
```

## 测试覆盖范围

### ✅ 基础功能 (basic_test.ts)
- [x] 首页渲染
- [x] HTML 内容验证
- [x] 查询参数解析
- [x] GET/POST 请求处理
- [x] 表单提交
- [x] Cookies 处理
- [x] 请求头读取

### ✅ 路由功能 (routing_test.ts)
- [x] 根路径映射到 index.tsx
- [x] 自动添加 .tsx 扩展名
- [x] 显式扩展名支持
- [x] 404 错误处理
- [x] 路径穿越攻击防护
- [x] 文件类型验证
- [x] URL 编码处理
- [x] 大小写敏感性

### ✅ 重定向功能 (redirect_test.ts)
- [x] 302 临时重定向
- [x] 301 永久重定向
- [x] 条件重定向（基于 cookies）
- [x] 重定向链处理
- [x] Location 头验证
- [x] 状态码验证

### ✅ 错误处理 (error_test.ts)
- [x] 生产模式错误隐藏
- [x] 开发模式错误详情
- [x] 堆栈跟踪显示
- [x] 模块加载错误
- [x] 语法错误处理
- [x] HTTP 方法支持
- [x] 边缘情况处理

### ✅ 自定义响应 (custom_response_test.ts)
- [x] JSON API 响应
- [x] 自定义状态码
- [x] 自定义 Headers
- [x] 不同内容类型
- [x] 空响应体 (204)

## 测试架构

### 端口分配
每个测试套件使用独立的端口以支持并行运行：
- Port 9100: basic_test.ts
- Port 9101: routing_test.ts
- Port 9102: redirect_test.ts
- Port 9103: error_test.ts (生产模式)
- Port 9104: error_test.ts (开发模式)
- Port 9105: custom_response_test.ts

### 测试生命周期
每个测试遵循以下模式：
1. 启动测试服务器（独立端口）
2. 等待服务器就绪（2秒）
3. 运行测试用例
4. 清理资源（关闭服务器）
5. 删除临时文件

## 测试用例结构

```typescript
Deno.test("E2E: 测试描述", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("应该做什么", async () => {
      const response = await makeRequest("/path");
      assertEquals(response.status, 200);
    });
  } finally {
    await cleanup(child);
  }
});
```

## 常见问题

### 测试超时
如果测试经常超时，可能需要增加服务器启动等待时间：
```typescript
await new Promise((resolve) => setTimeout(resolve, 3000)); // 增加到 3 秒
```

### 端口占用
如果测试端口被占用：
1. 检查是否有其他进程占用端口
2. 修改测试文件中的端口号
3. 或运行单个测试文件而不是并行运行

### 权限错误
确保运行测试时包含所有必要的权限：
```bash
deno test --allow-net --allow-read --allow-run
```

## 持续集成

在 CI/CD 管道中运行测试：

```yaml
# GitHub Actions 示例
- name: Run tests
  run: deno task test
```

## 贡献指南

添加新测试时：
1. 在 `tests/e2e/` 创建新文件
2. 使用独特的端口（9106+）
3. 遵循现有的测试结构
4. 更新 deno.json 添加测试任务
5. 更新 README.md 记录新测试

## 性能考虑

- 每个测试启动独立的服务器进程
- 测试顺序运行以确保端口不冲突
- 使用 `try...finally` 确保资源清理
- 临时测试页面在测试后删除

## 调试技巧

### 查看详细输出
```bash
deno test --allow-net --allow-read --allow-run tests/e2e/basic_test.ts -D
```

### 运行特定测试
```bash
deno test --allow-net --allow-read --allow-run tests/e2e/basic_test.ts --filter "首页"
```

### 保留测试服务器
修改测试代码注释掉 `cleanup(child)` 可以保留服务器运行进行手动测试。
