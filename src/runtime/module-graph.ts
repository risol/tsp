import { normalize, resolve } from "node:path";

export type ModuleKey = string;
export type PageKey = string;
export type GenerationId = number;

export interface TspModuleNode {
  key: ModuleKey;
  dependencies: Set<ModuleKey>;
  dependents: Set<ModuleKey>;
  owningPages: Set<PageKey>;
  dirty: boolean;
}

export interface TspPageSlot {
  key: PageKey;
  rootModule: ModuleKey;
  dirty: boolean;
  loading: boolean;
  currentGeneration: GenerationId | null;
  nextGeneration: GenerationId;
  lastError: unknown;
}

export interface PageGeneration<T> {
  id: GenerationId;
  page: PageKey;
  value: T;
  createdAt: number;
}

/**
 * Canonicalize a source path before it enters the graph.
 *
 * The Bun native loader applies the same policy to its module registry. The
 * lower-case step is intentionally limited to Windows, where drive-letter and
 * case variants must refer to one graph node.
 */
export function canonicalModuleKey(filepath: string): ModuleKey {
  const normalized = normalize(resolve(filepath)).replaceAll("\\", "/");
  return process.platform === "win32" ? normalized.toLowerCase() : normalized;
}

export class TspModuleGraph {
  private readonly modules = new Map<ModuleKey, TspModuleNode>();
  private readonly pages = new Map<PageKey, TspPageSlot>();

  private getOrCreateModule(key: ModuleKey): TspModuleNode {
    const existing = this.modules.get(key);
    if (existing) return existing;

    const node: TspModuleNode = {
      key,
      dependencies: new Set(),
      dependents: new Set(),
      owningPages: new Set(),
      dirty: false,
    };
    this.modules.set(key, node);
    return node;
  }

  registerPage(pagePath: string, rootModulePath: string): TspPageSlot {
    const key = canonicalModuleKey(pagePath);
    const rootModule = canonicalModuleKey(rootModulePath);
    const current = this.pages.get(key);
    if (current) {
      current.rootModule = rootModule;
      this.getOrCreateModule(rootModule).owningPages.add(key);
      return current;
    }

    const slot: TspPageSlot = {
      key,
      rootModule,
      dirty: false,
      loading: false,
      currentGeneration: null,
      nextGeneration: 1,
      lastError: null,
    };
    this.pages.set(key, slot);
    this.getOrCreateModule(rootModule).owningPages.add(key);
    return slot;
  }

  updateModule(modulePath: string, dependencyPaths: string[]): TspModuleNode {
    const key = canonicalModuleKey(modulePath);
    const node = this.getOrCreateModule(key);
    const nextDependencies = new Set(dependencyPaths.map(canonicalModuleKey));

    for (const oldDependency of node.dependencies) {
      if (!nextDependencies.has(oldDependency)) {
        this.modules.get(oldDependency)?.dependents.delete(key);
      }
    }

    node.dependencies = nextDependencies;
    for (const dependency of nextDependencies) {
      this.getOrCreateModule(dependency).dependents.add(key);
    }
    return node;
  }

  attachPageModules(pagePath: string, modulePaths: string[]): void {
    const page = canonicalModuleKey(pagePath);
    for (const modulePath of modulePaths) {
      this.getOrCreateModule(canonicalModuleKey(modulePath)).owningPages.add(page);
    }
  }

  /** Mark a leaf and every reverse-dependent parent dirty. */
  markChanged(modulePath: string): Set<PageKey> {
    const changed = canonicalModuleKey(modulePath);
    const queue = [changed];
    const visited = new Set<ModuleKey>();
    const dirtyPages = new Set<PageKey>();

    while (queue.length > 0) {
      const key = queue.shift()!;
      if (visited.has(key)) continue;
      visited.add(key);

      const node = this.getOrCreateModule(key);
      node.dirty = true;
      for (const pageKey of node.owningPages) {
        const page = this.pages.get(pageKey);
        if (page) page.dirty = true;
        dirtyPages.add(pageKey);
      }
      queue.push(...node.dependents);
    }

    return dirtyPages;
  }

  beginReload(pagePath: string): TspPageSlot {
    const key = canonicalModuleKey(pagePath);
    const page = this.pages.get(key);
    if (!page) throw new Error(`Unknown TSP page: ${pagePath}`);
    if (page.loading) throw new Error(`TSP page is already reloading: ${pagePath}`);
    page.loading = true;
    return page;
  }

  publish<T>(pagePath: string, value: T): PageGeneration<T> {
    const key = canonicalModuleKey(pagePath);
    const page = this.pages.get(key);
    if (!page) throw new Error(`Unknown TSP page: ${pagePath}`);

    const generation: PageGeneration<T> = {
      id: page.nextGeneration++,
      page: key,
      value,
      createdAt: Date.now(),
    };
    page.currentGeneration = generation.id;
    page.dirty = false;
    page.loading = false;
    page.lastError = null;
    return generation;
  }

  failReload(pagePath: string, error: unknown): void {
    const page = this.pages.get(canonicalModuleKey(pagePath));
    if (!page) throw new Error(`Unknown TSP page: ${pagePath}`);
    page.loading = false;
    page.lastError = error;
    // Keep dirty=true and currentGeneration unchanged: last-known-good wins.
    page.dirty = true;
  }

  getPage(pagePath: string): TspPageSlot | undefined {
    return this.pages.get(canonicalModuleKey(pagePath));
  }

  getModule(modulePath: string): TspModuleNode | undefined {
    return this.modules.get(canonicalModuleKey(modulePath));
  }

  dirtyPages(): TspPageSlot[] {
    return [...this.pages.values()].filter((page) => page.dirty);
  }

  inspectPage(pagePath: string): {
    generation: GenerationId | null;
    dirty: boolean;
    loading: boolean;
    modules: string[];
    lastError: unknown;
  } | undefined {
    const page = this.getPage(pagePath);
    if (!page) return undefined;

    const modules = new Set<ModuleKey>();
    const queue = [page.rootModule];
    while (queue.length > 0) {
      const key = queue.shift()!;
      if (modules.has(key)) continue;
      modules.add(key);
      queue.push(...(this.modules.get(key)?.dependencies ?? []));
    }

    return {
      generation: page.currentGeneration,
      dirty: page.dirty,
      loading: page.loading,
      modules: [...modules],
      lastError: page.lastError,
    };
  }
}
