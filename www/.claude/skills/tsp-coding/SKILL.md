---
name: tsp-coding
description: Technical TSP coding guidance for writing, modifying, refactoring, or understanding TSP code. Use when Codex needs to identify the TSP web root, write `.tsp` pages or APIs, use `Page`, dependency injection, `body()` or `query()` validation, sessions, or MySQL. Do not use for business modeling or system architecture.
---

# TSP Coding

Use this skill for technical TSP authoring.

## Use this skill when

- The task is to write or modify `.tsp`, `.ts`, or `.tsx` files in a TSP project.
- The user asks how to write a TSP page, API route, form handler, redirect, session check, or DB query.
- The user asks how to place files in a TSP project or how to recognize the web root.
- The task is technical TSP coding, not business modeling.

## Read first

- `<project-root>/types.d.ts` when the project has one, especially `interface AppDeps`
- `references/tsp-root-detection.md`
- `references/tsp-hard-rules.md`
- `assets/tsp-page-patterns.md`

## Core workflow

1. Locate the TSP web root before coding.
2. Read the current project's `types.d.ts` before choosing injected deps, TSP globals, or helper signatures.
3. Follow the repo's existing route-placement convention.
4. Start from a concrete TSP skeleton instead of inventing a new pattern.
5. Use injected deps and Zod validation whenever request input or DB output is involved.
6. Keep the change minimal and idiomatic to TSP.

## TSP implementation rules

- Always decide the TSP web root first.
- Treat the current project's `types.d.ts` as the source of truth for injected deps and TSP helper types.
- Use `.tsp` for route files. Use `.ts` and `.tsx` only for import-only modules and components.
- Always write route handlers as `export default Page(async function(ctx, deps) { ... })`.
- Use only injected helpers that are actually declared in the current project's `types.d.ts`.
- Common helpers include `createZod`, `createMySQL`, `body`, `query`, `response`, `session`, `logger`, and `createNanoid`, but do not assume they all exist.
- When a TSP page needs inline JavaScript, prefer the public global `Script` component from the project's `types.d.ts` instead of raw `<script>`.
- Validate all request input with Zod before use.
- Validate all database reads with Zod schemas.
- Close database connections explicitly.
- Reuse existing shared components and helpers if the repo already has them.
- Start from `assets/tsp-page-patterns.md` when writing new pages or APIs.

## Root detection rules

- Determine whether the project uses:
  - direct web root, such as `index.tsp` at the workspace root
  - nested web root, such as `www/index.tsp`
- Follow the existing convention instead of inventing a new one.
- If the user explicitly names another TSP project path, switch to that path for that task.
- Never scatter `.tsp` routes outside the chosen root.

## Code-writing rules

- Default route placement:
  - home page: `<web-root>/index.tsp`
  - flat page route `/home`: `<web-root>/home.tsp`
  - directory route `/home/`: `<web-root>/home/index.tsp`
  - detail page: `<web-root>/<module>/detail.tsp`
  - create page: `<web-root>/<module>/create.tsp`
  - edit page: `<web-root>/<module>/edit.tsp`
  - APIs: `<web-root>/<module>/api/*.tsp`
- Route access mapping rules:
  - `/` resolves to `<web-root>/index.tsp`
  - `/home` resolves to `<web-root>/home.tsp`
  - `/home/` resolves to `<web-root>/home/index.tsp`
  - files starting with `__` are private helpers and are not HTTP routes
- Default component placement:
  - shared UI: `<web-root>/components/` or existing shared-component location
  - helpers: `<web-root>/lib/` or module-local `.ts/.tsx`

## What this skill should not do

- Do not teach business modeling.
- Do not infer approval flows, data architecture, or product structure unless the code task explicitly needs them.
- Do not turn a simple code task into a product-design exercise.

## Quality bar

Before finishing, check that:

- the chosen web root is correct
- injected deps match the current project's `types.d.ts`
- all new `.tsp` files follow the chosen web-root convention
- no code imports from TSP runtime `src/`
- no unvalidated request input is used when `body()` or `query()` fits
- database read/write schemas exist when data is involved
- protected operations run auth checks first when auth is part of the task
- database connections are closed
- response shapes are consistent with the local codebase

## References

- `references/tsp-root-detection.md`
- `references/tsp-hard-rules.md`
- `assets/tsp-page-patterns.md`
