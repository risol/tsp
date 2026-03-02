import { RedisClientImpl } from "./client.ts";

/**
 * Redis client factory function
 * Used to create Redis client instances
 *
 * @param config - Redis connection configuration
 * @returns Redis client instance
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
