# GitHub CI/CD

This repository's GitHub automation is intentionally configured to avoid
metered GitHub services.

## Workflows

- `.github/workflows/ci.yml` runs native tests and the embedded-worker smoke
  test for Linux, Windows, and macOS on standard GitHub-hosted runners. It
  runs for pull requests and pushes to both `master` and `main`.
- `.github/workflows/release.yml` runs only when a `v*` tag is pushed. It builds
  and uploads the embedded-worker package for the three desktop targets,
  then publishes the draft GitHub Release.

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
- The embedded-worker release job builds the Bun fork and native host on the same
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
embedded-worker packages, then publishes the release only after all builds
and smoke tests succeed.

## Local verification

Build and package the single-file runtime:

```bash
./tsp.sh build
```

Run the embedded-worker and hot-reload smoke test using the packaged executable:

```bash
sh ./scripts/smoke-tspserver.sh \
  dist/tspserver/tspserver
```

On Windows, use `scripts/smoke-tspserver.ps1` with the `.exe` path. The package
contains one runtime executable and the default `tsp.config.json`; the Master
creates worker children automatically.
