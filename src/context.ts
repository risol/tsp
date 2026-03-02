/**
 * Context handling module
 * Responsible for building page context objects
 */

import type { UploadedFile } from "./files.ts";

/**
 * HTTP request method type
 */
type HttpMethod =
  | "GET"
  | "POST"
  | "PUT"
  | "PATCH"
  | "DELETE"
  | "HEAD"
  | "OPTIONS";

/**
 * Page context parameters (input parameters for building context)
 */
interface ContextParams {
  method: HttpMethod;
  url: URL;
  headers: Headers;
  query: Record<string, string>;
  body: unknown;
  cookies: Record<string, string>;
  files: Record<string, UploadedFile | UploadedFile[]>;
  file: string;
  root: string;
}

/**
 * Page context type
 * Context object passed to each TSX page function
 */
export type PageContext = Readonly<{
  /** HTTP request method (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS) */
  method: HttpMethod;

  /** Complete request URL object */
  url: URL;

  /** Request headers object */
  headers: Headers;

  /** Cookie data (parsed as key-value pairs) */
  cookies: Record<string, string>;

  /** Uploaded files (multipart/form-data)
   * - Single file: UploadedFile
   * - Multiple files with same name: UploadedFile[]
   */
  files: Record<string, UploadedFile | UploadedFile[]>;

  /** Full path of current TSX page file (relative to project root) */
  file: string;

  /** Document root directory path */
  root: string;
}>;

/**
 * Internal page context type
 * Extends PageContext with internal fields (_query, _body)
 * Only used by dependency injection system, not accessible from external code
 */
export interface InternalPageContext extends PageContext {
  /** Internal use: query parameters */
  _query: Record<string, string>;
  /** Internal use: request body data */
  _body: unknown;
}

/**
 * Build page context object
 * @param params Context parameters
 * @returns Internal page context (includes _query and _body)
 */
export function buildContext(params: ContextParams): InternalPageContext {
  return {
    method: params.method,
    url: params.url,
    headers: params.headers,
    cookies: params.cookies,
    files: params.files,
    file: params.file,
    root: params.root,
    // Internal fields
    _query: params.query,
    _body: params.body,
  };
}
