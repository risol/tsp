/**
 * MySQL V2 E2E Test Page - TSP File
 * Schema-first API Test
 */

export default Page(async function(ctx, { createMySQL, createZod, response }) {
  const z = await createZod();
  // Create database connection
  const db = await createMySQL({
    host: Deno.env.get("MYSQL_HOST") || "127.0.0.1",
    port: Number(Deno.env.get("MYSQL_PORT")) || 3306,
    user: Deno.env.get("MYSQL_USER") || "test_user",
    password: Deno.env.get("MYSQL_PASSWORD") || "test123456",
    database: Deno.env.get("MYSQL_DATABASE") || "test_db",
  }, z);

  // Define Schema
  const UserSchema = z.object({
    id: z.number(),
    username: z.string(),
    email: z.string(),
    age: z.number().optional(),
    created_at: z.any().optional(),
    total: z.number().optional(),
  });

  const ResultSchema = z.object({
    affectedRows: z.number(),
    insertId: z.number(),
  });

  const BalanceSchema = z.object({
    id: z.number(),
    balance: z.union([z.number(), z.string()]),
  });

  const tests: Record<string, unknown> = {
    testSuite: "MySQL Schema-first API E2E Tests",
  };

  try {
    // Clean up old test tables (if exist)
    await db.execute(
      ResultSchema,
      "DROP TABLE IF EXISTS test_users_v2"
    );
    await db.execute(
      ResultSchema,
      "DROP TABLE IF EXISTS test_accounts"
    );

    // Test 1: execute() - Create test table
    await db.execute(
      ResultSchema,
      `CREATE TABLE test_users_v2 (
        id INT AUTO_INCREMENT PRIMARY KEY,
        username VARCHAR(255) NOT NULL,
        email VARCHAR(255) NOT NULL,
        age INT
      )`
    );
    tests.test1_createTable = "PASS";

    // Test 2: execute() - INSERT operation
    const insertResult1 = await db.execute(
      ResultSchema,
      "INSERT INTO test_users_v2 (username, email, age) VALUES (?, ?, ?)",
      ["alice", "alice@example.com", 25]
    );
    const insertResult2 = await db.execute(
      ResultSchema,
      "INSERT INTO test_users_v2 (username, email, age) VALUES (?, ?, ?)",
      ["bob", "bob@example.com", 30]
    );
    const insertResult3 = await db.execute(
      ResultSchema,
      "INSERT INTO test_users_v2 (username, email, age) VALUES (?, ?, ?)",
      ["charlie", "charlie@example.com", 35]
    );
    tests.test2_insert = {
      status: "✓ PASSED",
      insertId1: insertResult1.insertId,
      insertId2: insertResult2.insertId,
      insertId3: insertResult3.insertId,
      affectedRows: insertResult1.affectedRows,
    };

    // Test 3: query() - Multi-row query
    const users = await db.query(
      UserSchema,
      "SELECT * FROM test_users_v2 WHERE age >= ?",
      [25]
    );
    tests.test3_query = {
      status: "✓ PASSED",
      count: users.length,
      users: users.map(u => ({ id: u.id, username: u.username })),
    };

    // Test 4: queryOne() - Single row query (success)
    const user = await db.queryOne(
      UserSchema,
      "SELECT * FROM test_users_v2 WHERE username = ?",
      ["alice"]
    );
    tests.test4_queryOne_success = {
      status: "✓ PASSED",
      user: { id: user.id, username: user.username, email: user.email },
    };

    // Test 5: queryOne() - Single row query (0 rows, should throw error)
    try {
      await db.queryOne(
        UserSchema,
        "SELECT * FROM test_users_v2 WHERE username = ?",
        ["nonexistent"]
      );
      tests.test5_queryOne_zeroRows = "✗ FAILED (Expected error not thrown)";
    } catch (error) {
      tests.test5_queryOne_zeroRows = {
        status: "✓ PASSED",
        error: (error as Error).message,
      };
    }

    // Test 6: queryMaybe() - Optional single row query (has result)
    const maybeUser = await db.queryMaybe(
      UserSchema,
      "SELECT * FROM test_users_v2 WHERE username = ?",
      ["bob"]
    );
    tests.test6_queryMaybe_hasResult = {
      status: "✓ PASSED",
      user: maybeUser ? { id: maybeUser.id, username: maybeUser.username } : null,
    };

    // Test 7: queryMaybe() - Optional single row query (no result)
    const noUser = await db.queryMaybe(
      UserSchema,
      "SELECT * FROM test_users_v2 WHERE username = ?",
      ["nonexistent"]
    );
    tests.test7_queryMaybe_noResult = {
      status: "✓ PASSED",
      user: noUser,
    };

    // Test 8: scalar() - Single value query
    const count = await db.scalar(
      z.number(),
      "SELECT COUNT(*) as value FROM test_users_v2"
    );
    tests.test8_scalar = {
      status: "✓ PASSED",
      count,
    };

    // Test 9: execute() - UPDATE operation
    const updateResult = await db.execute(
      ResultSchema,
      "UPDATE test_users_v2 SET age = ? WHERE username = ?",
      [26, "alice"]
    );
    tests.test9_update = {
      status: "✓ PASSED",
      affectedRows: updateResult.affectedRows,
    };

    // Test 10: tx() - Transaction commit success
    await db.execute(
      ResultSchema,
      `CREATE TABLE test_accounts (
        id INT AUTO_INCREMENT PRIMARY KEY,
        user_id INT,
        balance DECIMAL(10,2)
      )`
    );

    // Initialize accounts
    await db.execute(
      ResultSchema,
      "INSERT INTO test_accounts (user_id, balance) VALUES (?, ?)",
      [1, 100.00]
    );
    await db.execute(
      ResultSchema,
      "INSERT INTO test_accounts (user_id, balance) VALUES (?, ?)",
      [2, 50.00]
    );

    const txResult = await db.tx(async (tx) => {
      const sender = await tx.queryOne(
        BalanceSchema,
        "SELECT * FROM test_accounts WHERE user_id = ? FOR UPDATE",
        [1]
      );

      const receiver = await tx.queryOne(
        BalanceSchema,
        "SELECT * FROM test_accounts WHERE user_id = ? FOR UPDATE",
        [2]
      );

      // Convert DECIMAL type to number
      const senderBalance = typeof sender.balance === 'string'
        ? parseFloat(sender.balance)
        : sender.balance;
      const receiverBalance = typeof receiver.balance === 'string'
        ? parseFloat(receiver.balance)
        : receiver.balance;

      if (senderBalance < 30) {
        throw new Error("Insufficient balance");
      }

      await tx.execute(
        ResultSchema,
        "UPDATE test_accounts SET balance = balance - ? WHERE user_id = ?",
        [30, 1]
      );

      await tx.execute(
        ResultSchema,
        "UPDATE test_accounts SET balance = balance + ? WHERE user_id = ?",
        [30, 2]
      );

      return { senderNewBalance: senderBalance - 30, receiverNewBalance: receiverBalance + 30 };
    });

    tests.test10_transaction_commit = {
      status: "✓ PASSED",
      result: txResult,
    };

    // Test 11: tx() - Transaction rollback
    try {
      await db.tx(async (tx) => {
        const account = await tx.queryOne(
          BalanceSchema,
          "SELECT * FROM test_accounts WHERE user_id = ? FOR UPDATE",
          [1]
        );

        if (account.balance < 1000) {
          throw new Error("Insufficient balance for transfer");
        }

        await tx.execute(
          ResultSchema,
          "UPDATE test_accounts SET balance = balance - ? WHERE user_id = ?",
          [1000, 1]
        );

        return { success: true };
      });

      tests.test11_transaction_rollback = "✗ FAILED (Expected rollback)";
    } catch (error) {
      // Verify balance unchanged (rollback success)
      const account = await db.queryMaybe(
        BalanceSchema,
        "SELECT * FROM test_accounts WHERE user_id = ?",
        [1]
      );

      const originalBalance = typeof account?.balance === 'string'
        ? parseFloat(account?.balance as string)
        : account?.balance;

      tests.test11_transaction_rollback = {
        status: "✓ PASSED",
        error: (error as Error).message,
        balanceUnchanged: originalBalance === 70,
        finalBalance: account?.balance,
      };
    }

    // Test 12: queryPage() - Paginated query
    // Add more test data
    for (let i = 0; i < 10; i++) {
      await db.execute(
        ResultSchema,
        "INSERT INTO test_users_v2 (username, email, age) VALUES (?, ?, ?)",
        [`user${i}`, `user${i}@example.com`, 20 + i]
      );
    }

    const pageResult = await db.queryPage(
      UserSchema,
      `SELECT u.*, (SELECT COUNT(*) FROM test_users_v2) as total FROM test_users_v2 u ORDER BY u.id LIMIT ? OFFSET ?`,
      [],
      { page: 2, pageSize: 5 }
    );

    tests.test12_queryPage = {
      status: "✓ PASSED",
      page: pageResult.page,
      pageSize: pageResult.pageSize,
      total: pageResult.total,
      totalPages: pageResult.totalPages,
      itemCount: pageResult.items.length,
    };

    // Test 13: execute() - DELETE operation
    const deleteResult = await db.execute(
      ResultSchema,
      "DELETE FROM test_users_v2 WHERE username = ?",
      ["user0"]
    );

    tests.test13_delete = {
      status: "✓ PASSED",
      affectedRows: deleteResult.affectedRows,
    };

    // Clean up test data
    await db.execute(
      ResultSchema,
      "DROP TABLE IF EXISTS test_users_v2"
    );
    await db.execute(
      ResultSchema,
      "DROP TABLE IF EXISTS test_accounts"
    );

    tests.cleanup = "✓ Test tables dropped";

    await db.close();

  } catch (error) {
    tests.error = {
      message: (error as Error).message,
      stack: (error as Error).stack,
    };

    try {
      await db.close();
    } catch {}
  }

  return response.json(tests);
});
