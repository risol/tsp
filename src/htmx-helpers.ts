/**
 * htmx integration helpers.
 *
 * Three small building blocks that make htmx usage on top of TSP's
 * fragment routing feel native:
 *
 *   hxUrl(page, name)       -> "/users.tsp/__fragment/table"
 *   <HtmxScript />           -> <script src=/__static/htmx.js>
 *                             + optional <meta name=htmx-config>
 *   <HtmxFragment ... />     -> <div hx-get=... hx-trigger=...>
 *                             with the initial content auto-fetched
 *                             from the same page's `fragments[name]`
 *
 * The auto-fetch happens during a JSX tree pre-walk in
 * `resolveHtmxFragments` (called by main.ts before renderToString).
 * If the user passes explicit children to HtmxFragment, those win
 * and no lookup happens.
 */

import {
  Fragment,
  cloneElement,
  createElement,
  type ReactNode,
} from "react";

/**
 * Public URL of the vendored htmx client. Must match the route
 * served by main.ts:handleRequest.
 */
export const HTMX_ASSET_PATH = "/__static/htmx.js";

/**
 * Fragment URL convention; mirrors FRAGMENT_MARKER in router.ts.
 */
const FRAGMENT_MARKER = "/__fragment/";

/**
 * Build a fragment URL from a page path and a fragment name.
 *
 * The page argument accepts either:
 *   - "/users"        -> appends ".tsp" automatically
 *   - "/users.tsp"    -> used as-is
 *   - "users"         -> prepended with "/" + appended with ".tsp"
 *
 * The name is passed through. Empty / invalid names throw so typos
 * surface immediately rather than producing silently broken URLs.
 */
export function hxUrl(page: string, name: string): string {
  if (!name) {
    throw new Error("hxUrl: fragment name is required");
  }
  let normalized = page.startsWith("/") ? page : "/" + page;
  if (!normalized.endsWith(".tsp")) {
    normalized = normalized + ".tsp";
  }
  return normalized + FRAGMENT_MARKER + name;
}

/**
 * Subset of htmx config options that the meta tag accepts.
 * Keep this list small on purpose — full htmx config has 30+ fields
 * and most are noise to a TSP user.
 */
export interface HtmxConfigOptions {
  defaultSwap?: string;
  defaultSwapDelay?: number;
  defaultSettleDelay?: number;
  timeout?: number;
  historyCacheSize?: number;
  withCredentials?: boolean;
  indicatorClass?: string;
  inlineScriptNonce?: string;
}

/**
 * <HtmxScript /> renders the vendored htmx client and, if any config
 * option is set, a `<meta name="htmx-config">` tag right after it.
 * Drop one of these in <head> to opt a page into htmx.
 */
export function HtmxScript(props: HtmxConfigOptions = {}): ReactNode {
  const entries: Record<string, unknown> = {};
  if (props.defaultSwap !== undefined) entries.defaultSwapStyle = props.defaultSwap;
  if (props.defaultSwapDelay !== undefined) {
    entries.defaultSwapDelay = props.defaultSwapDelay;
  }
  if (props.defaultSettleDelay !== undefined) {
    entries.defaultSettleDelay = props.defaultSettleDelay;
  }
  if (props.timeout !== undefined) entries.timeout = props.timeout;
  if (props.historyCacheSize !== undefined) {
    entries.historyCacheSize = props.historyCacheSize;
  }
  if (props.withCredentials !== undefined) {
    entries.withCredentials = props.withCredentials;
  }
  if (props.indicatorClass !== undefined) {
    entries.indicatorClass = props.indicatorClass;
  }
  if (props.inlineScriptNonce !== undefined) {
    entries.inlineScriptNonce = props.inlineScriptNonce;
  }

  const children: ReactNode[] = [
    createElement("script", { src: HTMX_ASSET_PATH, key: "htmx-js" }),
  ];
  if (Object.keys(entries).length > 0) {
    children.push(
      createElement("meta", {
        key: "htmx-config",
        name: "htmx-config",
        content: JSON.stringify(entries),
      }),
    );
  }
  return createElement(Fragment, null, children);
}

export interface HtmxFragmentProps {
  /** Page path the fragment lives on, e.g. "/users" or "/users.tsp". */
  page: string;
  /** Named export from that page's `fragments` map. */
  name: string;
  /** hx-trigger value, e.g. "every 5s" or "click from:#btn". */
  trigger?: string;
  /** hx-swap value, defaults to "outerHTML". */
  swap?: string;
  /** hx-target CSS selector, defaults to the wrapper itself. */
  target?: string;
  /** hx-include CSS selector for additional inputs. */
  include?: string;
  /** hx-confirm prompt. */
  confirm?: string;
  /**
   * Explicit initial content. If omitted, the framework calls
   * `fragments[name](ctx)` from the same page during SSR.
   */
  children?: ReactNode;
}

/**
 * <HtmxFragment name="table" page="/users" trigger="every 5s" />
 * renders a <div hx-get="/users.tsp/__fragment/table" ...> with the
 * initial content pre-resolved from `fragments.table(ctx)`.
 *
 * If children are passed explicitly, the framework does NOT call
 * fragments[name] and just uses what the user supplied.
 */
export function HtmxFragment(props: HtmxFragmentProps): ReactNode {
  const url = hxUrl(props.page, props.name);
  const attrs: Record<string, string> = { "hx-get": url };
  if (props.trigger !== undefined) attrs["hx-trigger"] = props.trigger;
  if (props.swap !== undefined) attrs["hx-swap"] = props.swap;
  if (props.target !== undefined) attrs["hx-target"] = props.target;
  if (props.include !== undefined) attrs["hx-include"] = props.include;
  if (props.confirm !== undefined) attrs["hx-confirm"] = props.confirm;
  // `id` is useful for hx-target="this" and CSS hooks; allow a passthrough
  // via any extra props if we ever extend this.
  return createElement("div", attrs, props.children);
}

/**
 * Type guard for React elements. We avoid a hard dep on `react` types
 * beyond the named imports above so this file stays portable.
 */
function isElement(value: unknown): value is { type: unknown; props: Record<string, unknown> } {
  return typeof value === "object" && value !== null && "type" in (value as object) &&
    "props" in (value as object);
}

/**
 * Pre-walk a JSX tree returned by a page function. Every HtmxFragment
 * node is replaced with one whose children are the result of calling
 * `fragments[name](ctx)`. Other nodes are recursed into so nested
 * HtmxFragments are also resolved.
 *
 * If children were passed explicitly to HtmxFragment, they win and
 * no fragment lookup is performed. A missing fragment name produces
 * a console warning in dev and a null children placeholder in the
 * rendered tree.
 */
export async function resolveHtmxFragments(jsx: unknown, ctx: unknown): Promise<unknown> {
  // Primitives pass through.
  if (
    jsx === null || jsx === undefined || typeof jsx === "string" ||
    typeof jsx === "number" || typeof jsx === "boolean"
  ) {
    return jsx;
  }

  // Arrays: recurse over each element.
  if (Array.isArray(jsx)) {
    const out = await Promise.all(jsx.map((child) => resolveHtmxFragments(child, ctx)));
    return out;
  }

  // React element: special-case HtmxFragment, otherwise recurse.
  if (isElement(jsx)) {
    if (jsx.type === HtmxFragment) {
      const props = jsx.props as Record<string, unknown>;
      const hasExplicitChildren = props.children !== undefined &&
        !(Array.isArray(props.children) && props.children.length === 0);
      let resolvedChildren: unknown;
      if (hasExplicitChildren) {
        resolvedChildren = await resolveHtmxFragments(props.children, ctx);
      } else {
        const fragments = (ctx as { _fragments?: Record<string, (c: unknown) => Promise<unknown>> })._fragments ?? {};
        const fragFn = fragments[String(props.name)];
        if (!fragFn) {
          console.warn(
            `[HtmxFragment] fragment "${props.name}" not found in this page; ` +
              `add it to the \`fragments\` export or pass \`children\` explicitly.`,
          );
          resolvedChildren = null;
        } else {
          resolvedChildren = await fragFn(ctx);
        }
      }
      return cloneElement(jsx, { children: resolvedChildren } as Record<string, unknown>);
    }
    // Recurse into the children of any other element.
    if (jsx.props && "children" in jsx.props && jsx.props.children !== undefined) {
      const newChildren = await resolveHtmxFragments(jsx.props.children, ctx);
      return cloneElement(jsx, { children: newChildren } as Record<string, unknown>);
    }
    return jsx;
  }

  return jsx;
}
