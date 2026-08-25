# GitHub CI/CD

This repository's GitHub automation is intentionally configured to avoid
metered GitHub services.

## Workflows

- `.github/workflows/ci.yml` runs TypeScript tests and native Rust checks for
  Linux, Windows, and macOS on standard GitHub-hosted runners.
- `.github/workflows/release.yml` runs only when a `v*` tag is pushed. It builds
  the regular TSP server for the three desktop targets and uploads the packages
  directly to a draft GitHub Release before publishing it.

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
packages, and publishes the release only after all three builds succeed.
