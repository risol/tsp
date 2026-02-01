/**
 * 系统信息获取测试
 * 测试 tspinfo 中系统相关信息的准确性
 */

import { assertEquals } from "@std/assert";

/**
 * 测试获取真实的 CPU 核心数
 */
Deno.test("system info: CPU cores", async () => {
  // 方法 1: Deno.systemCpus (Deno 1.16+)
  let cpus = 0;
  try {
    const cpuInfo = Deno.systemCpus?.();
    if (cpuInfo && cpuInfo.length > 0) {
      cpus = cpuInfo.length;
      console.log(`[方法1] Deno.systemCpus(): ${cpus} 核心`);
    }
  } catch (e) {
    console.log(`[方法1] Deno.systemCpus() 不可用: ${e}`);
  }

  // 方法 2: navigator.hardwareConcurrency (浏览器 API)
  const navCpus = navigator.hardwareConcurrency;
  console.log(`[方法2] navigator.hardwareConcurrency: ${navCpus} 核心`);

  // 方法 3: 通过 os 命令获取
  try {
    let command: string;
    if (Deno.build.os === "windows") {
      command = "powershell -Command \"(Get-WmiObject Win32_Processor).NumberOfCores\"";
    } else if (Deno.build.os === "linux") {
      command = "nproc";
    } else if (Deno.build.os === "darwin") {
      command = "sysctl -n hw.ncpu";
    } else {
      command = "echo unknown";
    }

    const process = new Deno.Command("sh", {
      args: ["-c", command],
      stdout: "piped",
      stderr: "piped",
    });

    const { stdout, code } = await process.output();
    if (code === 0) {
      const output = new TextDecoder().decode(stdout).trim();
      console.log(`[方法3] 系统命令: ${output} 核心`);
    }
  } catch (e) {
    console.log(`[方法3] 系统命令失败: ${e}`);
  }

  console.log(`\n推荐使用: ${cpus > 0 ? cpus : navCpus} 核心`);
});

/**
 * 测试获取真实的系统内存
 */
Deno.test("system info: system memory", async () => {
  // 方法 1: Deno.systemMemoryEstimate (Deno 1.41+)
  let memoryInfo: { total: number } | null = null;
  try {
    const info = Deno.systemMemoryEstimate?.();
    if (info) {
      memoryInfo = info;
      const totalGB = (info.total / (1024 ** 3)).toFixed(2);
      console.log(`[方法1] Deno.systemMemoryEstimate(): ${totalGB} GB`);
    }
  } catch (e) {
    console.log(`[方法1] Deno.systemMemoryEstimate() 不可用: ${e}`);
  }

  // 方法 2: 通过系统命令获取
  try {
    let command: string;
    if (Deno.build.os === "windows") {
      command = "powershell -Command \"(Get-WmiObject Win32_ComputerSystem).TotalPhysicalMemory / 1GB\"";
    } else if (Deno.build.os === "linux") {
      command = "free -b | grep Mem | awk '{print $2}'";
    } else if (Deno.build.os === "darwin") {
      command = "sysctl -n hw.memsize";
    } else {
      command = "echo unknown";
    }

    const process = new Deno.Command("sh", {
      args: ["-c", command],
      stdout: "piped",
      stderr: "piped",
    });

    const { stdout, code } = await process.output();
    if (code === 0) {
      const output = new TextDecoder().decode(stdout).trim();
      const bytes = parseFloat(output);
      if (!isNaN(bytes)) {
        const totalGB = (bytes / (1024 ** 3)).toFixed(2);
        console.log(`[方法2] 系统命令: ${totalGB} GB (${bytes} bytes)`);
      }
    }
  } catch (e) {
    console.log(`[方法2] 系统命令失败: ${e}`);
  }

  // 方法 3: OS 模块
  try {
    // Deno 2.0 的 os 模块
    const osModule = await import("node:os");
    const totalMem = osModule.totalmem();
    const totalGB = (totalMem / (1024 ** 3)).toFixed(2);
    console.log(`[方法3] node:os totalmem(): ${totalGB} GB`);
  } catch (e) {
    console.log(`[方法3] node:os 不可用: ${e}`);
  }

  console.log(`\n推荐使用: ${memoryInfo ? (memoryInfo.total / (1024 ** 3)).toFixed(2) : "未知"} GB`);
});

/**
 * 测试获取主机名
 */
Deno.test("system info: hostname", () => {
  // 方法 1: Deno.hostname()
  try {
    const hostname1 = Deno.hostname?.();
    console.log(`[方法1] Deno.hostname(): ${hostname1}`);
  } catch (e) {
    console.log(`[方法1] Deno.hostname() 失败: ${e}`);
  }

  // 方法 2: 通过系统命令
  // 略，因为 hostname 通常比较简单
});

/**
 * 测试系统信息 API 可用性
 */
Deno.test("system info: API availability", () => {
  console.log("\n=== Deno 版本信息 ===");
  console.log(`Deno.version.deno: ${Deno.version.deno}`);
  console.log(`Deno.build.os: ${Deno.build.os}`);
  console.log(`Deno.build.arch: ${Deno.build.arch}`);
  console.log(`Deno.build.target: ${Deno.build.target}`);

  console.log("\n=== 系统 API 可用性 ===");
  console.log(`Deno.systemCpus: ${typeof Deno.systemCpus}`);
  console.log(`Deno.systemMemoryEstimate: ${typeof Deno.systemMemoryEstimate}`);
  console.log(`Deno.hostname: ${typeof Deno.hostname}`);
  console.log(`Deno.pid: ${Deno.pid}`);
  console.log(`navigator.hardwareConcurrency: ${navigator.hardwareConcurrency}`);
});
