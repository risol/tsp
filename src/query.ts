/**
 * Query parameter validation module
 * Parses and validates query parameters from InternalPageContext._query
 */

import type { ZodSchema } from "zod";
import type { InternalPageContext } from "./context.ts";

/**
 * Create query validation function
 * Parses and validates query parameters from InternalPageContext._query
 *
 * @param ctx - Internal page context (contains _query field)
 * @returns Validation function
 *
 * @example
 * ```tsx
 * // Registered in injection-typed.ts
 * registerDep("query", (ctx: InternalPageContext) => createQuery(ctx));
 *
 * // Usage in TSX
 * export default Page(async function(ctx, { query }) {
 *   const { page, limit, keyword } = query(z.object({
 *     page: z.coerce.number().min(1).default(1),
 *     limit: z.coerce.number().min(1).max(100).default(10),
 *     keyword: z.string().optional()
 *   }));
 *
 *   return <div>Page {page}, {limit} items per page</div>;
 * });
 * ```
 */
export function createQuery(ctx: InternalPageContext) {
  return <T>(schema: ZodSchema<T>): T => {
    return schema.parse(ctx._query);
  };
}
