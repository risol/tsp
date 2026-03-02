/**
 * Redis E2E Test Page - TSP File
 */

export default Page(async function(ctx, { createRedis, response, query, createZod }) {
  const z = await createZod();
  const { action = 'basic' } = query(z.object({
    action: z.string().default('basic')
  }));

  try {
    const redis = await createRedis({
      host: '127.0.0.1',
      port: 6379,
      password: '',
      database: 0
    });

    switch (action) {
      case 'basic': {
        // Basic get/set test
        const testKey = `e2e_test_${Date.now()}`;
        await redis.set(testKey, 'test_value');
        const value = await redis.get(testKey);
        await redis.del(testKey);

        return response.json({
          success: true,
          action: 'basic',
          message: 'E2E basic CRUD test passed',
          data: { key: testKey, value }
        });
      }

      case 'expire': {
        // Expiration time test
        const testKey = `e2e_expire_${Date.now()}`;
        await redis.set(testKey, 'will_expire', 10);
        const ttl = await redis.ttl(testKey);
        const exists = await redis.exists(testKey);

        return response.json({
          success: true,
          action: 'expire',
          message: 'E2E expiration time test passed',
          data: { key: testKey, ttl, exists, verified: ttl > 0 && ttl <= 10 }
        });
      }

      case 'list': {
        // List operation test
        const listKey = `e2e_list_${Date.now()}`;
        await redis.del(listKey);

        await redis.lpush(listKey, 'item3', 'item2', 'item1');
        const items = await redis.lrange(listKey, 0, -1);
        const leftItem = await redis.lpop(listKey);
        const rightItem = await redis.rpop(listKey);

        return response.json({
          success: true,
          action: 'list',
          message: 'E2E list operation test passed',
          data: {
            list: items,
            poppedLeft: leftItem,
            poppedRight: rightItem,
            verified: items.length === 3 && leftItem === 'item3' && rightItem === 'item1'
          }
        });
      }

      case 'set': {
        // Set operation test
        const setKey = `e2e_set_${Date.now()}`;
        await redis.del(setKey);

        await redis.sadd(setKey, 'member1', 'member2', 'member3');
        const members = await redis.smembers(setKey);
        const isMember = await redis.sismember(setKey, 'member2');
        await redis.spop(setKey);
        const membersAfter = await redis.smembers(setKey);

        return response.json({
          success: true,
          action: 'set',
          message: 'E2E set operation test passed',
          data: {
            members,
            isMember,
            membersAfter,
            verified: members.length === 3 && isMember && membersAfter.length === 2
          }
        });
      }

      case 'hash': {
        // Hash operation test
        const hashKey = `e2e_hash_${Date.now()}`;
        await redis.del(hashKey);

        await redis.hset(hashKey, 'name', 'Alice');
        await redis.hset(hashKey, 'age', '25');
        await redis.hset(hashKey, 'city', 'Beijing');

        const name = await redis.hget(hashKey, 'name');
        const allFields = await redis.hgetall(hashKey);

        await redis.hdel(hashKey, 'age');
        const afterDelete = await redis.hgetall(hashKey);

        return response.json({
          success: true,
          action: 'hash',
          message: 'E2E hash operation test passed',
          data: {
            name,
            allFields,
            afterDelete,
            verified: name === 'Alice' && allFields.age === '25' && !afterDelete.age
          }
        });
      }

      case 'zset': {
        // Sorted set operation test
        const zsetKey = `e2e_zset_${Date.now()}`;
        await redis.del(zsetKey);

        await redis.zadd(zsetKey, 100, 'player1');
        await redis.zadd(zsetKey, 200, 'player2');
        await redis.zadd(zsetKey, 150, 'player3');

        const members = await redis.zrange(zsetKey, 0, -1);
        const removed = await redis.zrem(zsetKey, 'player2');
        const membersAfter = await redis.zrange(zsetKey, 0, -1);

        return response.json({
          success: true,
          action: 'zset',
          message: 'E2E sorted set operation test passed',
          data: {
            members,
            removed,
            membersAfter,
            verified: members.length === 3 && removed === 1 && membersAfter.length === 2
          }
        });
      }

      case 'counter': {
        // Counter operation test
        const counterKey = `e2e_counter_${Date.now()}`;
        await redis.del(counterKey);

        const val1 = await redis.incr(counterKey);
        const val2 = await redis.incrBy(counterKey, 5);
        const val3 = await redis.decr(counterKey);

        return response.json({
          success: true,
          action: 'counter',
          message: 'E2E counter operation test passed',
          data: {
            afterIncr: val1,
            afterIncrBy: val2,
            afterDecr: val3,
            verified: val1 === 1 && val2 === 6 && val3 === 5
          }
        });
      }

      case 'pubsub': {
        // Pub/sub test
        const channel = `e2e_channel_${Date.now()}`;
        const message = `E2E test message at ${Date.now()}`;
        const subscribers = await redis.publish(channel, message);

        return response.json({
          success: true,
          action: 'pubsub',
          message: 'E2E pub/sub test passed',
          data: {
            channel,
            message,
            subscribers,
            verified: typeof subscribers === 'number'
          }
        });
      }

      case 'pipeline': {
        // Batch operation test
        const testKey = `e2e_pipeline_${Date.now()}`;
        await redis.del(testKey);

        await redis.set(`${testKey}:1`, 'value1');
        await redis.set(`${testKey}:2`, 'value2');
        await redis.set(`${testKey}:3`, 'value3');

        const values = [
          await redis.get(`${testKey}:1`),
          await redis.get(`${testKey}:2`),
          await redis.get(`${testKey}:3`)
        ];

        await redis.del(`${testKey}:1`, `${testKey}:2`, `${testKey}:3`);

        return response.json({
          success: true,
          action: 'pipeline',
          message: 'E2E batch operation test passed',
          data: {
            values,
            verified: values.every(v => v !== null)
          }
        });
      }

      case 'stress': {
        // Stress test
        const stressKey = `e2e_stress_${Date.now()}`;
        const iterations = 100;
        const results = [];

        for (let i = 0; i < iterations; i++) {
          await redis.set(`${stressKey}:${i}`, `value${i}`);
          results.push(await redis.get(`${stressKey}:${i}`));
        }

        // Cleanup
        for (let i = 0; i < iterations; i++) {
          await redis.del(`${stressKey}:${i}`);
        }

        return response.json({
          success: true,
          action: 'stress',
          message: 'E2E stress test passed',
          data: {
            iterations,
            verified: results.length === iterations && results.every(r => r !== null)
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
