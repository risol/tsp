import { MySQLClientImpl } from "./client.ts";

/**
 * MySQL 客户端工厂函数
 * 用于创建 MySQL 客户端实例
 *
 * @param config - 数据库连接配置
 * @returns MySQL 客户端实例
 *
 * @example
 * ```tsx
 * export default Page(async function(ctx, { createMySQL, response }) {
 *   const db = await createMySQL({
 *     host: '127.0.0.1',
 *     port: 3306,
 *     user: 'test_user',
 *     password: 'test123456',
 *     database: 'test_db'
 *   });
 *
 *   const users = await db.query('SELECT * FROM users');
 *   return response.json(users);
 * });
 * ```
 */
export async function createMySQL(config: globalThis.MySQLConfig): Promise<globalThis.MySQLClient> {
  return new MySQLClientImpl(config);
}
