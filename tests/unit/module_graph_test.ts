import { describe, expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { TspModuleGraph, canonicalModuleKey } from "../../src/runtime/module-graph.ts";

describe("TSP module graph", () => {
  test("propagates a leaf change to every owning page", () => {
    const graph = new TspModuleGraph();
    graph.registerPage("/www/a.tsp", "/www/a.tsp");
    graph.registerPage("/www/b.tsp", "/www/b.tsp");
    graph.updateModule("/www/a.tsp", ["/www/shared.ts"]);
    graph.updateModule("/www/b.tsp", ["/www/shared.ts"]);
    graph.attachPageModules("/www/a.tsp", ["/www/shared.ts"]);
    graph.attachPageModules("/www/b.tsp", ["/www/shared.ts"]);

    const dirty = graph.markChanged("/www/shared.ts");
    expect(dirty).toEqual(new Set([
      canonicalModuleKey("/www/a.tsp"),
      canonicalModuleKey("/www/b.tsp"),
    ]));
  });

  test("publishes generations atomically and keeps LKG on failure", () => {
    const graph = new TspModuleGraph();
    graph.registerPage("/www/index.tsp", "/www/index.tsp");
    const first = graph.publish("/www/index.tsp", "v1");

    graph.markChanged("/www/index.tsp");
    graph.beginReload("/www/index.tsp");
    graph.failReload("/www/index.tsp", new Error("syntax error"));

    expect(graph.getPage("/www/index.tsp")?.currentGeneration).toBe(first.id);
    expect(graph.getPage("/www/index.tsp")?.dirty).toBe(true);
  });

  test("discovers nested source dependencies and marks the owning page dirty", async () => {
    const root = await mkdtemp(join(tmpdir(), "tsp-module-graph-"));
    const page = join(root, "page.tsp");
    const wrapper = join(root, "wrapper.tsx");
    const utility = join(root, "utility.ts");

    try {
      await writeFile(page, 'import { render } from "./wrapper.tsx";\nexport default render;\n');
      await writeFile(wrapper, 'import { value } from "./utility.ts";\nexport function render() { return value; }\n');
      await writeFile(utility, 'export const value = "v1";\n');

      const graph = new TspModuleGraph();
      await graph.discoverPage(page, root);

      const inspection = graph.inspectPage(page);
      expect(inspection?.modules).toHaveLength(3);
      expect(graph.getModule(page)?.dependencies).toEqual(new Set([canonicalModuleKey(wrapper)]));

      await writeFile(utility, 'export const value = "v2-updated";\n');
      const dirty = await graph.refreshChangedModules();

      expect(dirty).toEqual(new Set([canonicalModuleKey(page)]));
      expect(graph.getPage(page)?.dirty).toBe(true);
      expect(await readFile(utility, "utf8")).toContain("v2-updated");
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});
