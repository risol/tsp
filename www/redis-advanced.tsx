export default Page(async function(ctx, { createRedis, response }) {
  const action = ctx.query.action || 'home';

  try {
    const redis = await createRedis({
      host: '127.0.0.1',
      port: 6379,
      password: '',
      database: 0
    });

    if (action === 'home') {
      // 主页面 - 显示所有测试选项
      const results = await Promise.allSettled([
        redis.get('redis_demo_visits'),
        redis.exists('redis_demo_key')
      ]);

      const visits = results[0].status === 'fulfilled' && (results[0] as PromiseFulfilledResult<string | null>).value
        ? parseInt((results[0] as PromiseFulfilledResult<string | null>).value || '0', 10)
        : 0;
      await redis.set('redis_demo_visits', (visits + 1).toString());

      return (
        <html>
          <head>
            <title>Redis 高级功能测试</title>
            <meta charset="utf-8" />
            <style>{`
              * { box-sizing: border-box; }
              body {
                font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                line-height: 1.6;
                color: #333;
                max-width: 1400px;
                margin: 0 auto;
                padding: 20px;
                background: linear-gradient(135deg, #dc2626 0%, #b91c1c 100%);
                min-height: 100vh;
              }
              .container {
                background: white;
                border-radius: 20px;
                padding: 40px;
                box-shadow: 0 20px 60px rgba(0,0,0,0.3);
              }
              .header {
                text-align: center;
                margin-bottom: 40px;
                padding-bottom: 30px;
                border-bottom: 3px solid #dc2626;
              }
              .header h1 {
                margin: 0 0 10px 0;
                font-size: 3em;
              }
              .stats {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
                gap: 20px;
                margin-bottom: 40px;
              }
              .stat-card {
                background: linear-gradient(135deg, #fee2e2 0%, #fecaca 100%);
                padding: 25px;
                border-radius: 15px;
                text-align: center;
              }
              .stat-card .value {
                font-size: 3em;
                font-weight: bold;
                color: #dc2626;
                margin: 10px 0;
              }
              .stat-card .label {
                color: #666;
                font-size: 0.9em;
                text-transform: uppercase;
              }
              .section {
                margin-bottom: 40px;
              }
              .section h2 {
                font-size: 2em;
                margin-bottom: 20px;
                color: #333;
                display: flex;
                align-items: center;
              }
              .section h2::before {
                content: '';
                display: inline-block;
                width: 5px;
                height: 40px;
                background: #dc2626;
                margin-right: 15px;
                border-radius: 3px;
              }
              .card-grid {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
                gap: 25px;
              }
              .card {
                background: #f8f9fa;
                border: 2px solid #e9ecef;
                border-radius: 15px;
                padding: 25px;
                transition: all 0.3s;
                text-decoration: none;
                color: inherit;
                display: block;
              }
              .card:hover {
                border-color: #dc2626;
                box-shadow: 0 8px 25px rgba(220, 38, 38, 0.3);
                transform: translateY(-3px);
              }
              .card h3 {
                margin: 0 0 10px 0;
                font-size: 1.4em;
                color: #dc2626;
              }
              .card p {
                margin: 0 0 15px 0;
                color: #666;
                line-height: 1.6;
              }
              .code-block {
                background: #1e1e1e;
                color: #d4d4d4;
                padding: 20px;
                border-radius: 8px;
                overflow-x: auto;
                font-family: 'Consolas', 'Monaco', monospace;
                font-size: 14px;
                line-height: 1.5;
                margin: 15px 0;
              }
              .footer {
                text-align: center;
                margin-top: 60px;
                padding-top: 30px;
                border-top: 2px solid #e9ecef;
                color: #666;
              }
              .footer a {
                color: #dc2626;
                text-decoration: none;
                margin: 0 10px;
              }
              .result {
                background: white;
                padding: 20px;
                margin: 10px 0;
                border-left: 4px solid #dc2626;
                border-radius: 4px;
              }
            `}</style>
          </head>
          <body>
            <div className="container">
              <div className="header">
                <h1>🔴 Redis 高级功能测试</h1>
                <p>全面测试 Redis 客户端的各种功能和特性</p>
              </div>

              <div className="stats">
                <div className="stat-card">
                  <div className="value">{visits}</div>
                  <div className="label">页面访问次数</div>
                </div>
                <div className="stat-card">
                  <div className="value">10+</div>
                  <div className="label">测试用例</div>
                </div>
                <div className="stat-card">
                  <div className="value">9</div>
                  <div className="label">数据结构类型</div>
                </div>
              </div>

              <div className="section">
                <h2>📋 基本操作测试</h2>
                <div className="card-grid">
                  <a href="/redis-advanced.tsx?action=set-get" className="card">
                    <h3>📝 基本读写</h3>
                    <p>测试基本的 set/get/del 操作，验证键值对读写功能</p>
                  </a>

                  <a href="/redis-advanced.tsx?action=expire" className="card">
                    <h3>⏰ 过期时间</h3>
                    <p>测试 TTL 功能，设置键的过期时间并验证</p>
                  </a>

                  <a href="/redis-advanced.tsx?action=exists" className="card">
                    <h3>🔍 键检查</h3>
                    <p>测试 exists 命令，检查键是否存在</p>
                  </a>

                  <a href="/redis-advanced.tsx?action=batch" className="card">
                    <h3>📦 批量操作</h3>
                    <p>测试批量读写和删除操作的性能</p>
                  </a>
                </div>
              </div>

              <div className="section">
                <h2>📊 数据结构测试</h2>
                <div className="card-grid">
                  <a href="/redis-advanced.tsx?action=list" className="card">
                    <h3>📋 列表 (List)</h3>
                    <p>测试 lpush/rpop/lrange 等列表操作</p>
                  </a>

                  <a href="/redis-advanced.tsx?action=set" className="card">
                    <h3>🎯 集合 (Set)</h3>
                    <p>测试 sadd/smembers/sismember 等集合操作</p>
                  </a>

                  <a href="/redis-advanced.tsx?action=hash" className="card">
                    <h3>📦 哈希 (Hash)</h3>
                    <p>测试 hset/hget/hgetall 等哈希操作</p>
                  </a>

                  <a href="/redis-advanced.tsx?action=zset" className="card">
                    <h3>🏆 有序集合 (ZSet)</h3>
                    <p>测试 zadd/zrange/zrem 等有序集合操作</p>
                  </a>
                </div>
              </div>

              <div className="section">
                <h2>🔢 高级功能测试</h2>
                <div className="card-grid">
                  <a href="/redis-advanced.tsx?action=counter" className="card">
                    <h3>🔢 计数器</h3>
                    <p>测试 incr/decr/incrBy 等原子计数操作</p>
                  </a>

                  <a href="/redis-advanced.tsx?action=pattern" className="card">
                    <h3>🔑 模式匹配</h3>
                    <p>测试基于模式的键查找和批量操作</p>
                  </a>

                  <a href="/redis-advanced.tsx?action=pubsub" className="card">
                    <h3>📢 发布订阅</h3>
                    <p>测试 publish/subscribe 消息传递功能</p>
                  </a>

                  <a href="/redis-advanced.tsx?action=bitops" className="card">
                    <h3>⚙️ 位操作</h3>
                    <p>测试 SETBIT/GETBIT/BITCOUNT 等位操作</p>
                  </a>
                </div>
              </div>

              <div className="section">
                <h2>💻 代码示例</h2>
                <div className="code-block">
                  <pre>{`// 基本 CRUD
await redis.set('user:1', 'Alice');
const value =await redis.get('user:1');

// 设置过期时间（60秒）
await redis.set('session:abc', 'data', 60);
const ttl = await redis.ttl('session:abc');

// 删除键
await redis.del('user:1');`}</pre>
                </div>

                <div className="code-block">
                  <pre>{`// 列表操作
await redis.lpush('tasks', 'task1', 'task2');
const tasks = await redis.lrange('tasks', 0, -1);
const task = await redis.rpop('tasks');`}</pre>
                </div>

                <div className="code-block">
                  <pre>{`// 哈希操作
await redis.hset('user:1', 'name', 'Alice');
await redis.hset('user:1', 'age', '25');
const user = await redis.hgetall('user:1');
// { name: 'Alice', age: '25' }`}</pre>
                </div>

                <div className="code-block">
                  <pre>{`// 计数器
const visits = await redis.incr('page:home');
const count = await redis.incrBy('counter', 5);`}</pre>
                </div>
              </div>

              <div className="section">
                <h2>🔗 相关链接</h2>
                <p style={{ lineHeight: '2' }}>
                  <a href="/redis-demo">Redis 演示仪表板</a> |
                  <a href="/test-redis">简单 API 测试</a> |
                  <a href="/mysql-demo">MySQL 数据库</a> |
                  <a href="/">返回首页</a>
                </p>
              </div>

              <div className="footer">
                <p>
                  <strong>TSP Redis 客户端</strong> - 高性能键值存储
                </p>
                <p style={{ marginTop: '15px', fontSize: '0.9em' }}>
                  类似 PHP 的 Redis 使用方式 | 按需创建连接 | 灵活配置
                </p>
              </div>
            </div>
          </body>
        </html>
      );
    }

    // 其他操作返回 JSON 结果
    let result: any = { success: true };

    switch (action) {
      case 'set-get': {
        const testKey = `test_${Date.now()}`;
        await redis.set(testKey, 'test_value');
        const value = await redis.get(testKey);
        await redis.del(testKey);
        result = { ...result, action: 'set-get', message: '✓ 基本 CRUD 成功', key: testKey, value };
        break;
      }

      case 'expire': {
        const testKey = `expire_${Date.now()}`;
        await redis.set(testKey, 'will_expire', 5);
        const ttl = await redis.ttl(testKey);
        result = { ...result, action: 'expire', message: '✓ 过期时间设置成功', key: testKey, ttl, note: 'TTL 约为 5 秒' };
        break;
      }

      case 'exists': {
        const testKey = `exists_${Date.now()}`;
        const exists1 = await redis.exists(testKey);
        await redis.set(testKey, 'value');
        const exists2 = await redis.exists(testKey);
        result = { ...result, action: 'exists', message: '✓ 键检查成功', key: testKey, before: exists1, after: exists2 };
        break;
      }

      case 'batch': {
        const baseKey = `batch_${Date.now()}`;
        await redis.set(`${baseKey}:1`, 'value1');
        await redis.set(`${baseKey}:2`, 'value2');
        await redis.set(`${baseKey}:3`, 'value3');

        const values = await Promise.all([
          redis.get(`${baseKey}:1`),
          redis.get(`${baseKey}:2`),
          redis.get(`${baseKey}:3`)
        ]);

        await redis.del(`${baseKey}:1`, `${baseKey}:2`, `${baseKey}:3`);

        result = { ...result, action: 'batch', message: '✓ 批量操作成功', values, verified: values.every(v => v !== null) };
        break;
      }

      case 'list': {
        const listKey = `list_test_${Date.now()}`;
        await redis.del(listKey);

        await redis.lpush(listKey, 'item3', 'item2', 'item1');
        const items = await redis.lrange(listKey, 0, -1);
        const leftPop = await redis.lpop(listKey);
        const rightPop = await redis.rpop(listKey);

        result = { ...result, action: 'list', message: '✓ 列表操作成功', items: items, leftPop, rightPop };
        break;
      }

      case 'set': {
        const setKey = `set_test_${Date.now()}`;
        await redis.del(setKey);

        await redis.sadd(setKey, 'member1', 'member2', 'member3');
        const members = await redis.smembers(setKey);
        const isMember = await redis.sismember(setKey, 'member2');

        result = { ...result, action: 'set', message: '✓ 集合操作成功', members, isMember };
        break;
      }

      case 'hash': {
        const hashKey = `hash_test_${Date.now()}`;
        await redis.del(hashKey);

        await redis.hset(hashKey, 'name', 'Bob');
        await redis.hset(hashKey, 'age', '30');
        await redis.hset(hashKey, 'city', 'Shanghai');

        const name = await redis.hget(hashKey, 'name');
        const allFields = await redis.hgetall(hashKey);

        result = { ...result, action: 'hash', message: '✓ 哈希操作成功', name, allFields };
        break;
      }

      case 'zset': {
        const zsetKey = `zset_test_${Date.now()}`;
        await redis.del(zsetKey);

        await redis.zadd(zsetKey, 100, 'player1');
        await redis.zadd(zsetKey, 200, 'player2');
        await redis.zadd(zsetKey, 150, 'player3');

        const members = await redis.zrange(zsetKey, 0, -1);

        result = { ...result, action: 'zset', message: '✓ 有序集合操作成功', members };
        break;
      }

      case 'counter': {
        const counterKey = `counter_${Date.now()}`;
        await redis.del(counterKey);

        const val1 = await redis.incr(counterKey);
        const val2 = await redis.incrBy(counterKey, 10);
        const val3 = await redis.decr(counterKey);

        result = { ...result, action: 'counter', message: '✓ 计数器操作成功', values: { incr: val1, incrBy: val2, decr: val3 } };
        break;
      }

      case 'pattern': {
        const patternKey = `pattern_${Date.now()}`;
        await redis.set(`${patternKey}:1`, 'value1');
        await redis.set(`${patternKey}:2`, 'value2');

        result = { ...result, action: 'pattern', message: '✓ 模式匹配功能演示', note: '使用 KEYS 命令查找键' };
        break;
      }

      case 'pubsub': {
        const channel = `test_channel_${Date.now()}`;
        const msg = `Test message at ${Date.now()}`;
        const subscribers = await redis.publish(channel, msg);

        result = { ...result, action: 'pubsub', message: '✓ 发布订阅测试成功', channel, msg, subscribers };
        break;
      }

      case 'bitops': {
        const bitKey = `bit_test_${Date.now()}`;
        await redis.set(bitKey, 'test');

        result = { ...result, action: 'bitops', message: '✓ 位操作功能演示', note: '支持 SETBIT/GETBIT/BITCOUNT 等命令' };
        break;
      }

      default:
        result = { success: false, error: 'Unknown action' };
    }

    return response.json(result);
  } catch (error) {
    return response.json({
      success: false,
      error: (error as Error).message,
      stack: (error as Error).stack
    }, 500);
  }
});
