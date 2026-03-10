# Minimal Site Blueprint

Use this blueprint to scaffold a new TSP site in the smallest practical way.

## Minimum tree

```text
<site-root>/
  index.tsp
```

## Recommended tree

```text
<site-root>/
  .vscode/
    settings.json
  components/
    shared/
      Layout.tsx
  static/
  index.tsp
```

## Alternative nested web-root tree

```text
<workspace>/
  www/
    index.tsp
    components/
      shared/
        Layout.tsx
    static/
```

## `.vscode/settings.json`

If `deno.json` exists:

```json
{
  "deno.enable": true,
  "deno.lint": true,
  "deno.unstable": true,
  "deno.config": "./deno.json",
  "[typescript]": {
    "editor.defaultFormatter": "denoland.vscode-deno"
  },
  "[typescriptreact]": {
    "editor.defaultFormatter": "denoland.vscode-deno"
  },
  "typescript.tsdk": "deno",
  "typescript.enablePromptUseWorkspaceTsdk": true
}
```

If `deno.json` does not exist yet:

```json
{
  "deno.enable": true,
  "deno.lint": true,
  "deno.unstable": true,
  "[typescript]": {
    "editor.defaultFormatter": "denoland.vscode-deno"
  },
  "[typescriptreact]": {
    "editor.defaultFormatter": "denoland.vscode-deno"
  },
  "typescript.tsdk": "deno",
  "typescript.enablePromptUseWorkspaceTsdk": true
}
```

## `<web-root>/index.tsp`

Minimal page:

```tsx
export default Page(async function(ctx) {
  return (
    <html lang="en">
      <head>
        <meta charSet="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>TSP Sample Site</title>
        <style>{`
          body {
            font-family: sans-serif;
            max-width: 960px;
            margin: 40px auto;
            padding: 0 16px;
            line-height: 1.6;
          }
          .hero {
            padding: 24px;
            border: 1px solid #ddd;
            border-radius: 12px;
            background: #fafafa;
          }
        `}</style>
      </head>
      <body>
        <div className="hero">
          <h1>TSP Site Ready</h1>
          <p>The site skeleton has been created successfully.</p>
          <p>Next, you can add modules, list pages, form pages, and APIs.</p>
          <p>Current path: {ctx.url.pathname}</p>
        </div>
      </body>
    </html>
  );
});
```

## Optional `components/shared/Layout.tsx`

```tsx
interface LayoutProps {
  title?: string;
  children?: any;
}

export default function Layout({ title = 'TSP Site', children }: LayoutProps) {
  return (
    <html lang="en">
      <head>
        <meta charSet="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>{title}</title>
      </head>
      <body>{children}</body>
    </html>
  );
}
```

## Notes

This blueprint is intentionally minimal. Use the same layout style that the local repo already uses.
