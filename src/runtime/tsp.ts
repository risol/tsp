/**
 * TSP page loading boundary.
 *
 * The TSP-enabled Bun fork owns canonical module identity and page source
 * loading. The application keeps the page generation/LKG boundary here; it
 * must never append a cache-busting query to a module URL.
 */

import {
  canonicalModuleKey,
  TspModuleGraph,
  type ModuleKey,
} from "./module-graph.ts";

export interface TspPageModule {
  default: (context: unknown) => Promise<unknown> | unknown;
  fragments?: Record<string, (context: unknown) => Promise<unknown> | unknown>;
  [exportName: string]: unknown;
}

interface TspRuntimeBinding {
  loadPage(
    filepath: string,
    reload?: boolean,
    scopeRoot?: string,
    invalidatePaths?: string[],
  ): Promise<TspPageModule>;
  inspectPage?(filepath: string): unknown;
  stats?(): unknown;
}

function getTspRuntime(): TspRuntimeBinding {
  const globalRuntime = globalThis as typeof globalThis & {
    Bun?: { TSP?: TspRuntimeBinding };
  };
  const runtime = globalRuntime.Bun?.TSP;

  if (!runtime || typeof runtime.loadPage !== "function") {
    throw new Error(
      "This server requires a TSP-enabled Bun runtime. " +
        "Build or run it with the Bun fork that provides Bun.TSP.loadPage().",
    );
  }

  return runtime;
}

const pageGraph = new TspModuleGraph();
const pageCache = new Map<ModuleKey, TspPageModule>();
const inFlight = new Map<ModuleKey, Promise<TspPageModule>>();

export function loadPage(
  filepath: string,
  reload = false,
  scopeRoot?: string,
  watchChanges = true,
): Promise<TspPageModule> {
  const key = canonicalModuleKey(filepath);
  const active = inFlight.get(key);
  if (active) return active;

  const promise = (async () => {
    const root = scopeRoot ?? filepath;
    const knownPage = pageGraph.getPage(filepath);
    const previousModules = pageGraph.getPageModulePaths(filepath);
    if (knownPage && watchChanges) {
      await pageGraph.refreshChangedModules();
    }
    if (!knownPage || pageGraph.getPage(filepath)?.dirty) {
      await pageGraph.discoverPage(filepath, root);
    }

    const cached = pageCache.get(key);
    const page = pageGraph.getPage(filepath);
    const shouldReload = reload || !cached || page?.dirty === true;
    if (!shouldReload && cached) return cached;
    const invalidatePaths = [
      ...new Set([...previousModules, ...pageGraph.getPageModulePaths(filepath)]),
    ];

    pageGraph.beginReload(filepath);

    try {
      const loaded = await getTspRuntime().loadPage(
        filepath,
        true,
        scopeRoot,
        invalidatePaths,
      );
      if (!loaded || typeof loaded.default !== "function") {
        throw new Error(`TSP page has no default function export: ${filepath}`);
      }

      pageCache.set(key, loaded);
      pageGraph.publish(filepath, loaded);
      return loaded;
    } catch (error) {
      pageGraph.failReload(filepath, error);
      const lastKnownGood = pageCache.get(key);
      if (lastKnownGood) return lastKnownGood;
      throw error;
    } finally {
      inFlight.delete(key);
    }
  })();

  inFlight.set(key, promise);
  return promise;
}

export function inspectPage(filepath: string): unknown {
  return pageGraph.inspectPage(filepath);
}

export function getStats(): unknown {
  return {
    pages: pageGraph.dirtyPages().length,
    cachedPages: pageCache.size,
    loadingPages: inFlight.size,
  };
}
