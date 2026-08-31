/** Public TypeScript surface for TSP route modules. */

declare module "tsp:html" {
  export interface HtmlNode { readonly __brand?: "HtmlNode"; }
  export interface TrustedHtml { readonly __brand?: "TrustedHtml"; }
  export function raw(value: string): TrustedHtml;
  export function escape(value: unknown): string;
}

declare module "tsp:server" {
  import type { HtmlNode, TrustedHtml } from "tsp:html";
  export interface CookieOptions {
    maxAge?: number; expires?: Date; path?: string; domain?: string;
    secure?: boolean; httpOnly?: boolean; sameSite?: "strict" | "lax" | "none";
  }
  export interface Cookies {
    get(name: string): string | undefined;
    set(name: string, value: string, options?: CookieOptions): void;
    delete(name: string, options?: CookieOptions): void;
  }
  export interface TspRequest {
    readonly method: string; readonly headers: Headers; readonly url: string;
    text(): Promise<string>; json<T = unknown>(): Promise<T>;
    formData(): Promise<FormData>; arrayBuffer(): Promise<ArrayBuffer>;
  }
  export interface Session {
    readonly id: string | null;
    get<T = unknown>(key: string): T | undefined;
    set(key: string, value: unknown): void; delete(key: string): void;
    clear(): void; regenerate(): Promise<void>; destroy(): Promise<void>;
  }
  export interface ServiceRegistry { readonly [name: string]: unknown; }
  export interface RouteInfo {
    readonly path: string; readonly methods: readonly string[];
    readonly source?: string; readonly generation?: number;
  }
  export interface Context<S extends ServiceRegistry = ServiceRegistry> {
    readonly request: TspRequest; readonly url: URL; readonly method: string;
    readonly params: Readonly<Record<string, string>>; readonly query: URLSearchParams;
    readonly cookies: Cookies; readonly session: Session; readonly services: S;
    readonly signal: AbortSignal; readonly route: RouteInfo;
    fragment(name: string, params?: Record<string, string>): string;
  }
  export type HandlerResult = Response | HtmlNode | TrustedHtml | string | number | null | undefined;
  export type PageHandler<S extends ServiceRegistry = ServiceRegistry> =
    (ctx: Context<S>) => HandlerResult | Promise<HandlerResult>;
  export interface FragmentOptions<S extends ServiceRegistry = ServiceRegistry> {
    method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE"; handler: PageHandler<S>;
  }
  export function fragment<S extends ServiceRegistry>(handler: PageHandler<S>): PageHandler<S>;
  export function fragment<S extends ServiceRegistry>(options: FragmentOptions<S>): PageHandler<S>;
  export function json(value: unknown, init?: ResponseInit): Response;
  export function redirect(location: string, status?: number): Response;
  export function text(value: string, init?: ResponseInit): Response;
  export function html(node: HtmlNode | TrustedHtml | string, init?: ResponseInit): Response;
  export function notFound(init?: ResponseInit): Response;
  export class HttpError extends Error {
    readonly status: number; constructor(status: number, message: string, init?: ResponseInit);
  }
}

declare global {
  namespace JSX {
    interface Element extends import("tsp:html").HtmlNode {}
    interface IntrinsicElements { [elementName: string]: Record<string, unknown>; }
  }
}

export {};
