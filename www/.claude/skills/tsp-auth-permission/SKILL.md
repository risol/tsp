---
name: tsp-auth-permission
description: Authentication and authorization guidance for TSP projects. Use when implementing login, logout, sessions, admin checks, role or menu permissions, or protected APIs, using TSP session injection and server-side auth patterns instead of a heavy frontend auth layer.
---

# TSP Auth Permission

Use this skill for authentication and authorization work in TSP enterprise systems.

## Use this skill when

- The user asks for login, logout, session, current user, admin-only pages, role-based menus, or API permission checks.
- The request mentions users, roles, groups, permissions, menus, or protected operations.
- If the workspace has no TSP site root yet, use `tsp-site-bootstrap` first.

## Read first

- `../tsp-coding/references/tsp-root-detection.md`
- `../tsp-coding/references/tsp-hard-rules.md`
- `../tsp-coding/assets/tsp-page-patterns.md`

## Root and placement rule

- Determine the TSP site root first.
- Put login/logout routes under the chosen web root.
- Put auth middleware under `<web-root>/lib/middleware/` when the project uses that pattern.

## Preferred auth pattern

- Use injected `session`.
- For page routes, run auth checks before business logic.
- For API routes, return JSON permission errors instead of HTML redirects.
- Reuse middleware helpers if the repo already has them, such as `requireAuth`, `requireAdmin`, or `apiAuthMiddleware`.

## Page defaults

- unauthenticated page access: redirect to `/login`
- authenticated but forbidden page access: return a 403 page or friendly forbidden block
- logout: clear or destroy the session and redirect to login

## API defaults

- unauthenticated API access: `401` plus JSON error
- authenticated but forbidden API access: `403` plus JSON error
- success response: `success`, `message`, optional `data`

## Implementation rules

- Check the current project's `types.d.ts` before using injected deps such as `session`, `response`, `createMySQL`, or `createZod`.
- Do not build a SPA auth layer for a normal TSP project.
- Keep the source of truth on the server session and database.
- Refresh or verify the current user from DB when the existing codebase requires it.
- Put permission checks close to the protected action.
- Keep page auth and API auth behavior consistent across the module.

## Common deliverables

- `login.tsp`
- `logout.tsp`
- auth middleware under `lib/middleware/`
- protected page guards
- protected API guards
- role or permission lookup helpers
- Start from the login and API skeletons in `../tsp-coding/assets/tsp-page-patterns.md` when writing new auth routes.
