/**
 * Request body validation module
 * Parses and validates request data from InternalPageContext._body
 */

import type { ZodSchema } from "zod";
import type { InternalPageContext } from "./context.ts";

/**
 * Create body validation function
 * Parses and validates request data from InternalPageContext._body
 *
 * @param ctx - Internal page context (contains _body field)
 * @returns Validation function
 *
 * @example
 * ```tsx
 * // Registered in injection-typed.ts
 * registerDep("body", (ctx: InternalPageContext) => createBody(ctx));
 *
 * // Usage in TSX
 * export default Page(async function(ctx, { body, response }) {
 *   const userData = body(z.object({
 *     name: z.string().min(2),
 *     email: z.string().email()
 *   }));
 *   return response.json({ success: true, data: userData });
 * });
 * ```
 */
export function createBody(ctx: InternalPageContext) {
  return <T>(schema: ZodSchema<T>): T => {
    return schema.parse(ctx._body);
  };
}
