#!/usr/bin/env -S deno run --allow-all

/**
 * Build Tool
 * 编译并打包 TSP 服务器到 dist 目录
 */

import { join } from "std/path";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/ensure_dir.ts";
import { copy } from "https://deno.land/std@0.224.0/fs/copy.ts";
import { exists } from "https://deno.land/std@0.224.0/fs/exists.ts";

const DIST_DIR = "dist";
const BINARY_NAME = "tspserver";
const CONFIG_FILES = ["config.json", "config.jsonc"];
const EXAMPLE_CONFIG = "config.example.jsonc";

/**
 * 获取平台相关的二进制文件名
 */
function getBinaryName(): string {
  return Deno.build.os === "windows" ? `${BINARY_NAME}.exe` : BINARY_NAME;
}

/**
 * 清理 dist 目录
 */
async function cleanDist(): Promise<void> {
  try {
    await Deno.remove(DIST_DIR, { recursive: true });
    console.log(`✓ 清理旧版本: ${DIST_DIR}/`);
  } catch (error) {
    if (!(error instanceof Deno.errors.NotFound)) {
      throw error;
    }
  }
}

/**
 * 创建 dist 目录结构
 */
async function createDistStructure(): Promise<void> {
  await ensureDir(join(DIST_DIR, "www"));
  await ensureDir(join(DIST_DIR, ".deno"));
  console.log("✓ 创建目录结构");
}

/**
 * 编译二进制文件
 */
async function compileBinary(): Promise<void> {
  const binaryName = getBinaryName();
  const outputPath = join(DIST_DIR, binaryName);

  console.log(`🔨 编译二进制文件: ${outputPath}`);

  const command = new Deno.Command("deno", {
    args: [
      "compile",
      "--allow-net",
      "--allow-read",
      "--allow-write",
      "--allow-sys",
      "--allow-env",
      "--output",
      outputPath,
      "src/main.ts",
    ],
    stdout: "inherit",
    stderr: "inherit",
  });

  const { code } = await command.output();

  if (code !== 0) {
    throw new Error("编译失败");
  }

  console.log(`✓ 二进制文件编译成功`);
}

/**
 * 复制 www 目录
 */
async function copyWwwDir(): Promise<void> {
  console.log("📁 复制 www 目录...");

  const sourceDir = "www";
  const targetDir = join(DIST_DIR, "www");

  await copy(sourceDir, targetDir, { overwrite: true });
  console.log("✓ www 目录已复制");
}

/**
 * 复制配置文件（如果存在）
 */
async function copyConfigFiles(): Promise<void> {
  let copied = false;

  // 优先复制实际配置文件
  for (const filename of CONFIG_FILES) {
    const sourcePath = filename;
    const targetPath = join(DIST_DIR, filename);

    if (await exists(sourcePath)) {
      await Deno.copyFile(sourcePath, targetPath);
      console.log(`✓ 配置文件已复制: ${filename}`);
      copied = true;
      break; // 找到一个就停止
    }
  }

  // 如果没有实际配置文件，复制示例配置文件
  if (!copied && await exists(EXAMPLE_CONFIG)) {
    const targetPath = join(DIST_DIR, "config.jsonc");
    await Deno.copyFile(EXAMPLE_CONFIG, targetPath);
    console.log(`✓ 示例配置文件已复制: config.jsonc`);
    copied = true;
  }

  if (!copied) {
    console.log("ℹ  未找到配置文件（跳过）");
  }
}

/**
 * 创建 README
 */
async function createReadme(): Promise<void> {
  const binaryName = getBinaryName();
  const content = `
# TSP Server - Distribution Package

## 快速开始

### 1. 运行服务器

\`\`\`bash
# Windows
DENO_DIR=./.deno .\\${binaryName}

# Linux/Mac
DENO_DIR=./.deno ./${binaryName}
\`\`\`

### 2. 配置服务器

创建 \`config.json\` 或 \`config.jsonc\`:

\`\`\`json
{
  "root": "./www",
  "port": 9000,
  "dev": false
}
\`\`\`

### 3. 命令行参数

\`\`\`bash
# 指定端口
./${binaryName} --port 8080

# 指定文档根目录
./${binaryName} --root ./www

# 开发模式
./${binaryName} --dev

# 查看帮助
./${binaryName} --help
\`\`\`

## 目录结构

\`\`\`
dist/
├── ${binaryName}       # 服务器二进制文件
├── www/                # 网站文件
│   ├── index.tsx
│   ├── form.tsx
│   └── ...
├── config.json         # 可选配置文件
└── README.md           # 本文件
\`\`\`

## 环境要求

- 需要 Deno 运行时环境
- 运行时需要设置 \`DENO_DIR\` 环境变量

## 更多信息

访问项目主页获取更多文档和更新。
`.trim();

  await Deno.writeTextFile(join(DIST_DIR, "README.md"), content + "\n");
  console.log("✓ 创建 README.md");
}

/**
 * 显示构建摘要
 */
function showSummary(): void {
  const binaryName = getBinaryName();

  console.log(`
╔════════════════════════════════════════╗
║         构建完成                        ║
╚════════════════════════════════════════╝

📦 构建产物: ${DIST_DIR}/
├── ${binaryName}          # 服务器二进制
├── www/                   # 网站文件
├── config.json            # 配置文件（如果存在）
└── README.md              # 使用说明

🚀 快速启动:
   $ cd ${DIST_DIR}
   $ DENO_DIR=./.deno ./${binaryName}

💡 提示:
   - 确保 DENO_DIR 环境变量指向 ./.deno
   - 修改 config.json 配置服务器行为
   - 查看 README.md 了解更多信息
`);
}

/**
 * 主函数
 */
async function main(): Promise<void> {
  console.log(`
╔════════════════════════════════════════╗
║     TSP Server Build Tool              ║
╚════════════════════════════════════════╝

`);

  // 1. 清理旧版本
  await cleanDist();

  // 2. 创建目录结构
  await createDistStructure();

  // 3. 编译二进制文件
  await compileBinary();

  // 4. 复制 www 目录
  await copyWwwDir();

  // 5. 复制配置文件
  await copyConfigFiles();

  // 6. 创建 README
  await createReadme();

  // 7. 显示摘要
  showSummary();
}

// 运行
if (import.meta.main) {
  await main().catch((error) => {
    console.error("❌ 构建失败:", error.message);
    Deno.exit(1);
  });
}
