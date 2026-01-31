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
              <style>{`
                body { font-family: Arial, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; }
                h1 { color: #333; border-bottom: 3px solid #007bff; padding-bottom: 10px; }
                h2 { color: #555; margin-top: 30px; }
                .section { background: #f8f9fa; padding: 20px; margin: 20px 0; border-radius: 8px; }
                .button {
                  background: #007bff;
                  color: white;
                  padding: 10px 20px;
                  border: none;
                  border-radius: 5px;
                  cursor: pointer;
                  margin: 5px;
                  text-decoration: none;
                  display: inline-block;
                }
                .button:hover { background: #0056b3; }
                .success { background: #28a745; }
                .success:hover { background: #1e7e34; }
                .warning { background: #ffc107; color: #000; }
                .warning:hover { background: #e0a800; }
                table { width: 100%; border-collapse: collapse; margin: 20px 0; }
                th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }
                th { background: #007bff; color: white; }
                tr:hover { background: #f5f5f5; }
                .code {
                  background: #f4f4f4;
                  padding: 15px;
                  border-radius: 5px;
                  overflow-x: auto;
                  font-family: 'Courier New', monospace;
                  font-size: 14px;
                }
                .result {
                  background: white;
                  padding: 15px;
                  margin: 10px 0;
                  border-left: 4px solid #28a745;
                  border-radius: 4px;
                }
                .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px; }
              `}</style>
            </head>
            <body>
              <h1>🗄️ MySQL 功能演示仪表板</h1>

              <div className="section">
                <h2>📊 数据库概览</h2>
                <div className="grid">
                  <div>
                    <h3>当前用户数</h3>
                    <p style={{ fontSize: '48px', margin: '20px 0', color: '#007bff' }}>
                      {users.length}
                    </p>
                  </div>
                  <div>
                    <h3>数据库连接</h3>
                    <p><strong>主机:</strong> 127.0.0.1:3306</p>
                    <p><strong>数据库:</strong> test_db</p>
                    <p><strong>用户:</strong> test_user</p>
                    <p><strong>状态:</strong> <span style={{ color: '#28a745', fontWeight: 'bold' }}>✓ 连接成功</span></p>
                  </div>
                </div>
              </div>

              <div className="section">
                <h2>🧪 功能测试</h2>
                <div className="grid">
                  <div>
                    <h3>1. 基本查询</h3>
                    <p>查询所有用户数据</p>
                    <a href="/mysql-demo.tsx?action=query" className="button">测试查询</a>
                  </div>
                  <div>
                    <h3>2. 参数化查询</h3>
                    <p>使用参数防止 SQL 注入</p>
                    <a href="/mysql-demo.tsx?action=param-query&id=1" className="button">测试参数化</a>
                  </div>
                  <div>
                    <h3>3. 插入数据</h3>
                    <p>向 users 表插入新记录</p>
                    <a href="/mysql-demo.tsx?action=insert" className="button success">测试插入</a>
                  </div>
                  <div>
                    <h3>4. 更新数据</h3>
                    <p>更新 ID=1 的用户名</p>
                    <a href="/mysql-demo.tsx?action=update" className="button warning">测试更新</a>
                  </div>
                  <div>
                    <h3>5. 删除数据</h3>
                    <p>创建并删除一条记录</p>
                    <a href="/mysql-demo.tsx?action=delete" className="button">测试删除</a>
                  </div>
                  <div>
                    <h3>6. 事务操作</h3>
                    <p>测试事务提交和回滚</p>
                    <a href="/mysql-demo.tsx?action=transaction" className="button success">测试事务</a>
                  </div>
                </div>
              </div>

              <div className="section">
                <h2>👥 最近 5 条用户记录</h2>
                <table>
                  <thead>
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

              <div className="section">
                <h2>💡 使用示例</h2>

                <h3>基本查询</h3>
                <div className="code">
                  <pre>{`const db = await createMySQL({
  host: '127.0.0.1',
  user: 'test_user',
  password: 'test123456',
  database: 'test_db'
});

const users = await db.query('SELECT * FROM users');
return response.json(users);`}</pre>
                </div>

                <h3>参数化查询（防止 SQL 注入）</h3>
                <div className="code">
                  <pre>{`const user = await db.query(
  'SELECT * FROM users WHERE id = ?',
  [userId]
);`}</pre>
                </div>

                <h3>插入数据</h3>
                <div className="code">
                  <pre>{`const insertId = await db.insert('users', {
  username: 'Alice',
  email: 'alice@example.com',
  password_hash: 'hashed_password'
});`}</pre>
                </div>

                <h3>事务操作</h3>
                <div className="code">
                  <pre>{`await db.beginTransaction();
try {
  await db.insert('posts', { title: 'Post 1' });
  await db.insert('posts', { title: 'Post 2' });
  await db.commit();
} catch (error) {
  await db.rollback();
  throw error;
}`}</pre>
                </div>
              </div>

              <div className="section">
                <h2>📚 更多资源</h2>
                <ul style={{ lineHeight: '2' }}>
                  <li><a href="/test-mysql">查看简单的 JSON API 示例</a></li>
                  <li><a href="/test-mysql-insert">测试插入操作</a></li>
                  <li><a href="/test-mysql-update">测试更新操作</a></li>
                  <li><a href="/test-mysql-transaction">测试事务操作</a></li>
                  <li><a href="/MYSQL_INTEGRATION.md">查看完整文档</a></li>
                </ul>
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
