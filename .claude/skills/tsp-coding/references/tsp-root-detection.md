# TSP Root Detection

Use this file before coding so Claude knows where the real TSP web root lives.

## Common TSP web-root layouts

TSP projects commonly use one of these layouts:

### Layout A: Direct web root

```text
<workspace>/
  index.tsp
  products/
    index.tsp
```

### Layout B: Nested web root

```text
<workspace>/
  www/
    index.tsp
    products/
      index.tsp
```

Both are valid. Follow the layout that already exists.

## Detection procedure

1. Check the current workspace root.
2. If the workspace root already has route files like `index.tsp`, use the workspace root as the web root.
3. Otherwise, if the workspace has a nested directory like `www/` that already contains route files, use that nested directory as the web root.
4. If the user explicitly named another TSP project path, switch to that path for that task.
5. If the task is to create a new TSP site from scratch and no web root exists yet, use the current workspace root unless the repo already established a different convention.
6. If the task is to modify an existing site and no root can be identified safely, ask one short question for the path.

## Placement rules after root selection

Once the TSP web root is chosen:

- route files go under the chosen web root
- shared components usually go under `<web-root>/components/` or the repo's existing shared-component location
- helpers usually go under `<web-root>/lib/` or module-local `.ts/.tsx`

## Examples

### Example 1: Direct-root app

The workspace contains `index.tsp` and `products/index.tsp`.

Result:

- the workspace root is the web root
- new routes should follow that direct-root layout

### Example 2: Runtime repo, not business app

A workspace may contain the TSP framework/runtime source. It can be used as a reference repo, but business routes should not be mixed into runtime source unless the user explicitly wants that.

Result:

- use it as framework reference
- do not default to creating business pages in runtime `src/`

### Example 3: Nested web-root app

The workspace contains `www/index.tsp`.

Result:

- `www/` is the web root
- write new pages under `www/`
