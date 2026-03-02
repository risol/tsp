import { createClient } from "npm:redis@^4.6.0";

/**
 * Redis client implementation class
 * Provides complete Redis operations including basic operations, transactions, pub/sub, etc.
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
   * Ensure connection is established
   */
  private async ensureConnected(): Promise<void> {
    if (!this.isConnected) {
      await this.client.connect();
      this.isConnected = true;
    }
  }

  /**
   * Set key-value pair
   * @param key - Key
   * @param value - Value
   * @param ttl - Expiration time in seconds, optional
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
   * Get value by key
   * @param key - Key
   * @returns Value, or null if not exists
   */
  async get(key: string): Promise<string | null> {
    await this.ensureConnected();
    return await this.client.get(key);
  }

  /**
   * Delete keys
   * @param keys - One or more keys
   * @returns Number of keys deleted
   */
  async del(...keys: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.del(keys);
  }

  /**
   * Check if key exists
   * @param key - Key
   * @returns Whether exists
   */
  async exists(key: string): Promise<boolean> {
    await this.ensureConnected();
    const result = await this.client.exists(key);
    return result === 1;
  }

  /**
   * Set expiration time for key
   * @param key - Key
   * @param seconds - Expiration time in seconds
   * @returns Whether set successfully
   */
  async expire(key: string, seconds: number): Promise<boolean> {
    await this.ensureConnected();
    const result = await this.client.expire(key, seconds);
    // expire returns boolean (true: success, false: failure)
    return result === true;
  }

  /**
   * Get remaining time to live for key
   * @param key - Key
   * @returns Remaining seconds, -1 means never expires, -2 means key doesn't exist
   */
  async ttl(key: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.ttl(key);
  }

  /**
   * List operation: push elements to left of list
   */
  async lpush(key: string, ...values: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.lPush(key, values);
  }

  /**
   * List operation: push elements to right of list
   */
  async rpush(key: string, ...values: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.rPush(key, values);
  }

  /**
   * List operation: pop element from left of list
   */
  async lpop(key: string): Promise<string | null> {
    await this.ensureConnected();
    return await this.client.lPop(key);
  }

  /**
   * List operation: pop element from right of list
   */
  async rpop(key: string): Promise<string | null> {
    await this.ensureConnected();
    return await this.client.rPop(key);
  }

  /**
   * List operation: get range of elements from list
   */
  async lrange(key: string, start: number, stop: number): Promise<string[]> {
    await this.ensureConnected();
    return await this.client.lRange(key, start, stop);
  }

  /**
   * Set operation: add members to set
   */
  async sadd(key: string, ...members: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.sAdd(key, members);
  }

  /**
   * Set operation: get all members of set
   */
  async smembers(key: string): Promise<string[]> {
    await this.ensureConnected();
    return await this.client.sMembers(key);
  }

  /**
   * Set operation: remove and return a random member from set
   */
  async spop(key: string): Promise<string | null> {
    await this.ensureConnected();
    const result = await this.client.sPop(key);
    // sPop may return string[] or string, handle both cases
    return Array.isArray(result) ? (result[0] ?? null) : (result ?? null);
  }

  /**
   * Set operation: check if member is in set
   */
  async sismember(key: string, member: string): Promise<boolean> {
    await this.ensureConnected();
    return await this.client.sIsMember(key, member);
  }

  /**
   * Hash operation: set hash field value
   */
  async hset(key: string, field: string, value: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.hSet(key, field, value);
  }

  /**
   * Hash operation: get hash field value
   */
  async hget(key: string, field: string): Promise<string | null> {
    await this.ensureConnected();
    const result = await this.client.hGet(key, field);
    return result ?? null;
  }

  /**
   * Hash operation: get all fields and values from hash
   */
  async hgetall(key: string): Promise<Record<string, string>> {
    await this.ensureConnected();
    return await this.client.hGetAll(key);
  }

  /**
   * Hash operation: delete hash fields
   */
  async hdel(key: string, ...fields: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.hDel(key, fields);
  }

  /**
   * Sorted set operation: add or update member
   */
  async zadd(key: string, score: number, member: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.zAdd(key, { score, value: member });
  }

  /**
   * Sorted set operation: return members by score range
   */
  async zrange(key: string, start: number, stop: number): Promise<string[]> {
    await this.ensureConnected();
    return await this.client.zRange(key, start, stop);
  }

  /**
   * Sorted set operation: remove members
   */
  async zrem(key: string, ...members: string[]): Promise<number> {
    await this.ensureConnected();
    return await this.client.zRem(key, members);
  }

  /**
   * Increment operation
   */
  async incr(key: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.incr(key);
  }

  /**
   * Decrement operation
   */
  async decr(key: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.decr(key);
  }

  /**
   * Increment by specified value
   */
  async incrBy(key: string, increment: number): Promise<number> {
    await this.ensureConnected();
    return await this.client.incrBy(key, increment);
  }

  /**
   * Publish message to channel
   */
  async publish(channel: string, message: string): Promise<number> {
    await this.ensureConnected();
    return await this.client.publish(channel, message);
  }

  /**
   * Subscribe to channel
   * @param channel - Channel name
   * @param callback - Message callback function
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
   * Close connection
   */
  async close(): Promise<void> {
    if (this.isConnected) {
      await this.client.quit();
      this.isConnected = false;
    }
  }
}
