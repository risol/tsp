import { stat, readFile } from "node:fs/promises";
import { dirname, extname, normalize, resolve } from "node:path";

export type ModuleKey = string;
export type PageKey = string;
export type GenerationId = number;

export interface TspModuleNode {
  key: ModuleKey;
  dependencies: Set<ModuleKey>;
  dependents: Set<ModuleKey>;
  owningPages: Set<PageKey>;
  dirty: boolean;
  sourceStamp: SourceStamp | null;
}

export interface SourceStamp {
  mtimeMs: number;
  size: number;
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
  private readonly pageModules = new Map<PageKey, Set<ModuleKey>>();

  private getOrCreateModule(key: ModuleKey): TspModuleNode {
    const existing = this.modules.get(key);
    if (existing) return existing;

    const node: TspModuleNode = {
      key,
      dependencies: new Set(),
      dependents: new Set(),
      owningPages: new Set(),
      dirty: false,
      sourceStamp: null,
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
    const previous = this.pageModules.get(page) ?? new Set<ModuleKey>();
    for (const modulePath of previous) {
      this.modules.get(modulePath)?.owningPages.delete(page);
    }

    const next = new Set(modulePaths.map(canonicalModuleKey));
    this.pageModules.set(page, next);
    for (const modulePath of next) {
      this.getOrCreateModule(modulePath).owningPages.add(page);
    }
  }

  setSourceStamp(modulePath: string, sourceStamp: SourceStamp | null): void {
    this.getOrCreateModule(canonicalModuleKey(modulePath)).sourceStamp = sourceStamp;
  }

  /**
   * Discover static file dependencies for one page without evaluating code.
   * Bare package imports remain outside the TSP page graph and are owned by
   * Bun's normal resolver/module cache.
   */
  async discoverPage(pagePath: string, scopeRoot: string): Promise<void> {
    const page = canonicalModuleKey(pagePath);
    const root = canonicalModuleKey(scopeRoot);
    const discovered = new Map<ModuleKey, ModuleKey[]>();
    const stamps = new Map<ModuleKey, SourceStamp>();
    const visiting = new Set<ModuleKey>();

    const visit = async (modulePath: string): Promise<void> => {
      const key = canonicalModuleKey(modulePath);
      if (discovered.has(key) || visiting.has(key)) return;
      if (!isWithinScope(key, root)) return;

      visiting.add(key);
      let source: string;
      let fileStat: { mtimeMs: number; size: number };
      try {
        [source, fileStat] = await Promise.all([
          readFile(key, "utf8"),
          stat(key),
        ]);
      } catch {
        visiting.delete(key);
        return;
      }

      const dependencies: ModuleKey[] = [];
      for (const specifier of collectStaticSpecifiers(source)) {
        const dependency = await resolveSourceSpecifier(key, specifier);
        if (!dependency || !isWithinScope(dependency, root)) continue;
        dependencies.push(dependency);
        await visit(dependency);
      }

      discovered.set(key, dependencies);
      stamps.set(key, { mtimeMs: fileStat.mtimeMs, size: fileStat.size });
      visiting.delete(key);
    };

    await visit(page);
    this.registerPage(page, page);
    this.attachPageModules(page, [...discovered.keys()]);
    for (const [modulePath, dependencies] of discovered) {
      this.updateModule(modulePath, dependencies);
      this.setSourceStamp(modulePath, stamps.get(modulePath) ?? null);
    }
  }

  /** Check known source files and propagate changes to owning pages. */
  async refreshChangedModules(): Promise<Set<PageKey>> {
    const dirtyPages = new Set<PageKey>();
    for (const node of this.modules.values()) {
      if (!node.sourceStamp) continue;

      try {
        const fileStat = await stat(node.key);
        const nextStamp = { mtimeMs: fileStat.mtimeMs, size: fileStat.size };
        if (
          nextStamp.mtimeMs !== node.sourceStamp.mtimeMs ||
          nextStamp.size !== node.sourceStamp.size
        ) {
          node.sourceStamp = nextStamp;
          for (const page of this.markChanged(node.key)) dirtyPages.add(page);
        }
      } catch {
        // A missing file is still a source change. Let Bun report the stable
        // module-not-found error when the affected page is requested.
        node.sourceStamp = null;
        for (const page of this.markChanged(node.key)) dirtyPages.add(page);
      }
    }
    return dirtyPages;
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

  getPageModulePaths(pagePath: string): string[] {
    const page = this.getPage(pagePath);
    if (!page) return [];

    const modules = new Set<ModuleKey>();
    const queue = [page.rootModule];
    while (queue.length > 0) {
      const key = queue.shift()!;
      if (modules.has(key)) continue;
      modules.add(key);
      queue.push(...(this.modules.get(key)?.dependencies ?? []));
    }
    return [...modules];
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

    return {
      generation: page.currentGeneration,
      dirty: page.dirty,
      loading: page.loading,
      modules: this.getPageModulePaths(pagePath),
      lastError: page.lastError,
    };
  }
}

const SOURCE_EXTENSIONS = [".tsp", ".tsx", ".ts", ".jsx", ".js"] as const;

function isWithinScope(modulePath: string, scopeRoot: string): boolean {
  const moduleKey = canonicalModuleKey(modulePath);
  const scopeKey = canonicalModuleKey(scopeRoot).replace(/\/$/, "");
  return moduleKey === scopeKey || moduleKey.startsWith(`${scopeKey}/`);
}

function collectStaticSpecifiers(source: string): string[] {
  const specifiers = new Set<string>();
  const importPattern = /\b(?:import|export)\s+(?:type\s+)?(?:[\s\S]*?\s+from\s+)?["']([^"']+)["']/g;
  const dynamicImportPattern = /\bimport\s*\(\s*["']([^"']+)["']\s*\)/g;

  for (const pattern of [importPattern, dynamicImportPattern]) {
    for (const match of source.matchAll(pattern)) {
      const specifier = match[1];
      if (specifier?.startsWith(".")) specifiers.add(specifier);
    }
  }
  return [...specifiers];
}

async function resolveSourceSpecifier(
  importer: string,
  specifier: string,
): Promise<string | null> {
  const candidate = resolve(dirname(importer), specifier);
  const candidates = extname(candidate)
    ? [candidate]
    : [candidate, ...SOURCE_EXTENSIONS.map((extension) => `${candidate}${extension}`), ...SOURCE_EXTENSIONS.map((extension) => `${candidate}/index${extension}`)];

  for (const path of candidates) {
    try {
      const fileStat = await stat(path);
      if (fileStat.isFile()) return canonicalModuleKey(path);
    } catch {
      // Try the next supported source extension.
    }
  }
  return null;
}
