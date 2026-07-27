# Bootstrap Modes

Use this file to decide how to initialize a new TSP site.

## Mode A: Existing web root already present

Use when the current workspace already has:

- a usable route entry such as `index.tsp`
- or a nested route entry such as `www/index.tsp`
- or business modules already laid out under a clear web root

Action:

- Do not bootstrap again.
- Move directly to business implementation.

## Mode B: Minimal bootstrap in empty or plain repo

Use when:

- the current workspace has no usable route entry yet
- the user wants a new sample site or new business app here

Action:

- Choose a web root:
  - use the workspace root by default
  - use a nested web root only if the repo already established that convention
- Create:
  - `<web-root>/index.tsp`
  - `components/` when needed
  - `.vscode/settings.json` only if useful and missing

## Decision rule

Default choice:

- Existing site: Mode A
- New sample site in an empty repo: Mode B
