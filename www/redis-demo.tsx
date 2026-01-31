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
            <meta name="viewport" content="width=device-width, initial-scale=1" />
            <link href="/static/css/bootstrap.min.css" rel="stylesheet" />
          </head>
          <body className="container py-4">
            <div className="bg-danger text-white rounded p-5 mb-5 shadow text-center">
              <h1 className="display-5 mb-2">🔴 Redis 功能演示</h1>
              <p className="mb-0 opacity-75">高性能键值存储 - 快速、灵活、可靠</p>
            </div>

            <div className="row g-4 mb-5">
              <div className="col-md-4">
                <div className="card bg-danger bg-opacity-10 h-100">
                  <div className="card-body text-center">
                    <div className="display-4 fw-bold text-danger">{visits}</div>
                    <div className="text-muted small text-uppercase">页面访问次数</div>
                  </div>
                </div>
              </div>
              <div className="col-md-4">
                <div className="card bg-danger bg-opacity-10 h-100">
                  <div className="card-body text-center">
                    <div className="display-4 fw-bold text-danger">9</div>
                    <div className="text-muted small text-uppercase">数据结构类型</div>
                  </div>
                </div>
              </div>
              <div className="col-md-4">
                <div className="card bg-danger bg-opacity-10 h-100">
                  <div className="card-body text-center">
                    <div className="display-4 fw-bold text-danger">20+</div>
                    <div className="text-muted small text-uppercase">支持命令</div>
                  </div>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-4">🧪 功能测试</h2>
                <div className="row g-4">
                  <div className="col-md-6 col-lg-3">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">📝 基本 CRUD</h3>
                        <p className="card-text small text-muted">测试基本的 get/set/del 操作</p>
                        <a href="/redis-demo.tsx?action=basic" className="btn btn-danger btn-sm">测试</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">⏰ 过期时间</h3>
                        <p className="card-text small text-muted">设置键的过期时间（TTL）</p>
                        <a href="/redis-demo.tsx?action=expire" className="btn btn-danger btn-sm">测试</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">📋 列表操作</h3>
                        <p className="card-text small text-muted">List 数据结构的 lpush/rpop 等操作</p>
                        <a href="/redis-demo.tsx?action=list" className="btn btn-danger btn-sm">测试</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">🎯 集合操作</h3>
                        <p className="card-text small text-muted">Set 数据结构的 sadd/smembers 等操作</p>
                        <a href="/redis-demo.tsx?action=set" className="btn btn-danger btn-sm">测试</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">📦 哈希操作</h3>
                        <p className="card-text small text-muted">Hash 数据结构的 hset/hget 等操作</p>
                        <a href="/redis-demo.tsx?action=hash" className="btn btn-danger btn-sm">测试</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">🏆 有序集合</h3>
                        <p className="card-text small text-muted">Sorted Set 数据结构的 zadd/zrange 等操作</p>
                        <a href="/redis-demo.tsx?action=zset" className="btn btn-danger btn-sm">测试</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">🔢 计数器</h3>
                        <p className="card-text small text-muted">原子递增递减操作 incr/decr</p>
                        <a href="/redis-demo.tsx?action=counter" className="btn btn-danger btn-sm">测试</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">📢 发布订阅</h3>
                        <p className="card-text small text-muted">Pub/Sub 消息传递功能</p>
                        <a href="/redis-demo.tsx?action=pubsub" className="btn btn-danger btn-sm">测试</a>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-4">💻 使用示例</h2>

                <h3 className="h5 mb-3">基本操作</h3>
                <div className="bg-dark text-light p-3 rounded mb-4">
                  <pre className="mb-0 small">{`const redis = await createRedis({
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

                <h3 className="h5 mb-3">列表操作</h3>
                <div className="bg-dark text-light p-3 rounded mb-4">
                  <pre className="mb-0 small">{`// 从左侧推入
await redis.lpush('mylist', 'item1', 'item2');

// 获取所有元素
const items = await redis.lrange('mylist', 0, -1);

// 从左侧弹出
const item = await redis.lpop('mylist');`}</pre>
                </div>

                <h3 className="h5 mb-3">哈希操作</h3>
                <div className="bg-dark text-light p-3 rounded">
                  <pre className="mb-0 small">{`// 设置字段
await redis.hset('user:1', 'name', 'Alice');
await redis.hset('user:1', 'age', '25');

// 获取字段
const name = await redis.hget('user:1', 'name');

// 获取所有字段
const user = await redis.hgetall('user:1');`}</pre>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-3">📚 相关链接</h2>
                <ul className="list-unstyled mb-0">
                  <li className="mb-2"><a href="/test-redis" className="text-decoration-none">简单 JSON API 测试</a></li>
                  <li className="mb-2"><a href="/mysql-demo" className="text-decoration-none">MySQL 数据库演示</a></li>
                  <li className="mb-2"><a href="/" className="text-decoration-none">返回首页</a></li>
                </ul>
              </div>
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
