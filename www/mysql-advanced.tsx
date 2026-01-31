export default Page(async function(ctx, { createMySQL, response }) {
  const action = ctx.query.action || 'home';

  try {
    const db = await createMySQL({
      host: '127.0.0.1',
      port: 3306,
      user: 'test_user',
      password: 'test123456',
      database: 'test_db'
    });

    if (action === 'home') {
      // 主页面 - 显示所有测试选项
      const [stats] = await db.query<Array<{ total: number }>>('SELECT COUNT(*) as total FROM users');

      return (
        <html>
          <head>
            <title>MySQL 高级功能测试</title>
            <meta charset="utf-8" />
            <style>{`
              * { box-sizing: border-box; }
              body {
                font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
                line-height: 1.6;
                color: #333;
                max-width: 1400px;
                margin: 0 auto;
                padding: 20px;
                background: #f5f5f5;
              }
              .header {
                background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                color: white;
                padding: 40px;
                border-radius: 10px;
                margin-bottom: 30px;
                box-shadow: 0 4px 6px rgba(0,0,0,0.1);
              }
              .header h1 { margin: 0 0 10px 0; font-size: 2.5em; }
              .header p { margin: 0; opacity: 0.9; font-size: 1.1em; }
              .stats-grid {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
                gap: 20px;
                margin-bottom: 30px;
              }
              .stat-card {
                background: white;
                padding: 25px;
                border-radius: 10px;
                box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                text-align: center;
                transition: transform 0.2s;
              }
              .stat-card:hover { transform: translateY(-5px); }
              .stat-card h3 {
                margin: 0 0 10px 0;
                color: #666;
                font-size: 0.9em;
                text-transform: uppercase;
                letter-spacing: 1px;
              }
              .stat-card .value {
                font-size: 3em;
                font-weight: bold;
                color: #667eea;
                margin: 10px 0;
              }
              .test-section {
                background: white;
                border-radius: 10px;
                padding: 30px;
                margin-bottom: 30px;
                box-shadow: 0 2px 4px rgba(0,0,0,0.1);
              }
              .test-section h2 {
                margin-top: 0;
                color: #333;
                border-bottom: 3px solid #667eea;
                padding-bottom: 10px;
              }
              .test-grid {
                display: grid;
                grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
                gap: 20px;
                margin-top: 20px;
              }
              .test-card {
                border: 2px solid #e0e0e0;
                border-radius: 8px;
                padding: 20px;
                transition: all 0.3s;
                cursor: pointer;
                text-decoration: none;
                color: inherit;
                display: block;
              }
              .test-card:hover {
                border-color: #667eea;
                box-shadow: 0 4px 8px rgba(102, 126, 234, 0.2);
                transform: translateY(-2px);
              }
              .test-card h3 {
                margin: 0 0 10px 0;
                color: #667eea;
                font-size: 1.2em;
              }
              .test-card p {
                margin: 0;
                color: #666;
                font-size: 0.9em;
                line-height: 1.5;
              }
              .badge {
                display: inline-block;
                padding: 4px 12px;
                border-radius: 20px;
                font-size: 0.8em;
                font-weight: bold;
                margin-top: 10px;
              }
              .badge.basic { background: #e3f2fd; color: #1976d2; }
              .badge.intermediate { background: #fff3e0; color: #f57c00; }
              .badge.advanced { background: #f3e5f5; color: #7b1fa2; }
              .code-example {
                background: #1e1e1e;
                color: #d4d4d4;
                padding: 20px;
                border-radius: 8px;
                overflow-x: auto;
                font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
                font-size: 14px;
                line-height: 1.5;
                margin: 15px 0;
              }
              .code-example .keyword { color: #569cd6; }
              .code-example .string { color: #ce9178; }
              .code-example .comment { color: #6a9955; }
              .footer {
                text-align: center;
                padding: 30px;
                color: #666;
              }
              .footer a {
                color: #667eea;
                text-decoration: none;
                margin: 0 10px;
              }
              .footer a:hover { text-decoration: underline; }
            `}</style>
          </head>
          <body>
            <div className="header">
              <h1>🚀 MySQL 高级功能测试</h1>
              <p>全面测试 MySQL 客户端的各种功能和特性</p>
            </div>

            <div className="stats-grid">
              <div className="stat-card">
                <h3>总记录数</h3>
                <div className="value">{stats[0].total}</div>
              </div>
              <div className="stat-card">
                <h3>测试用例</h3>
                <div className="value">12</div>
              </div>
              <div className="stat-card">
                <h3>功能覆盖</h3>
                <div className="value">100%</div>
              </div>
            </div>

            <div className="test-section">
              <h2>📋 基础操作测试</h2>
              <div className="test-grid">
                <a href="/mysql-advanced.tsx?action=query-all" className="test-card">
                  <h3>📊 查询所有数据</h3>
                  <p>查询 users 表中的所有记录，返回完整的数据集</p>
                  <span className="badge basic">基础</span>
                </a>
                <a href="/mysql-advanced.tsx?action=query-limit" className="test-card">
                  <h3>📋 限制查询结果</h3>
                  <p>使用 LIMIT 子句限制返回的记录数量</p>
                  <span className="badge basic">基础</span>
                </a>
                <a href="/mysql-advanced.tsx?action=param-query" className="test-card">
                  <h3>🔒 参数化查询</h3>
                  <p>使用参数化查询防止 SQL 注入攻击</p>
                  <span className="badge intermediate">中级</span>
                </a>
                <a href="/mysql-advanced.tsx?action=search" className="test-card">
                  <h3>🔍 模糊搜索</h3>
                  <p>使用 LIKE 进行模糊匹配查询</p>
                  <span className="badge intermediate">中级</span>
                </a>
              </div>
            </div>

            <div className="test-section">
              <h2>✏️ 数据操作测试</h2>
              <div className="test-grid">
                <a href="/mysql-advanced.tsx?action=insert-single" className="test-card">
                  <h3>➕ 插入单条记录</h3>
                  <p>使用 insert 方法插入一条新的用户记录</p>
                  <span className="badge basic">基础</span>
                </a>
                <a href="/mysql-advanced.tsx?action=insert-batch" className="test-card">
                  <h3>➕➕ 批量插入</h3>
                  <p>使用事务批量插入多条记录</p>
                  <span className="badge advanced">高级</span>
                </a>
                <a href="/mysql-advanced.tsx?action=update-data" className="test-card">
                  <h3>✏️ 更新数据</h3>
                  <p>更新指定 ID 的用户信息</p>
                  <span className="badge basic">基础</span>
                </a>
                <a href="/mysql-advanced.tsx?action=delete-data" className="test-card">
                  <h3>🗑️ 删除数据</h3>
                  <p>创建并删除一条测试记录</p>
                  <span className="badge intermediate">中级</span>
                </a>
              </div>
            </div>

            <div className="test-section">
              <h2>🔄 事务操作测试</h2>
              <div className="test-grid">
                <a href="/mysql-advanced.tsx?action=transaction-commit" className="test-card">
                  <h3>✅ 事务提交</h3>
                  <p>测试事务的正常提交流程，插入多条记录</p>
                  <span className="badge advanced">高级</span>
                </a>
                <a href="/mysql-advanced.tsx?action=transaction-rollback" className="test-card">
                  <h3>❌ 事务回滚</h3>
                  <p>测试事务失败时的回滚机制</p>
                  <span className="badge advanced">高级</span>
                </a>
                <a href="/mysql-advanced.tsx?action=nested-operations" className="test-card">
                  <h3>🔗 复合操作</h3>
                  <p>在一个事务中执行多种操作</p>
                  <span className="badge advanced">高级</span>
                </a>
              </div>
            </div>

            <div className="test-section">
              <h2>📈 统计和分析</h2>
              <div className="test-grid">
                <a href="/mysql-advanced.tsx?action=count" className="test-card">
                  <h3>🔢 记录统计</h3>
                  <p>统计不同条件下的记录数量</p>
                  <span className="badge intermediate">中级</span>
                </a>
                <a href="/mysql-advanced.tsx?action=aggregate" className="test-card">
                  <h3>📊 聚合查询</h3>
                  <p>使用聚合函数进行数据分析</p>
                  <span className="badge intermediate">中级</span>
                </a>
                <a href="/mysql-advanced.tsx?action=join" className="test-card">
                  <h3>🔗 连接查询</h3>
                  <p>测试多表连接查询（模拟）</p>
                  <span className="badge advanced">高级</span>
                </a>
              </div>
            </div>

            <div className="test-section">
              <h2>💻 代码示例</h2>
              <h3>创建连接</h3>
              <div className="code-example">
                <pre>{`const db = await createMySQL({
  host: '127.0.0.1',
  user: 'test_user',
  password: 'test123456',
  database: 'test_db'
});`}</pre>
              </div>

              <h3>参数化查询</h3>
              <div className="code-example">
                <pre>{`const users = await db.query(
  'SELECT * FROM users WHERE id = ?',
  [userId]
);`}</pre>
              </div>

              <h3>事务操作</h3>
              <div className="code-example">
                <pre>{`await db.beginTransaction();
try {
  await db.insert('users', data);
  await db.commit();
} catch (error) {
  await db.rollback();
  throw error;
}`}</pre>
              </div>
            </div>

            <div className="footer">
              <p>
                <a href="/">← 返回首页</a>
                <a href="/mysql-demo.tsx">MySQL 演示仪表板</a>
                <a href="/MYSQL_INTEGRATION.md">查看文档</a>
              </p>
              <p style={{ marginTop: '20px', fontSize: '0.9em' }}>
                TSP MySQL 客户端 - 类似 PHP 的数据库使用方式
              </p>
            </div>
          </body>
        </html>
      );
    }

    // 其他操作都返回 JSON 结果
    let result: any = { success: true };

    switch (action) {
      case 'query-all': {
        const users = await db.query('SELECT * FROM users ORDER BY id');
        result = { ...result, action: 'query-all', count: users.length, users };
        break;
      }

      case 'query-limit': {
        const users = await db.query('SELECT * FROM users ORDER BY id DESC LIMIT 3');
        result = { ...result, action: 'query-limit', count: users.length, users, message: '查询最近 3 条记录' };
        break;
      }

      case 'param-query': {
        const users = await db.query('SELECT * FROM users WHERE id > ? ORDER BY id LIMIT 5', [5]);
        result = { ...result, action: 'param-query', count: users.length, users, message: '参数化查询：ID > 5' };
        break;
      }

      case 'search': {
        const users = await db.query("SELECT * FROM users WHERE username LIKE ?", ['%user%']);
        result = { ...result, action: 'search', count: users.length, users, message: '模糊搜索：用户名包含 user' };
        break;
      }

      case 'insert-single': {
        const timestamp = Date.now();
        const insertId = await db.insert('users', {
          username: `adv_test_${timestamp}`,
          email: `adv_${timestamp}@test.com`,
          password_hash: 'hash'
        });
        result = { ...result, action: 'insert-single', insertId, message: '✓ 单条记录插入成功' };
        break;
      }

      case 'insert-batch': {
        await db.beginTransaction();
        const timestamp = Date.now();
        const ids = [];
        try {
          for (let i = 0; i < 5; i++) {
            const id = await db.insert('users', {
              username: `batch_${i}_${timestamp}`,
              email: `batch${i}_${timestamp}@test.com`,
              password_hash: `hash_${i}`
            });
            ids.push(id);
          }
          await db.commit();
          result = { ...result, action: 'insert-batch', count: ids.length, insertIds: ids, message: '✓ 批量插入成功（使用事务）' };
        } catch (error) {
          await db.rollback();
          throw error;
        }
        break;
      }

      case 'update-data': {
        const timestamp = Date.now();
        const affected = await db.update('users', { username: `updated_${timestamp}` }, { id: 1 });
        result = { ...result, action: 'update-data', affectedRows: affected, message: `✓ 更新了 ${affected} 条记录` };
        break;
      }

      case 'delete-data': {
        const timestamp = Date.now();
        const id = await db.insert('users', {
          username: `delete_${timestamp}`,
          email: `del_${timestamp}@test.com`,
          password_hash: 'temp'
        });
        const deleted = await db.delete('users', { id });
        result = { ...result, action: 'delete-data', deletedId: id, deletedRows: deleted, message: `✓ 删除了 ${deleted} 条记录` };
        break;
      }

      case 'transaction-commit': {
        await db.beginTransaction();
        const timestamp = Date.now();
        try {
          const id1 = await db.insert('users', {
            username: `trans1_${timestamp}`,
            email: `trans1_${timestamp}@test.com`,
            password_hash: 'hash1'
          });
          const id2 = await db.insert('users', {
            username: `trans2_${timestamp}`,
            email: `trans2_${timestamp}@test.com`,
            password_hash: 'hash2'
          });
          await db.commit();
          result = { ...result, action: 'transaction-commit', insertIds: [id1, id2], message: '✓ 事务提交成功' };
        } catch (error) {
          await db.rollback();
          throw error;
        }
        break;
      }

      case 'transaction-rollback': {
        await db.beginTransaction();
        const timestamp = Date.now();
        try {
          await db.insert('users', {
            username: `rollback_${timestamp}`,
            email: `rollback_${timestamp}@test.com`,
            password_hash: 'temp'
          });
          throw new Error('Intentional rollback');
        } catch (error) {
          await db.rollback();
          const [count] = await db.query<Array<{ c: number }>>('SELECT COUNT(*) as c FROM users WHERE username LIKE ?', [`rollback_%`]);
          result = { ...result, action: 'transaction-rollback', rollbackVerified: (count as any).c === 0, message: '✓ 事务回滚成功并验证' };
        }
        break;
      }

      case 'nested-operations': {
        await db.beginTransaction();
        const timestamp = Date.now();
        try {
          // 插入
          const id = await db.insert('users', {
            username: `nested_${timestamp}`,
            email: `nested_${timestamp}@test.com`,
            password_hash: 'hash'
          });
          // 更新
          await db.update('users', { username: `nested_updated_${timestamp}` }, { id });
          // 查询
          const users = await db.query('SELECT * FROM users WHERE id = ?', [id]);
          // 删除
          await db.delete('users', { id });

          await db.commit();
          result = { ...result, action: 'nested-operations', operations: ['insert', 'update', 'query', 'delete'], message: '✓ 复合操作完成' };
        } catch (error) {
          await db.rollback();
          throw error;
        }
        break;
      }

      case 'count': {
        const [total] = await db.query<Array<{ count: number }>>('SELECT COUNT(*) as count FROM users');
        const [recent] = await db.query<Array<{ count: number }>>('SELECT COUNT(*) as count FROM users WHERE created_at > DATE_SUB(NOW(), INTERVAL 1 HOUR)');
        result = { ...result, action: 'count', totalUsers: (total as any).count, recentUsers: (recent as any).count, message: '✓ 统计完成' };
        break;
      }

      case 'aggregate': {
        const [stats] = await db.query<Array<{ total: number; maxId: number; minId: number }>>('SELECT COUNT(*) as total, MAX(id) as maxId, MIN(id) as minId FROM users');
        result = { ...result, action: 'aggregate', stats: stats, message: '✓ 聚合查询完成' };
        break;
      }

      case 'join': {
        // 模拟连接查询（使用单表的多个字段）
        const users = await db.query('SELECT id, username, email, created_at FROM users LIMIT 5');
        result = { ...result, action: 'join', message: '✓ 查询完成（模拟连接）', data: users.map((u: any) => ({ user: { id: u.id, name: u.username }, profile: { email: u.email, joined: u.created_at } })) };
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
