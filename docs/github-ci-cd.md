# GitHub CI/CD

This repository's GitHub automation is intentionally configured to avoid
metered GitHub services.

## Workflows

- `.github/workflows/ci.yml` runs TypeScript tests and native Rust checks for
  Linux, Windows, and macOS on standard GitHub-hosted runners. It runs for
  pull requests and pushes to both `master` and `main`.
- `.github/workflows/release.yml` runs only when a `v*` tag is pushed. It builds
  the regular TSP server and the v2.4 embedded-worker package for the three
  desktop targets, then uploads the packages directly to a draft GitHub
  Release before publishing it.

## Cost protection

- All runners use standard labels such as `ubuntu-latest`, `windows-latest`,
  and `macos-latest`.
- Larger GitHub-hosted runners are not used.
- No self-hosted runner is required.
- No Actions artifacts are uploaded.
- No Docker image is pushed automatically.
- No external deployment service, cloud account, or paid secret is used.
- CI cancels obsolete runs for the same branch or pull request.
- Release builds require an explicit version tag, so normal pushes do not
  start the release workflow.
- The v2.4 release job builds the Bun fork and native v2 host on the same
  standard runners; it does not use a paid build service or persistent build
  machine.

GitHub currently documents standard GitHub-hosted runner usage as free for
public repositories. If the repository is later made private, these workflows
must be reviewed before use because private-repository quotas and billing are
different.

## Release procedure

```bash
git tag v0.1.6
git push origin v0.1.6
```

The release workflow creates a draft, uploads the Linux, Windows, and macOS
packages for both the regular server and v2.4 embedded-worker runtime, then
publishes the release only after all builds and smoke tests succeed.

## v2.4 local verification

Build the native v2 host and the Bun worker separately:

```bash
sh ./tsp.sh build:tspserver:v2:rel
cd bun
bun install --frozen-lockfile
bun run build:release
cd ..
```

Run the embedded-worker and hot-reload smoke test using the resulting binaries:

```bash
sh ./scripts/smoke-tspserver-v2.sh \
  bun/target/release/tspserver_v2 \
  bun/build/release/bun
```

On Windows, use `scripts/smoke-tspserver-v2.ps1` with the `.exe` paths. The
package scripts emit a `tsp-v2-runtime.json` manifest and place the worker
beside the host, so the packaged runtime can use `TSP_WORKER_BIN` explicitly
or resolve the sibling `bun` executable automatically.
