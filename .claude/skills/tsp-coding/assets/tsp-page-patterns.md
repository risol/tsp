# TSP Page Patterns

Use these skeletons when Claude needs to write TSP pages directly instead of inventing a new pattern.

Adjust file paths and import paths to the local project's web-root convention.
Before copying a skeleton, check the local `types.d.ts` and remove any injected deps that are not declared in `interface AppDeps`.

## 1. Minimal route page

```tsx
export default Page(async function(ctx, { response }) {
  return (
    <html>
      <head>
        <title>Page</title>
      </head>
      <body>
        <h1>Page</h1>
      </body>
    </html>
  );
});
```

## 2. Standard page with DB and auth

```tsx
import Layout from '../components/shared/Layout.tsx';
import { requireAuth } from '../lib/middleware/auth.ts';
import config from '../config.ts';

export default Page(async function(ctx, { session, createMySQL, createZod, query }) {
  const z = await createZod();
  const db = await createMySQL(config.mysql, z);

  try {
    const authResult = await requireAuth(session, db, z);
    if (!authResult.success) {
      return { redirect: authResult.redirect || '/login' };
    }

    const params = query(z.object({
      page: z.coerce.number().min(1).default(1)
    }));

    return (
      <Layout title="Page">
        <div>{params.page}</div>
      </Layout>
    );
  } finally {
    await db.close();
  }
});
```

## 3. Form submit API

```tsx
import config from '../../config.ts';

export default Page(async function(ctx, { createMySQL, createZod, response, body, logger, createNanoid }) {
  const z = await createZod();
  const db = await createMySQL(config.mysql, z);
  const nanoid = await createNanoid();

  try {
    const formData = await body(z.object({
      name: z.string().min(1).max(200)
    }));

    const uid = nanoid();

    await db.execute(
      z.object({ affectedRows: z.number().optional(), insertId: z.number().optional() }),
      'INSERT INTO demo_table (uid, name) VALUES (?, ?)',
      [uid, formData.name]
    );

    return response.json({
      success: true,
      data: { uid }
    });
  } catch (error: any) {
    logger.error('Create failed', error);
    return response.json({ success: false, message: error?.message || 'Failed' }, 400);
  } finally {
    await db.close();
  }
});
```

## 4. List API

```tsx
import config from '../../config.ts';

export default Page(async function(ctx, { createMySQL, createZod, response, query }) {
  const z = await createZod();
  const db = await createMySQL(config.mysql, z);

  try {
    const params = query(z.object({
      page: z.coerce.number().min(1).default(1),
      pageSize: z.coerce.number().min(1).max(100).default(20)
    }));

    const items = await db.query(
      z.object({
        id: z.number(),
        uid: z.string(),
        name: z.string()
      }),
      `SELECT id, uid, name
       FROM demo_table
       ORDER BY id DESC
       LIMIT ${params.pageSize} OFFSET ${(params.page - 1) * params.pageSize}`
    );

    return response.json({ success: true, data: { items } });
  } finally {
    await db.close();
  }
});
```

## 5. Login page

```tsx
import config from './config.ts';

export default Page(async function(ctx, { createMySQL, createZod, createBcryptjs, session, body, response }) {
  const z = await createZod();

  if (ctx.method === 'GET') {
    return (
      <html>
        <head><title>Login</title></head>
        <body>
          <form method="POST">
            <input name="username" />
            <input name="password" type="password" />
            <button type="submit">Login</button>
          </form>
        </body>
      </html>
    );
  }

  const formData = await body(z.object({
    username: z.string().min(1),
    password: z.string().min(1)
  }));

  const db = await createMySQL(config.mysql, z);
  try {
    const user = await db.queryMaybe(
      z.object({
        id: z.number(),
        username: z.string(),
        password: z.string()
      }),
      'SELECT id, username, password FROM users WHERE username = ?',
      [formData.username]
    );

    if (!user) {
      return response.text('Invalid credentials', 400);
    }

    const bcrypt = await createBcryptjs({ saltRounds: 10 });
    if (!bcrypt.compare(formData.password, user.password)) {
      return response.text('Invalid credentials', 400);
    }

    await session.init();
    await session.set('userId', user.id);

    return response.redirect('/');
  } finally {
    await db.close();
  }
});
```

## 6. Inline JavaScript with `Script`

Use the public global `Script` component when a page needs inline JavaScript.

```tsx
export default Page(async function(ctx, { response }) {
  return (
    <html>
      <head>
        <title>Script Demo</title>
      </head>
      <body>
        <button id="copy-btn" type="button">Copy</button>
        <Script>
          {`
            document.getElementById('copy-btn')?.addEventListener('click', () => {
              console.log('clicked');
            });
          `}
        </Script>
      </body>
    </html>
  );
});
```

Do not write raw `<script>` with string children in JSX for this case.

## N. Page with an htmx fragment

A `.tsp` file can declare multiple sub-renders under a
`fragments` named export and expose them at
`<page>/__fragment/<name>`. Use this when part of the page should
be refreshed in place without a full reload. The `tsp-htmx`
skill covers the full htmx integration; this is the minimum
scaffold.

```tsx
export const fragments: FragmentMap = {
  table: Fragment(async (_ctx, { db, createZod }) => {
    const z = await createZod();
    const rows = await db.query(
      z.object({ id: z.number(), name: z.string() }),
      "SELECT id, name FROM users ORDER BY id",
    );
    return (
      <table>
        <tr><th>id</th><th>name</th></tr>
        {rows.map((r) => <tr key={r.id}><td>{r.id}</td><td>{r.name}</td></tr>)}
      </table>
    );
  }),
};

export default Page(async function () {
  return (
    <html>
      <head>
        <title>Users</title>
        <HtmxScript />
      </head>
      <body>
        <h1>Users</h1>
        <HtmxFragment
          page="/users"
          name="table"
          trigger="every 5s"
        />
      </body>
    </html>
  );
});
```

Key rules:

- Use `Fragment(...)` (semantic alias of `Page`) for each entry
  in `fragments`. The function gets the same `PageContext` and
  deps as the default page.
- The fragment URL is `<page>/__fragment/<name>`. The router
  serves bare HTML (no `<!DOCTYPE>`) so htmx `hx-swap="outerHTML"`
  works as a drop-in.
- `<HtmxFragment>` auto-fetches the initial content from
  `fragments[name]` during SSR; do not call the fragment function
  twice.
- If the user needs htmx on a different page's fragment, pass
  that page path to `page={...}` and the URL is built for you.

See `.claude/skills/tsp-htmx/SKILL.md` for the full htmx
reference (hxUrl, HtmxScript config options, cross-page
fragments, error handling).
