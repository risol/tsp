/**
 * Zod error formatting tool
 * Converts ZodError to user-friendly error messages
 */

import type { ZodError } from "zod";

/**
 * Format Zod error to user-friendly error message
 *
 * @param error - Zod error object
 * @returns Formatted error object
 *
 * @example
 * ```tsx
 * export default Page(async function(ctx, { createZod, body, formatZodError, response }) {
 *   const z = await createZod();
 *   try {
 *     const userData = body(z.object({
 *       name: z.string().min(2),
 *       email: z.string().email()
 *     }));
 *     return response.json({ success: true, data: userData });
 *   } catch (error) {
 *     if (error instanceof z.ZodError) {
 *       return response.json(formatZodError(error), 400);
 *     }
 *     throw error;
 *   }
 * });
 * ```
 */
export function formatZodError(error: ZodError) {
  return {
    success: false,
    message: "Data validation failed",
    errors: error.errors.map((e) => ({
      path: e.path.map(String),
      message: e.message,
      code: e.code,
    })),
  };
}

/**
 * Create format error function (for dependency injection)
 */
export function createFormatZodError() {
  return formatZodError;
}
