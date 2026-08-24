import { describe, expect, test } from "bun:test";
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
});
