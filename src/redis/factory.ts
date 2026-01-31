import { RedisClientImpl } from "./client.ts";

/**
 * Redis 客户端工厂函数
 * 用于创建 Redis 客户端实例
 *
 * @param config - Redis 连接配置
 * @returns Redis 客户端实例
 *
 * @example
 * ```tsx
 * export default Page(async function(ctx, { createRedis, response }) {
 *   const redis = await createRedis({
 *     host: '127.0.0.1',
 *     port: 6379,
 *     password: 'your_password',
 *     database: 0
 *   });
 *
 *   await redis.set('key', 'value');
 *   const value = await redis.get('key');
 *   return response.json({ key, value });
 * });
 * ```
 */
export async function createRedis(config: globalThis.RedisConfig): Promise<globalThis.RedisClient> {
  return new RedisClientImpl(config);
}
