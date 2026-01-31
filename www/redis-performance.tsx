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
      // 性能测试仪表板
      return (
        <html>
          <head>
            <title>Redis 性能测试</title>
            <meta charset="utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1" />
            <link href="/assets/bootstrap.min.css" rel="stylesheet" />
          </head>
          <body>
            <div className="container">
              <div className="header">
                <h1>⚡ Redis 性能测试</h1>
                <p>测试 Redis 客户端的性能指标</p>
              </div>

              <div className="stats-grid">
                <div className="stat-card">
                  <div className="value">10+</div>
                  <div className="label">测试项</div>
                </div>
                <div className="stat-card">
                  <div className="value">9</div>
                  <div className="label">数据类型</div>
                </div>
                <div className="stat-card">
                  <div className="value">100ms</div>
                  <div className="label">目标响应</div>
                </div>
              </div>

              <div className="section">
                <h2>🧪 性能测试</h2>
                <div className="test-grid">
                  <div className="test-card">
                    <h3>📝 基本读写</h3>
                    <p>测试 get/set 操作的性能</p>
                    <a href="/redis-performance.tsx?action=basic" className="button">测试</a>
                  </div>

                  <div className="test-card">
                    <h3>⏰ 过期时间</h3>
                    <p>测试 expire/ttl 操作的性能</p>
                    <a href="/redis-performance.tsx?action=expire" className="button">测试</a>
                  </div>

                  <div className="test-card">
                    <h3>📋 列表操作</h3>
                    <p>测试 lpush/lpop 等列表操作</p>
                    <a href="/redis-performance.tsx?action=list" className="button">测试</a>
                  </div>

                  <div className="test-card">
                    <h3>🎯 集合操作</h3>
                    <p>测试 sadd/smembers 等集合操作</p>
                    <a href="/redis-performance.tsx?action=set" className="button">测试</a>
                  </div>

                  <div className="test-card">
                    <h3>📦 哈希操作</h3>
                    <p>测试 hset/hget 等哈希操作</p>
                    <a href="/redis-performance.tsx?action=hash" className="button">测试</a>
                  </div>

                  <div className="test-card">
                    <h3>🏆 有序集合</h3>
                    <p>测试 zadd/zrange 等有序集合操作</p>
                    <a href="/redis-performance.tsx?action=zset" className="button">测试</a>
                  </div>

                  <div className="test-card">
                    <h3>🔢 计数器</h3>
                    <p>测试 incr/decr 等计数器操作</p>
                    <a href="/redis-performance.tsx?action=counter" className="button">测试</a>
                  </div>

                  <div className="test-card">
                    <h3>🔥 压力测试</h3>
                    <p>连续执行 1000 次操作</p>
                    <a href="/redis-performance.tsx?action=stress" className="button danger">测试</a>
                  </div>

                  <div className="test-card">
                    <h3>📊 完整基准</h3>
                    <p>运行所有性能测试</p>
                    <a href="/redis-performance.tsx?action=benchmark" className="button success">测试</a>
                  </div>
                </div>
              </div>

              <div className="section">
                <h2>📖 使用建议</h2>
                <div style={{ background: '#f8f9fa', padding: '20px', borderRadius: '10px' }}>
                  <h3 style={{ marginTop: '0' }}>性能优化建议</h3>
                  <ul style={{ lineHeight: '2' }}>
                    <li><strong>使用批量操作</strong>: 减少 RTT，提高吞吐量</li>
                    <li><strong>合理使用 TTL</strong>: 自动清理过期数据，节省内存</li>
                    <li><strong>选择合适的数据结构</strong>: 根据场景选择最优结构</li>
                    <li><strong>使用连接池</strong>: 复用连接，减少握手开销</li>
                    <li><strong>管道操作</strong>: 在需要时使用 pipeline</li>
                    <li><strong>避免大对象</strong>: 控制 value 大小，避免阻塞</li>
                  </ul>
                </div>
              </div>

              <div className="section">
                <h2>🔗 相关链接</h2>
                <p style={{ lineHeight: '2' }}>
                  <a href="/redis-advanced">高级功能测试</a> |
                  <a href="/redis-demo">演示仪表板</a> |
                  <a href="/test-redis">API 测试</a> |
                  <a href="/">返回首页</a>
                </p>
              </div>
            </div>
          </body>
        </html>
      );
    }

    // 性能测试逻辑
    const start = performance.now();

    switch (action) {
      case 'basic': {
        const testKey = `perf_basic_${Date.now()}`;
        await redis.set(testKey, 'test_value');
        const value = await redis.get(testKey);
        await redis.del(testKey);
        const time = performance.now() - start;

        return response.json({
          success: true,
          action: 'basic',
          time: Math.round(time),
          message: `✓ 基本读写耗时 ${Math.round(time)}ms`,
          status: time < 10 ? 'fast' : time < 50 ? 'medium' : 'slow'
        });
      }

      case 'expire': {
        const testKey = `perf_expire_${Date.now()}`;
        await redis.set(testKey, 'value', 100);
        const ttl = await redis.ttl(testKey);
        const time = performance.now() - start;

        return response.json({
          success: true,
          action: 'expire',
          time: Math.round(time),
          ttl,
          message: `✓ 过期时间设置耗时 ${Math.round(time)}ms`,
          status: time < 10 ? 'fast' : time < 50 ? 'medium' : 'slow'
        });
      }

      case 'list': {
        const listKey = `perf_list_${Date.now()}`;
        await redis.del(listKey);

        for (let i = 0; i < 100; i++) {
          await redis.lpush(listKey, `item${i}`);
        }

        const items = await redis.lrange(listKey, 0, -1);
        const time = performance.now() - start;

        return response.json({
          success: true,
          action: 'list',
          time: Math.round(time),
          count: items.length,
          message: `✓ 列表操作耗时 ${Math.round(time)}ms (100次操作)`,
          avgTime: Math.round(time / 100),
          status: time < 100 ? 'fast' : time < 500 ? 'medium' : 'slow'
        });
      }

      case 'set': {
        const setKey = `perf_set_${Date.now()}`;
        await redis.del(setKey);

        for (let i = 0; i < 100; i++) {
          await redis.sadd(setKey, `member${i}`);
        }

        const members = await redis.smembers(setKey);
        const time = performance.now() - start;

        return response.json({
          success: true,
          action: 'set',
          time: Math.round(time),
          count: members.length,
          message: `✓ 集合操作耗时 ${Math.round(time)}ms (100次操作)`,
          avgTime: Math.round(time / 100),
          status: time < 100 ? 'fast' : time < 500 ? 'medium' : 'slow'
        });
      }

      case 'hash': {
        const hashKey = `perf_hash_${Date.now()}`;
        await redis.del(hashKey);

        for (let i = 0; i < 100; i++) {
          await redis.hset(hashKey, `field${i}`, `value${i}`);
        }

        const data = await redis.hgetall(hashKey);
        const time = performance.now() - start;

        return response.json({
          success: true,
          action: 'hash',
          time: Math.round(time),
          fieldCount: Object.keys(data).length,
          message: `✓ 哈希操作耗时 ${Math.round(time)}ms (100次操作)`,
          avgTime: Math.round(time / 100),
          status: time < 100 ? 'fast' : time < 500 ? 'medium' : 'slow'
        });
      }

      case 'zset': {
        const zsetKey = `perf_zset_${Date.now()}`;
        await redis.del(zsetKey);

        for (let i = 0; i < 100; i++) {
          await redis.zadd(zsetKey, i, `player${i}`);
        }

        const members = await redis.zrange(zsetKey, 0, -1);
        const time = performance.now() - start;

        return response.json({
          success: true,
          action: 'zset',
          time: Math.round(time),
          count: members.length,
          message: `✓ 有序集合操作耗时 ${Math.round(time)}ms (100次操作)`,
          avgTime: Math.round(time / 100),
          status: time < 100 ? 'fast' : time < 500 ? 'medium' : 'slow'
        });
      }

      case 'counter': {
        const counterKey = `perf_counter_${Date.now()}`;
        await redis.del(counterKey);

        for (let i = 0; i < 1000; i++) {
          await redis.incr(counterKey);
        }

        const final = await redis.get(counterKey);
        const time = performance.now() - start;

        return response.json({
          success: true,
          action: 'counter',
          time: Math.round(time),
          finalValue: final,
          iterations: 1000,
          message: `✓ 计数器操作耗时 ${Math.round(time)}ms (1000次递增)`,
          avgTime: Math.round(time / 1000),
          ops: Math.round(1000 / (time / 1000)),
          status: time < 200 ? 'fast' : time < 1000 ? 'medium' : 'slow'
        });
      }

      case 'stress': {
        const iterations = 1000;
        const stressKey = `stress_${Date.now()}`;

        const time = performance.now();

        for (let i = 0; i < iterations; i++) {
          await redis.set(`${stressKey}:${i}`, `value${i}`);
          await redis.get(`${stressKey}:${i}`);
        }

        // 清理
        for (let i = 0; i < iterations; i++) {
          await redis.del(`${stressKey}:${i}`);
        }

        const totalTime = performance.now() - time;

        return response.json({
          success: true,
          action: 'stress',
          time: Math.round(totalTime),
          iterations,
          ops: Math.round(iterations / (totalTime / 1000)),
          message: `✓ 压力测试完成 (${iterations}次读写)`,
          status: totalTime < 1000 ? 'fast' : totalTime < 3000 ? 'medium' : 'slow'
        });
      }

      case 'benchmark': {
        // 完整基准测试
        const benchmark: any = { action: 'benchmark', tests: [] };

        // 基本 CRUD
        let start = performance.now();
        const basicKey = `bench_basic_${Date.now()}`;
        await redis.set(basicKey, 'test');
        await redis.get(basicKey);
        await redis.del(basicKey);
        let time = performance.now() - start;
        benchmark.tests.push({ name: '基本 CRUD', time: Math.round(time), status: time < 10 ? 'fast' : 'medium' });

        // 过期时间
        start = performance.now();
        const expireKey = `bench_expire_${Date.now()}`;
        await redis.set(expireKey, 'value', 60);
        await redis.ttl(expireKey);
        time = performance.now() - start;
        benchmark.tests.push({ name: '过期时间', time: Math.round(time), status: time < 10 ? 'fast' : 'medium' });

        // 列表操作
        start = performance.now();
        const listKey = `bench_list_${Date.now()}`;
        await redis.del(listKey);
        for (let i = 0; i < 50; i++) {
          await redis.lpush(listKey, `item${i}`);
        }
        await redis.lrange(listKey, 0, -1);
        time = performance.now() - start;
        benchmark.tests.push({ name: '列表操作(50次)', time: Math.round(time), status: time < 100 ? 'fast' : 'medium' });

        // 计数器
        start = performance.now();
        const counterKey = `bench_counter_${Date.now()}`;
        await redis.del(counterKey);
        for (let i = 0; i < 500; i++) {
          await redis.incr(counterKey);
        }
        time = performance.now() - start;
        benchmark.tests.push({ name: '计数器(500次)', time: Math.round(time), status: time < 200 ? 'fast' : 'medium' });

        const totalTime = benchmark.tests.reduce((sum: number, t: any) => sum + t.time, 0);

        return response.json({
          success: true,
          action: 'benchmark',
          totalTime: Math.round(totalTime),
          tests: benchmark.tests,
          message: `✓ 完整基准测试完成`
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
