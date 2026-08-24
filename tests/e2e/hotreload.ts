/**
 * Hot Reload E2E Tests
 */

import { getTestRoot, TEST_PORT, RELOAD_DELAY, printSubsection, printTestResult, COLORS, assertEquals, assertExists } from "./helpers.ts";
import { rm, writeFile } from "node:fs/promises";

export function getHotReloadTests() {
  return [
    {
      name: "hot reload - Nested dependency hot reload",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Hot Reload Test - Nested Dependencies (v5.0)");

        // Clean up test files
        const testRoot = getTestRoot();
        const componentPath = `${testRoot}/components/HotReloadComponent.tsx`;
        const wrapperPath = `${testRoot}/components/HotReloadWrapper.tsx`;
        const utilsPath = `${testRoot}/components/HotReloadUtils.ts`;

        try {
          await rm(componentPath, { force: true });
        } catch {}
        try {
          await rm(wrapperPath, { force: true });
        } catch {}
        try {
          await rm(utilsPath, { force: true });
        } catch {}

        // Create initial files
        await writeFile(
          utilsPath,
          `export function getVersion(): string {
  return "INITIAL_VERSION";
}`,
        );
        await writeFile(
          componentPath,
          `import { getVersion } from "./HotReloadUtils.ts";

export function HotReloadComponent() {
  return <div data-testid="component">{getVersion()}</div>;
}`,
        );
        await writeFile(
          wrapperPath,
          `import { HotReloadComponent } from "./HotReloadComponent.tsx";

export function HotReloadWrapper() {
  return <div data-testid="wrapper"><HotReloadComponent /></div>;
}`,
        );
        await new Promise((r) => setTimeout(r, 500));

        // First access - verify initial content
        let response = await fetch(
          `http://localhost:${TEST_PORT}/hot_reload_page.tsp`,
        );
        assertEquals(response.status, 200);
        let content = await response.text();
        assertExists(
          content.includes("INITIAL_VERSION"),
          "Page should contain INITIAL_VERSION",
        );
        printTestResult("Initial content verified", true);

        // Modify .ts utility file (three-level dependency)
        await writeFile(
          utilsPath,
          `export function getVersion(): string {
  return "MODIFIED_VERSION";
}`,
        );
        printTestResult(".ts utility file modified", true);

        // Wait for file system detection
        await new Promise((r) => setTimeout(r, RELOAD_DELAY));

        // Access again - verify content is updated
        response = await fetch(
          `http://localhost:${TEST_PORT}/hot_reload_page.tsp`,
        );
        assertEquals(response.status, 200);
        content = await response.text();
        const hasModified = content.includes("MODIFIED_VERSION");
        const hasInitial = content.includes("INITIAL_VERSION");

        if (hasModified && !hasInitial) {
          printTestResult("Nested dependency hot reload works (including .ts files)", true);
        } else {
          printTestResult("Nested dependency hot reload failed", false);
          throw new Error(
            `Hot reload failed: hasModified=${hasModified}, hasInitial=${hasInitial}`,
          );
        }

        // Cleanup
        try {
          await rm(componentPath, { force: true });
        } catch {}
        try {
          await rm(wrapperPath, { force: true });
        } catch {}
        try {
          await rm(utilsPath, { force: true });
        } catch {}

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms}${COLORS.reset}`);
      },
    },
  ];
}
