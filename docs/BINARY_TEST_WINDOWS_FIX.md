# 二进制测试 Windows 路径修复

## 问题描述

在 Windows 系统上运行 `deno task test:binary` 时出现错误：

```
NotFound: 系统找不到指定的文件。 (os error 2): stat 'tsp-fpm-test'
```

## 根本原因

1. **Windows 下 Deno Compile 的行为**
   - `deno compile` 在 Windows 下会自动添加 `.exe` 后缀
   - 输出文件名为 `tsp-fpm-test.exe` 而不是 `tsp-fpm-test`

2. **清理函数不完整**
   - `cleanupBinary()` 只删除不带后缀的文件
   - Windows 下遗留 `.exe` 文件

3. **编译后的文件验证错误**
   - 代码尝试 `stat('tsp-fpm-test')`
   - 实际文件是 `tsp-fpm-test.exe`

## 修复方案

### 1. 修正清理函数

**修复前：**
```typescript
async function cleanupBinary(): Promise<void> {
  try {
    await Deno.remove(OUTPUT_BINARY);
    console.log("✓ 清理旧的二进制文件");
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      // 文件不存在，忽略
    } else {
      throw error;
    }
  }
}
```

**修复后：**
```typescript
async function cleanupBinary(): Promise<void> {
  const filesToRemove = [OUTPUT_BINARY];
  if (Deno.build.os === "windows") {
    filesToRemove.push(`${OUTPUT_BINARY}.exe`);
  }

  for (const file of filesToRemove) {
    try {
      await Deno.remove(file);
      console.log(`✓ 清理旧文件: ${file}`);
    } catch (error) {
      if (error instanceof Deno.errors.NotFound) {
        // 文件不存在，忽略
      } else {
        throw error;
      }
    }
  }
}
```

### 2. 修正文件验证

**修复前：**
```typescript
// 验证文件存在
const stat = await Deno.stat(OUTPUT_BINARY);  // ❌ Windows 失败
console.log(`✓ 二进制文件大小: ${(stat.size / 1024 / 1024).toFixed(2)} MB`);

// Windows 下添加 .exe 后缀
if (Deno.build.os === "windows") {
  const oldPath = OUTPUT_BINARY;
  const newPath = `${OUTPUT_BINARY}.exe`;
  await Deno.rename(oldPath, newPath);  // ❌ 文件不存在
  console.log("✓ Windows 系统，重命名为 .exe");
}
```

**修复后：**
```typescript
// 验证文件存在（Windows 下自动添加 .exe）
const binaryPath = getBinaryPath();
const stat = await Deno.stat(binaryPath);  // ✅ 正确的文件名
console.log(`✓ 二进制文件大小: ${(stat.size / 1024 / 1024).toFixed(2)} MB`);
```

### 3. 函数定义顺序

将 `getBinaryPath()` 函数移到 `cleanupBinary()` 和 `compileBinary()` 之前：

```typescript
// 1. 辅助函数（最前）
function getBinaryPath(): string {
  return Deno.build.os === "windows"
    ? `${OUTPUT_BINARY}.exe`
    : OUTPUT_BINARY;
}

// 2. 清理函数（使用辅助函数）
async function cleanupBinary(): Promise<void> {
  // ...
}

// 3. 编译函数（使用辅助函数）
async function compileBinary(): Promise<void> {
  const binaryPath = getBinaryPath();
  // ...
}
```

## Deno Compile 的平台差异

### Linux / macOS

```bash
$ deno compile --output app src/main.ts
# 生成文件: app
# 文件类型: ELF executable / Mach-O binary
```

### Windows

```bash
$ deno compile --output app src/main.ts
# 生成文件: app.exe (自动添加 .exe)
# 文件类型: PE executable
```

## 跨平台处理模式

### 模式 1: 编译时指定完整文件名

```typescript
const OUTPUT = Deno.build.os === "windows"
  ? "tsp-fpm.exe"
  : "tsp-fpm";

await Deno.compile({
  output: OUTPUT,
  // ...
});
```

### 模式 2: 编译后处理（当前方案）

```typescript
// 编译时使用不带后缀的名称
await Deno.compile({
  output: "tsp-fpm",
  // ...
});

// 运行时根据平台添加后缀
const binaryPath = Deno.build.os === "windows"
  ? "tsp-fpm.exe"
  : "tsp-fpm";

await Deno.stat(binaryPath);
```

### 模式 3: 使用统一的路径函数

```typescript
function getBinaryPath(name: string): string {
  if (Deno.build.os === "windows") {
    return name.endsWith(".exe") ? name : `${name}.exe`;
  }
  return name;
}

// 使用
const output = getBinaryPath("tsp-fpm");
await Deno.compile({ output });
await Deno.stat(getBinaryPath("tsp-fpm"));
```

## 测试验证

### 修复前

```
✓ 二进制文件编译成功
✗ 测试失败！
NotFound: stat 'tsp-fpm-test'
```

### 修复后

```
✓ 二进制文件编译成功
✓ 二进制文件大小: 85.23 MB
✓ 服务器进程已启动
✓ 所有测试通过
```

## 相关代码修改

### 文件: tests/binary_build_test.ts

**修改内容：**
1. ✅ 添加 `getBinaryPath()` 辅助函数
2. ✅ 修改 `cleanupBinary()` 支持 `.exe` 文件
3. ✅ 修改 `compileBinary()` 使用正确的文件路径
4. ✅ 删除重复的函数定义

**关键改动：**
- 第26-32行：添加 `getBinaryPath()` 函数
- 第35-57行：重写 `cleanupBinary()` 函数
- 第66-69行：修改 `compileBinary()` 中的文件验证

## 运行测试

```bash
# 类型检查
deno check tests/binary_build_test.ts

# 运行二进制构建测试
deno task test:binary

# 清理生成的文件
deno task clean
```

## 最佳实践

### 1. 总是考虑平台差异

```typescript
// ✅ 好的做法
const binary = Deno.build.os === "windows"
  ? `${name}.exe`
  : name;

// ❌ 不好的做法
const binary = `${name}.exe`;  // Unix 上会出错
```

### 2. 使用辅助函数统一处理

```typescript
// ✅ 好的做法
function getBinaryPath(name: string): string {
  return Deno.build.os === "windows"
    ? `${name}.exe`
    : name;
}
```

### 3. 测试跨平台功能

```bash
# Windows
deno task test:binary

# Linux/macOS
deno task test:binary
```

## 其他注意事项

### 清理任务

确保 `deno.json` 中的 `clean` 任务也处理 `.exe` 文件：

```json
{
  "tasks": {
    "clean": "rm -rf tsp-fpm tsp-fpm.exe tests/tsp-fpm-test tests/tsp-fpm-test.exe"
  }
}
```

### .gitignore

确保忽略所有平台的二进制文件：

```
# 二进制文件
tsp-fpm
tsp-fpm.exe
tests/tsp-fpm-test
tests/tsp-fpm-test.exe
```

## 相关资源

- [Deno Compile](https://deno.com/manual/deploying/compile)
- [Deno.build API](https://deno.com/runtime/manual/api/deno/build)
- [Cross-platform TypeScript](https://www.typescriptlang.org/docs/handbook/modules.html#the-import-meta-meta-property)

## 更新时间

2026-01-27
