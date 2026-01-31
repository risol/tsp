export default Page(async function(ctx, { createMySQL, response }) {
  const action = ctx.query.action || 'demo';

  try {
    const db = await createMySQL({
      host: '127.0.0.1',
      port: 3306,
      user: 'test_user',
      password: 'test123456',
      database: 'test_db'
    });

    switch (action) {
      case 'demo': {
        // 演示模式：返回 HTML 页面
        const users = await db.query('SELECT * FROM users ORDER BY id DESC LIMIT 5');

        return (
          <html>
          <head>
            <title>MySQL 功能演示</title>
            <meta charset="utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1" />
            <link href="/static/css/bootstrap.min.css" rel="stylesheet" />
          </head>
          <body className="container py-4">
            <h1 className="display-5 mb-4 pb-2 border-bottom border-primary">🗄️ MySQL 功能演示仪表板</h1>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="card-title h4 mb-4">📊 数据库概览</h2>
                <div className="row g-4">
                  <div className="col-md-6">
                    <div className="card bg-light">
                      <div className="card-body text-center">
                        <h3 className="h6 text-muted mb-3">当前用户数</h3>
                        <p className="display-4 fw-bold text-primary mb-0">
                          {users.length}
                        </p>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6">
                    <div className="card bg-light">
                      <div className="card-body">
                        <h3 className="h6 text-muted mb-3">数据库连接</h3>
                        <p className="mb-1"><strong>主机:</strong> 127.0.0.1:3306</p>
                        <p className="mb-1"><strong>数据库:</strong> test_db</p>
                        <p className="mb-1"><strong>用户:</strong> test_user</p>
                        <p><strong>状态:</strong> <span className="badge bg-success fs-6">✓ 连接成功</span></p>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="card-title h4 mb-4">🧪 功能测试</h2>
                <div className="row g-4">
                  <div className="col-md-6 col-lg-4">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">1. 基本查询</h3>
                        <p className="card-text text-muted">查询所有用户数据</p>
                        <a href="/mysql-demo.tsx?action=query" className="btn btn-primary">测试查询</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-4">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">2. 参数化查询</h3>
                        <p className="card-text text-muted">使用参数防止 SQL 注入</p>
                        <a href="/mysql-demo.tsx?action=param-query&id=1" className="btn btn-primary">测试参数化</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-4">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">3. 插入数据</h3>
                        <p className="card-text text-muted">向 users 表插入新记录</p>
                        <a href="/mysql-demo.tsx?action=insert" className="btn btn-success">测试插入</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-4">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">4. 更新数据</h3>
                        <p className="card-text text-muted">更新 ID=1 的用户名</p>
                        <a href="/mysql-demo.tsx?action=update" className="btn btn-warning">测试更新</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-4">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">5. 删除数据</h3>
                        <p className="card-text text-muted">创建并删除一条记录</p>
                        <a href="/mysql-demo.tsx?action=delete" className="btn btn-primary">测试删除</a>
                      </div>
                    </div>
                  </div>
                  <div className="col-md-6 col-lg-4">
                    <div className="card h-100">
                      <div className="card-body">
                        <h3 className="h5 card-title">6. 事务操作</h3>
                        <p className="card-text text-muted">测试事务提交和回滚</p>
                        <a href="/mysql-demo.tsx?action=transaction" className="btn btn-success">测试事务</a>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="card-title h4 mb-4">👥 最近 5 条用户记录</h2>
                <div className="table-responsive">
                  <table className="table table-striped table-hover">
                    <thead className="table-dark">
                      <tr>
                        <th>ID</th>
                        <th>用户名</th>
                        <th>邮箱</th>
                        <th>创建时间</th>
                      </tr>
                    </thead>
                    <tbody>
                      {users.map((user: any) => (
                        <tr key={user.id}>
                          <td>{user.id}</td>
                          <td>{user.username}</td>
                          <td>{user.email}</td>
                          <td>{new Date(user.created_at).toLocaleString('zh-CN')}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="card-title h4 mb-4">💡 使用示例</h2>

                <h3 className="h5 mb-3">基本查询</h3>
                <div className="bg-light p-3 rounded mb-4">
                  <pre className="mb-0 text-dark"><code>{`const db = await createMySQL({
  host: '127.0.0.1',
  user: 'test_user',
  password: 'test123456',
  database: 'test_db'
});

const users = await db.query('SELECT * FROM users');
return response.json(users);`}</code></pre>
                </div>

                <h3 className="h5 mb-3">参数化查询（防止 SQL 注入）</h3>
                <div className="bg-light p-3 rounded mb-4">
                  <pre className="mb-0 text-dark"><code>{`const user = await db.query(
  'SELECT * FROM users WHERE id = ?',
  [userId]
);`}</code></pre>
                </div>

                <h3 className="h5 mb-3">插入数据</h3>
                <div className="bg-light p-3 rounded mb-4">
                  <pre className="mb-0 text-dark"><code>{`const insertId = await db.insert('users', {
  username: 'Alice',
  email: 'alice@example.com',
  password_hash: 'hashed_password'
});`}</code></pre>
                </div>

                <h3 className="h5 mb-3">事务操作</h3>
                <div className="bg-light p-3 rounded">
                  <pre className="mb-0 text-dark"><code>{`await db.beginTransaction();
try {
  await db.insert('posts', { title: 'Post 1' });
  await db.insert('posts', { title: 'Post 2' });
  await db.commit();
} catch (error) {
  await db.rollback();
  throw error;
}`}</code></pre>
                </div>
              </div>
            </div>

            <div className="card shadow-sm mb-4">
              <div className="card-body">
                <h2 className="card-title h4 mb-3">📚 更多资源</h2>
                <ul className="list-unstyled">
                  <li className="mb-2"><a href="/test-mysql">查看简单的 JSON API 示例</a></li>
                  <li className="mb-2"><a href="/test-mysql-insert">测试插入操作</a></li>
                  <li className="mb-2"><a href="/test-mysql-update">测试更新操作</a></li>
                  <li className="mb-2"><a href="/test-mysql-transaction">测试事务操作</a></li>
                  <li className="mb-2"><a href="/MYSQL_INTEGRATION.md">查看完整文档</a></li>
                </ul>
              </div>
            </div>
          </body>
        </html>
        );
      }

      case 'query': {
        // 基本查询测试 - 返回 JSON
        const users = await db.query('SELECT * FROM users ORDER BY id');
        return response.json({
          success: true,
          action: 'query',
          count: users.length,
          message: `成功查询 ${users.length} 条记录`,
          users: users
        });
      }

      case 'param-query': {
        // 参数化查询测试
        const userId = ctx.query.id || '1';
        const users = await db.query(
          'SELECT * FROM users WHERE id = ?',
          [userId]
        );
        return response.json({
          success: true,
          action: 'param-query',
          message: `参数化查询：ID = ${userId}`,
          users: users
        });
      }

      case 'insert': {
        // 插入数据测试
        const timestamp = Date.now();
        const username = `demo_user_${timestamp}`;
        const insertId = await db.insert('users', {
          username: username,
          email: `demo_${timestamp}@example.com`,
          password_hash: 'hashed_password_demo'
        });

        return response.json({
          success: true,
          action: 'insert',
          message: '✓ 数据插入成功',
          insertId: insertId,
          data: {
            username: username,
            email: `demo_${timestamp}@example.com`
          }
        });
      }

      case 'update': {
        // 更新数据测试
        const timestamp = Date.now();
        const affectedRows = await db.update(
          'users',
          { username: `updated_user_${timestamp}` },
          { id: 1 }
        );

        return response.json({
          success: true,
          action: 'update',
          message: `✓ 成功更新 ${affectedRows} 条记录`,
          affectedRows: affectedRows,
          newUsername: `updated_user_${timestamp}`
        });
      }

      case 'delete': {
        // 删除数据测试
        const timestamp = Date.now();

        // 先创建一条记录
        const insertId = await db.insert('users', {
          username: `to_be_deleted_${timestamp}`,
          email: `delete_${timestamp}@example.com`,
          password_hash: 'will_be_deleted'
        });

        // 然后删除它
        const deletedRows = await db.delete('users', { id: insertId });

        return response.json({
          success: true,
          action: 'delete',
          message: `✓ 成功删除 ${deletedRows} 条记录`,
          deletedRows: deletedRows,
          deletedId: insertId
        });
      }

      case 'transaction': {
        // 事务测试
        await db.beginTransaction();

        try {
          const timestamp = Date.now();

          // 插入两条记录
          await db.insert('users', {
            username: `trans_user_1_${timestamp}`,
            email: `trans1_${timestamp}@example.com`,
            password_hash: 'hashed_trans_1'
          });

          await db.insert('users', {
            username: `trans_user_2_${timestamp}`,
            email: `trans2_${timestamp}@example.com`,
            password_hash: 'hashed_trans_2'
          });

          // 提交事务
          await db.commit();

          return response.json({
            success: true,
            action: 'transaction',
            message: '✓ 事务成功提交，插入 2 条记录',
            timestamp: timestamp
          });
        } catch (error) {
          await db.rollback();
          throw error;
        }
      }

      case 'transaction-rollback': {
        // 事务回滚测试
        await db.beginTransaction();

        try {
          const timestamp = Date.now();

          await db.insert('users', {
            username: `rollback_test_${timestamp}`,
            email: `rollback_${timestamp}@example.com`,
            password_hash: 'will_be_rolled_back'
          });

          // 故意抛出错误触发回滚
          throw new Error('Intentional error for rollback test');
        } catch (error) {
          await db.rollback();

          // 验证回滚是否成功
          const users = await db.query(
            'SELECT * FROM users WHERE username LIKE ?',
            [`rollback_test_%`]
          );

          return response.json({
            success: true,
            action: 'transaction-rollback',
            message: '✓ 事务回滚成功',
            rolledBackRecords: users.length,
            verified: users.length === 0
          });
        }
      }

      case 'batch-insert': {
        // 批量插入测试（使用事务）
        await db.beginTransaction();

        try {
          const timestamp = Date.now();
          const insertIds = [];

          for (let i = 1; i <= 5; i++) {
            const insertId = await db.insert('users', {
              username: `batch_user_${i}_${timestamp}`,
              email: `batch${i}_${timestamp}@example.com`,
              password_hash: `hashed_batch_${i}`
            });
            insertIds.push(insertId);
          }

          await db.commit();

          return response.json({
            success: true,
            action: 'batch-insert',
            message: `✓ 批量插入成功（使用事务）`,
            count: insertIds.length,
            insertIds: insertIds
          });
        } catch (error) {
          await db.rollback();
          throw error;
        }
      }

      case 'stats': {
        // 统计信息
        const [userCount] = await db.query<Array<{ count: number }>>('SELECT COUNT(*) as count FROM users');
        const [latestUser] = await db.query<Array<any>>('SELECT * FROM users ORDER BY id DESC LIMIT 1');
        const [oldestUser] = await db.query<Array<any>>('SELECT * FROM users ORDER BY id ASC LIMIT 1');

        return response.json({
          success: true,
          action: 'stats',
          stats: {
            totalUsers: (userCount as any).count,
            latestUser: latestUser,
            oldestUser: oldestUser
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
