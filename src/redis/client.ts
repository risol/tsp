import { createClient } from "npm:redis@^4.6.0";

/**
 * Redis 客户端实现类
 * 提供完整的 Redis 操作功能，包括基本操作、事务、发布订阅等
 */
export class RedisClientImpl implements globalThis.RedisClient {
  private client: ReturnType<typeof createClient>;
  private isConnected: boolean = false;

  constructor(config: globalThis.RedisConfig) {
    this.client = createClient({
      socket: {
        host: config.host,
        port: config.port || 6379,
      },
      password: config.password,
      database: config.database || 0,
    });
  }

  /**
   * 确保连接已建立
   */
  private async ensureConnected(): Promise<void> {
    if (!this.isConnected) {
      await this.client.connect();
      this.isConnected = true;
    }
  }

  /**
   * 设置键值对
   * @param key - 键
   * @param value - 值
   * @param ttl - 过期时间（秒），可选
   */
  async set(key: string, value: string, ttl?: number): Promise<void> {
    await this.ensureConnected();
    if (ttl) {
      await this.client.setEx(key, ttl, value);
    } else {
      await this.client.set(key, value);
    }
  }

  /**
   * 获取键值
   * @param key - 键
   * @returns 值，如果不存在则返回 null
   */
  async get(key: string): Promise<string | null> {
    await this.ensureConnected();
    return await this.client.get(key);
  }

  /**
   * 删除键
   * @param keys - 一个或多个键
   * @returns 删除的键数量
   */
  async del(...keys: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.del(keys);
  }

  /**
   * 检查键是否存在
   * @param key - 键
   * @returns 是否存在
   */
  async exists(key: string): Promise<boolean> {
    await this.ensureConnected();
    const result = await this.client.exists(key);
    return result === 1;
  }

  /**
   * 设置键的过期时间
   * @param key - 键
   * @param seconds - 过期时间（秒）
   * @returns 是否设置成功
   */
  async expire(key: string, seconds: number): Promise<boolean> {
    await this.ensureConnected();
    const result = await this.client.expire(key, seconds);
    // expire 返回 boolean (true: 成功, false: 失败)
    return result === true;
  }

  /**
   * 获取键的剩余生存时间
   * @param key - 键
   * @returns 剩余秒数，-1 表示永不过期，-2 表示键不存在
   */
  async ttl(key: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.ttl(key);
  }

  /**
   * 列表操作：在列表左侧推入元素
   */
  async lpush(key: string, ...values: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.lPush(key, values);
  }

  /**
   * 列表操作：在列表右侧推入元素
   */
  async rpush(key: string, ...values: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.rPush(key, values);
  }

  /**
   * 列表操作：从列表左侧弹出元素
   */
  async lpop(key: string): Promise<string | null> {
    await this.ensureConnected();
    return await this.client.lPop(key);
  }

  /**
   * 列表操作：从列表右侧弹出元素
   */
  async rpop(key: string): Promise<string | null> {
    await this.ensureConnected();
    return await this.client.rPop(key);
  }

  /**
   * 列表操作：获取列表指定范围内的元素
   */
  async lrange(key: string, start: number, stop: number): Promise<string[]> {
    await this.ensureConnected();
    return await this.client.lRange(key, start, stop);
  }

  /**
   * 集合操作：向集合添加成员
   */
  async sadd(key: string, ...members: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.sAdd(key, members);
  }

  /**
   * 集合操作：获取集合所有成员
   */
  async smembers(key: string): Promise<string[]> {
    await this.ensureConnected();
    return await this.client.sMembers(key);
  }

  /**
   * 集合操作：移除并返回集合中的一个随机成员
   */
  async spop(key: string): Promise<string | null> {
    await this.ensureConnected();
    const result = await this.client.sPop(key);
    // sPop 可能返回 string[] 或 string，处理两种情况
    return Array.isArray(result) ? (result[0] ?? null) : (result ?? null);
  }

  /**
   * 集合操作：检查成员是否在集合中
   */
  async sismember(key: string, member: string): Promise<boolean> {
    await this.ensureConnected();
    return await this.client.sIsMember(key, member);
  }

  /**
   * 哈希操作：设置哈希字段值
   */
  async hset(key: string, field: string, value: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.hSet(key, field, value);
  }

  /**
   * 哈希操作：获取哈希字段值
   */
  async hget(key: string, field: string): Promise<string | null> {
    await this.ensureConnected();
    const result = await this.client.hGet(key, field);
    return result ?? null;
  }

  /**
   * 哈希操作：获取哈希所有字段和值
   */
  async hgetall(key: string): Promise<Record<string, string>> {
    await this.ensureConnected();
    return await this.client.hGetAll(key);
  }

  /**
   * 哈希操作：删除哈希字段
   */
  async hdel(key: string, ...fields: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.hDel(key, fields);
  }

  /**
   * 有序集合操作：添加或更新成员
   */
  async zadd(key: string, score: number, member: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.zAdd(key, { score, value: member });
  }

  /**
   * 有序集合操作：按分数范围返回成员
   */
  async zrange(key: string, start: number, stop: number): Promise<string[]> {
    await this.ensureConnected();
    return await this.client.zRange(key, start, stop);
  }

  /**
   * 有序集合操作：移除成员
   */
  async zrem(key: string, ...members: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.zRem(key, members);
  }

  /**
   * 递增操作
   */
  async incr(key: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.incr(key);
  }

  /**
   * 递减操作
   */
  async decr(key: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.decr(key);
  }

  /**
   * 按指定值递增
   */
  async incrBy(key: string, increment: number): Promise<number> {
    await this.ensureConnected();
    return await this.client.incrBy(key, increment);
  }

  /**
   * 发布消息到频道
   */
  async publish(channel: string, message: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.publish(channel, message);
  }

  /**
   * 订阅频道
   * @param channel - 频道名
   * @param callback - 消息回调函数
   */
  async subscribe(
    channel: string,
    callback: (message: string) => void | Promise<void>
  ): Promise<void> {
    await this.ensureConnected();
    const subscriber = this.client.duplicate();
    await subscriber.connect();

    await subscriber.subscribe(channel, (message) => {
      callback(message);
    });
  }

  /**
   * 关闭连接
   */
  async close(): Promise<void> {
    if (this.isConnected) {
      await this.client.quit();
      this.isConnected = false;
    }
  }
}
