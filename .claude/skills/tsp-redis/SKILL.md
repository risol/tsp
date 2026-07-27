---
name: tsp-redis
description: Redis cache operations for TSP. Use when working with Redis key-value storage, caching, or session sharing.
---

# TSP Redis

Use this skill for Redis operations in TSP.

## Redis Usage

```typescript
export default Page(async function(ctx, { createRedis, response }) {
  const redis = await createRedis({
    host: '127.0.0.1',
    port: 6379,
    password: 'your_password',  // optional
    database: 0
  });

  // String operations
  await redis.set('key', 'value');
  await redis.set('key', 'value', { ex: 3600 }); // with TTL (seconds)
  const value = await redis.get('key');

  // Number operations
  await redis.incr('counter');
  await redis.incrBy('counter', 5);
  await redis.decr('counter');

  // Hash operations
  await redis.hSet('user:1', 'name', 'John');
  await redis.hSet('user:1', 'age', '30');
  const user = await redis.hGetAll('user:1');
  // Returns: { name: 'John', age: '30' }

  // List operations
  await redis.lPush('queue', 'task1');
  await redis.rPush('queue', 'task2');
  const tasks = await redis.lRange('queue', 0, -1);

  // Set operations
  await redis.sAdd('tags', 'tag1', 'tag2', 'tag3');
  const tags = await redis.sMembers('tags');

  // Check if key exists
  const exists = await redis.exists('key');

  // Delete keys
  await redis.del('key');
  await redis.del('key1', 'key2', 'key3');

  // Expire / TTL
  await redis.expire('key', 3600);
  const ttl = await redis.ttl('key');

  return response.json({ value, user, tasks, tags, exists, ttl });
});
```

## Key Methods

### String
| Method | Description |
|--------|-------------|
| `set()` | Set value |
| `get()` | Get value |
| `incr()` | Increment |
| `decr()` | Decrement |

### Hash
| Method | Description |
|--------|-------------|
| `hSet()` | Set hash field |
| `hGetAll()` | Get all fields |
| `hGet()` | Get single field |

### List
| Method | Description |
|--------|-------------|
| `lPush()` | Push to left |
| `rPush()` | Push to right |
| `lRange()` | Get range |

### Set
| Method | Description |
|--------|-------------|
| `sAdd()` | Add to set |
| `sMembers()` | Get all members |

## Best Practices

- Use Redis for caching frequently accessed data
- Set appropriate TTL for cached data
- Use Redis for session sharing in multi-worker deployments
- Use hash for structured data (e.g., user objects)
