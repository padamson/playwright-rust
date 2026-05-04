---
name: release-process
description: End-to-end release runbook for playwright-rust — version bump, supply-chain refresh, per-crate CHANGELOGs, tag-prefix routing for the three workspace crates, the safer push-then-tag workflow that waits for CI before publishing, and the post-release follow-ups.
---

# Release Process

This skill captures the procedural steps for shipping a playwright-rust
release. For deeper automation (full pre-flight checks, mutation testing,
CHANGELOG validation), use the **release-preparation** sub-agent. This
skill exists for in-context reference when you're walking through a
release manually or guiding the user.

## Workspace layout — three independently-versioned crates

Three publishable crates, each with its own CHANGELOG and tag prefix:

| Crate                  | Path                              | CHANGELOG                                          | Tag prefix     |
|------------------------|-----------------------------------|----------------------------------------------------|----------------|
| `playwright-rs`        | `crates/playwright/`              | `crates/playwright/CHANGELOG.md`                   | `vX.Y.Z`       |
| `playwright-rs-macros` | `crates/playwright-rs-macros/`    | `crates/playwright-rs-macros/CHANGELOG.md`         | `macros-vX.Y.Z`|
| `playwright-rs-trace`  | `crates/playwright-rs-trace/`     | `crates/playwright-rs-trace/CHANGELOG.md`          | `trace-vX.Y.Z` |

The top-level `CHANGELOG.md` is an **index file**, not a changelog —
release notes are generated per-crate from each crate's own CHANGELOG.

The `xtask` workspace member is `publish = false` and has no CHANGELOG;
its workflow is documented in [`crates/xtask/`](../../../crates/xtask/).

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

Steps below assume you're releasing **`playwright-rs`** (the main crate).
For the macros or trace crate, swap the paths and tag prefix per the
table above; the workflow is otherwise identical.

1. **Tests are green on `main`** before starting
2. **Decide the version** (`X.Y.Z`) — independent of the other crates
3. **Bump the version** in the relevant `Cargo.toml`:
   - For `playwright-rs`: workspace `version` in the top-level `Cargo.toml`
   - For `playwright-rs-macros`: `version` in `crates/playwright-rs-macros/Cargo.toml`
   - For `playwright-rs-trace`: `version` in `crates/playwright-rs-trace/Cargo.toml`
4. **If a sibling crate's version changed too**, update the dep line in
   `crates/playwright/Cargo.toml`:
   - `playwright-rs-macros = { version = "...", path = "..." }` for the macros bump
   - `xtask`'s `playwright-rs = { path = "...", version = "..." }` if the main crate version changes (cargo-deny's no-wildcard rule)
5. **Refresh `cargo vet`** — see the **supply-chain** skill for the
   `cargo vet regenerate unpublished` / `cargo vet regenerate exemptions`
   flow
6. **Update the relevant CHANGELOG** (`crates/<crate>/CHANGELOG.md`):
   - Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD`
   - Add a fresh empty `## [Unreleased]` heading above
   - Update any compare-link footer if present
7. **Update README install snippet** if minor or major version (`"0.X"`)
   — applies to `playwright-rs` only
8. **Verify locally**:
   - `cargo nextest run --workspace`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - `cargo test --doc --workspace`
   - `cargo audit && cargo deny check && cargo vet`
   - **Dry-run the publish**: `cargo publish --dry-run -p <crate>` —
     catches packaging issues (missing files, license check, README
     path) before the irreversible real `cargo publish`

## The safer push-then-tag workflow

A pushed git tag triggers `release.yml` which publishes to crates.io.
**Crates.io publishing is irreversible** — you cannot unpublish, only
yank. Always validate on CI before tagging.

### Single-crate release (the common case)

```bash
# 1. Commit the version-bump changes for the chosen crate
git add Cargo.toml Cargo.lock crates/<crate>/Cargo.toml \
        crates/<crate>/CHANGELOG.md \
        supply-chain/imports.lock supply-chain/config.toml
# (also stage README.md if you bumped the main crate's minor version)
git commit -m "Bump <crate> to <prefix>vX.Y.Z"

# 2. Push the COMMIT first (no tag yet)
git push origin main

# 3. Watch CI — Test on linux/mac/windows + Security & Quality
gh run watch  # or check the Actions tab in GitHub

# 4. Only after ALL required checks are green, create and push the tag
#    Tag prefix maps to crate (see workspace table at top of file):
#      v0.13.0          → playwright-rs
#      macros-v0.1.1    → playwright-rs-macros
#      trace-v0.1.1     → playwright-rs-trace
git tag -a <prefix>vX.Y.Z -m "Release <crate> <prefix>vX.Y.Z — <one-line summary>"
git push origin <prefix>vX.Y.Z
```

### Coordinated release (multiple crates bumped together)

When a `playwright-rs` release also requires bumping a sibling crate
(e.g. v0.13.0 wants a fresh `playwright-rs-macros` 0.2.0), publish the
sibling first so the dep is available on crates.io when the main
crate's `cargo publish` runs:

```bash
# Single commit bumps all relevant Cargo.toml + CHANGELOG files.
git push origin main
gh run watch                      # CI must be green

# Push tags in dependency order. Wait ~30s between tags so the
# crates.io index propagates before the next publish runs.
git tag -a macros-vA.B.C -m "Release playwright-rs-macros vA.B.C"
git push origin macros-vA.B.C
sleep 30                          # crates.io index propagation
git tag -a trace-vD.E.F -m "Release playwright-rs-trace vD.E.F"
git push origin trace-vD.E.F
sleep 30
git tag -a vX.Y.Z -m "Release playwright-rs vX.Y.Z"
git push origin vX.Y.Z
```

If a sibling's version is unchanged this cycle, just skip its tag.

If CI fails on `main` after the version-bump commit:

- Don't tag. Land a follow-up commit fixing the failure (or revert).
- A failed `main` is recoverable; a published bad version is not.

## What `release.yml` does on tag push

The workflow handles all three tag prefixes (`v*`, `macros-v*`,
`trace-v*`) with one routed pipeline:

1. Runs the test suite on linux/macOS/windows as a final pre-publish
   gate (always, regardless of tag prefix).
2. **Resolves the tag** — `Resolve crate, changelog, and version from
   tag` step parses `${GITHUB_REF#refs/tags/}` and routes:
   - `macros-v*` → `playwright-rs-macros` + `crates/playwright-rs-macros/CHANGELOG.md`
   - `trace-v*` → `playwright-rs-trace` + `crates/playwright-rs-trace/CHANGELOG.md`
   - `v*` → `playwright-rs` + `crates/playwright/CHANGELOG.md`
3. **Generates release notes** from the resolved CHANGELOG via
   `parse-changelog <CHANGELOG> <VERSION>` — the **per-crate
   CHANGELOG is the single source of truth** for release notes; no
   manual paste needed.
4. **Creates the GitHub Release** with `name = "<crate> <version>"`
   and the body from the parsed CHANGELOG section.
5. **Publishes to crates.io** — exactly one publish step fires per
   tag, gated by `startsWith(github.ref_name, '<prefix>')`. Failure
   aborts the workflow (no `continue-on-error`); the release tag
   stays in place but the publish didn't happen, so re-running with
   the same tag after fixing the issue is safe.

The workflow is library-only: no binary artifacts are built, archived,
or attested. If a CLI is ever shipped, add a separate binary-release
pipeline rather than bolting onto this workflow.

## Post-release

1. **Verify** the GitHub Release at
   `https://github.com/padamson/playwright-rust/releases/tag/<prefix>vX.Y.Z`
2. **Verify** crates.io has the new version (e.g.
   `https://crates.io/crates/playwright-rs/0.13.0`)
3. **First-time publish bookkeeping** — if this is the first crates.io
   release of a workspace crate, add
   `[policy.<crate>] audit-as-crates-io = true` to
   `supply-chain/config.toml` in a follow-up commit. Cannot be done
   pre-release because `cargo vet` rejects the policy until the crate
   exists on crates.io.
4. **Update tracking issues** if this release closes any
5. **Announce** if applicable (depends on release significance)

## Common pitfalls

- **Hand-editing `supply-chain/imports.lock`** — never; use
  `cargo vet regenerate unpublished` (see supply-chain skill)
- **Tagging before CI** — a single failing platform is enough to make a
  release un-rerunnable
- **Forgetting the `[Unreleased]` reset** in the per-crate CHANGELOG —
  leaves the next session's CHANGELOG additions homeless
- **Editing the top-level `CHANGELOG.md`** — it's an index, not a
  changelog. Per-crate CHANGELOGs are the source of truth; if you
  tried adding a release entry to the index it won't appear in the
  generated GitHub release notes.
- **Skipping the README version bump** on minor releases of
  `playwright-rs` — the install snippet's pinned `"0.X"` controls what
  new users see in the README on GitHub before the version on
  crates.io is current
- **Coordinated release without sleep between tags** — crates.io index
  propagation takes ~10–30s; `cargo publish -p playwright-rs` will fail
  to resolve a freshly-published `playwright-rs-macros` if the tags
  are pushed back-to-back without a wait
- **Pushing the wrong prefix** — `vX.Y.Z` always means `playwright-rs`;
  `macros-vX.Y.Z` is the macros crate; `trace-vX.Y.Z` is the trace
  crate. The `release.yml` "Resolve crate, changelog, and version from
  tag" step rejects unknown prefixes
