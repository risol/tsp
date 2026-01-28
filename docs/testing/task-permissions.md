# Deno 任务权限说明

## 任务配置（已更新）

所有任务都已配置正确的权限，确保正常运行：

### 开发相关

| 任务 | 命令 | 权限说明 |
|------|------|----------|
| **dev** | `deno run --watch --allow-net --allow-read --allow-write --allow-env --allow-run` | 开发模式（热重载）<br>- ✅ 网络访问<br>- ✅ 文件读取<br>- ✅ 文件写入（缓存）<br>- ✅ 环境变量<br>- ✅ 子进程 |
| **start** | `deno run --allow-net --allow-read --allow-write --allow-env` | 生产模式<br>- ✅ 网络访问<br>- ✅ 文件读取<br>- ✅ 文件写入（缓存）<br>- ✅ 环境变量 |
| **compile** | `deno compile --allow-net --allow-read --allow-write --allow-env --output tspserver` | 编译二进制<br>- ✅ 网络访问<br>- ✅ 文件读取<br>- ✅ 文件写入<br>- ✅ 环境变量 |

### 测试相关

| 任务 | 命令 | 权限说明 |
|------|------|----------|
| **test** | `deno run --allow-all tests/run_all_tests.ts` | 运行全部测试<br>- ✅ 所有权限 |
| **test:unit** | `deno run --allow-all tests/run_unit_tests.ts` | 单元测试<br>- ✅ 所有权限 |
| **test:e2e** | `deno run --allow-all tests/run_e2e_tests.ts` | E2E 测试<br>- ✅ 所有权限 |
| **test:injection** | `deno test --allow-all tests/unit/injection_test.ts` | 依赖注入测试<br>- ✅ 所有权限 |

### 代码质量

| 任务 | 命令 | 权限说明 |
|------|------|----------|
| **check** | `deno check --allow-net --allow-read --allow-write --allow-env` | 类型检查<br>- ✅ 网络访问（导入检查）<br>- ✅ 文件读取<br>- ✅ 文件写入<br>- ✅ 环境变量 |
| **check:all** | `deno check --allow-all src/ www/` | 检查所有代码<br>- ✅ 所有权限 |
| **fmt** | `deno fmt --allow-write src/ www/ tests/` | 格式化代码<br>- ✅ 文件写入 |
| **fmt:check** | `deno fmt --check src/ www/ tests/` | 检查格式<br>- 只读，无需权限 |
| **lint** | `deno lint src/ www/ tests/` | 代码检查<br>- 只读，无需权限 |

### 工具

| 任务 | 命令 | 权限说明 |
|------|------|----------|
| **clean** | `rm -rf ...` | 清理临时文件<br>- 需要 shell 权限（--allow-run） |

## 权限详解

### 基础权限

- **--allow-net** - 网络访问（HTTP 服务器、模块下载）
- **--allow-read** - 文件读取（读取模板文件）
- **--allow-write** - 文件写入（缓存、编译输出）
- **allow-env** - 环境变量（配置读取）
- **--allow-run** - 子进程（编译、运行测试）

### 为 dev 和 start 添加的权限

之前：
```json
"dev": "deno run --watch --allow-net --allow-read --allow-env src/main.ts --dev"
```

现在：
```json
"dev": "deno run --watch --allow-net --allow-read --allow-write --allow-env --allow-run src/main.ts --dev"
```

**添加原因：**
- `--allow-write` - 模块缓存需要写入缓存文件
- `--allow-run` - 某些工具可能需要 spawn 子进程

## 使用示例

### 开发模式（支持热重载）
```bash
deno task dev
```

### 生产模式
```bash
deno task start
```

### 运行所有测试
```bash
deno task test
```

### 只运行单元测试
```bash
deno task test:unit
```

### 类型检查
```bash
deno task check
```

### 格式化代码
```bash
deno task fmt
```

### 清理临时文件
```bash
deno task clean
```

## 常见权限错误

### 错误：Requires read access to ...
**原因：** 缺少 `--allow-read` 权限
**解决：** 在任务中添加 `--allow-read`

### 错误：Requires write access to ...
**原因：** 缺少 `--allow-write` 权限
**解决：** 在任务中添加 `--allow-write`

### 错误：Requires net access to ...
**原因：** 缺少 `--allow-net` 权限
**解决：** 在任务中添加 `--allow-net`

### 错误：Requires run access to ...
**原因：** 缺少 `--allow-run` 权限
**解决：** 在任务中添加 `--allow-run`

## IDE 支持

如果使用 VS Code，确保安装 Deno 扩展，它会自动识别 `deno.json` 中的任务。

所有任务现在都已正确配置，可以放心使用！

## 相关文档

- [测试文档首页](./README.md) - 测试文档导航
- [开发任务](../tasks.md) - 所有 Deno 任务说明
- [测试概述](./overview.md) - 测试框架介绍
- [开发指南](../development.md) - 开发环境配置

---

[← 返回测试文档](./README.md) | [← 返回文档中心](../README.md)
