# Deno 任务配置更新总结

## 更新时间
2026-01-27

## 更新内容

### 1. 清理的测试任务（已删除）

以下任务已移除，因为 `tests/e2e/` 目录已被删除：

- `test:e2e` - E2E 测试
- `test:basic` (旧) - E2E 基本测试
- `test:routing` - E2E 路由测试
- `test:redirect` - E2E 重定向测试
- `test:error` - E2E 错误测试
- `test:custom` - E2E 自定义响应测试
- `test:compiled` - E2E 编译测试

### 2. 新增的任务

#### 开发相关
- ✅ `dev` - 开发模式（热重载 + --allow-env）
- ✅ `start` - 生产模式（+ --allow-env）
- ✅ `compile` - 编译为二进制文件

#### 测试相关
- ✅ `test` - 运行所有测试（使用测试运行器）
- ✅ `test:basic` - 基本功能测试
- ✅ `test:binary` - 二进制构建测试
- ✅ `test:all` - 运行所有测试（deno test）

#### 代码质量
- ✅ `check` - 类型检查
- ✅ `fmt` - 格式化代码
- ✅ `fmt:check` - 检查代码格式
- ✅ `lint` - 代码质量检查
- ✅ `clean` - 清理生成的文件

### 3. 更新的任务

| 任务 | 旧版本 | 新版本 | 变化 |
|------|--------|--------|------|
| dev | 缺少 --allow-env | 添加 --allow-env | 支持编译后的动态加载 |
| start | 缺少 --allow-env | 添加 --allow-env | 支持编译后的动态加载 |
| test | 测试 e2e 目录 | 测试新测试文件 | 更新测试路径 |

### 4. Import Map 更新

添加了 `std/path` 别名以兼容现有代码：

```json
{
  "imports": {
    "@std/path": "jsr:@std/path@1.0.0",
    "std/path": "jsr:@std/path@1.0.0"  // 新增
  }
}
```

### 5. 修正的问题

#### 问题 1: 测试路径不存在
**错误：** 测试任务指向已删除的 `tests/e2e/` 目录

**解决：** 更新为新的测试文件路径
- `tests/basic_test.ts`
- `tests/binary_build_test.ts`

#### 问题 2: 缺少环境权限
**错误：** 编译后的二进制需要 `--allow-env` 权限

**解决：** 在 `dev` 和 `start` 任务中添加 `--allow-env`

#### 问题 3: 类型检查失败
**错误：** `std/path` 导入不在 import map 中

**解决：** 在 import map 中添加 `std/path` 别名

## 任务列表对比

### 之前（旧版本）

```json
{
  "dev": "deno run --watch --allow-net --allow-read src/main.ts",
  "start": "deno run --allow-net --allow-read src/main.ts",
  "test": "deno test --allow-net --allow-read --allow-run --allow-write tests/e2e/",
  "test:basic": "deno test --allow-net --allow-read --allow-run --allow-write tests/e2e/basic_test.ts",
  "test:routing": "deno test ... tests/e2e/routing_test.ts",
  "test:redirect": "deno test ... tests/e2e/redirect_test.ts",
  "test:error": "deno test ... tests/e2e/error_test.ts",
  "test:custom": "deno test ... tests/e2e/custom_response_test.ts",
  "test:compiled": "deno test ... tests/e2e/compiled_binary_test.ts"
}
```

### 现在（新版本）

```json
{
  "dev": "deno run --watch --allow-net --allow-read --allow-env src/main.ts --dev",
  "start": "deno run --allow-net --allow-read --allow-env src/main.ts",
  "compile": "deno compile --allow-net --allow-read --allow-env --output tsp-fpm src/main.ts",

  "test": "deno run --allow-all tests/run_all_tests.ts",
  "test:basic": "deno test --allow-net --allow-read tests/basic_test.ts",
  "test:binary": "deno test --allow-all tests/binary_build_test.ts",
  "test:all": "deno test --allow-all tests/",

  "check": "deno check src/main.ts src/cache.ts src/context.ts src/router.ts",
  "fmt": "deno fmt src/ www/ tests/",
  "fmt:check": "deno fmt --check src/ www/ tests/",
  "lint": "deno lint src/ www/ tests/",
  "clean": "rm -rf tsp-fpm tsp-fpm.exe tests/tsp-fpm-test tests/tsp-fpm-test.exe"
}
```

## 改进点

### 1. 完整性
- ✅ 添加了编译任务
- ✅ 添加了代码质量任务
- ✅ 添加了清理任务

### 2. 一致性
- ✅ 所有任务都使用正确的权限
- ✅ 测试路径统一
- ✅ 命名规范一致

### 3. 可用性
- ✅ 添加了 `--allow-env` 支持编译后的程序
- ✅ 测试文件实际存在
- ✅ 包含详细的文档（TASKS.md）

### 4. 维护性
- ✅ 类型检查通过
- ✅ 导入路径统一
- ✅ 任务说明完整

## 验证结果

```bash
$ deno task check
✓ 类型检查通过

$ deno task
✓ 所有任务正常显示
```

## 使用示例

### 日常开发
```bash
deno task dev
```

### 运行测试
```bash
deno task test
```

### 发布流程
```bash
deno task check    # 检查
deno task test     # 测试
deno task compile  # 编译
```

## 相关文档

- [TASKS.md](../TASKS.md) - 详细任务使用说明
- [tests/README.md](../tests/README.md) - 测试文档
- [deno.json](../deno.json) - 配置文件

## 注意事项

1. **环境权限**
   - 编译后的程序需要 `DENO_DIR` 环境变量
   - 所有运行任务都包含 `--allow-env`

2. **测试端口**
   - 开发服务器: 9000
   - 测试服务器: 9001

3. **清理任务**
   - Windows 用户需要使用 Git Bash 或 WSL
   - 或者手动删除二进制文件

## 后续建议

1. ✅ 添加 CI/CD 集成
2. ✅ 添加性能基准测试
3. ✅ 添加代码覆盖率报告
4. ✅ 添加自动化发布流程
