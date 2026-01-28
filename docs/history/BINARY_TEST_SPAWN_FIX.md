# 二进制测试启动问题修复

## 问题描述

编译成功后，启动二进制文件时出现错误：

```
NotFound: Failed to spawn 'tsp-fpm-test.exe': entity not found
```

## 根本原因

### 1. Windows 下编译输出文件名

**问题：**
```typescript
// 编译命令
"--output", OUTPUT_BINARY,  // "tsp-fpm-test"
```

在 Windows 下：
- 如果不指定 `.exe` 后缀，Deno 可能生成不同的文件名
- 或者生成的文件与期望的不匹配

### 2. 相对路径在 Deno.Command 中的问题

**问题：**
```typescript
new Deno.Command("tsp-fpm-test.exe", { ... })  // 可能找不到
```

在 Windows 下可能需要：
```typescript
new Deno.Command("./tsp-fpm-test.exe", { ... })  // 添加 ./
```

## 修复方案

### 1. 显式指定 .exe 后缀

**修复前：**
```typescript
const command = new Deno.Command("deno", {
  args: [
    "compile",
    "--allow-net",
    "--allow-read",
    "--allow-env",
    "--output", OUTPUT_BINARY,  // "tsp-fpm-test"
    "src/main.ts",
  ],
  // ...
});
```

**修复后：**
```typescript
// 在 Windows 下显式添加 .exe 后缀
const outputFile = Deno.build.os === "windows"
  ? `${OUTPUT_BINARY}.exe`
  : OUTPUT_BINARY;

console.log(`✓ 编译输出: ${outputFile}`);

const command = new Deno.Command("deno", {
  args: [
    "compile",
    "--allow-net",
    "--allow-read",
    "--allow-env",
    "--output", outputFile,  // "tsp-fpm-test.exe" (Windows)
    "src/main.ts",
  ],
  // ...
});
```

### 2. 使用正确的命令路径

**修复前：**
```typescript
const binaryPath = getBinaryPath();  // "tsp-fpm-test.exe"

new Deno.Command(binaryPath, { ... })  // 可能失败
```

**修复后：**
```typescript
const binaryPath = getBinaryPath();  // "tsp-fpm-test.exe"

// 验证文件存在
const stat = await Deno.stat(binaryPath);
console.log(`✓ 找到二进制文件: ${binaryPath}`);

// Windows 下使用相对路径 ./ 前缀
const commandPath = Deno.build.os === "windows" && !binaryPath.startsWith("./")
  ? `./${binaryPath}`
  : binaryPath;

console.log(`✓ 启动命令: ${commandPath}`);

new Deno.Command(commandPath, { ... })  // 正确
```

### 3. 添加调试信息

**增强的错误处理：**
```typescript
try {
  const stat = await Deno.stat(binaryPath);
  console.log(`✓ 二进制文件大小: ${(stat.size / 1024 / 1024).toFixed(2)} MB`);
} catch (error) {
  // 列出当前目录的文件用于调试
  console.error(`❌ 文件不存在: ${binaryPath}`);
  console.error("当前目录相关文件:");
  for await (const entry of Deno.readDir(".")) {
    if (entry.name.includes("tsp-fpm") || entry.name.includes(".exe")) {
      console.error(`  - ${entry.name}`);
    }
  }
  throw error;
}
```

## Deno.Command 的路径处理

### Windows 平台

| 路径格式 | 说明 | 是否推荐 |
|---------|------|----------|
| `program.exe` | 相对路径（无 ./） | ❌ 可能失败 |
| `./program.exe` | 相对路径（有 ./） | ✅ 推荐 |
| `C:\path\to\program.exe` | 绝对路径 | ✅ 推荐 |
| `.\program.exe` | 相对路径（Windows 风格） | ✅ 可用 |

### Unix/Linux/macOS 平台

| 路径格式 | 说明 | 是否推荐 |
|---------|------|----------|
| `program` | 相对路径 | ✅ 推荐 |
| `./program` | 相对路径（显式） | ✅ 推荐 |
| `/path/to/program` | 绝对路径 | ✅ 推荐 |

## 完整的修复流程

### 编译阶段

```typescript
async function compileBinary(): Promise<void> {
  // 1. 确定输出文件名（带正确的后缀）
  const outputFile = Deno.build.os === "windows"
    ? `${OUTPUT_BINARY}.exe`
    : OUTPUT_BINARY;

  console.log(`✓ 编译输出: ${outputFile}`);

  // 2. 执行编译命令
  const command = new Deno.Command("deno", {
    args: [
      "compile",
      "--allow-net",
      "--allow-read",
      "--allow-env",
      "--output", outputFile,
      "src/main.ts",
    ],
    stdout: "piped",
    stderr: "piped",
  });

  const { code, stderr } = await command.output();

  if (code !== 0) {
    throw new Error(`编译失败: ${new TextDecoder().decode(stderr)}`);
  }

  // 3. 验证编译产物
  const binaryPath = getBinaryPath();
  const stat = await Deno.stat(binaryPath);
  console.log(`✓ 文件大小: ${(stat.size / 1024 / 1024).toFixed(2)} MB`);
}
```

### 运行阶段

```typescript
async function startServer(): Promise<Deno.ChildProcess> {
  const binaryPath = getBinaryPath();

  // 1. 验证文件存在
  const stat = await Deno.stat(binaryPath);
  console.log(`✓ 找到二进制文件: ${binaryPath}`);

  // 2. 构造正确的命令路径
  let commandPath = binaryPath;
  if (Deno.build.os === "windows" && !commandPath.startsWith("./")) {
    commandPath = `./${commandPath}`;
  }

  console.log(`✓ 启动命令: ${commandPath}`);

  // 3. 设置环境变量（编译后的程序需要 DENO_DIR）
  const env = { ...Deno.env.toObject(), DENO_DIR: "./.deno" };

  // 4. 启动进程
  const process = new Deno.Command(commandPath, {
    args: ["--root", TEST_ROOT, "--port", TEST_PORT.toString(), "--dev"],
    env,
    stdout: "piped",
    stderr: "piped",
  }).spawn();

  console.log(`✓ 服务器进程已启动 (PID: ${process.pid})`);

  // 5. 等待启动
  await new Promise((resolve) => setTimeout(resolve, 2000));

  return process;
}
```

## 测试验证

### 修复前

```
✓ 二进制文件编译成功
╔════════════════════════════════════════════╗
║   ✗ 测试失败！                            ║
╚════════════════════════════════════════════╝

NotFound: Failed to spawn 'tsp-fpm-test.exe': entity not found
```

### 修复后（预期）

```
✓ 编译输出: tsp-fpm-test.exe
✓ 二进制文件编译成功
✓ 找到二进制文件: tsp-fpm-test.exe
✓ 二进制文件大小: 85.23 MB

=== 启动测试服务器 ===
✓ 启动命令: ./tsp-fpm-test.exe
✓ 服务器进程已启动 (PID: 12345)
⏳ 等待 2 秒让服务器完全启动...

✓ 所有测试通过！
```

## 关键要点

1. **显式指定 .exe 后缀**
   - Windows 下编译时必须指定 `.exe`
   - 不要依赖 Deno 自动添加

2. **使用 ./ 前缀**
   - Windows 下相对路径需要 `./`
   - 确保 Deno.Command 能找到文件

3. **验证文件存在**
   - 启动前先 `stat()` 确认文件存在
   - 失败时列出调试信息

4. **设置 DENO_DIR**
   - 编译后的程序需要 `DENO_DIR` 环境变量
   - 用于 @deno/loader 的缓存

## 相关修改

**文件：** tests/binary_build_test.ts

| 行号 | 修改内容 |
|------|---------|
| 64-68 | 添加 Windows 下显式 `.exe` 后缀 |
| 70 | 添加编译输出日志 |
| 85-106 | 添加文件验证和调试信息 |
| 109-120 | 修改命令路径处理（添加 ./） |
| 122 | 添加启动命令日志 |

## 运行测试

```bash
# 快速类型检查
deno check tests/binary_build_test.ts

# 完整测试（包含编译，需要 10-30 秒）
deno task test:binary

# 手动测试编译后的程序
DENO_DIR=./.deno ./tsp-fpm-test.exe --root ./www --port 9000
```

## 其他注意事项

### 清理旧文件

确保测试前清理所有可能的文件：

```typescript
async function cleanupBinary(): Promise<void> {
  const filesToRemove = [
    OUTPUT_BINARY,           // "tsp-fpm-test"
    `${OUTPUT_BINARY}.exe`,  // "tsp-fpm-test.exe" (Windows)
  ];

  for (const file of filesToRemove) {
    try {
      await Deno.remove(file);
    } catch (error) {
      if (!(error instanceof Deno.errors.NotFound)) {
        throw error;
      }
    }
  }
}
```

### 跨平台路径辅助函数

```typescript
function getBinaryPath(): string {
  return Deno.build.os === "windows"
    ? `${OUTPUT_BINARY}.exe`
    : OUTPUT_BINARY;
}

function getCommandPath(binaryPath: string): string {
  if (Deno.build.os === "windows" && !binaryPath.startsWith("./")) {
    return `./${binaryPath}`;
  }
  return binaryPath;
}
```

## 更新时间

2026-01-27
