export default Page(async function(ctx, { createMySQL, response }) {
  const action = ctx.query.action || 'query';

  try {
    const db = await createMySQL({
      host: '127.0.0.1',
      port: 3306,
      user: 'test_user',
      password: 'test123456',
      database: 'test_db'
    });

    switch (action) {
      case 'query': {
        const users = await db.query('SELECT * FROM users ORDER BY id');
        return response.json({
          success: true,
          action: 'query',
          count: users.length,
          users: users
        });
      }

      case 'param-query': {
        // 测试参数化查询
        const user = await db.query(
          'SELECT * FROM users WHERE id = ?',
          [1]
        );
        return response.json({
          success: true,
          action: 'param-query',
          user: user
        });
      }

      case 'insert': {
        // 测试插入数据
        const timestamp = Date.now();
        const insertId = await db.insert('users', {
          username: `e2e_test_user_${timestamp}`,
          email: `e2e_test_${timestamp}@example.com`,
          password_hash: 'hashed_password_e2e'
        });

        return response.json({
          success: true,
          action: 'insert',
          insertId: insertId
        });
      }

      case 'update': {
        // 测试更新数据
        const affectedRows = await db.update(
          'users',
          { username: 'updated_by_e2e' },
          { id: 1 }
        );

        return response.json({
          success: true,
          action: 'update',
          affectedRows: affectedRows
        });
      }

      case 'delete': {
        // 测试删除数据（先创建一个测试记录再删除）
        const timestamp = Date.now();
        const insertId = await db.insert('users', {
          username: `to_be_deleted_${timestamp}`,
          email: `delete_${timestamp}@example.com`,
          password_hash: 'will_be_deleted'
        });

        const deletedRows = await db.delete('users', { id: insertId });

        return response.json({
          success: true,
          action: 'delete',
          deletedRows: deletedRows
        });
      }

      case 'transaction': {
        // 测试事务操作
        await db.beginTransaction();

        try {
          const timestamp = Date.now();

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

          await db.commit();

          return response.json({
            success: true,
            action: 'transaction',
            message: '事务成功提交'
          });
        } catch (error) {
          await db.rollback();
          throw error;
        }
      }

      case 'transaction-rollback': {
        // 测试事务回滚
        await db.beginTransaction();

        try {
          const timestamp = Date.now();

          await db.insert('users', {
            username: `rollback_test_${timestamp}`,
            email: `rollback_${timestamp}@example.com`,
            password_hash: 'will_be_rolled_back'
          });

          // 故意抛出错误以触发回滚
          throw new Error('Intentional error for rollback test');
        } catch (error) {
          await db.rollback();

          // 验证回滚是否成功（查询刚才插入的记录应该不存在）
          const users = await db.query(
            'SELECT * FROM users WHERE username LIKE ?',
            [`rollback_test_%`]
          );

          return response.json({
            success: true,
            action: 'transaction-rollback',
            message: '事务回滚成功',
            rolledBackRecords: users.length
          });
        }
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
