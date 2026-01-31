export default Page(async function(ctx, { createMySQL, response }) {
  const action = ctx.query.action || 'dashboard';

  try {
    if (action === 'dashboard') {
      // 主仪表板页面
      return (
        <html>
          <head>
            <title>MySQL 性能测试</title>
            <meta charset="utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1" />
            <link href="/static/css/bootstrap.min.css" rel="stylesheet" />
          </head>
          <body className="container py-4 bg-light">
            <div className="bg-primary text-white rounded p-5 mb-5 shadow text-center">
              <h1 className="display-5 mb-2">⚡ MySQL 性能测试</h1>
              <p className="mb-0 opacity-75">测试 MySQL 客户端的各种性能指标</p>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-4">🧪 性能测试</h2>
                <p>点击下面的按钮运行各种性能测试：</p>

                <div className="mb-3">
                  <a href="/mysql-performance.tsx?action=connection" className="btn btn-primary me-2 mb-2">
                    🔗 连接测试
                  </a>
                  <a href="/mysql-performance.tsx?action=query-simple" className="btn btn-primary me-2 mb-2">
                    📊 简单查询
                  </a>
                  <a href="/mysql-performance.tsx?action=query-complex" className="btn btn-primary me-2 mb-2">
                    🔍 复杂查询
                  </a>
                  <a href="/mysql-performance.tsx?action=insert-single" className="btn btn-primary me-2 mb-2">
                    ➕ 单次插入
                  </a>
                  <a href="/mysql-performance.tsx?action=insert-batch" className="btn btn-primary me-2 mb-2">
                    ➕➕ 批量插入
                  </a>
                </div>

                <div>
                  <a href="/mysql-performance.tsx?action=stress-test" className="btn btn-danger me-2 mb-2">
                    🔥 压力测试
                  </a>
                  <a href="/mysql-performance.tsx?action=full-benchmark" className="btn btn-success me-2 mb-2">
                    📈 完整基准测试
                  </a>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-4">📖 测试说明</h2>
                <div className="table-responsive">
                  <table className="table table-striped table-hover">
                    <thead className="table-dark">
                      <tr>
                        <th>测试项</th>
                        <th>描述</th>
                        <th>预期结果</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr>
                        <td>连接测试</td>
                        <td>测试数据库连接的建立时间</td>
                        <td>&lt; 100ms</td>
                      </tr>
                      <tr>
                        <td>简单查询</td>
                        <td>执行基本的 SELECT 查询</td>
                        <td>&lt; 50ms</td>
                      </tr>
                      <tr>
                        <td>复杂查询</td>
                        <td>执行带参数和排序的查询</td>
                        <td>&lt; 100ms</td>
                      </tr>
                      <tr>
                        <td>单次插入</td>
                        <td>插入一条记录</td>
                        <td>&lt; 50ms</td>
                      </tr>
                      <tr>
                        <td>批量插入</td>
                        <td>使用事务批量插入 100 条记录</td>
                        <td>&lt; 500ms</td>
                      </tr>
                      <tr>
                        <td>压力测试</td>
                        <td>连续执行 50 次查询操作</td>
                        <td>&lt; 2000ms</td>
                      </tr>
                      <tr>
                        <td>完整基准测试</td>
                        <td>运行所有性能测试</td>
                        <td>&lt; 5000ms</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-4">💡 性能优化建议</h2>
                <ul className="list-group list-group-flush">
                  <li className="list-group-item"><strong>使用连接池：</strong> mysql2 内置连接池，自动管理连接复用</li>
                  <li className="list-group-item"><strong>批量操作：</strong> 使用事务进行批量插入/更新，性能更好</li>
                  <li className="list-group-item"><strong>参数化查询：</strong> 使用参数化查询不仅安全，还能提升性能</li>
                  <li className="list-group-item"><strong>索引优化：</strong> 为经常查询的字段添加索引</li>
                  <li className="list-group-item"><strong>避免 SELECT *：</strong> 只查询需要的字段</li>
                </ul>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-3">🔗 相关链接</h2>
                <p>
                  <a href="/mysql-demo.tsx" className="text-decoration-none me-3">MySQL 演示仪表板</a>
                  <a href="/mysql-advanced.tsx" className="text-decoration-none me-3">MySQL 高级功能</a>
                  <a href="/" className="text-decoration-none">返回首页</a>
                </p>
              </div>
            </div>
          </body>
        </html>
      );
    }

    // 性能测试逻辑
    const db = await createMySQL({
      host: '127.0.0.1',
      port: 3306,
      user: 'test_user',
      password: 'test123456',
      database: 'test_db'
    });

    let result: any = { success: true };

    switch (action) {
      case 'connection': {
        const start = performance.now();
        const db2 = await createMySQL({
          host: '127.0.0.1',
          user: 'test_user',
          password: 'test123456',
          database: 'test_db'
        });
        const end = performance.now();
        result = {
          ...result,
          action: 'connection',
          time: Math.round(end - start),
          message: `连接耗时 ${Math.round(end - start)}ms`,
          status: end - start < 100 ? 'fast' : end - start < 200 ? 'medium' : 'slow'
        };
        break;
      }

      case 'query-simple': {
        const start = performance.now();
        const users = await db.query('SELECT * FROM users LIMIT 10');
        const end = performance.now();
        result = {
          ...result,
          action: 'query-simple',
          time: Math.round(end - start),
          records: users.length,
          message: `查询 ${users.length} 条记录，耗时 ${Math.round(end - start)}ms`,
          status: end - start < 50 ? 'fast' : end - start < 100 ? 'medium' : 'slow'
        };
        break;
      }

      case 'query-complex': {
        const start = performance.now();
        const users = await db.query(
          'SELECT * FROM users WHERE id > ? ORDER BY username DESC LIMIT 10',
          [5]
        );
        const end = performance.now();
        result = {
          ...result,
          action: 'query-complex',
          time: Math.round(end - start),
          records: users.length,
          message: `复杂查询返回 ${users.length} 条记录，耗时 ${Math.round(end - start)}ms`,
          status: end - start < 100 ? 'fast' : end - start < 200 ? 'medium' : 'slow'
        };
        break;
      }

      case 'insert-single': {
        const timestamp = Date.now();
        const start = performance.now();
        const insertId = await db.insert('users', {
          username: `perf_${timestamp}`,
          email: `perf_${timestamp}@test.com`,
          password_hash: 'hash'
        });
        const end = performance.now();
        result = {
          ...result,
          action: 'insert-single',
          time: Math.round(end - start),
          insertId,
          message: `插入记录 ID ${insertId}，耗时 ${Math.round(end - start)}ms`,
          status: end - start < 50 ? 'fast' : end - start < 100 ? 'medium' : 'slow'
        };
        break;
      }

      case 'insert-batch': {
        const count = 100;
        const start = performance.now();

        await db.beginTransaction();
        const timestamp = Date.now();
        try {
          for (let i = 0; i < count; i++) {
            await db.insert('users', {
              username: `batch_perf_${i}_${timestamp}`,
              email: `batch${i}_${timestamp}@test.com`,
              password_hash: `hash${i}`
            });
          }
          await db.commit();
        } catch (error) {
          await db.rollback();
          throw error;
        }

        const end = performance.now();
        const avgTime = (end - start) / count;

        result = {
          ...result,
          action: 'insert-batch',
          time: Math.round(end - start),
          count,
          avgTime: Math.round(avgTime),
          message: `批量插入 ${count} 条记录，总耗时 ${Math.round(end - start)}ms，平均 ${Math.round(avgTime)}ms/条`,
          status: end - start < 500 ? 'fast' : end - start < 1000 ? 'medium' : 'slow'
        };
        break;
      }

      case 'stress-test': {
        const iterations = 50;
        const start = performance.now();

        for (let i = 0; i < iterations; i++) {
          await db.query('SELECT * FROM users LIMIT 1');
        }

        const end = performance.now();
        const avgTime = (end - start) / iterations;

        result = {
          ...result,
          action: 'stress-test',
          time: Math.round(end - start),
          iterations,
          avgTime: Math.round(avgTime),
          qps: Math.round(iterations / (end - start) * 1000),
          message: `执行 ${iterations} 次查询，总耗时 ${Math.round(end - start)}ms，平均 ${Math.round(avgTime)}ms/次`,
          status: end - start < 2000 ? 'fast' : end - start < 3000 ? 'medium' : 'slow'
        };
        break;
      }

      case 'full-benchmark': {
        const benchmark: any = { action: 'full-benchmark', tests: [] };

        // 测试 1: 连接
        let start = performance.now();
        await createMySQL({
          host: '127.0.0.1',
          user: 'test_user',
          password: 'test123456',
          database: 'test_db'
        });
        let end = performance.now();
        benchmark.tests.push({ name: '连接测试', time: Math.round(end - start), status: end - start < 100 ? 'fast' : 'medium' });

        // 测试 2: 简单查询
        start = performance.now();
        await db.query('SELECT * FROM users LIMIT 10');
        end = performance.now();
        benchmark.tests.push({ name: '简单查询', time: Math.round(end - start), status: end - start < 50 ? 'fast' : 'medium' });

        // 测试 3: 复杂查询
        start = performance.now();
        await db.query('SELECT * FROM users WHERE id > ? ORDER BY username DESC LIMIT 10', [5]);
        end = performance.now();
        benchmark.tests.push({ name: '复杂查询', time: Math.round(end - start), status: end - start < 100 ? 'fast' : 'medium' });

        // 测试 4: 单次插入
        const timestamp = Date.now();
        start = performance.now();
        await db.insert('users', {
          username: `bench_${timestamp}`,
          email: `bench_${timestamp}@test.com`,
          password_hash: 'hash'
        });
        end = performance.now();
        benchmark.tests.push({ name: '单次插入', time: Math.round(end - start), status: end - start < 50 ? 'fast' : 'medium' });

        // 测试 5: 批量插入（小批量）
        start = performance.now();
        await db.beginTransaction();
        try {
          for (let i = 0; i < 10; i++) {
            await db.insert('users', {
              username: `bench_batch_${i}_${timestamp}`,
              email: `batch${i}_${timestamp}@test.com`,
              password_hash: `hash${i}`
            });
          }
          await db.commit();
        } catch (error) {
          await db.rollback();
          throw error;
        }
        end = performance.now();
        benchmark.tests.push({ name: '批量插入(10条)', time: Math.round(end - start), status: end - start < 200 ? 'fast' : 'medium' });

        // 计算总时间
        const totalTime = benchmark.tests.reduce((sum: number, t: any) => sum + t.time, 0);

        result = {
          success: true,
          action: 'full-benchmark',
          totalTime,
          tests: benchmark.tests,
          message: `✓ 完整基准测试完成，总耗时 ${totalTime}ms`
        };
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
