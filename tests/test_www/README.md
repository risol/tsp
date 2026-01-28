# TSP 测试页面说明

本目录包含用于测试的简化 TSX 页面。

## 页面列表

### index.tsx
首页，测试基本的页面渲染和上下文访问。

**功能：**
- 显示欢迎信息
- 显示请求方法、URL、路径
- 支持 URL 参数（?name=xxx）

### form.tsx
表单测试页面，测试 POST 请求处理。

**功能：**
- GET 请求显示表单
- POST 请求显示提交结果
- 测试 body 解析

**测试方法：**
```bash
curl -X POST http://localhost:9000/form.tsx -d "username=Test"
```

### api.tsx
API 信息页面，测试请求上下文。

**功能：**
- 显示请求方法、URL
- 显示 User-Agent
- 测试 headers 访问

### redirect.tsx
重定向测试页面。

**功能：**
- 总是重定向到首页
- 返回 302 状态码

## 运行测试

```bash
# 使用测试目录运行服务器
deno run --allow-net --allow-read src/main.ts --root ./tests/test_www

# 或编译后运行
DENO_DIR=./.deno ./tspserver.exe --root ./tests/test_www
```

## 页面特点

1. **简化内容** - 只包含测试所需的最小功能
2. **独立运行** - 不依赖 www 目录
3. **快速测试** - 加载快，响应快
4. **明确用途** - 每个页面专注于测试特定功能

## 与 www 目录的区别

| 特性 | tests/test_www | www |
|------|-----------------|-----|
| 组件导入 | 无 | 有（Layout, Header, Footer） |
| 复杂度 | 简单 | 完整 |
| 用途 | 测试 | 示例 |
| 依赖 | 最小化 | Preact 组件 |
