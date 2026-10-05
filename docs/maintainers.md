# Maintainer setup (one-time, manual on GitHub)

1. **Secret `RELEASE_PLZ_TOKEN`**: Settings → Secrets and variables → Actions.
   Fine-grained PAT scoped to this repo with *Contents: read & write* and *Pull requests: read & write*.
   Needed because tags pushed with the default `GITHUB_TOKEN` do not trigger the release workflow.
2. **Squash merge only**: Settings → General → Pull Requests. Allow squash merging only; default message "Pull request title".
3. **Branch protection on `main`**: Settings → Rules → new ruleset for `main`. Require a PR, require status checks
   `fmt`, `clippy`, `test (ubuntu-latest)`, `test (macos-latest)`, `test (windows-latest)`, `msrv`, `scripts`, `coverage`, `deny`, `conventional`; block force pushes.
4. **Private vulnerability reporting**: Settings → Code security → enable.
5. **Social preview**: Settings → General → Social preview → upload `docs/assets/social-preview.png`.
6. **Topics**: `todo`, `rust`, `slint`, `desktop-app`, `productivity`, `local-first`, `keyboard-first`, `sqlite`.

## First release

release-plz runs in git-only mode. With no `v*` tag yet, the first push to `main` after this setup releases **v0.1.0**.
Finish steps 1–3 before merging the OSS-readiness PR.

## Releasing

Merge the "chore: release vX.Y.Z" PR that release-plz keeps open. Everything else is automatic.
