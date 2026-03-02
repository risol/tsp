# MySQL Schema-first API

TSP provides a type-safe MySQL database access API using Zod schema for runtime data validation.

## Features

- ✅ **Type Safe** - All query methods use Zod schema to validate returned data
- ✅ **Runtime Validation** - Automatically validates database return data structure
- ✅ **Transaction Support** - Auto-commit/rollback transaction operations
- ✅ **Singleton Connection** - Uses singleton connection in TSX mode, no connection pool needed
- ✅ **Multiple Query Modes** - Supports multi-row, single row, optional single row, scalar, paginated queries

## Quick Start

### 1. Create Database Connection

```tsx
export default Page(async function(ctx, { createMySQL, z, response }) {
  // Create database connection
  const db = await createMySQL({
    host: "127.0.0.1",
    port: 3306,
    user: "test_user",
    password: "test123456",
    database: "test_db"
  }, z);

  // Use database...
  await db.close();
});
```

### 2. Define Schema

Define data structure using Zod:

```tsx
const UserSchema = z.object({
  id: z.number(),
  username: z.string(),
  email: z.string(),
  age: z.number().optional(),
  created_at: z.string().optional()
});

const ResultSchema = z.object({
  affectedRows: z.number(),
  insertId: z.number()
});
```

## API Methods

### query() - Multi-row Query

```tsx
const users = await db.query(
  UserSchema,
  'SELECT * FROM users WHERE age > ?',
  [18]
);
// users type: User[] (automatically inferred)
```

### queryOne() - Strict Single Row Query

Must return exactly one row, otherwise throws exception:

```tsx
const user = await db.queryOne(
  UserSchema,
  'SELECT * FROM users WHERE id = ?',
  [userId]
);
// 0 rows → Error: Expected 1 row, got 0
// >1 rows → Error: Expected 1 row, got N
```

### queryMaybe() - Optional Single Row Query

Returns 0 or 1 row:

```tsx
const user = await db.queryMaybe(
  UserSchema,
  'SELECT * FROM users WHERE email = ?',
  [email]
);
// user type: User | null
// 0 rows → null
// >1 rows → Error: Expected 0 or 1 row, got N
```

### scalar() - Single Value Query

SQL must use `AS value` alias:

```tsx
const count = await db.scalar(
  z.number(),
  'SELECT COUNT(*) as value FROM users'
);
// count type: number
```

### execute() - Write Operations

INSERT/UPDATE/DELETE operations:

```tsx
const result = await db.execute(
  ResultSchema,
  'INSERT INTO users (username, email) VALUES (?, ?)',
  [username, email]
);

console.log(result.insertId);      // Inserted ID
console.log(result.affectedRows); // Affected rows
```

### tx() - Transaction Operations

Auto-commit/rollback:

```tsx
try {
  const result = await db.tx(async (tx) => {
    const sender = await tx.queryOne(
      BalanceSchema,
      'SELECT * FROM accounts WHERE id = ? FOR UPDATE',
      [senderId]
    );

    if (sender.balance < amount) {
      throw new Error('Insufficient balance');
    }

    await tx.execute(
      ResultSchema,
      'UPDATE accounts SET balance = balance - ? WHERE id = ?',
      [amount, senderId]
    );

    await tx.execute(
      ResultSchema,
      'UPDATE accounts SET balance = balance + ? WHERE id = ?',
      [amount, receiverId]
    );

    return { success: true };
  });

  // Transaction committed successfully
  console.log(result);
} catch (error) {
  // Transaction auto-rollback
  console.error('Transaction failed:', error);
}
```

### queryPage() - Paginated Query

Returns standard paginated structure:

```tsx
const result = await db.queryPage(
  UserSchema,
  `SELECT *, COUNT(*) OVER() as total FROM users WHERE age > ? LIMIT ? OFFSET ?`,
  [18, 10, 20],
  { page: 3, pageSize: 10 }
);

// Return structure:
// {
//   items: User[],      // Current page data
//   total: 150,         // Total records
//   page: 3,            // Current page number
//   pageSize: 10,       // Page size
//   totalPages: 15      // Total pages
// }
```

**Note**: Must use `COUNT(*) OVER() as total` in SQL to get total record count.

## Data Type Handling

### DECIMAL Type

MySQL's DECIMAL type returns string, needs special handling:

```tsx
// Solution 1: Use union type
const BalanceSchema = z.object({
  id: z.number(),
  balance: z.union([z.number(), z.string()]),
});

// Solution 2: Convert to number
const balance = typeof row.balance === 'string'
  ? parseFloat(row.balance)
  : row.balance;
```

### DATE/DATETIME Type

```tsx
const UserSchema = z.object({
  created_at: z.string().optional(),  // Handle as string
  // or use z.any() to accept any type
});
```

## Complete Example

```tsx
export default Page(async function(ctx, { createMySQL, z, response }) {
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
    email: z.string()
  });

  // Query users
  const users = await db.query(
    UserSchema,
    'SELECT * FROM users ORDER BY id LIMIT 10'
  );

  // Single value query
  const count = await db.scalar(
    z.number(),
    'SELECT COUNT(*) as value FROM users'
  );

  await db.close();

  return response.json({
    users,
    totalUsers: count
  });
});
```

## Testing

Run tests:

```bash
# Unit tests
./tsp.sh test:unit

# E2E tests (requires Docker services)
./tsp.sh test:e2e
```

## Demo

After starting the server, access:
- http://localhost:9000/mysql-schema-first.tsx - Interactive demo

## Notes

1. **Connection Management**: Each request in TSX files creates a new connection, must call `db.close()` after use
2. **SQL Injection Protection**: Always use parameterized queries, don't concatenate SQL strings
3. **Transaction Isolation**: Use `FOR UPDATE` to lock queried rows, prevent concurrent modifications
4. **Type Validation**: Schema definition must match database table structure, otherwise validation will fail

## Related Links

- [Zod Documentation](https://zod.dev/) - Schema validation library
- [mysql2 Documentation](https://github.com/sidorares/node-mysql2) - MySQL client
- [Docker Services Configuration](./../../docker/DOCKER_SERVICES.md) - Test environment setup
