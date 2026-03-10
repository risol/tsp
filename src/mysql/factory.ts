import { MySQLClientImpl } from "./client.ts";

/**
 * MySQL client factory function
 * Used to create MySQL client instances
 *
 * @param config - Database connection configuration
 * @param zod - Zod instance (from dependency injection)
 * @returns MySQL client instance
 *
 * @example
 * ```tsx
 * export default Page(async function(ctx, { createMySQL, createZod, response }) {
 *   const z = await createZod();
 *   const db = await createMySQL({
 *     host: '127.0.0.1',
 *     port: 3306,
 *     user: 'test_user',
 *     password: 'test123456',
 *     database: 'test_db'
 *   }, z);
 *
 *   const UserSchema = z.object({
 *     id: z.number(),
 *     username: z.string(),
 *     email: z.string()
 *   });
 *
 *   // Schema-first query
 *   const users = await db.query(UserSchema, 'SELECT * FROM users WHERE age > ?', [18]);
 *
 *   return response.json({ users });
 * });
 * ```
 */
export async function createMySQL(
  config: globalThis.MySQLConfig,
  zod: unknown
): Promise<globalThis.MySQLClient> {
  // Validate zod parameter
  if (!zod || typeof (zod as any).object !== 'function') {
    throw new Error('createMySQL requires valid Zod instance as second parameter. Get it from dependency injection: { createMySQL, z }');
  }

  return new MySQLClientImpl(config, zod as any);
}
