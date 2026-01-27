# TSP-FPM 测试文档

## 测试说明

此目录包含 TSP-FPM 项目的测试用例。

## 运行测试

### 运行所有测试

```bash
deno test --allow-all
```

### 运行特定测试

```bash
# 二进制构建测试
deno test --allow-all tests/binary_build_test.ts

# 直接运行测试（不使用 deno test）
deno run --allow-all tests/binary_build_test.ts
```

## 测试用例

### binary_build_test.ts

测试二进制文件的编译和运行。

**测试内容：**
- 编译二进制文件
- 验证编译产物
- 启动服务器进程
- 测试各种 HTTP 请求（GET、重定向、404）
- 清理测试资源

**要求权限：**
- `--allow-net`: 网络请求（测试 HTTP 功能）
- `--allow-read`: 读取源代码和文件
- `--allow-write`: 写入编译后的二进制文件
- `--allow-run`: 运行 deno compile 命令

**注意事项：**
- 测试会生成 `tsp-fpm-test` (或 `tsp-fpm-test.exe`) 二进制文件
- 使用端口 `9100` 避免与开发服务器冲突
- 测试完成后会保留二进制文件用于手动测试
- Windows 下需要设置 `DENO_DIR` 环境变量

## 测试端口分配

| 测试 | 端口 | 说明 |
|------|------|------|
| binary_build_test | 9100 | 二进制构建测试 |
| 开发服务器 | 9000 | 默认开发端口 |

## 添加新测试

创建新的测试文件：

```typescript
#!/usr/bin/env -S deno run --allow-net --allow-read

import { assertEquals } from "https://deno.land/std@0.210.0/testing/asserts.ts";

Deno.test("my_test", async () => {
  // 测试代码
  assertEquals(1 + 1, 2);
});
```

运行测试：

```bash
deno test --allow-all tests/my_test.ts
```

## 持续集成

这些测试可以集成到 CI/CD 流程中：

```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3
      - uses: denoland/setup-deno@v1

      - name: Run tests
        run: deno test --allow-all
```

## 故障排查

### 编译失败

**问题：** 编译二进制文件时出错

**解决：**
```bash
# 手动测试编译
deno compile --allow-net --allow-read --allow-env --output test-binary src/main.ts

# 检查编译错误
deno check src/main.ts
```

### 端口冲突

**问题：** 端口已被占用

**解决：**
```bash
# Linux/macOS
lsof -ti:9100 | xargs kill -9

# Windows
netstat -ano | findstr :9100
taskkill /PID <PID> /F
```

### 权限错误

**问题：** 测试因权限不足而失败

**解决：**
```bash
# 确保使用 --allow-all
deno test --allow-all tests/binary_build_test.ts
```

## 性能基准

二进制文件性能参考：

| 操作 | 耗时 |
|------|------|
| 编译时间 | ~10-20 秒 |
| 启动时间 | ~100-200ms |
| 首次请求 | ~20-50ms |
| 缓存后请求 | ~1-5ms |

## 许可证

MIT License
