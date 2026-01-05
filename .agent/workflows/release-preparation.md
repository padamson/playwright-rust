---
description: Use this workflow when preparing a release (version bump, CHANGELOG, verification). Automates pre-release checks, guides through release process, and ensures nothing is forgotten.
---

# Release Preparation Workflow

You are a specialized agent for preparing playwright-rust releases. You automate verification, guide the user through the release process step-by-step, and ensure quality gates are met.

## Your Role

Execute comprehensive pre-release verification, update version files, validate CHANGELOG completeness, and provide clear instructions for the manual release steps.

## Release Workflow

### Stage 1: Pre-Release Verification (Automated)

**Run these checks and report results**:

1. **Test Suite Verification**
   ```bash
   # Run all unit and integration tests
   cargo nextest run --workspace

   # Run doc-tests (ignored tests)
   cargo test --doc --workspace -- --ignored
   ```
   - Report: ✅ All tests pass / ❌ X tests failed (show failures)

2. **Code Quality Checks**
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   ```
   - Report: ✅ Format clean / ❌ Needs formatting
   - Report: ✅ Clippy clean / ❌ X warnings (show warnings)

3. **Example Compilation**
   ```bash
   cargo build --package playwright --examples
   ```
   - Report: ✅ All examples compile / ❌ Compilation failed (show error)

4. **Documentation Generation**
   ```bash
   cargo doc --no-deps --workspace
   ```
   - Report: ✅ Docs generate successfully / ❌ Doc errors (show errors)

5. **Git Status Check**
   ```bash
   git status --porcelain
   ```
   - Report: ✅ Clean working tree / ⚠️ Uncommitted changes (list files)

**If ANY check fails**: Stop and report issues. User must fix before proceeding.

**If ALL checks pass**: Proceed to Stage 2.

---

### Stage 2: Version Management (Interactive)

**Current version detection**:
1. Read `Cargo.toml` workspace version
2. Report current version to user
3. Ask user for target release version (e.g., "0.6.0")

**Version bump tasks**:
1. **Update Cargo.toml**
   - Update workspace version in root `Cargo.toml`
   - Verify all package versions inherit from workspace

2. **Update CHANGELOG.md**
   - If `## [X.Y.Z] - TBD` exists, replace TBD with today's date (YYYY-MM-DD)
   - Verify CHANGELOG has content for this version
   - Report what's documented (brief summary)

3. **Verify version consistency**
   ```bash
   cargo check --workspace
   ```
   - Ensures Cargo.lock is updated
   - Report: ✅ Version bump successful / ❌ Errors

**After version bump**: Show git diff and ask user to review changes.

---

### Stage 3: Pre-Release Validation (Automated)

**Run tests again after version bump**:
```bash
cargo nextest run --workspace
```
- Report: ✅ Tests still pass / ❌ Tests broken by version change

**Validate CHANGELOG format**:
- Check Keep a Changelog format compliance
- Verify links at bottom reference correct version
- Report: ✅ CHANGELOG valid / ⚠️ Issues found (describe)

**Check for common issues**:
- Grep for "TODO" or "FIXME" in public API docs
- Check for "TBD" or "unreleased" in docs
- Report: ✅ No blockers found / ⚠️ Issues to review (list them)

---

### Stage 4: Release Instructions (Manual Steps)

**After all automated checks pass**, provide user with clear instructions:

#### Git Commit and Tag

```bash
# Stage version changes
git add Cargo.toml Cargo.lock CHANGELOG.md

# Commit version bump
git commit -m "Bump version to vX.Y.Z"

# Create annotated tag
git tag -a vX.Y.Z -m "Release vX.Y.Z"

# Push commit and tag
git push origin main
git push origin vX.Y.Z
```

#### crates.io Publishing (Automated)

**Publishing is handled automatically via CI/CD**:
- The `.github/workflows/release.yml` workflow triggers on the tag push.
- It will build artifacts and publish the `playwright-rs` crate to crates.io.
- **Do NOT** run `cargo publish` manually.

#### GitHub Release (Automated + Manual Edit)

1. The CI workflow will automatically create/update the GitHub Release and upload assets.
2. After CI completes, **Edit** the release on GitHub: https://github.com/padamson/playwright-rust/releases
3. Paste the relevant section from `CHANGELOG.md` into the release description.

---

---

### Stage 5: Post-Release Tasks

**Update documentation**:
1. Mark slice as complete in implementation plan
2. Update roadmap.md if version is complete
3. Consider updating README.md development status

**Announce release** (optional):
- GitHub Discussions
- Project README
- Relevant Rust communities

---

## Best Practices

### CHANGELOG Quality
- Every release must have:
  - Clear summary of what's new
  - Migration notes for breaking changes
  - Known limitations
  - Links to issues/PRs (if applicable)

### Version Numbering (SemVer)
- **0.x.y** - Pre-1.0, breaking changes allowed in minor versions
- **x.Y.z** - Major version for breaking changes (post-1.0)
- **x.y.Z** - Patch version for bug fixes

### Release Timing
- **Don't rush**: All checks must pass
- **Clean state**: No uncommitted changes
- **Test on all platforms**: CI must be green

### Git Hygiene
- Annotated tags (`-a`) include tagger, date, message
- Tag message should be brief ("Release vX.Y.Z")
- Never force-push after tagging

---

## Error Recovery

**If tests fail after tagging**:
1. Delete tag: `git tag -d vX.Y.Z`
2. Delete remote tag: `git push origin :refs/tags/vX.Y.Z`
3. Fix issues
4. Re-run release process

**If published to crates.io with issues**:
- **Cannot unpublish** - crates.io doesn't allow deletions
- Immediately publish patch version (X.Y.Z+1) with fix
- Add note in CHANGELOG about the issue

---

## Checklist Summary

Generate a final checklist showing:
- [x] Automated checks completed
- [x] Version bumped and verified
- [ ] Git commit created
- [ ] Git tag created
- [ ] Pushed to GitHub (triggers CI)
- [ ] CI/CD verification (GitHub Actions)
- [ ] GitHub Release notes updated
- [ ] Published to crates.io (verified on crates.io)
- [ ] Post-release docs updated
- [ ] Release announced (optional)
