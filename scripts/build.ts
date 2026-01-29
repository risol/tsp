#!/usr/bin/env bun

/**
 * TSP 构建脚本
 * 编译二进制文件并打包到 dist 目录
 */

import { spawn } from "node:child_process";
import { existsSync, mkdirSync, cpSync, readdirSync, statSync, rmSync } from "node:fs";
import { join } from "node:path";

const PLATFORM = process.platform;
const BINARY_NAME = PLATFORM === "win32" ? "tsp.exe" : "tsp";
const DIST_DIR = "./dist";

console.log(`
==========================================
  TSP Build Script
==========================================

Platform: ${PLATFORM}
Binary: ${BINARY_NAME}
Output: ${DIST_DIR}

`);

/**
 * 删除目录
 */
function rmDir(dir: string): void {
  if (existsSync(dir)) {
    rmSync(dir, { recursive: true, force: true });
    console.log(`✓ Removed ${dir}`);
  }
}

/**
 * 创建目录
 */
function mkDir(dir: string): void {
  if (!existsSync(dir)) {
   .mkdirSync(dir, { recursive: true });
  }
}

/**
 * 复制文件或目录
 */
function copyFile(src: string, dest: string): void {
  cpSync(src, dest, { recursive: true });
  console.log(`✓ Copied ${src} → ${dest}`);
}

/**
 * 复制目录内容（不包含目录本身）
 */
function copyDirContents(srcDir: string, destDir: string): void {
  if (!existsSync(srcDir)) {
    console.log(`! Source directory not found: ${srcDir}`);
    return;
  }

  mkDir(destDir);

  const items = readdirSync(srcDir);
  for (const item of items) {
    const srcPath = join(srcDir, item);
    const destPath = join(destDir, item);
    const stat = statSync(srcPath);

    if (stat.isDirectory()) {
      copyDirContents(srcPath, destPath);
    } else {
      cpSync(srcPath, destPath);
      console.log(`✓ Copied ${srcPath} → ${destPath}`);
    }
  }
}

/**
 * 运行命令
 */
function runCommand(cmd: string, args: string[]): Promise<void> {
  return new Promise((resolve, reject) => {
    const proc = spawn(cmd, args, { stdio: "inherit" });

    proc.on("close", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`Command failed with exit code ${code}`));
      }
    });

    proc.on("error", reject);
  });
}

/**
 * 主构建流程
 */
async function build(): Promise<void> {
  try {
    // 1. 清理旧的 dist 目录
    console.log("\n1. Cleaning dist directory...");
    rmDir(DIST_DIR);
    mkDir(DIST_DIR);

    // 2. 编译二进制文件
    console.log("\n2. Compiling binary...");
    const tempBinary = join(DIST_DIR, BINARY_NAME);
    await runCommand("bun", [
      "build",
      "src/main.ts",
      "--outfile",
      tempBinary,
      "--target",
      "bun",
    ]);
    console.log(`✓ Binary compiled: ${tempBinary}`);

    // 3. 复制 www 目录内容
    console.log("\n3. Copying www directory...");
    copyDirContents("./www", join(DIST_DIR, "www"));

    // 4. 复制配置文件示例
    console.log("\n4. Copying configuration files...");
    const configFiles = [
      "config.example.json",
      "config.example.jsonc",
      "tsconfig.json",
    ];
    for (const file of configFiles) {
      if (existsSync(file)) {
        copyFile(file, join(DIST_DIR, file));
      }
    }

    // 5. 创建 README
    console.log("\n5. Creating README...");
    const readme = `# TSP Server

TypeScript Server Page - Template server built with Bun + TSX + Preact

## Usage

\`\`\`bash
# Start server
${PLATFORM === "win32" ? "./tsp.exe" : "./tsp"} --root ./www --port 9000

# With dev mode
${PLATFORM === "win32" ? "./tsp.exe" : "./tsp"} --root ./www --port 9000 --dev

# Using config file
${PLATFORM === "win32" ? "./tsp.exe" : "./tsp"} --config config.json
\`\`\`

## Configuration

Copy \`config.example.json\` to \`config.json\` and edit:

\`\`\`json
{
  "root": "./www",
  "port": 9000,
  "dev": false
}
\`\`\`

## CLI Options

- \`--root, -r <path>\` - Document root directory (default: ./www)
- \`--port, -p <port>\` - Listen port (default: 9000)
- \`--dev, -d\` - Development mode
- \`--config, -c <file>\` - Config file path
- \`--help, -h\` - Show help

## Project Structure

\`\`\`
dist/
├── ${BINARY_NAME}      # Server binary
├── www/               # Web pages (TSX files)
├── tsconfig.json      # TypeScript config
├── config.example.json # Example config
└── README.md          # This file
\`\`\`
`;
    Bun.write(join(DIST_DIR, "README.md"), readme);
    console.log("✓ README.md created");

    // 6. 显示二进制文件信息
    console.log("\n6. Binary info...");
    const stats = statSync(tempBinary);
    const sizeKB = (stats.size / 1024).toFixed(2);
    console.log(`   Size: ${sizeKB} KB`);

    console.log("\n==========================================");
    console.log("  Build completed successfully!");
    console.log("==========================================");
    console.log(`\nOutput directory: ${DIST_DIR}`);
    console.log(`Binary: ${tempBinary}`);
    console.log(`\nYou can now distribute the ${DIST_DIR} directory.\n`);
  } catch (error) {
    console.error("\nBuild failed:", error);
    process.exit(1);
  }
}

build();
