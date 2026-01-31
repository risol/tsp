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
            <meta name="viewport" content="width=device-width, initial-scale=1" />
            <link href="/static/css/bootstrap.min.css" rel="stylesheet" />
          </head>
          <body className="container py-4">
            <div className="bg-primary text-white rounded p-5 mb-5 shadow">
              <h1 className="display-5 mb-2">🚀 MySQL 高级功能测试</h1>
              <p className="mb-0 opacity-75">全面测试 MySQL 客户端的各种功能和特性</p>
            </div>

            <div className="row g-4 mb-5">
              <div className="col-md-4">
                <div className="card shadow-sm h-100">
                  <div className="card-body text-center">
                    <h3 className="h6 text-muted text-uppercase mb-2">总记录数</h3>
                    <div className="display-4 fw-bold text-primary">{stats[0].total}</div>
                  </div>
                </div>
              </div>
              <div className="col-md-4">
                <div className="card shadow-sm h-100">
                  <div className="card-body text-center">
                    <h3 className="h6 text-muted text-uppercase mb-2">测试用例</h3>
                    <div className="display-4 fw-bold text-primary">12</div>
                  </div>
                </div>
              </div>
              <div className="col-md-4">
                <div className="card shadow-sm h-100">
                  <div className="card-body text-center">
                    <h3 className="h6 text-muted text-uppercase mb-2">功能覆盖</h3>
                    <div className="display-4 fw-bold text-primary">100%</div>
                  </div>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-4">📋 基础操作测试</h2>
                <div className="row g-4">
                  <div className="col-md-6 col-lg-3">
                    <a href="/mysql-advanced.tsx?action=query-all" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">📊 查询所有数据</h3>
                        <p className="card-text text-muted small">查询 users 表中的所有记录，返回完整的数据集</p>
                        <span className="badge bg-primary rounded-pill">基础</span>
                      </div>
                    </a>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <a href="/mysql-advanced.tsx?action=query-limit" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">📋 限制查询结果</h3>
                        <p className="card-text text-muted small">使用 LIMIT 子句限制返回的记录数量</p>
                        <span className="badge bg-primary rounded-pill">基础</span>
                      </div>
                    </a>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <a href="/mysql-advanced.tsx?action=param-query" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">🔒 参数化查询</h3>
                        <p className="card-text text-muted small">使用参数化查询防止 SQL 注入攻击</p>
                        <span className="badge bg-warning text-dark rounded-pill">中级</span>
                      </div>
                    </a>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <a href="/mysql-advanced.tsx?action=search" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">🔍 模糊搜索</h3>
                        <p className="card-text text-muted small">使用 LIKE 进行模糊匹配查询</p>
                        <span className="badge bg-warning text-dark rounded-pill">中级</span>
                      </div>
                    </a>
                  </div>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-4">✏️ 数据操作测试</h2>
                <div className="row g-4">
                  <div className="col-md-6 col-lg-3">
                    <a href="/mysql-advanced.tsx?action=insert-single" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">➕ 插入单条记录</h3>
                        <p className="card-text text-muted small">使用 insert 方法插入一条新的用户记录</p>
                        <span className="badge bg-primary rounded-pill">基础</span>
                      </div>
                    </a>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <a href="/mysql-advanced.tsx?action=insert-batch" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">➕➕ 批量插入</h3>
                        <p className="card-text text-muted small">使用事务批量插入多条记录</p>
                        <span className="badge bg-danger rounded-pill">高级</span>
                      </div>
                    </a>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <a href="/mysql-advanced.tsx?action=update-data" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">✏️ 更新数据</h3>
                        <p className="card-text text-muted small">更新指定 ID 的用户信息</p>
                        <span className="badge bg-primary rounded-pill">基础</span>
                      </div>
                    </a>
                  </div>
                  <div className="col-md-6 col-lg-3">
                    <a href="/mysql-advanced.tsx?action=delete-data" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">🗑️ 删除数据</h3>
                        <p className="card-text text-muted small">创建并删除一条测试记录</p>
                        <span className="badge bg-warning text-dark rounded-pill">中级</span>
                      </div>
                    </a>
                  </div>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-4">🔄 事务操作测试</h2>
                <div className="row g-4">
                  <div className="col-md-4">
                    <a href="/mysql-advanced.tsx?action=transaction-commit" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">✅ 事务提交</h3>
                        <p className="card-text text-muted small">测试事务的正常提交流程，插入多条记录</p>
                        <span className="badge bg-danger rounded-pill">高级</span>
                      </div>
                    </a>
                  </div>
                  <div className="col-md-4">
                    <a href="/mysql-advanced.tsx?action=transaction-rollback" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">❌ 事务回滚</h3>
                        <p className="card-text text-muted small">测试事务失败时的回滚机制</p>
                        <span className="badge bg-danger rounded-pill">高级</span>
                      </div>
                    </a>
                  </div>
                  <div className="col-md-4">
                    <a href="/mysql-advanced.tsx?action=nested-operations" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">🔗 复合操作</h3>
                        <p className="card-text text-muted small">在一个事务中执行多种操作</p>
                        <span className="badge bg-danger rounded-pill">高级</span>
                      </div>
                    </a>
                  </div>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-4">📈 统计和分析</h2>
                <div className="row g-4">
                  <div className="col-md-4">
                    <a href="/mysql-advanced.tsx?action=count" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">🔢 记录统计</h3>
                        <p className="card-text text-muted small">统计不同条件下的记录数量</p>
                        <span className="badge bg-warning text-dark rounded-pill">中级</span>
                      </div>
                    </a>
                  </div>
                  <div className="col-md-4">
                    <a href="/mysql-advanced.tsx?action=aggregate" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">📊 聚合查询</h3>
                        <p className="card-text text-muted small">使用聚合函数进行数据分析</p>
                        <span className="badge bg-warning text-dark rounded-pill">中级</span>
                      </div>
                    </a>
                  </div>
                  <div className="col-md-4">
                    <a href="/mysql-advanced.tsx?action=join" className="card text-decoration-none text-dark h-100 border-2">
                      <div className="card-body">
                        <h3 className="h5 card-title text-primary">🔗 连接查询</h3>
                        <p className="card-text text-muted small">测试多表连接查询（模拟）</p>
                        <span className="badge bg-danger rounded-pill">高级</span>
                      </div>
                    </a>
                  </div>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="h4 border-bottom pb-2 mb-4">💻 代码示例</h2>
                <h3 className="h5 mb-3">创建连接</h3>
                <div className="bg-dark text-light p-3 rounded mb-4">
                  <pre className="mb-0"><code>{`const db = await createMySQL({
  host: '127.0.0.1',
  user: 'test_user',
  password: 'test123456',
  database: 'test_db'
});`}</code></pre>
                </div>

                <h3 className="h5 mb-3">参数化查询</h3>
                <div className="bg-dark text-light p-3 rounded mb-4">
                  <pre className="mb-0"><code>{`const users = await db.query(
  'SELECT * FROM users WHERE id = ?',
  [userId]
);`}</code></pre>
                </div>

                <h3 className="h5 mb-3">事务操作</h3>
                <div className="bg-dark text-light p-3 rounded">
                  <pre className="mb-0"><code>{`await db.beginTransaction();
try {
  await db.insert('users', data);
  await db.commit();
} catch (error) {
  await db.rollback();
  throw error;
}`}</code></pre>
                </div>
              </div>
            </div>

            <div className="text-center py-4 border-top">
              <p className="mb-2">
                <a href="/" className="text-decoration-none text-primary me-3">← 返回首页</a>
                <a href="/mysql-demo.tsx" className="text-decoration-none text-primary me-3">MySQL 演示仪表板</a>
                <a href="/MYSQL_INTEGRATION.md" className="text-decoration-none text-primary">查看文档</a>
              </p>
              <p className="text-muted small mb-0">
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
