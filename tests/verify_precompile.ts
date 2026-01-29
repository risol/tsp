#!/usr/bin/env -S deno run --allow-all

/**
 * Precompilation Feature Verification Tests
 * 验证预编译功能是否正常工作
 */

import { assertEquals, assertThrows } from "@std/assert";
import { checkRemoteImports, analyzeDependencies, getCachePath } from "../src/precompiler_lib.ts";

console.log("╔════════════════════════════════════════════╗");
console.log("║   预编译功能验证测试                      ║");
console.log("╚════════════════════════════════════════════╝\n");

// Test 1: Remote import detection
console.log("▶ Test 1: Remote import detection");
try {
  const testFile = "test_remote.tsx";
  await Deno.writeTextFile(testFile, `
    import { something } from "https://example.com/module.ts";
    import { other } from "npm:package";
    export default async function(ctx) { return <div>Test</div>; }
  `);

  const remoteImports = await checkRemoteImports(testFile);
  assertEquals(remoteImports.length, 2);
  assertEquals(remoteImports[0], "https://example.com/module.ts");
  assertEquals(remoteImports[1], "npm:package");

  await Deno.remove(testFile);
  console.log("  ✓ Remote import detection works correctly\n");
} catch (error) {
  console.error("  ✗ Remote import detection test failed:", error);
  Deno.exit(1);
}

// Test 2: Local imports are not detected as remote
console.log("▶ Test 2: Local imports are allowed");
try {
  const testFile = "test_local.tsx";
  await Deno.writeTextFile(testFile, `
    import { Component } from "./component.tsx";
    import { helper } from "../utils/helper.ts";
    export default async function(ctx) { return <div>Test</div>; }
  `);

  const remoteImports = await checkRemoteImports(testFile);
  assertEquals(remoteImports.length, 0);

  await Deno.remove(testFile);
  console.log("  ✓ Local imports are correctly identified\n");
} catch (error) {
  console.error("  ✗ Local imports test failed:", error);
  Deno.exit(1);
}

// Test 3: Dependency analysis
console.log("▶ Test 3: Dependency analysis");
try {
  const testDir = "test_deps";
  await Deno.mkdir(testDir, { recursive: true });

  // Create test files
  await Deno.writeTextFile(`${testDir}/main.tsx`, `
    import { A } from "./a";
    import { B } from "./b";
    export default async function(ctx) { return <div>Test</div>; }
  `);
  await Deno.writeTextFile(`${testDir}/a.tsx`, `
    export const A = () => <div>A</div>;
  `);
  await Deno.writeTextFile(`${testDir}/b.ts`, `
    export const B = () => <div>B</div>;
  `);

  // Use absolute path
  const absPath = `${Deno.cwd()}/${testDir}/main.tsx`;
  const dependencies = await analyzeDependencies(absPath);

  // Should find both dependencies
  const hasA = dependencies.some((dep) => dep.endsWith("a.tsx"));
  const hasB = dependencies.some((dep) => dep.endsWith("b.ts"));

  if (!hasA || !hasB) {
    throw new Error(`Expected dependencies not found. Got: ${dependencies.join(", ")}`);
  }

  await Deno.remove(testDir, { recursive: true });
  console.log("  ✓ Dependency analysis works correctly\n");
} catch (error) {
  console.error("  ✗ Dependency analysis test failed:", error);
  Deno.exit(1);
}

// Test 4: Cache path generation
console.log("▶ Test 4: Cache path generation");
try {
  const testPath = "www/test/page.tsx";
  const cachePath = getCachePath(testPath);

  // Should convert to .cache/tsp/test/page.js (platform-agnostic)
  const { join } = await import("std/path");
  const expectedCachePath = join(Deno.cwd(), ".cache", "tsp", "test", "page.js");
  assertEquals(cachePath, expectedCachePath);

  console.log("  ✓ Cache path generation works correctly\n");
} catch (error) {
  console.error("  ✗ Cache path generation test failed:", error);
  Deno.exit(1);
}

// Test 5: JSR imports are detected as remote
console.log("▶ Test 5: JSR imports are detected as remote");
try {
  const testFile = "test_jsr.tsx";
  await Deno.writeTextFile(testFile, `
    import { assert } from "jsr:@std/assert";
    export default async function(ctx) { return <div>Test</div>; }
  `);

  const remoteImports = await checkRemoteImports(testFile);
  assertEquals(remoteImports.length, 1);
  assertEquals(remoteImports[0], "jsr:@std/assert");

  await Deno.remove(testFile);
  console.log("  ✓ JSR imports are correctly detected\n");
} catch (error) {
  console.error("  ✗ JSR import detection test failed:", error);
  Deno.exit(1);
}

console.log("╔════════════════════════════════════════════╗");
console.log("║   验证结果                                 ║");
console.log("╚════════════════════════════════════════════╝");
console.log("✓ 所有预编译功能验证测试通过！");
