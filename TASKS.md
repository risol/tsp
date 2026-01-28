# Deno 任务说明

## 可用任务列表

### 开发相关

```bash
# 开发模式（热重载）
deno task dev

# 生产模式启动
deno task start

# 编译为二进制文件
deno task compile
```

### 测试相关

```bash
# 运行所有测试（使用测试运行器）
deno task test

# 运行基本功能测试
deno task test:basic

# 运行二进制构建测试
deno task test:binary

# 运行所有测试（使用 deno test）
deno task test:all
```

### 代码质量

```bash
# 类型检查
deno task check

# 格式化代码
deno task fmt

# 检查代码格式
deno task fmt:check

# 代码检查
deno task lint

# 清理生成的文件
deno task clean
```

## 详细说明

### dev
启动开发服务器，支持热重载。

```bash
deno task dev
# 等同于
deno run --watch --allow-net --allow-read --allow-env src/main.ts --dev
```

**特性：**
- 文件修改后自动重启
- 显示详细错误信息
- 端口：9000（默认）
- 根目录：./www（默认）

### start
启动生产服务器。

```bash
deno task start
# 等同于
deno run --allow-net --allow-read --allow-env src/main.ts
```

**特性：**
- 不支持热重载
- 隐藏详细错误信息
- 适合生产环境

### compile
编译为独立的二进制可执行文件。

```bash
deno task compile
# 等同于
deno compile --allow-net --allow-read --allow-env --output tspserver src/main.ts
```

**输出：**
- Windows: `tspserver.exe`
- Linux/macOS: `tspserver`

**运行编译后的程序：**
```bash
# 需要设置 DENO_DIR 环境变量
DENO_DIR=./.deno ./tspserver --root ./www --port 9000
```

### test
使用测试运行器运行所有测试。

```bash
deno task test
# 等同于
deno run --allow-all tests/run_all_tests.ts
```

**测试内容：**
- 基本功能测试（路由、上下文、安全）
- 二进制构建测试（编译、运行、HTTP）

**测试输出：**
- 详细的测试进度
- 测试总结报告
- 失败测试的详细信息

### test:basic
运行基本功能测试。

```bash
deno task test:basic
# 等同于
deno test --allow-net --allow-read tests/basic_test.ts
```

**测试内容：**
- 路由解析
- 上下文构建
- 安全检查

**特点：**
- 快速（无需编译）
- 适合开发时快速验证

### test:binary
运行二进制构建测试。

```bash
deno task test:binary
# 等同于
deno test --allow-all tests/binary_build_test.ts
```

**测试内容：**
- 编译二进制文件
- 启动服务器进程
- HTTP 功能测试

**特点：**
- 完整的端到端测试
- 验证编译后的程序是否正常工作
- 使用独立端口（9100）避免冲突

### test:all
使用 `deno test` 运行所有测试。

```bash
deno task test:all
# 等同于
deno test --allow-all tests/
```

### check
类型检查所有源代码。

```bash
deno task check
# 等同于
deno check src/**/*.ts src/**/*.tsx www/**/*.tsx
```

**检查内容：**
- TypeScript 类型错误
- 导入路径错误
- 语法错误

### fmt
格式化代码。

```bash
deno task fmt
# 等同于
deno fmt src/ www/ tests/
```

**格式化目录：**
- src/ - 源代码
- www/ - 页面文件
- tests/ - 测试文件

### fmt:check
检查代码格式（不修改文件）。

```bash
deno task fmt:check
# 等同于
deno fmt --check src/ www/ tests/
```

**用途：**
- CI/CD 检查
- 确保代码格式一致

### lint
代码质量检查。

```bash
deno task lint
# 等同于
deno lint src/ www/ tests/
```

**检查内容：**
- 代码风格
- 潜在的错误
- 最佳实践

### clean
清理生成的文件。

```bash
deno task clean
# 等同于
rm -rf tspserver tspserver.exe tests/tspserver-test tests/tspserver-test.exe
```

**删除内容：**
- tspserver / tspserver.exe - 编译的主程序
- tests/tspserver-test - 测试生成的二进制文件

## 使用示例

### 开发工作流

```bash
# 1. 启动开发服务器
deno task dev

# 2. 修改代码后自动重启

# 3. 运行基本测试
deno task test:basic

# 4. 提交前检查
deno task fmt:check
deno task lint
deno task check
```

### 发布流程

```bash
# 1. 运行所有测试
deno task test

# 2. 编译二进制
deno task compile

# 3. 测试编译后的程序
DENO_DIR=./.deno ./tspserver --root ./www --port 9000

# 4. 清理
deno task clean
```

### 故障排查

```bash
# 检查类型错误
deno task check

# 检查代码格式
deno task fmt:check

# 运行特定测试
deno task test:basic
deno task test:binary

# 查看详细日志
deno task dev
```

## 常见问题

### Q: dev 和 start 的区别？

**A:**
- `dev`: 开发模式，支持热重载，显示详细错误
- `start`: 生产模式，不支持热重载，隐藏详细错误

### Q: test 和 test:all 的区别？

**A:**
- `test`: 使用测试运行器，生成详细报告
- `test:all`: 使用 deno test，简单直接

### Q: 编译后的程序如何运行？

**A:**
```bash
# 需要设置 DENO_DIR 环境变量
DENO_DIR=./.deno ./tspserver --root ./www --port 9000
```

### Q: 如何添加新的任务？

**A:** 编辑 `deno.json` 的 `tasks` 字段：

```json
{
  "tasks": {
    "my-task": "deno run --allow-net src/my-script.ts"
  }
}
```

然后运行：
```bash
deno task my-task
```

## 注意事项

1. **权限要求**
   - 开发服务器需要 `--allow-net --allow-read --allow-env`
   - 测试需要 `--allow-all`
   - 编译需要包含所有用到的权限

2. **端口冲突**
   - 开发服务器默认使用 9000
   - 测试使用 9100 避免冲突
   - 可以通过 `--port` 参数修改

3. **环境变量**
   - 编译后的程序需要设置 `DENO_DIR`
   - Windows: `set DENO_DIR=./.deno`
   - Linux/Mac: `DENO_DIR=./.deno ./tspserver`

4. **性能**
   - `dev` 模式有文件监听开销
   - `start` 模式性能更好
   - 编译后的程序启动最快
