---
name: release-process
description: End-to-end release runbook for playwright-rust — version bump, supply-chain refresh, CHANGELOG, the safer push-then-tag workflow that waits for CI before publishing, and the post-release follow-ups.
---

# Release Process

This skill captures the procedural steps for shipping a playwright-rust
release. For deeper automation (full pre-flight checks, mutation testing,
CHANGELOG validation), use the **release-preparation** sub-agent. This
skill exists for in-context reference when you're walking through a
release manually or guiding the user.

## Versioning

- `0.x.y` — pre-1.0, API may change (current stage)
- `1.0.0` — stable API, ready for production
- Patch (`x.y.Z`) — bug fixes, security advisories, no API changes
- Minor (`x.Y.0`) — additive features, deprecations, no breaking changes
  permitted in 1.x but acceptable in 0.x
- Major (`X.0.0`) — breaking changes (post-1.0)

Security advisories against transitive deps **always** warrant a patch
release, even if functional behavior is unchanged. See the
**supply-chain** skill.

## Pre-release checklist

1. **Tests are green on `main`** before starting
2. **Decide the version** (`X.Y.Z`)
3. **Bump `Cargo.toml`** workspace version
4. **Refresh `cargo vet`** — see the **supply-chain** skill for the
   `cargo vet regenerate unpublished` flow and exemption bumping
5. **Update `CHANGELOG.md`**:
   - Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`
   - Add a fresh empty `## [Unreleased]` heading above
   - Update the compare-link footer (`[Unreleased]` and add `[X.Y.Z]`)
6. **Update README install snippet** if minor or major version (`"0.X"`)
7. **Verify locally**: `cargo nextest run -p playwright-rs`,
   `cargo clippy -- -D warnings`, `cargo test --doc`,
   `cargo audit`, `cargo deny check`, `cargo vet`

## The safer push-then-tag workflow

A pushed git tag triggers `release.yml` which publishes to crates.io.
**Crates.io publishing is irreversible** — you cannot unpublish, only
yank. Always validate on CI before tagging.

```bash
# 1. Commit the version-bump changes
git add Cargo.toml Cargo.lock CHANGELOG.md README.md \
        supply-chain/imports.lock supply-chain/config.toml
git commit -m "Bump version to vX.Y.Z"

# 2. Push the COMMIT first (no tag yet)
git push origin main

# 3. Watch CI — Test on linux/mac/windows + Security & Quality
gh run watch  # or check the Actions tab in GitHub

# 4. Only after ALL required checks are green, create and push the tag
git tag -a vX.Y.Z -m "Release vX.Y.Z — <one-line summary>"
git push origin vX.Y.Z
```

If CI fails on `main` after the version-bump commit:

- Don't tag. Land a follow-up commit fixing the failure (or revert).
- A failed `main` is recoverable; a published bad version is not.

## What `release.yml` does on tag push

1. Runs the test suite on linux/macOS/windows as a final pre-publish gate
2. Creates the GitHub Release with notes from the matching `[X.Y.Z]`
   CHANGELOG section via `parse-changelog` — **CHANGELOG.md is the
   single source of truth for release notes**, no manual paste needed
3. Publishes to crates.io

The workflow is library-only: no binary artifacts are built, archived,
or attested. If a CLI is ever shipped, add a separate binary-release
pipeline rather than bolting onto this workflow.

## Post-release

1. **Verify** the GitHub Release at
   `https://github.com/padamson/playwright-rust/releases/tag/vX.Y.Z`
2. **Verify** crates.io has the new version
3. **Update tracking issues** if this release closes any
4. **Announce** if applicable (depends on release significance)

## Common pitfalls

- **Hand-editing `supply-chain/imports.lock`** — never; use
  `cargo vet regenerate unpublished` (see supply-chain skill)
- **Tagging before CI** — a single failing platform is enough to make a
  release un-rerunnable
- **Forgetting the `[Unreleased]` reset** — leaves the next session's
  CHANGELOG additions homeless
- **Skipping the README version bump** on minor releases — the install
  snippet's pinned `"0.X"` controls what new users see in the README on
  GitHub before the version on crates.io is current
