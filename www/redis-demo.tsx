export default Page(async function(ctx, { createRedis, response }) {
  const action = ctx.query.action || 'dashboard';

  try {
    const redis = await createRedis({
      host: '127.0.0.1',
      port: 6379,
      password: '',
      database: 0
    });

    if (action === 'dashboard') {
      // 演示仪表板页面
      const info = await redis.get('tsp_demo_visits') || '0';
      const visits = parseInt(info, 10) + 1;
      await redis.set('tsp_demo_visits', visits.toString());

      return (
        <html>
          <head>
            <title>Redis 功能演示</title>
            <meta charset="utf-8" />
            <style>{`
              body {
                font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                max-width: 1200px;
                margin: 0 auto;
                padding: 20px;
                background: #f5f5f5;
              }
              .header {
                background: linear-gradient(135deg, #dc2626 0%, #ef4444 100%);
                color: white;
                padding: 40px;
                border-radius: 10px;
                margin-bottom: 30px;
                text-align: center;
              }
              .header h1 {
                margin: 0 0 10px 0;
                font-size: 3em;
              }
              .header p {
                margin: 0;
                opacity: 0.9;
                font-size: 1.2em;
              }
              .section {
                background: white;
                border-radius: 10px;
                padding: 30px;
                margin-bottom: 20px;
                box-shadow: 0 2px 4px rgba(0,0,0,0.1);
              }
              .section h2 {
                margin-top: 0;
                color: #333;
                border-bottom: 2px solid #dc2626;
                padding-bottom: 10px;
              }
              .button {
                display: inline-block;
                padding: 12px 24px;
                margin: 10px;
                background: #dc2626;
                color: white;
                text-decoration: none;
                border-radius: 5px;
                transition: background 0.3s;
              }
              .button:hover {
                background: #b91c1c;
              }
              .button.success {
                background: #16a34a;
              }
              .button.success:hover {
                background: #15803d;
              }
              .stat-grid {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
                gap: 20px;
                margin-bottom: 30px;
              }
              .stat-card {
                background: linear-gradient(135deg, #fee2e2 0%, #fecaca 100%);
                padding: 25px;
                border-radius: 10px;
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
              .test-grid {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
                gap: 20px;
                margin-top: 20px;
              }
              .test-card {
                border: 2px solid #e5e7eb;
                border-radius: 10px;
                padding: 20px;
                transition: all 0.3s;
              }
              .test-card:hover {
                border-color: #dc2626;
                box-shadow: 0 4px 12px rgba(220, 38, 38, 0.2);
              }
              .test-card h3 {
                margin: 0 0 10px 0;
                color: #dc2626;
              }
              .test-card p {
                margin: 0 0 15px 0;
                color: #666;
                font-size: 0.9em;
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
            `}</style>
          </head>
          <body>
            <div className="header">
              <h1>🔴 Redis 功能演示</h1>
              <p>高性能键值存储 - 快速、灵活、可靠</p>
            </div>

            <div className="stat-grid">
              <div className="stat-card">
                <div className="value">{visits}</div>
                <div className="label">页面访问次数</div>
              </div>
              <div className="stat-card">
                <div className="value">9</div>
                <div className="label">数据结构类型</div>
              </div>
              <div className="stat-card">
                <div className="value">20+</div>
                <div className="label">支持命令</div>
              </div>
            </div>

            <div className="section">
              <h2>🧪 功能测试</h2>
              <div className="test-grid">
                <div className="test-card">
                  <h3>📝 基本 CRUD</h3>
                  <p>测试基本的 get/set/del 操作</p>
                  <a href="/redis-demo.tsx?action=basic" className="button">测试</a>
                </div>

                <div className="test-card">
                  <h3>⏰ 过期时间</h3>
                  <p>设置键的过期时间（TTL）</p>
                  <a href="/redis-demo.tsx?action=expire" className="button">测试</a>
                </div>

                <div className="test-card">
                  <h3>📋 列表操作</h3>
                  <p>List 数据结构的 lpush/rpop 等操作</p>
                  <a href="/redis-demo.tsx?action=list" className="button">测试</a>
                </div>

                <div className="test-card">
                  <h3>🎯 集合操作</h3>
                  <p>Set 数据结构的 sadd/smembers 等操作</p>
                  <a href="/redis-demo.tsx?action=set" className="button">测试</a>
                </div>

                <div className="test-card">
                  <h3>📦 哈希操作</h3>
                  <p>Hash 数据结构的 hset/hget 等操作</p>
                  <a href="/redis-demo.tsx?action=hash" className="button">测试</a>
                </div>

                <div className="test-card">
                  <h3>🏆 有序集合</h3>
                  <p>Sorted Set 数据结构的 zadd/zrange 等操作</p>
                  <a href="/redis-demo.tsx?action=zset" className="button">测试</a>
                </div>

                <div className="test-card">
                  <h3>🔢 计数器</h3>
                  <p>原子递增递减操作 incr/decr</p>
                  <a href="/redis-demo.tsx?action=counter" className="button">测试</a>
                </div>

                <div className="test-card">
                  <h3>📢 发布订阅</h3>
                  <p>Pub/Sub 消息传递功能</p>
                  <a href="/redis-demo.tsx?action=pubsub" className="button">测试</a>
                </div>
              </div>
            </div>

            <div className="section">
              <h2>💻 使用示例</h2>

              <h3>基本操作</h3>
              <div className="code-block">
                <pre>{`const redis = await createRedis({
  host: '127.0.0.1',
  port: 6379,
  password: '',
  database: 0
});

// 设置键值
await redis.set('key', 'value');

// 获取值
const value = await redis.get('key');

// 设置过期时间（5秒）
await redis.set('key', 'value', 5);

// 删除键
await redis.del('key');`}</pre>
              </div>

              <h3>列表操作</h3>
              <div className="code-block">
                <pre>{`// 从左侧推入
await redis.lpush('mylist', 'item1', 'item2');

// 获取所有元素
const items = await redis.lrange('mylist', 0, -1);

// 从左侧弹出
const item = await redis.lpop('mylist');`}</pre>
              </div>

              <h3>哈希操作</h3>
              <div className="code-block">
                <pre>{`// 设置字段
await redis.hset('user:1', 'name', 'Alice');
await redis.hset('user:1', 'age', '25');

// 获取字段
const name = await redis.hget('user:1', 'name');

// 获取所有字段
const user = await redis.hgetall('user:1');`}</pre>
              </div>
            </div>

            <div className="section">
              <h2>📚 相关链接</h2>
              <ul style={{ lineHeight: '2' }}>
                <li><a href="/test-redis">简单 JSON API 测试</a></li>
                <li><a href="/mysql-demo">MySQL 数据库演示</a></li>
                <li><a href="/">返回首页</a></li>
              </ul>
            </div>
          </body>
        </html>
      );
    }

    // 其他操作返回 JSON
    let result: any = { success: true };

    switch (action) {
      case 'basic': {
        await redis.set('test_key', 'Hello from Redis!');
        const value = await redis.get('test_key');
        await redis.del('test_key');
        result = { ...result, action: 'basic', message: '✓ 基本操作成功', value };
        break;
      }

      case 'expire': {
        await redis.set('expire_key', 'will_expire', 10);
        const ttl = await redis.ttl('expire_key');
        result = { ...result, action: 'expire', message: '✓ 过期时间设置成功', ttl, note: 'TTL 约为 10 秒' };
        break;
      }

      case 'list': {
        const listKey = 'test_list';
        await redis.del(listKey);
        await redis.lpush(listKey, 'item3', 'item2', 'item1');
        const items = await redis.lrange(listKey, 0, -1);
        result = { ...result, action: 'list', message: '✓ 列表操作成功', items };
        break;
      }

      case 'set': {
        const setKey = 'test_set';
        await redis.del(setKey);
        await redis.sadd(setKey, 'member1', 'member2', 'member3');
        const members = await redis.smembers(setKey);
        result = { ...result, action: 'set', message: '✓ 集合操作成功', members };
        break;
      }

      case 'hash': {
        const hashKey = 'test_hash';
        await redis.del(hashKey);
        await redis.hset(hashKey, 'name', 'Alice');
        await redis.hset(hashKey, 'age', '25');
        const data = await redis.hgetall(hashKey);
        result = { ...result, action: 'hash', message: '✓ 哈希操作成功', data };
        break;
      }

      case 'zset': {
        const zsetKey = 'test_zset';
        await redis.del(zsetKey);
        await redis.zadd(zsetKey, 100, 'player1');
        await redis.zadd(zsetKey, 200, 'player2');
        const members = await redis.zrange(zsetKey, 0, -1);
        result = { ...result, action: 'zset', message: '✓ 有序集合操作成功', members };
        break;
      }

      case 'counter': {
        const counterKey = 'test_counter';
        await redis.del(counterKey);
        const val1 = await redis.incr(counterKey);
        const val2 = await redis.incrBy(counterKey, 5);
        const val3 = await redis.decr(counterKey);
        result = { ...result, action: 'counter', message: '✓ 计数器操作成功', values: { incr: val1, incrBy: val2, decr: val3 } };
        break;
      }

      case 'pubsub': {
        const channel = 'test_channel';
        const msg = `Hello at ${Date.now()}`;
        const subscribers = await redis.publish(channel, msg);
        result = { ...result, action: 'pubsub', message: '✓ 发布订阅成功', channel, msg, subscribers };
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
