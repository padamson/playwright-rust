# ADR 0003: Single-Crate Architecture

**Status:** Accepted

**Date:** 2025-11-15

**Related Documents:**
- Implementation Plan: [docs/implementation-plans/v1.0-real-world-validation.md](../implementation-plans/v1.0-real-world-validation.md)
- Issue: [#3 - Build script workspace detection](https://github.com/padamson/playwright-rust/issues/3)
- Triggered by: v0.6.1 bug fix revealing architectural complexity

---

## Context and Problem Statement

playwright-rust currently uses a two-crate architecture:
- `playwright-core` - Internal protocol implementation
- `playwright` (published as `playwright-rs`) - High-level public API

This split was designed for separation of concerns, but issue #3 revealed significant complexity in the two-crate approach. When users depend on `playwright-rs` from crates.io, the build script in `playwright-core` struggles to determine where to download Playwright drivers, leading to confusing failures.

**Core Question:** Does the two-crate split provide enough value to justify its complexity, or should we consolidate into a single crate?

### Requirements Summary

- **Functional Requirements:**
  - Provide idiomatic Rust bindings for Playwright
  - Support all Playwright features (browser automation, testing, assertions)
  - Enable future tools (codegen, inspector, trace viewer)
  - Work correctly as a crates.io dependency in downstream projects

- **Non-Functional Requirements:**
  - **Simplicity:** Easy to publish, version, and maintain
  - **User Experience:** Clear dependency model, minimal confusion
  - **API Compatibility:** Match official Playwright implementations
  - **Encapsulation:** Keep protocol details internal
  - **Maintainability:** Single developer, minimal overhead

- **Constraints:**
  - Must follow Playwright's proven architecture patterns
  - Must work with official Playwright server (not reimplement)
  - Solo developer with limited time for maintenance
  - Goal: Production-quality for broad adoption

### Current Architecture Context

**Existing Codebase:**
```
playwright-rust/
├── crates/
│   ├── playwright/          # High-level API
│   │   └── depends on → playwright-core
│   └── playwright-core/     # Protocol implementation
│       └── build.rs         # Driver download (source of issue #3)
```

**Current Dependencies:**
- Users add: `playwright-rs = "0.6.1"`
- This pulls in: `playwright-core = "0.6.1"` (internal)
- Both crates always versioned together

**Integration Points:**
- crates.io publishing (two packages to publish in order)
- Downstream projects (e.g., t2t) depending on playwright-rs
- Future: playwright codegen, inspector, trace viewer

---

## Decision Drivers

1. **Simplicity** - Minimize maintenance burden for solo developer
2. **Match Official Implementations** - Follow proven patterns from playwright-python/java/dotnet
3. **User Experience** - Clear, simple dependency model
4. **Revealed Complexity** - Issue #3 showed real-world problems with two-crate split
5. **Future Tooling** - Support codegen/inspector/traces without additional crates

---

## Options Considered

### Option 1: Keep Two-Crate Architecture (Status Quo)

**Description:**
Maintain current split with `playwright-core` (internal) and `playwright` (public API).

**Key Implementation Details:**
- `playwright-core` contains protocol, connection, transport, server management
- `playwright` contains high-level API (Browser, Page, Locator, assertions)
- Users depend on `playwright-rs`, which re-exports from `playwright-core`
- Publishing order: core first, then main crate

**Pros:**
- ✅ Already implemented - no migration needed
- ✅ Clear separation of protocol vs. API
- ✅ Could theoretically support multiple API styles

**Cons:**
- ❌ Complexity: Two crates to version, publish, maintain
- ❌ Publishing order matters (must publish core before main)
- ❌ Version coupling - both always versioned together anyway
- ❌ User confusion - which crate to depend on?
- ❌ Build complexity - Issue #3 proves this creates real problems
- ❌ No proven need for separate crates

**Dependencies Required:**
- Current setup - no changes

---

### Option 2: Single-Crate Architecture (Recommended)

**Description:**
Consolidate into a single `playwright` crate (published as `playwright-rs`), using Rust's module system and visibility controls for encapsulation.

**Key Implementation Details:**
- Single crate: `playwright`
- Public API in `src/api/` exported via `lib.rs`
- Protocol internals in `src/protocol/` (not exported, `pub(crate)` only)
- Server management in `src/server/` (not exported, `pub(crate)` only)
- Single `build.rs` handling driver downloads

**Project Structure:**
```rust
playwright/
├── src/
│   ├── lib.rs              // Public exports only
│   ├── api/                // Public API modules
│   │   ├── mod.rs
│   │   ├── browser.rs      // pub struct Browser
│   │   ├── page.rs         // pub struct Page
│   │   ├── locator.rs      // pub struct Locator
│   │   └── assertions.rs   // pub fn expect()
│   ├── protocol/           // Internal (not exported)
│   │   ├── mod.rs          // pub(crate) only
│   │   ├── connection.rs
│   │   ├── transport.rs
│   │   └── messages.rs
│   └── server/             // Internal (not exported)
│       ├── mod.rs
│       └── driver.rs
├── build.rs                // Single build script
├── Cargo.toml              // Single package
├── examples/
└── tests/
```

**Visibility Control:**
```rust
// lib.rs - Public exports
pub use api::{Browser, Page, Locator};
pub use api::assertions::expect;

// Internal modules - not visible to users
mod protocol;  // Not pub - invisible externally
mod server;    // Not pub - invisible externally

// Or use pub(crate) for internal cross-module visibility
pub(crate) mod protocol;
pub(crate) mod server;
```

**Code Example:**
```rust
// User code - same as before
use playwright::{Playwright, expect};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::initialize().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto("https://example.com", None).await?;
    expect(page.locator("h1")).to_be_visible().await?;

    Ok(())
}
```

**Pros:**
- ✅ **Matches official implementations** - All use single package (playwright-python, playwright-java, playwright-dotnet, playwright-js)
- ✅ **Simpler publishing** - One `cargo publish`, one version
- ✅ **Clearer for users** - Single dependency: `playwright-rs = "0.6.1"`
- ✅ **Better encapsulation** - Rust's `pub(crate)` is perfect for internal visibility
- ✅ **Simpler build** - One build.rs, no workspace detection ambiguity
- ✅ **Easier maintenance** - One CHANGELOG, one version, one crate to think about
- ✅ **Prevents issue #3 class of bugs** - No confusion about which crate downloads drivers
- ✅ **Supports future tools** - Codegen/inspector/traces can be simple helpers in same crate

**Cons:**
- ⚠️ Migration effort required (one-time cost)
- ⚠️ Breaking change for users (though mitigatable with version bump)
- ⚠️ All code compiles together (but workspace still available for dev)

**Dependencies Required:**
- Consolidate dependencies from both crates into single `Cargo.toml`
- No new dependencies needed

---

### Option 3: Three-Crate Split (Protocol, API, Tools)

**Description:**
Further split into three crates: `playwright-protocol`, `playwright-api`, `playwright-tools`.

**Pros:**
- ✅ Even more granular separation of concerns

**Cons:**
- ❌ **Much more complexity** - Three crates to version, publish, maintain
- ❌ **No proven need** - Official implementations don't do this
- ❌ **Overkill** - Tools are just CLI wrappers
- ❌ **Publishing nightmare** - Must publish in specific order
- ❌ **User confusion multiplied** - Which crate(s) to depend on?

**Not seriously considered** - included for completeness only.

---

## Comparison Matrix

### Architectural Comparison

| Aspect | Two-Crate (Current) | Single-Crate (Proposed) | Weight | Winner |
|--------|---------------------|------------------------|--------|--------|
| **Simplicity** | 2/5 (two crates) | 5/5 (one crate) | High | Single |
| **Matches Official Impl** | 2/5 (unique to us) | 5/5 (exact match) | High | Single |
| **Publishing Complexity** | 2/5 (ordered) | 5/5 (simple) | High | Single |
| **User Experience** | 3/5 (confusing) | 5/5 (clear) | High | Single |
| **Encapsulation** | 4/5 (crate boundary) | 5/5 (pub(crate)) | Medium | Single |
| **Flexibility** | 3/5 (could split API) | 3/5 (same) | Low | Tie |
| **Migration Cost** | 5/5 (none) | 2/5 (one-time) | Low | Two-Crate |

### Dependency Comparison

| Aspect | Two-Crate | Single-Crate | Notes |
|--------|-----------|--------------|-------|
| **Total Crates Published** | 2 | 1 | Both appear on crates.io |
| **Version Coupling** | Tight (always match) | N/A | Already coupled anyway |
| **Publishing Order** | Core → Main | Any | Must get order right |
| **User Dependencies** | 1 (playwright-rs) | 1 (playwright-rs) | Same for users |
| **Binary Size** | ~Same | ~Same | No real difference |
| **Compile Time** | ~Same | ~Same | Both in same workspace |

### Maintenance Comparison

| Factor | Two-Crate | Single-Crate | Weight | Winner |
|--------|-----------|--------------|--------|--------|
| **CHANGELOGs to Maintain** | 2 | 1 | Medium | Single |
| **Version Numbers to Bump** | 2 | 1 | High | Single |
| **Publishing Steps** | 2 (ordered) | 1 | High | Single |
| **Issue Tracking** | 2 crates | 1 crate | Low | Single |
| **Documentation Sites** | 2 | 1 | Low | Single |

### Real-World Evidence

**Proof: All official Playwright implementations use single packages:**

| Implementation | Package Structure | Crates/Packages |
|---------------|-------------------|-----------------|
| playwright (Node.js) | `playwright` (npm) | 1 |
| playwright-python | `playwright` (pip) | 1 |
| playwright-java | `com.microsoft.playwright:playwright` | 1 |
| playwright-dotnet | `Microsoft.Playwright` (NuGet) | 1 |
| **playwright-rust (current)** | `playwright-rs` + `playwright-core` | **2** ⚠️ |
| **playwright-rust (proposed)** | `playwright-rs` | **1** ✅ |

**No official implementation uses a two-package split.**

---

## Decision Outcome

**Chosen Option:** Option 2 - Single-Crate Architecture

**Release Plan:**
- Consolidation published as **v0.7.0** (not v1.0.0)
- v1.0.0 reserved for later after real-world validation
- This allows iteration on the single-crate architecture before API stability commitment

**Rationale:**

We chose to consolidate into a single `playwright` crate because:

1. **Matches Official Implementations** - ALL official Playwright implementations (Python, Java, .NET, Node.js) use a single package. This is the proven, battle-tested pattern. We should follow it unless we have a compelling reason not to (we don't).

2. **Solves Real Problems** - Issue #3 revealed that the two-crate split creates real-world complexity:
   - Build script in `playwright-core` must guess where drivers should go
   - Users depend on `playwright-rs` but drivers come from `playwright-core`
   - Created a 3-tier workspace detection hack to work around this
   - Single crate eliminates this entire class of problems

3. **Dramatically Simpler** - Comparison matrix is decisive:
   - **Publishing:** 1 step vs 2 ordered steps
   - **Versioning:** 1 number vs 2 coupled numbers
   - **Maintenance:** 1 CHANGELOG vs 2
   - **User clarity:** Clear vs confusing

4. **No Loss of Encapsulation** - Rust's module system is perfect for this:
   - `pub` exports for public API
   - `pub(crate)` for internal cross-module use
   - Private modules for protocol internals
   - Just as good as crate boundaries, simpler to work with

5. **Supports Future Tools** - Codegen, inspector, trace viewer:
   - Are provided by Playwright server (language-agnostic)
   - Need simple CLI wrappers in Rust (not separate crates)
   - Single crate is perfect for this

6. **Solo Developer Context** - With one maintainer:
   - Simpler is better
   - Less overhead = more time for features
   - Fewer places for bugs to hide

**Trade-offs Accepted:**

- **One-time migration cost** - Worth it for long-term simplicity
- **Breaking change for users** - Mitigated by clear migration guide and v0.7.0 version bump
- **All code compiles together** - Not actually a problem; workspace still available for incremental dev
- **Deferred v1.0.0** - Consolidation happens in v0.7.0, v1.0.0 comes after validation

**Why Not Keep Two Crates:**

The two-crate split provides **no proven benefits**:
- ❌ Not used by any official implementation
- ❌ No multiple API styles exist
- ❌ Core and API never version independently
- ❌ No external consumers of "core"
- ❌ Creates real problems (issue #3)
- ❌ Adds maintenance overhead

---

## Consequences

### Positive Consequences

- ✅ **Simpler publishing workflow** - One `cargo publish` command, no ordering concerns
- ✅ **Clearer user experience** - One crate to depend on, no confusion
- ✅ **Easier maintenance** - One version, one CHANGELOG, one docs site
- ✅ **Aligns with Playwright ecosystem** - Matches Python/Java/.NET patterns exactly
- ✅ **Prevents build.rs complexity** - No workspace detection needed
- ✅ **Better for future tools** - Codegen/inspector/traces integrate cleanly
- ✅ **Faster iteration** - Less overhead means more feature development

### Negative Consequences

- ⚠️ **Migration effort required** - One-time cost to consolidate code
- ⚠️ **Breaking change** - Users must update `Cargo.toml` (minor impact)
- ⚠️ **All code in one crate** - Slightly longer compile times (negligible in practice)

### Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| Migration breaks existing code | High | Low | Comprehensive test suite runs after migration; trunk-based development with continuous validation |
| Users upset by breaking change | Medium | Low | Clear migration guide, v0.7.0 version bump signals architectural change |
| Need two crates later | Medium | Very Low | Rust modules sufficient; all official impls prove this works |
| Compile times increase | Low | Medium | Workspace still available for incremental dev; impact minimal |
| Confusion from published playwright-core | Medium | Medium | Yank old versions (0.6.0, 0.6.1), publish final deprecation version (0.6.2) pointing to playwright-rs |
| Issues found before v1.0.0 | Low | Medium | v0.7.0 allows iteration; v1.0.0 only after validation |

---

## Validation

### How This Decision Will Be Validated

- [x] Research all official Playwright implementations (confirmed: all use single package)
- [ ] Implement consolidation on feature branch
- [ ] Verify all 248 tests pass
- [ ] Verify examples work unchanged
- [ ] Verify clippy/fmt pass
- [ ] Test publishing to crates.io (test account first)
- [ ] Get community feedback (GitHub issue/discussion)

### Success Criteria

- All existing tests pass after migration (248/248)
- All examples work without code changes
- Publishing workflow simpler (1 step vs 2)
- Users can upgrade with clear migration path
- No regression in functionality
- Docs clearly explain migration

### Benchmark Needed?

**Decision:** No

**Why Not:** This is an architectural reorganization, not a performance change. The code is the same, just moved. No performance impact expected or relevant.

---

## Implementation Notes

### Development Approach

**Trunk-Based Development:**
- Work directly on main branch with frequent small commits
- CI/CD validates each commit
- Tests must pass before each push
- No long-lived feature branches

### Migration Path

See detailed implementation plan: [docs/implementation-plans/v1.0-real-world-validation.md](../implementation-plans/v1.0-real-world-validation.md)

**High-Level Phases:**

- **Stage 1: Preparation** (1 day)
  - Document current structure
  - Plan module organization
  - Create implementation plan

- **Stage 2: Code Consolidation** (2 days)
  - Move `playwright-core` code into `playwright`
  - Update visibility (`pub` → `pub(crate)` for internals)
  - Merge `Cargo.toml` files
  - Merge build scripts

- **Stage 3: Testing & Validation** (1 day)
  - Run full test suite
  - Update examples
  - Verify clippy/fmt
  - Test publishing

- **Stage 4: Documentation & Release** (1 day)
  - Update README
  - Update CHANGELOG (v1.0.0 or v0.7.0)
  - Migration guide
  - Publish to crates.io

**Total Estimated Time:** 5 days

### Code Changes Required

1. **Move code from `crates/playwright-core/src/` to `crates/playwright/src/protocol/` and `src/server/`**
2. **Update visibility:**
   - `pub struct Connection` → `pub(crate) struct Connection`
   - `pub fn start_server()` → `pub(crate) fn start_server()`
3. **Merge Cargo.toml dependencies**
4. **Update lib.rs exports to only export public API**
5. **Remove `crates/playwright-core/` directory**

### Documentation Updates

- [ ] Update README - Installation section (remove core mention)
- [ ] Update CHANGELOG - Document breaking change
- [ ] Add MIGRATION.md - Guide for users upgrading
- [ ] Update rustdoc - Module organization
- [ ] Update CLAUDE.md - Remove two-crate references

### Testing Strategy

- [ ] Run full test suite (`cargo nextest run`)
- [ ] Run all examples
- [ ] Test as dependency in t2t (real-world validation)
- [ ] Verify publishing workflow
- [ ] Cross-platform CI (Linux, macOS, Windows)

### Handling Published playwright-core Versions

**Problem:** playwright-core v0.6.0 and v0.6.1 are already published to crates.io. How do we deprecate them?

**Solution: Three-Step Deprecation**

1. **Publish playwright-core v0.6.2 (deprecation notice):**
   ```toml
   # Cargo.toml
   [package]
   name = "playwright-core"
   version = "0.6.2"
   description = "⚠️ DEPRECATED: Use playwright-rs instead. Merged into playwright-rs as of v0.7.0."
   ```

   Update README.md:
   ```markdown
   # ⚠️ DEPRECATED

   This crate has been merged into `playwright-rs` as of v0.7.0.

   **Please use `playwright-rs` instead:**
   \```toml
   [dependencies]
   playwright-rs = "0.7"
   \```

   See: https://github.com/padamson/playwright-rust/blob/main/MIGRATION.md
   ```

2. **Yank old versions after v0.7.0 is stable:**
   ```bash
   cargo yank --vers 0.6.0 playwright-core
   cargo yank --vers 0.6.1 playwright-core
   ```
   - Prevents new projects from using them
   - Existing Cargo.lock files still work
   - Clear signal: "don't use this"

3. **Leave v0.6.2 un-yanked as deprecation marker:**
   - Visible on crates.io with warning
   - Points users to playwright-rs
   - Can be yanked later if desired

**Timeline:**
- Before v0.7.0 release: Publish playwright-core v0.6.2
- After v0.7.0 stable (1 week): Yank v0.6.0 and v0.6.1
- After v0.7.0 mature (1 month): Optionally yank v0.6.2

**User Impact:**
- Existing users: Cargo.lock protects them
- New users: See deprecation, use playwright-rs instead
- Clear migration path documented

### Rollback Plan

- Original code preserved in git history
- Can revert commits if critical issues found
- Trunk-based development = small, revertible commits
- Can un-yank playwright-core versions if needed (rare)
- Low risk: code unchanged, just reorganized

---

## References

**Official Playwright Implementations:**
- [playwright-python](https://github.com/microsoft/playwright-python) - Single pip package
- [playwright-java](https://github.com/microsoft/playwright-java) - Single Maven artifact
- [playwright-dotnet](https://github.com/microsoft/playwright-dotnet) - Single NuGet package
- [playwright (Node.js)](https://github.com/microsoft/playwright) - Single npm package

**Issue That Revealed This Problem:**
- [Issue #3: Build script workspace detection fails](https://github.com/padamson/playwright-rust/issues/3)

**Rust Module System:**
- [The Rust Book - Modules](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html)
- [Visibility and Privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html)

**Related ADRs:**
- None (first architectural decision about crate structure)

---

## Notes

**Key Insight from Issue #3:**

The build script problem wasn't just a bug to fix - it was a code smell revealing architectural issues. When we needed a 3-tier workspace detection strategy to make the two-crate split work, that was the codebase telling us something was wrong.

**Why This Matters for Adoption:**

For playwright-rust to achieve "production-quality for broad adoption," it needs to:
1. Work reliably as a dependency (issue #3 broke this)
2. Match patterns familiar to Playwright users (single package)
3. Be simple to maintain (solo developer constraint)

The single-crate architecture achieves all three. The two-crate split achieves none.

**Future Considerations:**

- **Codegen tool:** Will be simple helper in main crate invoking Playwright CLI
- **Inspector:** Already works (web UI from server)
- **Trace viewer:** Already works (web UI from server)
- **Multiple API styles:** If ever needed (unlikely), Rust's feature flags handle this

None of these justify bringing back a two-crate split.

---

**Author:** Paul Adamson (with Claude Code assistance)

**Reviewers:** N/A (solo developer)

**Last Updated:** 2025-11-15
