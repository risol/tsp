# MySQL 客户端集成文档

## 概述

TSP 现在支持类似 PHP 的 MySQL 数据库使用方式。每个 TSX 页面可以自己创建 MySQL 连接，通过依赖注入系统使用 `createMySQL` 工厂函数。

## 特性

- **按需创建**: 只有在需要时才创建连接
- **灵活配置**: 每个页面可以使用不同的数据库
- **类型安全**: 完整的 TypeScript 类型支持
- **连接池**: 自动管理连接池，提高性能
- **防注入**: 支持参数化查询，防止 SQL 注入
- **事务支持**: 支持数据库事务操作

## 安装

MySQL 客户端使用 `mysql2` npm 包，已在 `deno.json` 中配置：

```json
{
  "imports": {
    "mysql2": "npm:mysql2@^2.3.3"
  }
}
```

## 类型定义

所有 MySQL 相关类型都在 `types.d.ts` 中全局声明：

- `MySQLClient`: MySQL 客户端接口
- `MySQLConfig`: MySQL 连接配置接口
- `MySQLFactory`: MySQL 工厂函数类型
- `AppDeps.createMySQL`: 注入的工厂函数

## 基本用法

### 1. 简单查询

```tsx
export default Page(async function(ctx, { createMySQL, response }) {
  // 创建 MySQL 连接
  const db = await createMySQL({
    host: '127.0.0.1',
    port: 3306,
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  });

  // 执行查询
  const users = await db.query('SELECT * FROM users');

  return response.json({
    success: true,
    count: users.length,
    users: users
  });
});
```

### 2. 参数化查询（防止 SQL 注入）

```tsx
export default Page(async function(ctx, { createMySQL, response }) {
  const db = await createMySQL({
    host: '127.0.0.1',
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  });

  // 使用参数化查询
  const user = await db.query(
    'SELECT * FROM users WHERE id = ?',
    [ctx.query.id]
  );

  return response.json(user);
});
```

### 3. 插入数据

```tsx
export default Page(async function(ctx, { createMySQL, response }) {
  const db = await createMySQL({
    host: '127.0.0.1',
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  });

  // 插入数据并获取插入的 ID
  const insertId = await db.insert('users', {
    username: 'Alice',
    email: 'alice@example.com',
    password_hash: 'hashed_password'
  });

  return response.json({
    success: true,
    insertId: insertId
  });
});
```

### 4. 更新数据

```tsx
export default Page(async function(ctx, { createMySQL }) {
  const db = await createMySQL({
    host: '127.0.0.1',
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  });

  // 更新数据
  const affectedRows = await db.update(
    'users',
    { username: 'Bob' },
    { id: 1 }
  );

  return <div>更新了 {affectedRows} 行</div>;
});
```

### 5. 删除数据

```tsx
export default Page(async function(ctx, { createMySQL, response }) {
  const db = await createMySQL({
    host: '127.0.0.1',
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  });

  // 删除数据
  const affectedRows = await db.delete('users', { id: 1 });

  return response.json({
    success: true,
    deletedRows: affectedRows
  });
});
```

### 6. 事务操作

```tsx
export default Page(async function(ctx, { createMySQL, response, logger }) {
  const db = await createMySQL({
    host: '127.0.0.1',
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  });

  try {
    // 开启事务
    await db.beginTransaction();

    // 执行多个操作
    await db.insert('posts', {
      title: 'Post 1',
      content: 'Content 1',
      author_id: 1
    });

    await db.insert('posts', {
      title: 'Post 2',
      content: 'Content 2',
      author_id: 1
    });

    // 提交事务
    await db.commit();
    logger.info('事务提交成功');

    return response.json({ success: true });
  } catch (error) {
    // 发生错误时回滚
    await db.rollback();
    logger.error('事务失败，已回滚', error);

    return response.error('Transaction failed', 500);
  }
});
```

### 7. 使用环境变量

```tsx
export default Page(async function(ctx, { createMySQL, response }) {
  // 从环境变量读取配置
  const db = await createMySQL({
    host: Deno.env.get('MYSQL_HOST') || '127.0.0.1',
    port: Number(Deno.env.get('MYSQL_PORT')) || 3306,
    user: Deno.env.get('MYSQL_USER') || 'root',
    password: Deno.env.get('MYSQL_PASSWORD') || '',
    database: Deno.env.get('MYSQL_DATABASE') || 'test_db'
  });

  const users = await db.query('SELECT * FROM users');
  return response.json(users);
});
```

## API 参考

### MySQLConfig

数据库连接配置接口：

```typescript
interface MySQLConfig {
  host: string;          // MySQL 主机地址
  port?: number;         // MySQL 端口，默认 3306
  user: string;          // 用户名
  password: string;      // 密码
  database: string;      // 数据库名称
  charset?: string;      // 字符集，默认 'utf8mb4'
  pool?: {
    max?: number;        // 最大连接数，默认 10
    min?: number;        // 最小连接数
  };
}
```

### MySQLClient

MySQL 客户端接口，提供以下方法：

#### query<T>(sql: string, params?: unknown[]): Promise<T[]>

执行查询（支持参数化查询）

- **sql**: SQL 语句
- **params**: 查询参数（可选）
- **返回**: 查询结果数组

#### insert(table: string, data: Record<string, unknown>): Promise<number>

插入数据

- **table**: 表名
- **data**: 数据对象
- **返回**: 插入行的 ID

#### update(table: string, data: Record<string, unknown>, where: Record<string, unknown>): Promise<number>

更新数据

- **table**: 表名
- **data**: 更新的数据
- **where**: WHERE 条件
- **返回**: 影响的行数

#### delete(table: string, where: Record<string, unknown>): Promise<number>

删除数据

- **table**: 表名
- **where**: WHERE 条件
- **返回**: 影响的行数

#### beginTransaction(): Promise<void>

开启事务

#### commit(): Promise<void>

提交事务

#### rollback(): Promise<void>

回滚事务

#### close(): Promise<void>

关闭连接

## 测试

项目包含以下测试页面：

- `/test-mysql` - 基本查询
- `/test-mysql-insert` - 插入数据
- `/test-mysql-update` - 更新数据
- `/test-mysql-transaction` - 事务操作

### 启动测试环境

1. 启动 MySQL Docker 容器：
```powershell
.\docker-start.ps1
```

2. 启动开发服务器：
```bash
deno task dev
```

3. 访问测试页面：
```bash
curl http://localhost:9000/test-mysql
```

## 与 PHP 的对比

### PHP 方式

```php
<?php
// 每个页面自己创建连接
$mysqli = new mysqli("127.0.0.1", "user", "password", "database");

$result = $mysqli->query("SELECT * FROM users");
echo json_encode($result->fetch_all(MYSQLI_ASSOC));
?>
```

### TSP 方式

```tsx
export default Page(async function(ctx, { createMySQL, response }) {
  // 每个页面自己创建连接
  const db = await createMySQL({
    host: '127.0.0.1',
    user: 'test_user',
    password: 'test123456',
    database: 'test_db'
  });

  const users = await db.query('SELECT * FROM users');
  return response.json(users);
});
```

## 架构说明

### 实现文件

- `src/mysql/client.ts` - MySQLClientImpl 类实现
- `src/mysql/factory.ts` - createMySQL 工厂函数
- `types.d.ts` - 全局类型声明
- `src/main.ts` - 依赖注册

### 依赖注入

`createMySQL` 工厂函数通过依赖注入系统注册：

```typescript
// src/main.ts
import { createMySQL } from "./mysql/factory.ts";

registerDep("createMySQL", () => {
  return createMySQL;
});
```

### 连接池管理

MySQL 客户端使用连接池自动管理连接：

- 默认最大连接数：10
- 自动获取和释放连接
- 支持事务操作时会保持连接直到提交或回滚

## 安全注意事项

1. **永远使用参数化查询**：防止 SQL 注入
```tsx
// ✅ 正确
await db.query('SELECT * FROM users WHERE id = ?', [userId]);

// ❌ 错误 - 容易被 SQL 注入
await db.query(`SELECT * FROM users WHERE id = ${userId}`);
```

2. **密码管理**：使用环境变量存储密码
```tsx
const db = await createMySQL({
  password: Deno.env.get('MYSQL_PASSWORD') ?? ''
});
```

3. **错误处理**：始终使用 try-catch 处理数据库错误
```tsx
try {
  await db.insert('users', data);
} catch (error) {
  logger.error('数据库错误', error);
  return response.error('Database error', 500);
}
```

## 性能优化建议

1. **连接池配置**：根据应用负载调整连接池大小
```tsx
const db = await createMySQL({
  // ...其他配置
  pool: {
    max: 20,  // 根据并发需求调整
    min: 5    // 保持最小连接数
  }
});
```

2. **及时关闭连接**：使用完毕后关闭连接（可选，连接池会自动管理）
```tsx
const db = await createMySQL(config);
try {
  // 使用数据库
} finally {
  await db.close();
}
```

3. **批量操作**：使用事务进行批量插入/更新
```tsx
await db.beginTransaction();
for (const item of items) {
  await db.insert('table', item);
}
await db.commit();
```

## 故障排除

### 连接失败

如果遇到连接失败，检查：

1. MySQL 服务是否运行
```powershell
docker ps | findstr tsp-mysql
```

2. 连接配置是否正确
3. 防火墙是否阻止连接
4. 用户权限是否足够

### 类型错误

如果遇到类型错误，确保：

1. `types.d.ts` 中的类型声明正确
2. 使用 `globalThis.` 访问全局类型（在 src/ 文件中）
3. 重新运行 `deno task check`

## 许可证

MIT
