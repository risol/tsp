# TSP Hard Rules

Use this file whenever implementing or reviewing code in a TSP project.

## Core mental model

TSP is a server-rendered, route-file framework. Treat it more like PHP with TypeScript and dependency injection than like a SPA framework.

## Non-negotiable rules

### 1. Route/file rules

- `.tsp` files are HTTP routes.
- `.ts` and `.tsx` files are import-only modules and components.
- Files starting with `__` are private helpers and should not be directly routed.
- Route mapping is path-based:
  - `/` resolves to `index.tsp`
  - `/home` resolves to `home.tsp`
  - `/home/` resolves to `home/index.tsp`
- Do not assume `/home` will read `home/index.tsp`; that only applies to trailing-slash directory access.

### 2. Page entry rule

Always write route handlers like this:

```tsx
export default Page(async function(ctx, { response }) {
  return <div>Hello</div>;
});
```

Do not invent another page wrapper pattern.

### 3. Import rule

- Do not import from TSP runtime `src/` in app route files.
- Use global `Page`, `PageContext`, `RedirectResult`, and injected dependencies from the current project's `types.d.ts`.

### 4. Dependency discovery rule

- Read the current project's `types.d.ts` before using injected dependencies or TSP global helper types.
- Treat `interface AppDeps` as the canonical list of injectable dependencies.
- Use only the deps that are declared there. Do not invent names or assume framework examples always exist locally.
- If the project has no local `types.d.ts`, use the framework reference only as a fallback.

### 5. Request parsing rule

Prefer injected helpers over reading raw request data directly:

```tsx
const z = await createZod();
const params = query(z.object({
  page: z.coerce.number().min(1).default(1)
}));

const formData = await body(z.object({
  name: z.string().min(1)
}));
```

Use `body()` and `query()` when validation is needed. Do not trust raw user input.

### 6. Database rule

- Use `createMySQL(config, z)`.
- Validate every query result with Zod.
- Close the DB connection when finished.
- Prefer `query`, `queryOne`, `queryMaybe`, `scalar`, `execute`, and `tx`.

### 7. Response rule

Prefer injected `response` helpers:

- `response.json(...)`
- `response.text(...)`
- `response.html(...)`
- `response.redirect(...)`
- `response.file(...)`

For page redirects, returning `{ redirect: '/login' }` is also valid when the project already uses it.

### 8. Session/auth rule

- Use injected `session`.
- Initialize session if the project pattern requires `await session.init()`.
- Put auth checks before protected logic.

### 9. UI rule

- Prefer multi-page server-rendered flows.
- Use normal forms first.
- Add HTMX or small inline scripts only when it clearly reduces friction.
- When writing inline JavaScript, use the public global `Script` component instead of raw `<script>` so the script body is not escaped by React.
- Do not over-engineer client-side state.

## Practical dependency shortlist

Common injected dependencies in TSP projects:

- `createZod`
- `createMySQL`
- `query`
- `body`
- `response`
- `session`
- `logger`
- `createNanoid`
- `createExcelJS`
- `createRedis`
- `createLdap`

Only access dependencies you actually use, and only when they exist in the local `types.d.ts`.
