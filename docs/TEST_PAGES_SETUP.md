# 测试页面独立化

## 更新内容

### 1. 创建独立测试目录

创建了 `tests/test_www/` 目录，包含专门的测试页面：

```
tests/test_www/
├── index.tsx       # 首页测试
├── form.tsx        # 表单/POST 测试
├── api.tsx         # API/上下文测试
├── redirect.tsx    # 重定向测试
└── README.md       # 说明文档
```

### 2. 简化页面内容

**特点：**
- ✅ 无组件依赖，避免编译后的导入问题
- ✅ 专注于测试特定功能
- ✅ 加载快，响应快
- ✅ 独立于 www 目录

### 3. 更新测试配置

**修改：** tests/binary_build_test.ts

```typescript
// 使用独立测试目录
const TEST_ROOT = "./tests/test_www";

// 测试内容
await testHttpRequest(`http://localhost:${TEST_PORT}/`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/index.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/form.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/api.tsx`, 200);
await testHttpRequest(`http://localhost:${TEST_PORT}/redirect.tsx`, 302);
await testHttpRequest(`http://localhost:${TEST_PORT}/nonexistent.tsx`, 404);
```

## 测试页面说明

### index.tsx - 首页测试

**测试内容：**
- 基本页面渲染
- 请求上下文访问
- URL 参数处理

**URL 示例：**
- `/` → 显示欢迎信息
- `/?name=Test` → 显示 "Hello Test"

### form.tsx - 表单测试

**测试内容：**
- GET 请求：显示表单
- POST 请求：处理表单提交
- Body 解析验证

**测试方法：**
```bash
curl -X POST http://localhost:9100/form.tsx \
  -H "Content-Type: application/json" \
  -d '{"username":"TestUser"}'
```

### api.tsx - API 信息测试

**测试内容：**
- 请求信息显示
- Headers 访问
- 上下文数据结构

**验证内容：**
- method 正确传递
- url 对象可访问
- headers 可读取

### redirect.tsx - 重定向测试

**测试内容：**
- 返回重定向对象
- 验证 302 状态码
- Location 头设置

## 优点

### 1. 独立性
- 测试不依赖 www 目录
- 测试失败不影响示例页面
- 可以随意修改测试页面

### 2. 简洁性
- 页面内容最小化
- 专注于测试目标
- 易于理解和维护

### 3. 可靠性
- 无组件导入问题
- 无外部依赖
- 编译后稳定运行

### 4. 清晰性
- 每个页面测试一个功能
- 测试意图明确
- 失败时容易定位

## 与 www 目录的对比

| 特性 | tests/test_www | www |
|------|-----------------|-----|
| 目的 | 测试 | 示例 |
| 复杂度 | 简单 | 完整 |
| 组件 | 无 | 有（Layout, Header, Footer） |
| 依赖 | 最小化 | Preact + 组件 |
| 页面数 | 4个 | 6个 |
| 维护者 | 测试开发者 | 项目维护者 |

## 运行测试

```bash
# 二进制构建测试（使用测试目录）
deno task test:binary

# 手动测试（使用测试目录）
deno run --allow-net --allow-read src/main.ts --root ./tests/test_www

# 编译后运行
DENO_DIR=./.deno ./tspserver.exe --root ./tests/test_www
```

## 访问测试页面

假设服务器运行在 localhost:9100：

- **首页**: http://localhost:9100/
- **表单**: http://localhost:9100/form.tsx
- **API**: http://localhost:9100/api.tsx
- **重定向**: http://localhost:9100/redirect.tsx

## 清理

如果需要清理测试生成的文件：

```bash
deno task clean
```

## 后续改进

可以添加更多测试页面：

- `json.tsx` - JSON 响应测试
- `cookies.tsx` - Cookie 读写测试
- `error.tsx` - 错误处理测试
- `headers.tsx` - 自定义响应头测试

## 更新时间

2026-01-27
