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
            <style>{`
              body {
                font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                max-width: 1200px;
                margin: 0 auto;
                padding: 20px;
                background: #f8f9fa;
              }
              .header {
                background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                color: white;
                padding: 40px;
                border-radius: 10px;
                margin-bottom: 30px;
                text-align: center;
              }
              .header h1 { margin: 0 0 10px 0; }
              .header p { margin: 0; opacity: 0.9; }
              .card {
                background: white;
                border-radius: 10px;
                padding: 30px;
                margin-bottom: 20px;
                box-shadow: 0 2px 4px rgba(0,0,0,0.1);
              }
              .card h2 {
                margin-top: 0;
                color: #333;
                border-bottom: 2px solid #667eea;
                padding-bottom: 10px;
              }
              .test-button {
                display: inline-block;
                padding: 12px 24px;
                margin: 10px;
                background: #667eea;
                color: white;
                text-decoration: none;
                border-radius: 5px;
                transition: background 0.3s;
              }
              .test-button:hover { background: #5568d3; }
              .result {
                background: #f8f9fa;
                padding: 20px;
                border-radius: 5px;
                margin-top: 20px;
                border-left: 4px solid #28a745;
              }
              .result h3 { margin-top: 0; color: #28a745; }
              .metric {
                display: inline-block;
                margin: 20px;
                text-align: center;
              }
              .metric-value {
                font-size: 3em;
                font-weight: bold;
                color: #667eea;
              }
              .metric-label {
                color: #666;
                margin-top: 5px;
              }
              table {
                width: 100%;
                border-collapse: collapse;
                margin: 20px 0;
              }
              th, td {
                padding: 12px;
                text-align: left;
                border-bottom: 1px solid #ddd;
              }
              th { background: #667eea; color: white; }
              .fast { color: #28a745; font-weight: bold; }
              .medium { color: #ffc107; font-weight: bold; }
              .slow { color: #dc3545; font-weight: bold; }
            `}</style>
          </head>
          <body>
            <div className="header">
              <h1>⚡ MySQL 性能测试</h1>
              <p>测试 MySQL 客户端的各种性能指标</p>
            </div>

            <div className="card">
              <h2>🧪 性能测试</h2>
              <p>点击下面的按钮运行各种性能测试：</p>

              <div style={{ marginTop: '20px' }}>
                <a href="/mysql-performance.tsx?action=connection" className="test-button">
                  🔗 连接测试
                </a>
                <a href="/mysql-performance.tsx?action=query-simple" className="test-button">
                  📊 简单查询
                </a>
                <a href="/mysql-performance.tsx?action=query-complex" className="test-button">
                  🔍 复杂查询
                </a>
                <a href="/mysql-performance.tsx?action=insert-single" className="test-button">
                  ➕ 单次插入
                </a>
                <a href="/mysql-performance.tsx?action=insert-batch" className="test-button">
                  ➕➕ 批量插入
                </a>
              </div>

              <div style={{ marginTop: '20px' }}>
                <a href="/mysql-performance.tsx?action=stress-test" className="test-button" style={{ background: '#dc3545' }}>
                  🔥 压力测试
                </a>
                <a href="/mysql-performance.tsx?action=full-benchmark" className="test-button" style={{ background: '#28a745' }}>
                  📈 完整基准测试
                </a>
              </div>
            </div>

            <div className="card">
              <h2>📖 测试说明</h2>
              <table>
                <thead>
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

            <div className="card">
              <h2>💡 性能优化建议</h2>
              <ul>
                <li><strong>使用连接池：</strong> mysql2 内置连接池，自动管理连接复用</li>
                <li><strong>批量操作：</strong> 使用事务进行批量插入/更新，性能更好</li>
                <li><strong>参数化查询：</strong> 使用参数化查询不仅安全，还能提升性能</li>
                <li><strong>索引优化：</strong> 为经常查询的字段添加索引</li>
                <li><strong>避免 SELECT *：</strong> 只查询需要的字段</li>
              </ul>
            </div>

            <div className="card">
              <h2>🔗 相关链接</h2>
              <p>
                <a href="/mysql-demo.tsx">MySQL 演示仪表板</a> |
                <a href="/mysql-advanced.tsx">MySQL 高级功能</a> |
                <a href="/">返回首页</a>
              </p>
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
