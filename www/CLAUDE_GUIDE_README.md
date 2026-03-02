# TSP Server Development Guide

> TSP (TypeScript Server Page) - Develop like PHP, write server pages with TypeScript/TSX

---

## Table of Contents

1. [Introduction](#introduction)
2. [Quick Start](#quick-start)
3. [Writing TSP Pages](#writing-tsp-pages)
4. [Dependency Injection System](#dependency-injection-system)
5. [Injectable Dependencies Details](#injectable-dependencies-details)
6. [Common Development Patterns](#common-development-patterns)
7. [Best Practices](#best-practices)

---

## Introduction

TSP is a TypeScript server page framework that lets you develop Web applications like writing PHP:

- **Direct Execution**: `.tsp` files run directly without build steps
- **Type-safe**: Complete TypeScript type support
- **Dependency Injection**: Type-safe dependency injection system
- **Smart Caching**: Automatic caching based on file modification time
- **Hot Reload**: Automatically detect file changes and reload in development mode

### Core Concepts

```
www/
├── index.tsp          # Home page (access /)
├── about.tsp          # About page (access /about)
├── api/
│   └── users.tsp      # API endpoint (access /api/users)
└── components/
    └── Layout.tsx     # Reusable component (TSX, cannot be accessed directly)
```

Each `.tsp` file corresponds to a URL path, and the HTML generated after executing the file is returned to the client.

**Note**: TSP v4.0+ uses `.tsp` as the route file suffix. `.tsx` and `.ts` files cannot be accessed directly via HTTP, but can be imported as modules by `.tsp` files.

---

## Migration from Old Versions

If you are migrating from v3.x to v4.0+, you need to rename route files from `.tsx` to `.tsp`:

```bash
# Rename all .tsx route files to .tsp
mv page.tsx page.tsp
mv index.tsp index.tsp
mv api.tsx api.tsp
```

Component files (`.tsx`) and utility modules (`.ts`) do not need to be renamed, they can still be imported by `.tsp` files.

---

## Quick Start

### 1. Create Your First Page

Create `www/index.tsp`:

```tsx
export default Page(async function(ctx, { logger }) {
  logger.info('Home page visited');

  return (
    <html>
      <head>
        <title>My TSP Website</title>
      </head>
      <body>
        <h1>Welcome to TSP!</h1>
        <p>Current time: {new Date().toLocaleString()}</p>
      </body>
    </html>
  );
});
```

### 2. Access the Page

Start the server:

```bash
# Development mode (with hot reload support)
./tsp.sh dev

# Or production mode
./tsp.sh start
```

Visit `http://localhost:9000/` to see the page.

### 3. Using Components

Create `www/components/Layout.tsx` (TSX component, cannot be accessed directly, but can be imported):

```tsx
export function Layout({ title, children }: {
  title: string;
  children: unknown;
}) {
  return (
    <html>
      <head>
        <title>{title}</title>
        <meta charset="UTF-8" />
      </head>
      <body>
        <nav>
          <a href="/">Home</a>
          <a href="/about">About</a>
        </nav>
        <main>{children}</main>
      </body>
    </html>
  );
}
```

Use in `.tsp` pages:

```tsx
import { Layout } from "./components/Layout.tsx";

export default Page(async function(ctx) {
  return (
    <Layout title="Home">
      <h1>Welcome!</h1>
    </Layout>
  );
});
```

---

## Writing TSP Pages

### Basic Structure

Each TSP page must export a default function wrapped with `Page`:

```tsx
export default Page(async function(ctx, { logger, response }) {
  // ctx: PageContext - request context
  // logger, response: injected dependencies

  return <div>Page content</div>;
});
```

**Note**: TSP v4.0+ uses `.tsp` as the route file suffix. `.tsp` files can import `.ts` and `.tsx` files, but cannot import other `.tsp` files.

### PageContext - Request Context

The `ctx` parameter contains all request information:

```tsx
export default Page(async function(ctx) {
  // HTTP method
  console.log(ctx.method); // GET, POST, etc.

  // URL information
  console.log(ctx.url.href); // Full URL
  console.log(ctx.url.pathname); // Path /about

  // Query parameters
  console.log(ctx.query); // { id: '123', page: '1' }

  // Request headers
  console.log(ctx.headers.get('User-Agent'));

  // Cookies
  console.log(ctx.cookies); // { session: 'abc123' }

  // Request body (JSON)
  console.log(ctx.body);

  // Uploaded files
  console.log(ctx.files); // { avatar: UploadedFile }

  // File paths
  console.log(ctx.file); // /path/to/www/index.tsp
  console.log(ctx.root); // /path/to/www
});
```

### Return Different Types

```tsx
// 1. Return JSX (rendered as HTML)
export default Page(async function(ctx) {
  return <div>Hello</div>;
});

// 2. Return string (as plain text)
export default Page(async function(ctx) {
  return 'Plain text response';
});

// 3. Return RedirectResult (redirect)
export default Page(async function(ctx, { response }) {
  return response.redirect('/login');
});

// 4. Return Response (fully custom)
export default Page(async function(ctx, { response }) {
  return response.json({ success: true, data: [1, 2, 3] });
});
```

### Conditional Rendering

```tsx
export default Page(async function(ctx, { session }) {
  await session.init();
  const userName = await session.get('userName');

  if (!userName) {
    return <a href="/login">Please login</a>;
  }

  return (
    <div>
      <h1>Welcome, {userName}!</h1>
    </div>
  );
});
```

### List Rendering

```tsx
export default Page(async function(ctx, { createMySQL }) {
  const db = await createMySQL({
    host: '127.0.0.1',
    user: 'root',
    password: 'password',
    database: 'mydb'
  });

  const users = await db.query('SELECT * FROM users');

  return (
    <ul>
      {users.map((user: any) => (
        <li key={user.id}>
          {user.name} ({user.email})
        </li>
      ))}
    </ul>
  );
});
```

### Form Handling

```tsx
export default Page(async function(ctx, { response }) {
  if (ctx.method === 'POST') {
    const form = ctx.body as { name: string; email: string };

    // Handle form submission
    // ...

    return response.json({ success: true });
  }

  // GET request shows form
  return (
    <form method="POST">
      <input type="text" name="name" placeholder="Name" />
      <input type="email" name="email" placeholder="Email" />
      <button type="submit">Submit</button>
    </form>
  );
});
```

### File Upload

```tsx
export default Page(async function(ctx, { response }) {
  if (ctx.method === 'POST') {
    const file = ctx.files.avatar as UploadedFile;

    if (file) {
      // Save file
      await file.save(`./uploads/${file.name}`);

      return response.json({ success: true, filename: file.name });
    }
  }

  return (
    <form method="POST" enctype="multipart/form-data">
      <input type="file" name="avatar" />
      <button type="submit">Upload</button>
    </form>
  );
});
```

### Inline Script Tags

TSP uses React as the rendering engine, fully supporting `<script>` tags in TSX pages.

```tsx
export default Page(async function() {
  return (
    <html>
      <head><title>Script Example</title></head>
      <body>
        <div id="app">Loading...</div>

        {/* Recommended way - use dangerouslySetInnerHTML to avoid HTML escaping */}
        <script dangerouslySetInnerHTML={{
          __html: `document.getElementById('app').textContent = 'Hello World!';`
        }} />
      </body>
    </html>
  );
});
```

**Note**:
- TSP (React) will HTML entity encode the content of `<script>` tags
- Using `dangerouslySetInnerHTML` can avoid quotes being escaped to `&quot;`
- External JS file inclusion is the same as regular HTML

### Include External JS Files

Including external JavaScript files in TSX pages is exactly the same as regular HTML:

```tsx
export default Page(async function() {
  return (
    <html>
      <head>
        <title>External JS Example</title>
        {/* Include local JS file */}
        <script src="/static/js/app.js"></script>
        {/* Include CDN JS file */}
        <script src="https://cdn.example.com/library.js"></script>
      </head>
      <body>
        <h1>Page Title</h1>
        <div id="app"></div>
      </body>
    </html>
  );
});
```

**Static file directory**:
- Files placed in `www/static/js/` can be accessed via `/static/js/` path
- Example: `www/static/js/app.js` -> access path: `/static/js/app.js`

```tsx
// Complete example: using external library
export default Page(async function() {
  return (
    <html>
      <head>
        <title>jQuery Example</title>
        {/* Include jQuery */}
        <script src="https://code.jquery.com/jquery-3.7.1.min.js"></script>
      </head>
      <body>
        <button id="btn">Click me</button>
        <div id="output"></div>

        <script dangerouslySetInnerHTML={{
          __html: `
            $(document).ready(function() {
              $('#btn').click(function() {
                $('#output').text('Button clicked! Time: ' + new Date());
              });
            });
          `
        }} />
      </body>
    </html>
  );
});
```

---

## Dependency Injection System

TSP uses a type-safe dependency injection system, allowing you to use databases, caching, logging, and other services in your pages.

### Basic Usage

```tsx
export default Page(async function(ctx, { logger, session, response }) {
  // logger - logger
  logger.info('Page loaded');

  // session - session management
  const user = await session.getUser();

  // response - response helper
  return response.json({ user });
});
```

### Lazy Loading Mechanism

Dependencies are only built when accessed (lazy loading), unused dependencies are not initialized:

```tsx
export default Page(async function(ctx, { logger }) {
  // Only logger is initialized
  logger('Hello');

  // createMySQL is not initialized (not used)
  return <div>Done</div>;
});
```

### Available Dependencies

| Dependency | Type | Description |
|------------|------|-------------|
| `logger` | `Logger` | Logger |
| `session` | `SessionManager` | Session management |
| `cookies` | `CookieManager` | Cookie management |
| `response` | `ResponseHelper` | Response helper |
| `createMySQL` | `MySQLFactory` | MySQL factory function |
| `createRedis` | `RedisFactory` | Redis factory function |
| `createLdap` | `LdapFactory` | LDAP factory function |
| `createExcelJS` | `ExcelJSFactory` | Excel file operation factory function |
| `nanoid` | `(size?) => string` | Unique ID generator |
| `tspinfo` | `TspInfo` | Server info viewer |
| `testHelper` | `TestHelper` | Test helper |
| `testFunc` | `() => string` | Test function |

---

## Injectable Dependencies Details

### 1. logger - Logger

Structured logging with multiple levels and color output.

```tsx
export default Page(async function(ctx, { logger }) {
  // Different log levels
  logger.debug('Debug info');
  logger.info('Normal info');
  logger.warn('Warning info');
  logger.error('Error info');

  // Support multiple parameters
  logger.info('User logged in:', { id: 123, name: 'Alice' });

  return <div>Check console</div>;
});
```

### 2. session - Session Management

Secure general-purpose key-value storage based on Cookie storage.

```tsx
export default Page(async function(ctx, { session, response }) {
  // Initialize session (if not exists)
  await session.init();

  // Store user data
  await session.set('userId', '123');
  await session.set('userName', 'Alice');
  await session.set('userRole', 'admin');

  // Read user data
  const userId = await session.get('userId');
  const userName = await session.get('userName');
  const userRole = await session.get('userRole');

  if (!userId) {
    return response.redirect('/login');
  }

  // Get all session data
  const allData = await session.all();
  // allData = { userId: '123', userName: 'Alice', userRole: 'admin' }

  // Delete specific data
  await session.delete('userRole');

  // Clear all session data
  await session.clear();

  // Refresh session expiration time
  await session.touch();

  // Regenerate session ID (prevent session fixation attacks)
  await session.regenerateId();

  // Check if session is valid
  const isValid = await session.isValid();

  // Get session ID
  const sessionId = session.getId();

  // Destroy session (logout)
  await session.destroy();

  return <div>Hello, {userName}</div>;
});
```

### 3. cookies - Cookie Management

Set and delete HTTP Cookies.

```tsx
export default Page(async function(ctx, { cookies }) {
  // Set single cookie
  cookies.set('theme', 'dark', {
    maxAge: 3600,      // 1 hour
    httpOnly: true,    // Block JavaScript access
    secure: true,      // HTTPS only
    sameSite: 'Strict' // Prevent CSRF
  });

  // Delete cookie
  cookies.delete('theme');

  // Batch set
  cookies.setMultiple({
    theme: { value: 'dark', options: { maxAge: 3600 } },
    language: { value: 'en-US', options: { maxAge: 3600 } }
  });

  // Batch delete
  cookies.deleteMultiple(['theme', 'language']);

  return <div>Cookies set</div>;
});
```

### 4. response - Response Helper

Convenient HTTP response creation methods.

```tsx
export default Page(async function(ctx, { response }) {
  // JSON response
  return response.json({ success: true, data: [1, 2, 3] }, 200);

  // Text response
  return response.text('Hello World', 200);

  // HTML response
  return response.html('<h1>Hello</h1>', 200);

  // Redirect
  return response.redirect('/login', 302);

  // Error response
  return response.error('Not Found', 404);

  // File download
  return response.file(
    new TextEncoder().encode('file content'),
    'document.txt',
    { 'Content-Type': 'text/plain' }
  );

  // 204 No Content
  return response.noContent();

  // Custom Response
  return response.custom(
    JSON.stringify({ custom: true }),
    { status: 200, headers: { 'X-Custom': 'value' } }
  );
});
```

### 5. createMySQL - MySQL Database (Schema-first API)

Create type-safe MySQL database connection with Zod schema for runtime validation.

```tsx
export default Page(async function(ctx, { createMySQL, z, response }) {
  // Create database connection (need to pass z parameter)
  const db = await createMySQL({
    host: '127.0.0.1',
    port: 3306,
    user: 'root',
    password: 'password',
    database: 'mydb',
    charset: 'utf8mb4'
  }, z);

  // Define Schema
  const UserSchema = z.object({
    id: z.number(),
    name: z.string(),
    email: z.string(),
    created_at: z.string().optional()
  });

  const ResultSchema = z.object({
    affectedRows: z.number(),
    insertId: z.number()
  });

  // 1. Multi-row query
  const users = await db.query(
    UserSchema,
    'SELECT * FROM users WHERE role = ?',
    ['admin']
  );
  // Return type: User[] (auto-inferred)

  // 2. Single row query (must return exactly one row)
  const user = await db.queryOne(
    UserSchema,
    'SELECT * FROM users WHERE id = ?',
    [userId]
  );
  // 0 rows or >1 row throws exception

  // 3. Optional single row query (returns 0 or 1 row)
  const maybeUser = await db.queryMaybe(
    UserSchema,
    'SELECT * FROM users WHERE email = ?',
    [email]
  );
  // Return type: User | null

  // 4. Scalar query (SQL must use AS value alias)
  const count = await db.scalar(
    z.number(),
    'SELECT COUNT(*) as value FROM users'
  );

  // 5. Write operations (INSERT/UPDATE/DELETE)
  const result = await db.execute(
    ResultSchema,
    'INSERT INTO users (name, email) VALUES (?, ?)',
    ['Alice', 'alice@example.com']
  );
  console.log(result.insertId);
  console.log(result.affectedRows);

  // 6. Transaction operations (auto-commit/rollback)
  await db.tx(async (tx) => {
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
  // Auto-commit on success, auto-rollback on failure

  // 7. Paginated query
  const pageResult = await db.queryPage(
    UserSchema,
    `SELECT *, COUNT(*) OVER() as total FROM users LIMIT ? OFFSET ?`,
    [],
    { page: 2, pageSize: 20 }
  );
  // Returns: { items: User[], total: number, page: number, pageSize: number, totalPages: number }

  // Close connection
  await db.close();

  return response.json({ users });
});
```

**Key Features:**
- Type-safe: All methods use Zod schema to validate returned data
- Runtime validation: Automatically validate database return data structure
- Transaction support: Auto-commit/rollback transaction operations
- Singleton connection: Uses singleton connection in TSX mode, no connection pool needed

**Note:**
- Old API (`insert`, `update`, `delete`, `beginTransaction`, `commit`, `rollback`) has been removed
- All queries must use Schema-first API (first parameter is Zod schema)
- See: `/mysql-schema-first.tsx` interactive demo

### 6. createRedis - Redis Cache

Create Redis connection.

```tsx
export default Page(async function(ctx, { createRedis, response }) {
  // Create Redis connection
  const redis = await createRedis({
    host: '127.0.0.1',
    port: 6379,
    password: 'password',
    database: 0
  });

  // String operations
  await redis.set('key', 'value', 3600); // 1 hour expiration
  const value = await redis.get('key');
  await redis.del('key');

  // Check if key exists
  const exists = await redis.exists('key');

  // Set expiration
  await redis.expire('key', 3600);
  const ttl = await redis.ttl('key');

  // List operations
  await redis.lpush('list', 'item1', 'item2');
  const item = await redis.lpop('list');
  const items = await redis.lrange('list', 0, -1);

  // Set operations
  await redis.sadd('set', 'member1', 'member2');
  const members = await redis.smembers('set');
  const isMember = await redis.sismember('set', 'member1');

  // Hash operations
  await redis.hset('hash', 'field1', 'value1');
  const fieldValue = await redis.hget('hash', 'field1');
  const hashAll = await redis.hgetall('hash');

  // Sorted set
  await redis.zadd('zset', 100, 'member1');
  const ranked = await redis.zrange('zset', 0, -1);

  // Counter
  await redis.incr('counter');
  await redis.incrBy('counter', 10);

  // Close connection
  await redis.close();

  return response.json({ value });
});
```

### 7. createLdap - LDAP Directory Service

Create LDAP connection.

```tsx
export default Page(async function(ctx, { createLdap, response }) {
  // Create LDAP connection
  const ldap = await createLdap({
    url: 'ldap://127.0.0.1:389',
    bindDN: 'cn=admin,dc=example,dc=org',
    bindCredentials: 'password',
    baseDN: 'dc=example,dc=org',
    startTLS: true,
    timeout: 5000
  });

  // User authentication
  try {
    await ldap.bind(`cn=user,dc=example,dc=org`, 'userpassword');
    console.log('Authentication successful');
  } catch (error) {
    console.log('Authentication failed', error);
  }

  // Search
  const entries = await ldap.search('dc=example,dc=org', {
    scope: 'sub',        // sub (subtree), one (one level), base (base only)
    filter: '(objectClass=person)',
    attributes: ['cn', 'mail', 'uid'],
    sizeLimit: 100,
    timeout: 10
  });

  entries.forEach((entry) => {
    console.log(entry.dn);
    console.log(entry.attributes);
  });

  // Add entry
  await ldap.add('cn=newuser,dc=example,dc=org', {
    objectClass: ['person', 'organizationalPerson'],
    cn: 'newuser',
    sn: 'User',
    mail: 'newuser@example.com'
  });

  // Modify entry
  await ldap.modify('cn=user,dc=example,dc=org', [
    {
      operation: 'replace',
      modification: { mail: 'newemail@example.com' }
    }
  ]);

  // Delete entry
  await ldap.del('cn=user,dc=example,dc=org');

  // Close connection
  await ldap.close();

  return response.json({ entries });
});
```

### 8. createExcelJS - Excel File Operations

Returns ExcelJS library instance, can directly use its full API.

```tsx
export default Page(async function(ctx, { createExcelJS, response, z }) {
  // Get ExcelJS library
  const ExcelJS = await createExcelJS();

  // 1. Write data to Excel
  const workbook = new ExcelJS.Workbook();
  const worksheet = workbook.addWorksheet('Users');

  // Define columns
  worksheet.columns = [
    { header: 'Name', key: 'name', width: 20 },
    { header: 'Age', key: 'age', width: 10 },
    { header: 'Email', key: 'email', width: 30 },
  ];

  // Add data
  worksheet.addRow({ name: 'Alice', age: 25, email: 'alice@example.com' });
  worksheet.addRow({ name: 'Bob', age: 30, email: 'bob@example.com' });

  // Save file
  await workbook.xlsx.writeFile('./users.xlsx');

  // 2. Read Excel file
  const readWorkbook = new ExcelJS.Workbook();
  await readWorkbook.xlsx.readFile('./users.xlsx');
  const readWorksheet = readWorkbook.getWorksheet('Users');

  const users: any[] = [];
  readWorksheet?.eachRow((row, rowNumber) => {
    if (rowNumber > 1) { // Skip header
      users.push({
        name: row.getCell(1).value,
        age: row.getCell(2).value,
        email: row.getCell(3).value,
      });
    }
  });

  // 3. Use Zod schema validation
  const UserSchema = z.object({
    name: z.string(),
    age: z.number(),
    email: z.string().email()
  });

  const validated = UserSchema.parse(users[0]);

  // 4. Get worksheet info
  const sheets = readWorkbook.worksheets.map((ws) => ({
    name: ws.name,
    rowCount: ws.rowCount,
  }));

  // 5. Export as Buffer
  const buffer = await workbook.xlsx.writeBuffer();

  // 6. Cell operations
  const ws = workbook.addWorksheet('Test');
  ws.getCell('A1').value = 'Hello ExcelJS!';
  ws.getCell('B1').value = 123;
  ws.getCell('A1').font = { bold: true, size: 16 };

  await workbook.xlsx.writeFile('./test.xlsx');

  // 7. Worksheet operations
  const wb2 = new ExcelJS.Workbook();
  wb2.addWorksheet('Sheet1');
  wb2.addWorksheet('Sheet2');
  await wb2.xlsx.writeFile('./multi-sheet.xlsx');

  // Rename
  const sheet = wb2.getWorksheet('Sheet2');
  if (sheet) sheet.name = 'NewSheet';

  // Delete
  const sheetToDelete = wb2.getWorksheet('NewSheet');
  if (sheetToDelete) wb2.removeWorksheet(sheetToDelete.id);

  await wb2.xlsx.writeFile('./multi-sheet.xlsx');

  // 8. Template filling
  const templateWorkbook = new ExcelJS.Workbook();
  const templateWs = templateWorkbook.addWorksheet('Template');
  templateWs.addRow(['Name: {{name}}']);
  templateWs.addRow(['Age: {{age}}']);
  await templateWorkbook.xlsx.writeFile('./template.xlsx');

  // Fill template
  const filledWorkbook = new ExcelJS.Workbook();
  await filledWorkbook.xlsx.readFile('./template.xlsx');
  filledWorkbook.eachSheet((ws) => {
    ws.eachRow((row) => {
      row.eachCell((cell) => {
        if (typeof cell.value === 'string') {
          let value = cell.value;
          value = value.replace(/\{\{name\}\}/g, 'David');
          value = value.replace(/\{\{age\}\}/g, '28');
          cell.value = value;
        }
      });
    });
  });
  await filledWorkbook.xlsx.writeFile('./filled.xlsx');

  return response.json({ success: true, bufferLength: buffer.length });
});
```

**Key Features:**

Returns ExcelJS library instance, can use its full API. For detailed usage, see [ExcelJS Official Documentation](https://github.com/exceljs/exceljs).

**Notes:**
- File paths are relative to server root
- Supports .xlsx and .csv formats
- For full API, see ExcelJS official documentation

### 9. nanoid - Unique ID Generator

Generates URL-safe unique identifiers.

```tsx
export default Page(async function(ctx, { nanoid, createMySQL }) {
  // Generate default length (21 characters)
  const id = nanoid(); // "V1StGXR8_Z5jdHi6B-myT"

  // Generate specified length
  const shortId = nanoid(10); // "V1StGXR8_Z5"

  // Usage: Create unique ID
  const db = await createMySQL({
    host: '127.0.0.1',
    user: 'root',
    password: 'password',
    database: 'mydb'
  });

  const userId = nanoid();
  await db.insert('users', {
    id: userId,
    name: 'Alice',
    created_at: new Date()
  });

  return <div>User ID: {userId}</div>;
});
```

### 10. tspinfo - Server Info Viewer

Similar to PHP's `phpinfo()`, displays server runtime information.

```tsx
export default Page(async function(ctx, { tspinfo }) {
  // Returns HTML format server info page
  return tspinfo.renderHTML();
});
```

Accessing this page displays:
- Server version and architecture
- Configuration information
- Logging configuration
- Runtime information (memory, CPU, uptime)
- System information
- Environment variables
- Cache statistics
- Registered dependencies

### 11. testHelper - Test Helper

Built-in testing framework providing Jest/PHPUnit-like testing functionality without additional tool installation.

#### Basic Testing Features

```tsx
export default Page(async function(ctx, { testHelper, response }) {
  // Clear previous test results
  testHelper.clear();

  // Write test cases
  await testHelper.test('Addition', () => {
    testHelper.assertEqual(1 + 1, 2);
  });

  await testHelper.test('API test', async () => {
    const resp = await fetch('/api/users');
    const data = await resp.json();
    testHelper.assertTrue(Array.isArray(data));
  });

  // Get test results
  const results = testHelper.getResults();
  // { total: 2, passed: 2, failed: 0, duration: 15, results: [...] }

  return response.json(results);
});
```

**Basic assertion methods:**
- `assertEqual(actual, expected, message?)` - Equal assertion
- `assertNotEqual(actual, expected, message?)` - Not equal assertion
- `assertTrue(value, message?)` - Boolean true assertion
- `assertFalse(value, message?)` - Boolean false assertion
- `assertNull(value, message?)` - Null assertion
- `assertNotNull(value, message?)` - Not null assertion
- `assertContains(haystack, needle, message?)` - Contains assertion
- `assertThrows(fn, expectedError?, message?)` - Throws assertion
- `fail(message?)` - Directly fail test

##### Special Assertion: fail - Actively Mark Failure

The `fail()` method is used to actively mark test failures in test code, typically for:

1. **Mark code paths that should not execute**
2. **Explicit failure scenarios in conditional tests**
3. **Terminate test early and mark failure**

```tsx
// Example 1: Mark code path that should not execute
await testHelper.test('Conditional branch test', () => {
  const isAdmin = false;

  if (isAdmin) {
    testHelper.fail('Regular user should not enter admin branch');
  }

  testHelper.assertTrue(true);
});

// Example 2: Verify exception handling
await testHelper.test('Error handling test', async () => {
  try {
    await someOperation();
    testHelper.fail('Should throw exception but did not');
  } catch (error) {
    testHelper.assertEqual(error.message, 'Expected error');
  }
});

// Example 3: Early test termination
await testHelper.test('Config validation', () => {
  const config = loadConfig();

  if (!config.required) {
    testHelper.fail('Missing required config item');
  }

  // Continue testing...
});
```

#### Advanced Testing Features

##### 1. mockContext - Mock PageContext

Create mock HTTP request context for unit testing.

```tsx
// Create mock context
const mockCtx = testHelper.mockContext({
  method: 'POST',
  url: 'http://localhost:9000/api/users',
  query: { page: '1', limit: '10' },
  body: { name: 'Alice', age: 25 },
  headers: { 'X-Custom-Header': 'test-value' },
  cookies: { sessionId: 'abc123' }
});

// Use mock context
testHelper.assertEqual(mockCtx.method, 'POST');
testHelper.assertEqual(mockCtx.query.page, '1');
```

##### 2. runPage - Direct Page Function Call

Test page function directly without HTTP layer (unit testing).

```tsx
import userPage from './user.tsx';

export default Page(async function(ctx, { testHelper, response }) {
  testHelper.clear();

  await testHelper.test('User page unit test', async () => {
    // Create mock context
    const mockCtx = testHelper.mockContext({
      method: 'GET',
      url: 'http://localhost:9000/user/123',
      query: { id: '123' }
    });

    // Call page function directly
    const result = await testHelper.runPage(userPage, mockCtx);

    // Verify return value
    testHelper.assertNotNull(result);
  });

  const results = testHelper.getResults();
  return response.json(results);
});
```

##### 3. mockFetch - Mock HTTP Requests

Intercept and mock fetch requests to avoid real network calls.

```tsx
export default Page(async function(ctx, { testHelper, response }) {
  testHelper.clear();

  await testHelper.test('Mock API request', async () => {
    // Mock specific URL
    testHelper.mockFetch('/api/users', {
      response: [{ id: 1, name: 'Alice' }],
      status: 200
    });

    // All requests return mock data
    const resp = await fetch('/api/users');
    const users = await resp.json();

    testHelper.assertEqual(users.length, 1);
    testHelper.assertEqual(users[0].name, 'Alice');

    // Clear mock
    testHelper.clearMocks();
  });

  const results = testHelper.getResults();
  return response.json(results);
});
```

**Using regex matching:**
```tsx
// Mock matches /api/users/123, /api/users/456, etc.
testHelper.mockFetch(/\/api\/users\/\d+/, {
  response: { id: 123, name: 'Alice' },
  status: 200
});
```

**Simulate delayed response:**
```tsx
testHelper.mockFetch('/api/slow', {
  response: { data: 'slow response' },
  delay: 100  // 100ms delay
});
```

**Simulate error response:**
```tsx
testHelper.mockFetch('/api/error', {
  response: { error: 'Not found' },
  status: 404,
  headers: { 'X-Error-Code': 'NOT_FOUND' }
});
```

##### 4. assertSnapshot - Snapshot Testing

Compare values against snapshot files for UI regression testing.

```tsx
await testHelper.test('User list snapshot', async () => {
  const users = [
    { id: 1, name: 'Alice', email: 'alice@example.com' },
    { id: 2, name: 'Bob', email: 'bob@example.com' }
  ];

  // First run: creates snapshot file .tests/snapshots/user-list.snap.json
  // Subsequent runs: compare with snapshot, fail if mismatch
  await testHelper.assertSnapshot('user-list', users);
});

// Force update snapshot
await testHelper.assertSnapshot('user-list', users, true);
```

##### Complete Example

```tsx
export default Page(async function(ctx, { testHelper, response }) {
  testHelper.clear();

  // 1. Mock API
  await testHelper.test('Complete user registration flow', async () => {
    // Mock all API calls
    testHelper.mockFetch('/api/users', {
      response: { id: 1, name: 'New User' },
      status: 201
    });

    testHelper.mockFetch('/api/email/verify', {
      response: { success: true },
      status: 200
    });

    // 2. Create mock context
    const mockCtx = testHelper.mockContext({
      method: 'POST',
      url: 'http://localhost:9000/register',
      body: { name: 'New User', email: 'new@example.com' }
    });

    // 3. Call registration page
    const registerPage = await import('./register.tsx');
    const result = await testHelper.runPage(registerPage.default, mockCtx);

    // 4. Verify result
    testHelper.assertNotNull(result);

    // 5. Snapshot test
    await testHelper.assertSnapshot('register-success', result);

    // 6. Cleanup
    testHelper.clearMocks();
  });

  const results = testHelper.getResults();
  return response.json(results);
});
```

**Quick Start:**
```bash
# 1. Access test center
http://localhost:9000/__tests__/

# 2. Run example tests
http://localhost:9000/__tests__/simple.tsp
http://localhost:9000/__tests__/advanced-test.tsp  # Advanced features example

# 3. Copy app.tsp as test template
cp www/__tests__/app.tsp www/__tests__/my_tests.tsp

# 4. After modifying test cases, access
http://localhost:9000/__tests__/my_tests.tsp
```

**Detailed Documentation:**
- **Complete Guide**: `TESTHELPER.md` - All features detailed description
- **Basic Testing**: `TESTING.md` - Basic testing guide
- **Advanced Examples**: `www/__tests__/advanced-test.tsx` - Runnable examples

### 12. testFunc - Test Function

Simple test function for verifying dependency injection system.

```tsx
export default Page(async function(ctx, { testFunc }) {
  const result = testFunc(); // "testFunc called"
  return <div>{result}</div>;
});
```

---

## Common Development Patterns

### RESTful API

```tsx
export default Page(async function(ctx, { createMySQL, response }) {
  const db = await createMySQL({
    host: '127.0.0.1',
    user: 'root',
    password: 'password',
    database: 'mydb'
  });

  // GET /api/users - Get list
  if (ctx.method === 'GET') {
    const users = await db.query('SELECT * FROM users');
    return response.json(users);
  }

  // POST /api/users - Create user
  if (ctx.method === 'POST') {
    const body = ctx.body as { name: string; email: string };
    const id = await db.insert('users', {
      name: body.name,
      email: body.email,
      created_at: new Date()
    });
    return response.json({ success: true, id }, 201);
  }

  return response.error('Method Not Allowed', 405);
});
```

### Login/Logout

```tsx
import { Layout } from "./components/Layout.tsx";

export default Page(async function(ctx, { session, response, createMySQL }) {
  // Logout
  if (ctx.url.pathname === '/logout') {
    await session.destroy();
    return response.redirect('/login');
  }

  // Login
  if (ctx.method === 'POST') {
    const form = ctx.body as { email: string; password: string };
    const db = await createMySQL({
      host: '127.0.0.1',
      user: 'root',
      password: 'password',
      database: 'mydb'
    });

    const users = await db.query(
      'SELECT * FROM users WHERE email = ?',
      [form.email]
    );

    if (users.length > 0) {
      const user = users[0] as any;

      // Verify password (should use bcrypt hash)
      if (user.password === form.password) {
        // Initialize session
        await session.init();

        // Store user info
        await session.set('userId', user.id);
        await session.set('userName', user.name);
        await session.set('userEmail', user.email);

        return response.redirect('/dashboard');
      }
    }

    return response.error('Login failed', 401);
  }

  // Show login form
  return (
    <Layout title="Login">
      <form method="POST">
        <input type="email" name="email" placeholder="Email" required />
        <input type="password" name="password" placeholder="Password" required />
        <button type="submit">Login</button>
      </form>
    </Layout>
  );
});
```

### Permission Check

```tsx
export default Page(async function(ctx, { session, response }) {
  await session.init();
  const userId = await session.get('userId');
  const userRole = await session.get('userRole');

  // Not logged in
  if (!userId) {
    return response.redirect('/login');
  }

  // Check role
  if (userRole !== 'admin') {
    return response.error('Insufficient permissions', 403);
  }

  const userName = await session.get('userName');

  return (
    <div>
      <h1>Admin Panel</h1>
      <p>Welcome, {userName}</p>
    </div>
  );
});
```

### Paginated Query

```tsx
export default Page(async function(ctx, { createMySQL, response }) {
  const page = parseInt(ctx.query.page || '1');
  const limit = 20;
  const offset = (page - 1) * limit;

  const db = await createMySQL({
    host: '127.0.0.1',
    user: 'root',
    password: 'password',
    database: 'mydb'
  });

  const users = await db.query(
    'SELECT * FROM users LIMIT ? OFFSET ?',
    [limit, offset]
  );

  const totalResult = await db.query('SELECT COUNT(*) as total FROM users');
  const total = (totalResult[0] as any).total;
  const totalPages = Math.ceil(total / limit);

  return response.json({
    data: users,
    pagination: {
      page,
      limit,
      total,
      totalPages
    }
  });
});
```

### Error Handling

```tsx
export default Page(async function(ctx, { logger, response }) {
  try {
    // Code that might error
    const data = await someAsyncOperation();

    return response.json({ success: true, data });
  } catch (error) {
    // Log error
    logger.error('Operation failed:', error);

    // Return error response
    return response.error(
      error instanceof Error ? error.message : 'Unknown error',
      500
    );
  }
});
```

---

## Best Practices

### 1. Directory Structure

```
www/
├── index.tsp              # Home page (access /)
├── api/                   # API endpoints
│   ├── users.tsp
│   └── posts.tsp
├── components/            # Reusable components (TSX, cannot access directly)
│   ├── Layout.tsx
│   └── Header.tsx
├── lib/                   # Utility modules (TS, cannot access directly)
│   └── utils.ts
├── admin/                 # Admin pages
│   └── dashboard.tsp
├── static/                # Static resources (served automatically)
│   ├── css/
│   ├── js/
│   └── images/
└── uploads/               # User uploaded files
```

### 2. Component Reuse

```tsx
// components/Button.tsx - TSX component, cannot access directly, but can be imported by .tsp
export function Button({ children, onClick }: {
  children?: unknown;
  onClick?: string;
}) {
  return (
    <button
      class="btn btn-primary"
      onclick={onClick}
    >
      {children}
    </button>
  );
}

// Usage - import in .tsp file
import { Button } from "./components/Button.tsx";

export default Page(async function(ctx) {
  return (
    <Button onclick="alert('Hello!')">
      Click me
    </Button>
  );
});
```

### 3. Environment Configuration

Configure server using `config.jsonc`:

```jsonc
{
  "root": "./www",
  "port": 9000,
  "dev": true,

  "logger": {
    "level": "INFO",
    "file": ".logs/app.log",
    "colorize": true,
    "format": "text"
  }
}
```

Load config automatically when starting server:

```bash
./tspserver --config config.jsonc
```

### 4. Security Recommendations

- **SQL injection prevention**: Use parameterized queries
  ```tsx
  // Correct
  await db.query('SELECT * FROM users WHERE id = ?', [userId]);

  // Wrong
  await db.query(`SELECT * FROM users WHERE id = ${userId}`);
  ```

- **XSS prevention**: TSX escapes output by default
  ```tsx
  const userInput = '<script>alert("XSS")</script>';
  return <div>{userInput}</div>; // Automatically escaped
  ```

- **CSRF prevention**: Use session and CSRF token
- **Password security**: Use bcrypt or argon2 to hash passwords

### 5. Performance Optimization

- **Use indexes**: Add indexes for database queries
- **Cache results**: Use Redis to cache frequently accessed data
- **Connection pool**: Configure database connection pool
- **Static resources**: Put CSS, JS, images in `static/` directory

### 6. Error Handling

Always wrap code that might error with try-catch:

```tsx
export default Page(async function(ctx, { logger, response }) {
  try {
    const result = await riskyOperation();
    return response.json(result);
  } catch (error) {
    logger.error('Operation failed:', error);
    return response.error('Internal server error', 500);
  }
});
```

### 7. Logging

Use log levels appropriately:

```tsx
export default Page(async function(ctx, { logger, createMySQL }) {
  logger.debug('Request params:', ctx.query);  // Debug info
  logger.info('User logged in:', userId);      // Important events
  logger.warn('Disk space low');               // Warning
  logger.error('Database connection failed:', error); // Error
});
```

---

## Type Reference

### PageContext

```typescript
interface PageContext {
  readonly method: HttpMethod;           // HTTP method
  readonly url: URL;                    // Full URL
  readonly headers: Headers;            // Request headers
  readonly query: Record<string, string>; // Query parameters
  readonly body: unknown;                // Request body
  readonly cookies: Record<string, string>; // Cookies
  readonly files: Record<string, UploadedFile | UploadedFile[]>; // Uploaded files
  readonly file: string;                // Current file path
  readonly root: string;                // Root directory
}
```

### RedirectResult

```typescript
interface RedirectResult {
  redirect: string;                      // Target URL
  status?: 301 | 302 | 303 | 307 | 308; // Status code
}
```

### UploadedFile

```typescript
interface UploadedFile {
  readonly name: string;                 // File name
  readonly type: string;                // MIME type
  readonly size: number;                // File size
  readonly data: Uint8Array;            // File content

  save(path: string): Promise<void>;    // Save file
  text(): Promise<string>;              // Read as text
}
```

---

## Frequently Asked Questions

### Q: How to debug pages?

A: Use `logger` to output debug info, or view console output in development mode:

```tsx
export default Page(async function(ctx, { logger }) {
  logger.debug('Current URL:', ctx.url.href);
  logger.debug('Query params:', ctx.query);
  logger.debug('Request body:', ctx.body);

  return <div>Check console</div>;
});
```

### Q: How to handle database connections?

A: Create connection per request, automatically close after request ends:

```tsx
export default Page(async function(ctx, { createMySQL }) {
  const db = await createMySQL(config);

  // Use database
  const users = await db.query('SELECT * FROM users');

  // No need to manually close, automatically closed after request ends
  return response.json(users);
});
```

### Q: How to use middleware?

A: Create components as layout wrappers:

```tsx
// components/AuthLayout.tsx
export function AuthLayout({ children }: { children?: unknown }) {
  return (
    <html>
      <body>
        <nav>Site Navigation</nav>
        <main>{children}</main>
        <footer>Copyright</footer>
      </body>
    </html>
  );
}

// Usage
export default Page(async function(ctx) {
  return (
    <AuthLayout>
      <h1>Page requiring authentication</h1>
    </AuthLayout>
  );
});
```

### Q: How to implement API authentication?

A: Use session or JWT:

```tsx
export default Page(async function(ctx, { session, response }) {
  await session.init();
  const userId = await session.get('userId');

  if (!userId) {
    return response.json({ error: 'Unauthorized' }, 401);
  }

  // Handle API request
  return response.json({ data: 'protected data' });
});
```

---

## More Resources

- **Config file**: `config.jsonc` - Server configuration options
- **Type definitions**: `types.d.ts` - Global type declarations
- **Example code**: Example files in `www/` directory

---

**Last updated**: 2026-02-25
**Version**: 4.0.0
