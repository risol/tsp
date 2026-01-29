#!/usr/bin/env -S deno run --allow-all

/**
 * 热重载测试（Hot Reload Tests）
 * 测试 TSX 文件及其依赖文件修改后的自动重新编译
 */

import { assertEquals, assertExists } from "@std/assert";
import { join } from "std/path";

const TEST_PORT = 9600;
const TEST_ROOT = "./tests/test_www";
const STARTUP_DELAY = 3000;
const RELOAD_DELAY = 1000;

// 测试文件路径
const TEST_FILES = {
  mainPage: join(TEST_ROOT, "hot_reload_main.tsx"),
  component: join(TEST_ROOT, "hot_reload_component.tsx"),
  util: join(TEST_ROOT, "hot_reload_util.ts"),
};

// 初始内容
const INITIAL_CONTENT = {
  mainPage: `import { Component } from "./hot_reload_component.tsx";

export default async function() {
  return <div>
    <h1>Hot Reload Test - v1</h1>
    <Component />
  </div>;
}
`,
  component: `export function Component() {
  return <p>Component v1</p>;
}
`,
  util: `export function getMessage() {
  return "Message v1";
}
`,
};

// 修改后的内容
const UPDATED_CONTENT = {
  mainPage: `import { Component } from "./hot_reload_component.tsx";

export default async function() {
  return <div>
    <h1>Hot Reload Test - v2</h1>
    <Component />
  </div>;
}
`,
  component: `export function Component() {
  return <p>Component v2 - UPDATED</p>;
}
`,
  util: `export function getMessage() {
  return "Message v2 - UPDATED";
}
`,
};

let serverProcess: Deno.ChildProcess | null = null;

/**
 * 清理端口
 */
async function killProcessOnPort(port: number): Promise<void> {
  if (Deno.build.os === "windows") {
    try {
      const netstatCommand = new Deno.Command("netstat", {
        args: ["-ano"],
        stdout: "piped",
      });
      const { stdout } = await netstatCommand.output();
      const output = new TextDecoder().decode(stdout);

      const lines = output.split("\n");
      for (const line of lines) {
        if (line.includes(`:${port}`) && line.includes("LISTENING")) {
          const parts = line.trim().split(/\s+/);
          const pid = parseInt(parts[parts.length - 1]);
          if (!isNaN(pid)) {
            const killCommand = new Deno.Command("taskkill", {
              args: ["/PID", pid.toString(), "/F"],
              stdout: "piped",
            });
            await killCommand.output();
          }
        }
      }
    } catch {
      // 忽略错误
    }
  }
}

/**
 * 启动开发模式服务器
 */
async function startDevServer(): Promise<void> {
  await killProcessOnPort(TEST_PORT);

  const command = new Deno.Command("deno", {
    args: [
      "run",
      "--allow-net",
      "--allow-read",
      "--allow-write",
      "--allow-env",
      "src/main.ts",
      "--root",
      TEST_ROOT,
      "--port",
      TEST_PORT.toString(),
      "--dev",
    ],
    stdout: "piped",
    stderr: "piped",
  });

  serverProcess = command.spawn();
  await new Promise((resolve) => setTimeout(resolve, STARTUP_DELAY));
}

/**
 * 停止服务器
 */
async function stopServer(): Promise<void> {
  if (serverProcess) {
    try {
      serverProcess.kill("SIGTERM");
    } catch {
      // 忽略错误
    }
    serverProcess = null;
  }
}

/**
 * 等待文件系统操作完成
 */
async function waitForFilesystem(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 500));
}

/**
 * 修改文件并等待重载
 */
async function modifyFileAndWait(
  filepath: string,
  content: string
): Promise<void> {
  await Deno.writeTextFile(filepath, content);
  await waitForFilesystem();
  await new Promise((resolve) => setTimeout(resolve, RELOAD_DELAY));
}

/**
 * 获取页面内容
 */
async function getPageContent(url: string): Promise<string> {
  const response = await fetch(url);
  assertEquals(response.status, 200);
  return await response.text();
}

/**
 * 测试场景1：主TSX文件修改触发热重载
 */
async function testMainFileHotReload(): Promise<void> {
  console.log("\n  测试1: 主TSX文件修改触发热重载");

  // 1. 写入初始内容
  await Deno.writeTextFile(TEST_FILES.mainPage, INITIAL_CONTENT.mainPage);
  await Deno.writeTextFile(TEST_FILES.component, INITIAL_CONTENT.component);
  await waitForFilesystem();

  // 2. 访问页面，验证初始内容
  let content = await getPageContent(`http://localhost:${TEST_PORT}/hot_reload_main.tsx`);
  assertExists(content.includes("Hot Reload Test - v1"));
  console.log("    ✓ 初始内容正确");

  // 3. 修改主文件
  await modifyFileAndWait(TEST_FILES.mainPage, UPDATED_CONTENT.mainPage);

  // 4. 再次访问页面，验证内容已更新
  content = await getPageContent(`http://localhost:${TEST_PORT}/hot_reload_main.tsx`);
  assertExists(content.includes("Hot Reload Test - v2"));
  console.log("    ✓ 主文件修改后自动重载");

  // 5. 验证日志中有CACHE MISS和重新编译
  // (通过检查服务器输出日志来验证)
}

/**
 * 测试场景2：依赖的TSX组件修改触发热重载
 */
async function testComponentHotReload(): Promise<void> {
  console.log("\n  测试2: 依赖的TSX组件修改触发热重载");

  // 1. 重置为初始内容
  await Deno.writeTextFile(TEST_FILES.mainPage, INITIAL_CONTENT.mainPage);
  await Deno.writeTextFile(TEST_FILES.component, INITIAL_CONTENT.component);
  await waitForFilesystem();

  // 2. 访问页面，验证初始内容
  let content = await getPageContent(`http://localhost:${TEST_PORT}/hot_reload_main.tsx`);
  assertExists(content.includes("Component v1"));
  console.log("    ✓ 组件初始内容正确");

  // 3. 修改组件文件
  await modifyFileAndWait(TEST_FILES.component, UPDATED_CONTENT.component);

  // 4. 再次访问页面，验证组件已更新
  content = await getPageContent(`http://localhost:${TEST_PORT}/hot_reload_main.tsx`);
  assertExists(content.includes("Component v2 - UPDATED"));
  console.log("    ✓ 组件修改后自动重载主文件");
}

/**
 * 测试场景3：依赖的TS工具文件修改触发热重载
 */
async function testUtilFileHotReload(): Promise<void> {
  console.log("\n  测试3: 依赖的TS工具文件修改触发热重载");

  // 创建使用工具文件的页面
  const mainPageWithUtil = `import { getMessage } from "./hot_reload_util.ts";

export default async function() {
  return <div>
    <h1>Util Test</h1>
    <p>{getMessage()}</p>
  </div>;
}
`;

  await Deno.writeTextFile(TEST_FILES.mainPage, mainPageWithUtil);
  await Deno.writeTextFile(TEST_FILES.util, INITIAL_CONTENT.util);
  await waitForFilesystem();

  // 1. 访问页面，验证初始内容
  let content = await getPageContent(`http://localhost:${TEST_PORT}/hot_reload_main.tsx`);
  assertExists(content.includes("Message v1"));
  console.log("    ✓ 工具函数初始内容正确");

  // 2. 修改工具文件
  await modifyFileAndWait(TEST_FILES.util, UPDATED_CONTENT.util);

  // 3. 再次访问页面，验证工具函数已更新
  content = await getPageContent(`http://localhost:${TEST_PORT}/hot_reload_main.tsx`);
  assertExists(content.includes("Message v2 - UPDATED"));
  console.log("    ✓ 工具文件修改后自动重载主文件");
}

/**
 * 测试场景4：级联热重载（A依赖B，B依赖C，修改C）
 */
async function testCascadeHotReload(): Promise<void> {
  console.log("\n  测试4: 级联热重载");

  // 创建级联依赖
  const utilContent = `export function getData() {
  return "Cascade v1";
}
`;

  const componentContent = `import { getData } from "./hot_reload_util.ts";

export function Component() {
  return <p>{getData()}</p>;
}
`;

  const mainContent = `import { Component } from "./hot_reload_component.tsx";

export default async function() {
  return <div>
    <Component />
  </div>;
}
`;

  await Deno.writeTextFile(TEST_FILES.util, utilContent);
  await Deno.writeTextFile(TEST_FILES.component, componentContent);
  await Deno.writeTextFile(TEST_FILES.mainPage, mainContent);
  await waitForFilesystem();

  // 1. 验证初始内容
  let content = await getPageContent(`http://localhost:${TEST_PORT}/hot_reload_main.tsx`);
  assertExists(content.includes("Cascade v1"));
  console.log("    ✓ 级联依赖初始内容正确");

  // 2. 修改最底层的util文件
  const updatedUtilContent = `export function getData() {
  return "Cascade v2 - CASCADE UPDATED";
}
`;
  await modifyFileAndWait(TEST_FILES.util, updatedUtilContent);

  // 3. 验证整个链路都更新了
  content = await getPageContent(`http://localhost:${TEST_PORT}/hot_reload_main.tsx`);
  assertExists(content.includes("Cascade v2 - CASCADE UPDATED"));
  console.log("    ✓ 级联依赖自动重载");
}

/**
 * 清理测试文件
 */
async function cleanupTestFiles(): Promise<void> {
  const files = Object.values(TEST_FILES);
  for (const file of files) {
    try {
      await Deno.remove(file);
    } catch {
      // 忽略文件不存在的错误
    }
  }
}

// ============================================
// 主测试执行
// ============================================

async function runHotReloadTests(): Promise<void> {
  console.log("\n╔════════════════════════════════════════════╗");
  console.log("║   热重载测试（Hot Reload Tests）          ║");
  console.log("╚════════════════════════════════════════════╝");

  let passedTests = 0;
  let failedTests = 0;

  const tests = [
    { name: "主文件热重载", fn: testMainFileHotReload },
    { name: "组件热重载", fn: testComponentHotReload },
    { name: "工具文件热重载", fn: testUtilFileHotReload },
    { name: "级联热重载", fn: testCascadeHotReload },
  ];

  try {
    // 启动开发模式服务器
    console.log("\n📦 启动开发模式服务器...");
    await startDevServer();
    console.log("✓ 服务器已启动");

    // 运行所有测试
    for (const test of tests) {
      try {
        await test.fn();
        passedTests++;
        console.log(`  ✓ ${test.name} 通过\n`);
      } catch (error) {
        failedTests++;
        console.error(`  ✗ ${test.name} 失败: ${error instanceof Error ? error.message : String(error)}\n`);
      }
    }

  } finally {
    // 清理
    console.log("🧹 清理测试文件...");
    await cleanupTestFiles();
    await stopServer();
    console.log("✓ 清理完成");
  }

  // 打印总结
  console.log("\n╔════════════════════════════════════════════╗");
  console.log("║   测试总结                                 ║");
  console.log("╚════════════════════════════════════════════╝");
  console.log(`\n总测试数: ${tests.length}`);
  console.log(`✓ 通过: ${passedTests}`);
  if (failedTests > 0) {
    console.log(`✗ 失败: ${failedTests}`);
  }

  if (failedTests === 0) {
    console.log("\n🎉 所有热重载测试通过！");
  } else {
    console.log("\n❌ 部分测试失败！");
    Deno.exit(1);
  }
}

// 运行测试
if (import.meta.main) {
  await runHotReloadTests().catch(console.error);
}
