# Bun upstream tracking

TSP embeds and builds the Bun source tree under `bun/`. The Bun version used by
`oven-sh/setup-bun` in CI is only a bootstrap runtime; it is not the version
that is shipped inside the TSP server. The WebKit/JSC revision is a separate
native dependency and must be reviewed with Bun updates.

The current accepted baseline is recorded in
[`bun-upstream.json`](./bun-upstream.json). The daily workflow
[`check-bun-upstream.yml`](../../../.github/workflows/check-bun-upstream.yml)
checks the latest Bun release and Bun repository security advisories. When it
finds an actionable change, it opens one deduplicated Issue. It never changes
the source pins automatically.

Run the same check locally with:

```text
bun run scripts/check-bun-upstream.ts
```

To get machine-readable output:

```text
bun run scripts/check-bun-upstream.ts --json
```

## Updating the baseline

Review the upstream release notes and relevant fix commits first. If the fix
touches JavaScriptCore, WebKit, allocators, FFI, or embedded workers, follow
the native-runtime verification rules in `AGENTS.md` and the relevant ADR or
bug record.

At minimum, an accepted update must pass the focused Rust tests, the Linux
embedded-worker release build, the Windows embedded-worker smoke test, and the
TSP smoke test. Then update `bun-upstream.json` with the tested Bun revision,
release tag when known, WebKit revision, and advisory IDs that have been
reviewed.

The tracker intentionally treats the following as separate values:

| Value | Meaning |
| --- | --- |
| `bun.version` / `bun.revision` | Embedded Bun source baseline shipped by TSP |
| `bootstrap.version` | Temporary Bun executable used to run the build scripts |
| `webkit.revision` | Pinned WebKit/JSC native dependency |
| `security.knownAdvisoryIds` | Bun advisories already reviewed by TSP |
