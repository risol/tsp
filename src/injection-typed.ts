/**
 * Type-safe dependency injection module
 * Automatically infers types from function signatures, provides complete type hints
 *
 * Usage:
 * 1. Declare dependency types in AppDeps interface in types.d.ts
 * 2. Register dependency implementations in main.ts using registerDep()
 * 3. Use global Page() in TSX files (no import needed)
 */

import type { PageContext, InternalPageContext } from "./context.ts";
import { createBody } from "./body.ts";
import { createQuery } from "./query.ts";
import { createFormatZodError } from "./zod.ts";
import { createTypesRegistry } from "./types.ts";

// Zod instance cache
let zodInstance: typeof import("zod") | null = null;

/**
 * Create Zod factory function
 * Returns Zod library instance
 */
async function createZodFactory(): Promise<typeof import("zod")> {
  if (!zodInstance) {
    const zod = await import("zod");
    zodInstance = zod;
  }
  // Return non-null value
  return zodInstance!;
}

// Nanoid instance cache
let nanoidInstance: ((size?: number) => string) | null = null;

/**
 * Create Nanoid factory function
 * Returns nanoid function
 */
async function createNanoidFactory(): Promise<(size?: number) => string> {
  if (!nanoidInstance) {
    const { nanoid } = await import("nanoid");
    nanoidInstance = nanoid;
  }
  // Return non-null value
  return nanoidInstance!;
}

// ============================================
// Type declarations are in types.d.ts
// ============================================

/**
 * Dependency builder registry (internal use)
 */
const depBuilders = new Map<
  keyof AppDeps,
  (ctx: InternalPageContext) => Promise<unknown> | unknown
>();

/**
 * Register a single dependency (with type inference)
 *
 * @example
 * ```typescript
 * // main.ts
 * import { registerDep } from "./src/injection-typed.ts";
 *
 * registerDep('testFunc', () => {
 *   console.log('testFunc called');
 *   return 'testFunc called';
 * });
 * ```
 */
export function registerDep<K extends keyof AppDeps>(
  name: K,
  builder: (ctx: InternalPageContext) => Promise<AppDeps[K]> | AppDeps[K],
): void {
  depBuilders.set(name, builder);
}

/**
 * Batch register dependencies
 *
 * @example
 * ```typescript
 * // main.ts
 * import { registerDeps } from "./src/injection-typed.ts";
 *
 * registerDeps({
 *   testFunc: () => {
 *     console.log('testFunc called');
 *     return 'testFunc called';
 *   },
 *   createMySQL: (ctx) => createMySQL,
 * });
 * ```
 */
export function registerDeps<
  T extends Record<string, (ctx: InternalPageContext) => unknown>,
>(
  deps: T,
): void {
  for (const [name, builder] of Object.entries(deps)) {
    depBuilders.set(
      name as keyof AppDeps,
      builder as (ctx: InternalPageContext) => unknown,
    );
  }
}

/**
 * Unregister a dependency
 */
export function unregisterDep(name: keyof AppDeps): void {
  depBuilders.delete(name);
}

/**
 * Get list of registered dependencies
 */
export function getRegisteredDeps(): string[] {
  return Array.from(depBuilders.keys()) as string[];
}

/**
 * Page wrapper
 * This is a higher-order function that returns a wrapper function for automatic dependency injection
 *
 * @example
 * ```tsx
 * // Simplified usage (recommended) - auto lazy-load dependencies
 * export default Page(async function(ctx, { session, createMySQL }) {
 *   const user = await session.getUser();
 *   const db = await createMySQL({
 *     host: '127.0.0.1',
 *     user: 'test_user',
 *     password: 'test123456',
 *     database: 'test_db'
 *   });
 *   const data = await db.query('SELECT * FROM users');
 *   return <div>{data}</div>;
 * });
 *
 * // Dependencies are only built when accessed, unaccessed dependencies won't be built
 * export default Page(async function(ctx, { logger }) {
 *   // Only logger is built, other dependencies won't be built
 *   logger('Page loaded');
 *   return <div>Hello</div>;
 * });
 * ```
 */
export function createPage() {
  /**
   * Create lazy-loading dependency Proxy
   *
   * Dependencies are only built when accessed
   * Built dependencies are cached
   *
   * @param ctx - Page context
   * @returns Proxy object
   */
  function createLazyDeps(ctx: InternalPageContext): AppDeps {
    const cache = new Map<string, unknown>();
    const resolvedDeps: Partial<AppDeps> = {};

    return new Proxy(resolvedDeps as AppDeps, {
      get(target, prop: string) {
        // If it's a Symbol or special property, return directly
        if (typeof prop === "symbol") {
          return (target as Record<string, unknown>)[prop];
        }

        // Return cached dependency
        if (cache.has(prop)) {
          return cache.get(prop);
        }

        // Check if dependency is registered
        const builder = depBuilders.get(prop as keyof AppDeps);
        if (!builder) {
          throw new Error(
            `Dependency "${prop}" is requested but not registered. ` +
              `Please register it using registerDep('${prop}', builder) in main.ts. ` +
              `Available deps: ${Array.from(depBuilders.keys()).join(", ")}`,
          );
        }

        // Build dependency
        const dep = builder(ctx);

        // Fix: If it's a Promise, return the Promise directly
        // This ensures await session.getUser() still works in user code
        if (dep instanceof Promise) {
          // Cache the Promise first to avoid repeated building
          cache.set(prop, dep);

          // Return the Promise directly
          // User code can use: const session = await deps.session
          return dep as unknown;
        }

        // Synchronous dependencies are cached directly
        cache.set(prop, dep);
        (target as Record<string, unknown>)[prop] = dep;

        return dep;
      },

      // Support 'in' operator
      has(target, prop: string) {
        if (typeof prop === "symbol") {
          return prop in target;
        }
        return depBuilders.has(prop as keyof AppDeps);
      },

      // Support Object.keys() and similar operations
      ownKeys() {
        return Array.from(depBuilders.keys()).map(String);
      },

      // Support getOwnPropertyDescriptor
      getOwnPropertyDescriptor(target, prop: string) {
        if (typeof prop === "symbol") {
          return Object.getOwnPropertyDescriptor(target, prop);
        }
        if (depBuilders.has(prop as keyof AppDeps)) {
          return {
            enumerable: true,
            configurable: true,
            value: (target as Record<string, unknown>)[prop],
          };
        }
        return undefined;
      },
    });
  }

  /**
   * Wrap page function, auto-inject dependencies (lazy-loading version)
   *
   * @param fn - Page function
   * @returns Wrapped page function
   */
  function Page<T>(
    fn: (ctx: PageContext, deps: AppDeps) => Promise<T> | T,
  ): (ctx: InternalPageContext) => Promise<T> {
    return async (ctx: InternalPageContext) => {
      // Create lazy-loading dependency Proxy
      const deps = createLazyDeps(ctx);

      // Call original function, inject lazy-loading dependencies (convert InternalPageContext to PageContext)
      return fn(ctx as PageContext, deps);
    };
  }

  return Page;
}

/**
 * Initialize global Page
 * Automatically executes when module is loaded, no manual call needed
 */
export function initGlobalPage(): void {
  if (typeof (globalThis as any).Page === "undefined") {
    (globalThis as any).Page = createPage();
  }
}

// Auto-initialize (executes when module is loaded)
initGlobalPage();

// Export Page
const _pageFn = createPage();
export const Page = _pageFn;

// ============================================
// Register default dependencies (types, body, query, zod)
// ============================================

/**
 * Register types dependency
 * Provides all business type definitions (PageContext, RedirectResult, etc.)
 *
 * Note: These types are processed by TypeScript type system at compile time
 * Returns type metadata object at runtime
 */
registerDep("types", () => createTypesRegistry());

/**
 * Register createZod factory function
 * Provides full functionality of Zod validation library
 */
registerDep("createZod", () => createZodFactory);

/**
 * Register createNanoid factory function
 * Provides nanoid unique ID generation functionality
 */
registerDep("createNanoid", () => createNanoidFactory);

/**
 * Register body dependency
 * Parses and validates request data from InternalPageContext._body
 */
registerDep("body", (ctx: InternalPageContext) => createBody(ctx));

/**
 * Register query dependency
 * Parses and validates query parameters from InternalPageContext._query
 */
registerDep("query", (ctx: InternalPageContext) => createQuery(ctx));

/**
 * Register formatZodError dependency
 * Provides user-friendly Zod error formatting functionality
 */
registerDep("formatZodError", () => createFormatZodError());

// Default export
export default {
  registerDep,
  registerDeps,
  unregisterDep,
  getRegisteredDeps,
  createPage,
  initGlobalPage,
  Page,
};
