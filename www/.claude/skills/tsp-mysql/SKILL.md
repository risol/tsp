---
name: tsp-mysql
description: MySQL database operations for TSP. Use when working with MySQL queries, transactions, pagination (queryPage), or database operations.
---

# TSP MySQL

Use this skill for MySQL database operations in TSP.

## MySQL Usage

```typescript
export default Page(async function(ctx, { createMySQL, createZod, response }) {
  const z = await createZod();
  const db = await createMySQL({
    host: '127.0.0.1',
    port: 3306,
    user: 'root',
    password: 'password',
    database: 'mydb'
  }, z);

  // Define schema
  const UserSchema = z.object({
    id: z.number(),
    name: z.string(),
    email: z.string().email()
  });

  // Query all
  const users = await db.query(UserSchema, 'SELECT * FROM users');

  // Query one
  const user = await db.queryOne(UserSchema, 'SELECT * FROM users WHERE id = ?', [id]);

  // Query maybe (returns null if not found)
  const maybe = await db.queryMaybe(UserSchema, 'SELECT * FROM users WHERE email = ?', [email]);

  // Scalar query
  const count = await db.scalar(z.object({ count: z.number() }), 'SELECT COUNT(*) as count FROM users');

  // Insert
  const result = await db.execute(
    z.object({ affectedRows: z.number(), insertId: z.number() }),
    'INSERT INTO users (name, email) VALUES (?, ?)',
    ['John', 'john@example.com']
  );

  // Update
  await db.execute(
    z.object({ affectedRows: z.number() }),
    'UPDATE users SET name = ? WHERE id = ?',
    ['Jane', id]
  );

  // Delete
  await db.execute(
    z.object({ affectedRows: z.number() }),
    'DELETE FROM users WHERE id = ?',
    [id]
  );

  // Transaction
  await db.tx(async (tx) => {
    await tx.execute(z.object({}), 'INSERT INTO logs (action) VALUES (?)', ['create_user']);
    await tx.execute(z.object({}), 'UPDATE counters SET count = count + 1', []);
  });

  // IMPORTANT: Always use queryPage for pagination
  const pageResult = await db.queryPage(
    UserSchema,
    'SELECT id, name, email FROM users WHERE status = ? LIMIT ? OFFSET ?',
    ['active'],
    { page: 1, pageSize: 10 }
  );
  // Returns: { items: [...], total: 100, page: 1, pageSize: 10, totalPages: 10 }

  return response.json({ users, pageResult });
});
```

## Key Methods

| Method | Description |
|--------|-------------|
| `query()` | Query multiple rows |
| `queryOne()` | Query single row (throws if not found) |
| `queryMaybe()` | Query single row (returns null if not found) |
| `scalar()` | Query single scalar value |
| `execute()` | Insert/Update/Delete |
| `tx()` | Transaction |
| `queryPage()` | **ALWAYS use for pagination** |

## Best Practices

- Always define Zod schema for type safety
- Always close database connections (TSP handles this automatically)
- Use `queryPage` for paginated queries - never write LIMIT/OFFSET manually
- Use transactions for multi-step operations
