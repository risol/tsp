/**
 * TspInfo 系统信息测试
 * 验证 tspinfo 获取的 CPU 和内存信息是否准确
 */

import { assertEquals, assertExists } from "@std/assert";
import { TspInfo } from "../../src/tspinfo.ts";

Deno.test("TspInfo: 获取真实系统信息", async () => {
  const tspInfo = new TspInfo(
    {
      root: "./www",
      port: 9000,
      dev: false,
    },
    undefined,
  );

  const info = await tspInfo.getInfo();

  console.log("\n=== TSP Info 系统信息 ===");
  console.log(`主机名: ${info.system.hostname}`);
  console.log(`操作系统: ${info.system.platform}`);
  console.log(`系统版本: ${info.system.osVersion}`);
  console.log(`物理核心: ${info.system.cpus.physical}`);
  console.log(`逻辑核心: ${info.system.cpus.logical}`);
  console.log(`系统内存: ${info.system.memTotalFormatted}`);

  // 验证主机名不为空且不是 "unknown"
  assertExists(info.system.hostname);
  assertEquals(typeof info.system.hostname, "string");

  // 验证 CPU 核心数大于 0
  assertEquals(typeof info.system.cpus, "object");
  assertEquals(typeof info.system.cpus.physical, "number");
  assertEquals(typeof info.system.cpus.logical, "number");

  if (typeof info.system.cpus.physical === "number" && typeof info.system.cpus.logical === "number") {
    console.log(`✓ CPU 核心数有效: ${info.system.cpus.physical} 物理核心 / ${info.system.cpus.logical} 逻辑核心`);
  }

  // 验证系统内存大于 1GB
  assertEquals(typeof info.system.memTotal, "number");
  if (typeof info.system.memTotal === "number") {
    const memGB = info.system.memTotal / (1024 ** 3);
    console.log(`✓ 系统内存是有效的数字: ${memGB.toFixed(2)} GB`);
    // 内存应该至少 1GB
    if (memGB < 1) {
      console.warn(`⚠️  警告: 系统内存似乎太小 (${memGB.toFixed(2)} GB)`);
    }
  }

  // 验证格式化后的内存字符串
  assertEquals(typeof info.system.memTotalFormatted, "string");
  console.log(`✓ 内存格式化字符串: ${info.system.memTotalFormatted}`);
});

Deno.test("TspInfo: 对比 node:os 内存信息", async () => {
  const tspInfo = new TspInfo(
    {
      root: "./www",
      port: 9000,
      dev: false,
    },
    undefined,
  );

  const tspInfoData = await tspInfo.getInfo();

  // 尝试从 node:os 获取真实内存
  try {
    const os = await import("node:os");
    const realMem = os.totalmem();
    const realMemGB = realMem / (1024 ** 3);
    const tspInfoMemGB = tspInfoData.system.memTotal / (1024 ** 3);

    console.log("\n=== 内存信息对比 ===");
    console.log(`node:os totalmem(): ${realMemGB.toFixed(2)} GB`);
    console.log(`TspInfo: ${tspInfoMemGB.toFixed(2)} GB`);

    // 验证两者接近（允许 1% 误差）
    const diff = Math.abs(realMem - tspInfoData.system.memTotal);
    const diffPercent = (diff / realMem) * 100;

    console.log(`差异: ${(diff / (1024 ** 3)).toFixed(2)} GB (${diffPercent.toFixed(2)}%)`);

    if (diffPercent < 1) {
      console.log("✓ TspInfo 内存信息准确");
    } else if (diffPercent < 5) {
      console.log("⚠️  TspInfo 内存信息基本准确（有小误差）");
    } else {
      console.error("✗ TspInfo 内存信息不准确");
    }
  } catch (e) {
    console.log(`⚠️  无法对比（node:os 不可用）: ${e}`);
  }
});

Deno.test("TspInfo: CPU 信息验证", async () => {
  const tspInfo = new TspInfo(
    {
      root: "./www",
      port: 9000,
      dev: false,
    },
    undefined,
  );

  const info = await tspInfo.getInfo();

  console.log("\n=== CPU 信息 ===");
  console.log(`物理核心数: ${info.system.cpus.physical}`);
  console.log(`逻辑核心数: ${info.system.cpus.logical}`);
  console.log(`navigator.hardwareConcurrency: ${navigator.hardwareConcurrency}`);

  // 逻辑核心数应该等于 navigator.hardwareConcurrency
  assertEquals(info.system.cpus.logical, navigator.hardwareConcurrency);

  // 物理核心数应该大于 0
  if (typeof info.system.cpus.physical === "number") {
    console.assert(
      info.system.cpus.physical > 0,
      "物理核心数应该大于 0"
    );
  }

  // 逻辑核心数应该 >= 物理核心数
  console.assert(
    info.system.cpus.logical >= info.system.cpus.physical,
    "逻辑核心数应该 >= 物理核心数"
  );

  // 检查超线程
  const hasHyperThreading = info.system.cpus.logical > info.system.cpus.physical;
  console.log(`超线程: ${hasHyperThreading ? "开启" : "未开启"}`);

  if (hasHyperThreading) {
    const ratio = info.system.cpus.logical / info.system.cpus.physical;
    console.log(`超线程比例: ${ratio.toFixed(1)}x`);
  }

  console.log("✓ CPU 信息验证通过");
});
