# TSP Getting Started

## Configuration Feature

TSP supports configuration files and auto-reload! Make configuration easier.

### Three Ways to Use

#### 1. Use Configuration File (Recommended)

Create `config.json`:
```json
{
  "root": "./www",
  "port": 9000,
  "dev": false
}
```

Run directly:
```bash
./tspserver
```

#### 2. Use JSONC (Supports Comments)

Create `config.jsonc`:
```json
{
  // Document root
  "root": "./www",

  // Listening port
  "port": 9000,

  // Development mode
  "dev": false
}
```

#### 3. Command Line Arguments

```bash
./tspserver --root ./www --port 9000 --dev
```

### Mixed Usage

Command line arguments override configuration file:

```bash
# Use config file but temporarily change port
./tspserver --port 8080
```

### Specify Configuration File

```bash
./tspserver --config ./my-config.json
```

### Priority

Command line arguments > Configuration file > Default values

## Configuration Auto-Reload ⭐

**New Feature**: Configuration changes take effect automatically after modifying the configuration file, no server restart needed!

Supported auto-reload configuration items:
- ✅ File Manager Password
- ✅ Logger Configuration
- ✅ Static File Extensions
- ❌ Port (requires restart)
- ❌ Root Directory (requires restart)

**Example**:
```bash
# 1. Modify password in configuration file
vim config.jsonc

# 2. Refresh browser, login with new password
# No server restart needed!
```

## Complete Example

For detailed documentation: [Configuration Documentation](./configuration.md)

## More Documentation

- [Command Line Arguments](./tasks.md) - Complete command line argument documentation
- [Development Guide](./development.md) - Development environment setup
- [Features](./features/README.md) - Learn TSP features

## Common Commands

```bash
# Development mode (with hot reload)
./tsp.sh dev

# Production mode
./tsp.sh start

# Compile binary
./tsp.sh build:tspserver

# Run tests
./tsp.sh test
```

## Database Integration

TSP provides a type-safe MySQL Schema-first API using Zod for runtime data validation.

### Quick Example

```tsx
// www/users.tsx
export default Page(async function(ctx, { createMySQL, z, response }) {
  const db = await createMySQL({
    host: "127.0.0.1",
    port: 3306,
    user: "test_user",
    password: "test123456",
    database: "test_db"
  }, z);

  // Define data schema
  const UserSchema = z.object({
    id: z.number(),
    username: z.string(),
    email: z.string()
  });

  // Query users (auto-validate type)
  const users = await db.query(
    UserSchema,
    'SELECT * FROM users WHERE age > ?',
    [18]
  );

  await db.close();

  return response.json({ users });
});
```

### Core Methods

- `query(schema, sql, params)` - Multi-row query
- `queryOne(schema, sql, params)` - Strict single row (0 or multiple rows throws error)
- `queryMaybe(schema, sql, params)` - Optional single row (returns 0 or 1 row)
- `scalar(schema, sql, params)` - Single value query
- `execute(schema, sql, params)` - Write operations (INSERT/UPDATE/DELETE)
- `tx(callback)` - Transaction operations (auto-commit/rollback)
- `queryPage(schema, sql, params, pageArgs)` - Paginated query

### Start Test Database

```bash
# Use Docker to quickly start MySQL test environment
sh ./docker/start.sh
```

### More Information

- [MySQL Complete Documentation](./features/mysql.md)
- [Interactive Demo](http://localhost:9000/mysql-schema-first.tsx)
- [Docker Services Configuration](./../docker/DOCKER_SERVICES.md)
