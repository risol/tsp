# Bootstrap Checklist

Use this checklist when initializing a new TSP site.

## Minimum checklist

- Choose the TSP site root.
- Create `<web-root>/index.tsp` when no page entry exists.
- Create `.vscode/settings.json` only if missing and useful.
- State whether `types.d.ts` already exists or still needs to be supplied.

## Strong recommended extras

- `components/`
- `components/shared/`
- `components/shared/Layout.tsx`
- `static/`

## `.vscode/settings.json` minimum fields

```json
{
  "deno.enable": true,
  "deno.lint": true,
  "deno.unstable": true,
  "deno.config": "./deno.json"
}
```

If the workspace does not use `deno.json`, omit `deno.config` instead of leaving a broken path.

## `<web-root>/index.tsp` minimum goal

The home page should confirm:

- TSP routing works
- TSX rendering works
- the page is easy to replace later

## Final reply checklist

The final reply should state:

- where the TSP site root is
- which bootstrap mode was used
- which core files were created
- any remaining assumptions that matter for later development
