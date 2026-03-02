/**
 * MySQL E2E Test Page - TSP File
 */

export default Page(async function(ctx, { createMySQL, createZod, response, query }) {
  const z = await createZod();
  const { action = 'query' } = query(z.object({
    action: z.string().default('query')
  }));

  try {
    const db = await createMySQL({
      host: Deno.env.get("MYSQL_HOST") || "127.0.0.1",
      port: Number(Deno.env.get("MYSQL_PORT")) || 3306,
      user: Deno.env.get("MYSQL_USER") || "test_user",
      password: Deno.env.get("MYSQL_PASSWORD") || "test123456",
      database: Deno.env.get("MYSQL_DATABASE") || "test_db"
    }, z);

    // Define Schema
    const UserSchema = z.object({
      id: z.number(),
      username: z.string(),
      email: z.string(),
      created_at: z.any().optional()
    });

    const ResultSchema = z.object({
      affectedRows: z.number(),
      insertId: z.number()
    });

    switch (action) {
      case 'query': {
        const users = await db.query(UserSchema, 'SELECT * FROM users ORDER BY id');
        return response.json({
          success: true,
          action: 'query',
          count: users.length,
          users: users
        });
      }

      case 'param-query': {
        const users = await db.query(
          UserSchema,
          'SELECT * FROM users WHERE id = ?',
          [1]
        );
        return response.json({
          success: true,
          action: 'param-query',
          user: users.length > 0 ? users[0] : null
        });
      }

      case 'insert': {
        const timestamp = Date.now();
        const result = await db.execute(
          ResultSchema,
          'INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)',
          [`e2e_test_user_${timestamp}`, `e2e_test_${timestamp}@example.com`, 'hashed_password_e2e']
        );

        return response.json({
          success: true,
          action: 'insert',
          insertId: result.insertId
        });
      }

      case 'update': {
        const result = await db.execute(
          ResultSchema,
          'UPDATE users SET username = ? WHERE id = ?',
          ['updated_by_e2e', 1]
        );

        return response.json({
          success: true,
          action: 'update',
          affectedRows: result.affectedRows
        });
      }

      case 'delete': {
        const timestamp = Date.now();

        // Insert first
        const insertResult = await db.execute(
          ResultSchema,
          'INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)',
          [`to_be_deleted_${timestamp}`, `delete_${timestamp}@example.com`, 'will_be_deleted']
        );

        // Then delete
        const deleteResult = await db.execute(
          ResultSchema,
          'DELETE FROM users WHERE id = ?',
          [insertResult.insertId]
        );

        return response.json({
          success: true,
          action: 'delete',
          deletedRows: deleteResult.affectedRows
        });
      }

      case 'transaction': {
        const timestamp = Date.now();

        const result = await db.tx(async (tx) => {
          await tx.execute(
            ResultSchema,
            'INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)',
            [`trans_user_1_${timestamp}`, `trans1_${timestamp}@example.com`, 'hashed_trans_1']
          );

          await tx.execute(
            ResultSchema,
            'INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)',
            [`trans_user_2_${timestamp}`, `trans2_${timestamp}@example.com`, 'hashed_trans_2']
          );

          return { message: 'Transaction committed successfully' };
        });

        return response.json({
          success: true,
          action: 'transaction',
          message: result.message
        });
      }

      case 'transaction-rollback': {
        const timestamp = Date.now();

        try {
          await db.tx(async (tx) => {
            await tx.execute(
              ResultSchema,
              'INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)',
              [`rollback_test_${timestamp}`, `rollback_${timestamp}@example.com`, 'will_be_rolled_back']
            );

            throw new Error('Intentional rollback');
          });

          return response.json({
            success: false,
            action: 'transaction-rollback',
            message: 'Rollback did not work'
          });
        } catch (error) {
          // Verify rollback succeeded
          const users = await db.query(
            UserSchema,
            'SELECT * FROM users WHERE username = ?',
            [`rollback_test_${timestamp}`]
          );

          return response.json({
            success: true,
            action: 'transaction-rollback',
            message: 'Transaction rolled back successfully',
            verified: users.length === 0
          });
        }
      }

      default:
        return response.error('Unknown action', 400);
    }
  } catch (error) {
    return response.json({
      success: false,
      error: (error as Error).message
    }, 500);
  }
});
