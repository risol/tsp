---
name: tsp-site-bootstrap
description: Bootstrap a new TSP site or initialize the smallest working page skeleton. Use when starting a new TSP web project from scratch, choosing the web-root convention, creating the home page, and setting up the minimum directory structure.
---

# TSP Site Bootstrap

Use this skill when the task is to start a new TSP website from scratch or turn a non-TSP workspace into a TSP site workspace.

## Use this skill when

- The user says "new website", "start from scratch", "start a new web project", "initialize a sample site", "set up a TSP site", or "build the project skeleton first".
- The current workspace does not yet contain a usable TSP site root.
- The user needs a minimal site structure before application pages or modules can be implemented.
- If the current workspace already has a usable page entry such as `index.tsp`, do not use this skill unless the user explicitly asks to rebuild the skeleton.

## Read first

- `../tsp-coding/references/tsp-root-detection.md`
- `references/bootstrap-modes.md`
- `references/bootstrap-checklist.md`
- `../tsp-coding/references/tsp-hard-rules.md`
- `../tsp-coding/assets/tsp-page-patterns.md`
- `assets/minimal-site-blueprint.md`

## Primary objective

Before building any application module, make sure the project has a clear web root and the minimum page skeleton needed for development.

## Bootstrap workflow

1. Detect whether a TSP site root already exists.
2. If a valid site root already exists, do not rebuild it. Hand off to the relevant TSP coding skill.
3. If no site root exists, choose a bootstrap mode from `references/bootstrap-modes.md`.
4. Create the minimum site tree from `assets/minimal-site-blueprint.md`.
5. Make sure route files live under the chosen web root.
6. Add a working home page first.
7. Only after the site skeleton exists, create application modules.

## Required output of a fresh bootstrap

A fresh site bootstrap should normally leave behind:

- a chosen web root
- `<web-root>/index.tsp`
- `components/` when needed
- `.vscode/settings.json` only if editor support is useful and missing

Optionally add:

- `components/shared/Layout.tsx`
- `static/`
- module directories such as `products/`, `about/`, `features/`

## Bootstrap rules

- If the workspace is empty and the user wants a new site here, choose the workspace root as the default web root unless the repo already implies a nested web-root convention.
- If the workspace already has a route entry such as `index.tsp` or `www/index.tsp`, treat it as initialized and skip bootstrap.
- Do not create an extra wrapper directory for routes.
- Do not create extra config files unless the user explicitly asks for them.
- Prefer the smallest working site, not a large framework scaffold.
- The first page should prove TSP routing works.
- Do not block on external runtime files.

## File-writing rules

- Put HTTP routes in the chosen web root.
- Use `.tsp` for pages and APIs.
- Use `.tsx` for reusable components.
- Use `.vscode/settings.json` only when the project needs editor support and it is missing.
- Do not require extra config files, `types.d.ts`, or external runtime files before writing pages.

## Handoff rules

After bootstrap:

- use `tsp-coding` for general TSP code authoring
- use `tsp-auth-permission` for login and permission work

## Completion check

Before finishing, verify that:

- the chosen TSP site root is explicit
- `<web-root>/index.tsp` exists when the task is to create a site home page
- no application routes were placed outside the current project tree
- the final response states whether this was bootstrap or direct page/module development
