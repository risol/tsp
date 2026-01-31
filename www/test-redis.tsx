export default Page(async function(ctx, { createRedis, response }) {
  const redis = await createRedis({
    host: '127.0.0.1',
    port: 6379,
    password: '',
    database: 0
  });

  const action = ctx.query.action || 'basic';

  try {
    switch (action) {
      case 'basic': {
        // 基本 get/set 测试
        await redis.set('test_key', 'Hello from Redis!');
        const value = await redis.get('test_key');
        await redis.del('test_key');

        return response.json({
          success: true,
          action: 'basic',
          message: '基本操作测试成功',
          data: { key: 'test_key', value }
        });
      }

      case 'expire': {
        // 过期时间测试
        await redis.set('expire_key', 'will_expire', 5);
        const ttl1 = await redis.ttl('expire_key');
        const exists1 = await redis.exists('expire_key');

        return response.json({
          success: true,
          action: 'expire',
          message: '过期时间测试成功',
          data: {
            key: 'expire_key',
            ttl: ttl1,
            exists: exists1,
            note: 'TTL 应该约为 5 秒'
          }
        });
      }

      case 'list': {
        // 列表操作测试
        const listKey = 'test_list';
        await redis.del(listKey);

        await redis.lpush(listKey, 'item3', 'item2', 'item1');
        const items = await redis.lrange(listKey, 0, -1);
        const leftItem = await redis.lpop(listKey);
        const rightItem = await redis.rpop(listKey);

        return response.json({
          success: true,
          action: 'list',
          message: '列表操作测试成功',
          data: {
            list: items,
            poppedLeft: leftItem,
            poppedRight: rightItem
          }
        });
      }

      case 'set': {
        // 集合操作测试
        const setKey = 'test_set';
        await redis.del(setKey);

        await redis.sadd(setKey, 'member1', 'member2', 'member3');
        const members = await redis.smembers(setKey);
        const isMember = await redis.sismember(setKey, 'member2');
        const poppedMember = await redis.spop(setKey);

        return response.json({
          success: true,
          action: 'set',
          message: '集合操作测试成功',
          data: {
            members,
            isMember,
            poppedMember
          }
        });
      }

      case 'hash': {
        // 哈希操作测试
        const hashKey = 'test_hash';
        await redis.del(hashKey);

        await redis.hset(hashKey, 'name', 'Alice');
        await redis.hset(hashKey, 'age', '25');
        await redis.hset(hashKey, 'city', 'Beijing');

        const name = await redis.hget(hashKey, 'name');
        const allFields = await redis.hgetall(hashKey);

        return response.json({
          success: true,
          action: 'hash',
          message: '哈希操作测试成功',
          data: {
            name,
            allFields
          }
        });
      }

      case 'zset': {
        // 有序集合操作测试
        const zsetKey = 'test_zset';
        await redis.del(zsetKey);

        await redis.zadd(zsetKey, 100, 'player1');
        await redis.zadd(zsetKey, 200, 'player2');
        await redis.zadd(zsetKey, 150, 'player3');

        const members = await redis.zrange(zsetKey, 0, -1);

        return response.json({
          success: true,
          action: 'zset',
          message: '有序集合操作测试成功',
          data: {
            members
          }
        });
      }

      case 'counter': {
        // 计数器操作测试
        const counterKey = 'test_counter';
        await redis.del(counterKey);

        const val1 = await redis.incr(counterKey);
        const val2 = await redis.incrBy(counterKey, 5);
        const val3 = await redis.decr(counterKey);

        return response.json({
          success: true,
          action: 'counter',
          message: '计数器操作测试成功',
          data: {
            afterIncr: val1,
            afterIncrBy: val2,
            afterDecr: val3
          }
        });
      }

      case 'pubsub': {
        // 发布订阅测试
        const channel = 'test_channel';
        const message = `Hello at ${Date.now()}`;

        const subscribers = await redis.publish(channel, message);

        return response.json({
          success: true,
          action: 'pubsub',
          message: '发布订阅测试成功',
          data: {
            channel,
            message,
            subscribers
          }
        });
      }

      default:
        return response.error('Unknown action', 400);
    }
  } catch (error) {
    return response.json({
      success: false,
      error: (error as Error).message,
      stack: (error as Error).stack
    }, 500);
  }
});
