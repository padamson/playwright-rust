# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**playwright-rust** is a Rust implementation of language bindings for Microsoft Playwright, following the same architecture as playwright-python, playwright-java, and playwright-dotnet.

### Vision and Design Philosophy

**Problem**: Rust developers need reliable, cross-browser testing, and Playwright is the modern standard for browser automation.

**Solution**: Provide production-quality Rust bindings for Microsoft Playwright that:
- Reuse Playwright's battle-tested server (don't reimplement browser protocols)
- Match Playwright's API across all languages (consistency)
- Leverage Rust's type safety and performance

**Key Principles:**
- **Microsoft-compatible architecture** - JSON-RPC to Playwright server (not direct protocol)
- **API consistency** - Match playwright-python/JS/Java exactly
- **Type safety** - Leverage Rust's type system for compile-time guarantees
- **Production quality** - Drive broad adoption
- **Testing-first** - Comprehensive test coverage from day one
- **Idiomatic Rust** - async/await, Result<T>, builder patterns

**Strategic Positioning:**
- **Not reinventing the wheel**: Uses official Playwright server for browser automation
- **Cross-language consistency**: Same API as Python/JS/Java/.NET implementations
- **Folio project driver**: Development driven by real-world need in [folio repo](https://github.com/padamson/folio) (browser testing for media ingestion tool)

### Technology Stack

**Primary Language: Rust**
- **Why Rust**: Type safety, performance, modern async/await, great for developer tools
- **Async runtime**: tokio (de facto standard for async Rust)
- **Serialization**: serde + serde_json for JSON-RPC protocol
- **Process management**: tokio::process for Playwright server lifecycle

### Project Structure

```
playwright-rust/
├── crates/                          # Rust workspace
│   ├── playwright/                  # High-level public API (like playwright-python)
│   │   ├── src/
│   │   │   ├── api/                # Public API modules
│   │   │   │   ├── browser.rs
│   │   │   │   ├── browser_context.rs
│   │   │   │   ├── page.rs
│   │   │   │   ├── locator.rs
│   │   │   │   └── assertions.rs
│   │   │   ├── lib.rs             # Public exports
│   │   │   └── playwright.rs      # Main entry point
│   │   └── Cargo.toml
│   ├── playwright-core/            # Protocol implementation (internal)
│   │   ├── src/
│   │   │   ├── connection.rs      # JSON-RPC client
│   │   │   ├── protocol/          # Protocol types
│   │   │   │   ├── generated.rs   # Auto-generated from Playwright protocol
│   │   │   │   └── mod.rs
│   │   │   ├── server.rs          # Playwright server management
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   └── playwright-codegen/         # Code generation from protocol (build-time)
│       └── Cargo.toml
├── drivers/                        # Playwright server binaries (gitignored)
├── examples/                       # Usage examples
│   ├── basic.rs                   # Simple example
│   ├── screenshots.rs             # Screenshot example
│   └── assertions.rs              # Testing example
├── tests/                         # Integration tests
│   ├── browser_test.rs
│   ├── page_test.rs
│   └── assertions_test.rs
├── docs/                          # Documentation
│   ├── architecture/              # Architecture docs
│   ├── adr/                       # Architecture Decision Records
│   └── templates/                 # Planning templates
├── scripts/                       # Helper scripts
└── README.md
```

## Development Approach

This project uses **test-driven development (TDD)** and **incremental delivery** with focus on Microsoft Playwright API compatibility.

### Specialized Development Agents

For complex workflows, use these specialized agents (located in `.claude/agents/`):

1. **TDD Feature Implementation Agent** (`tdd-feature-implementation.md`)
   - Use when: Implementing any new Playwright API feature
   - Automates: Red → Green → Refactor workflow, cross-browser testing, API compatibility checks
   - Ensures: Tests written first, API matches Playwright exactly, rustdoc complete

2. **Documentation Maintenance Agent** (`documentation-maintenance.md`)
   - Use when: Completing slices/phases, updating docs, releasing versions
   - Automates: Just-in-time doc updates, hierarchy enforcement, CHANGELOG generation
   - Ensures: README shows current features only, roadmap stays strategic, implementation plans stay current

3. **API Compatibility Validator Agent** (`api-compatibility-validator.md`)
   - Use when: Reviewing API implementations, validating compatibility
   - Automates: Cross-language API comparison, parameter validation, type checking
   - Ensures: API exactly matches playwright-python/JS/Java, no drift

4. **Release Preparation Agent** (`release-preparation.md`)
   - Use when: Preparing a version release (version bump, CHANGELOG, verification)
   - Automates: Pre-release checks, test verification, version management, validation
   - Ensures: All quality gates pass, CHANGELOG is complete, release process is smooth

#### Automatic Agent Invocation

**IMPORTANT**: Proactively use agents when user requests match these patterns:

**TDD Feature Implementation Agent** - Use automatically when user:
- Says "implement {feature}" or "add {method}"
- Mentions implementing a Playwright API (page.goto, browser.launch, etc.)
- Asks to "create a new feature" or "add support for"
- Example triggers: "implement page.screenshot()", "add browser.pdf() method"

**Documentation Maintenance Agent** - Use automatically when user:
- Says "Slice X complete", "Phase Y done", or "finished Slice Z"
- Asks to "update documentation" or "update docs"
- Mentions "release" or "preparing for release"
- Says "phase complete" or "slice finished"
- Example triggers: "Phase 5 complete", "Slice 6 done", "update docs for new features"

**API Compatibility Validator Agent** - Use automatically when user:
- Asks to "validate {API}" or "check {API} compatibility"
- Says "does {class} match Playwright?" or "is {method} compatible?"
- Asks to "review API implementation"
- Mentions "API compatibility" or "cross-language consistency"
- Example triggers: "validate Page API", "check if Locator matches Playwright"

**Release Preparation Agent** - Use automatically when user:
- Says "prepare release" or "release v{X.Y.Z}"
- Asks to "publish to crates.io" or "publish version {X.Y.Z}"
- Says "ready to release" or "let's release"
- Mentions "version bump" in context of releasing
- Example triggers: "prepare release for v0.6.0", "publish v0.6.0 to crates.io", "ready to release"

**Don't use agents for**:
- Simple single-file edits
- Reading files or searching code
- Running a single test
- Quick bug fixes (< 10 lines)
- Formatting or clippy fixes

**General rule**: If the task requires 3+ steps or involves checking multiple sources (Playwright docs, playwright-python, tests), proactively use the appropriate agent.

### Planning and Documentation Structure

**Documentation Hierarchy** (Just-In-Time Philosophy):

1. **README.md** - Project landing page for GitHub visitors
   - **Purpose**: First impression, quick start
   - **Content**: Vision, working example (current code ONLY), what works now, installation
   - **Update**: When phase completes or installation changes
   - **Keep brief**: < 250 lines, no future API previews, link to roadmap for details

2. **docs/roadmap.md** - Strategic vision and timeline
   - **Purpose**: Long-term direction, milestone planning
   - **Content**: High-level phase overview (1 paragraph each), milestones, API preview for FUTURE phases only
   - **Update**: When phase completes (mark complete), when planning new phase (add high-level scope)
   - **Keep strategic**: No slice details, no file lists, focus on what's coming

3. **docs/implementation-plans/phaseN-*.md** - Detailed work tracking
   - **Purpose**: Day-to-day development, historical record
   - **Content**: Slice-by-slice tasks, files modified, tests added, implementation gotchas, technical decisions
   - **Create**: Just-in-time when starting the phase (not before)
   - **Update**: Daily during active development
   - **After complete**: Becomes historical reference, rarely updated

4. **Architecture Decision Records** (`docs/adr/####-*.md`)
   - Document significant architectural decisions
   - Compare options with trade-off analysis
   - Record rationale for Playwright compatibility choices
   - Use template: `docs/templates/TEMPLATE_ADR.md`

5. **API Documentation** (Rust docs)
   - Every public API has rustdoc with examples
   - Match Playwright's documentation style
   - Include links to official Playwright docs

**Just-In-Time Principles:**
- ✅ README shows only what works TODAY
- ✅ Roadmap shows high-level vision for next 1-2 phases
- ✅ Implementation plans created ONLY when starting that phase
- ❌ Don't duplicate slice lists across docs
- ❌ Don't create Phase 4 plan while working on Phase 2
- ❌ Don't show future APIs in README examples

### Working on Features

**IMPORTANT**: Always check Playwright's official API docs first.

**For implementing new features**, use the **TDD Feature Implementation Agent** which automates:
- Research of Playwright API and playwright-python reference
- Writing failing tests first (Red → Green → Refactor)
- Protocol layer and high-level API implementation
- Cross-browser testing (Chromium, Firefox, WebKit)
- Rustdoc documentation with examples

**For simple tasks** (bug fixes, refactoring), work directly:
1. Check implementation plans in `docs/implementation-plans/`
2. Follow TDD workflow: Write test → Implement → Refactor
3. Run tests: `cargo test`
4. Ensure clippy clean: `cargo clippy -- -D warnings`

## Documentation

### Documentation Philosophy

- **Rust docs for implementation** - rustdoc with examples
- **Markdown for architecture** - ADRs, design decisions
- **Link to Playwright docs** - Don't duplicate, reference official docs
- **Show Rust-specific patterns** - Where we diverge for idiomatic reasons
- **Just-in-time updates** - Use **Documentation Maintenance Agent** for coordinated doc updates

### API Documentation Standards

Every public API must have:
- Summary (what it does)
- Example usage
- Link to Playwright docs (e.g., `// See: https://playwright.dev/docs/api/class-page#page-goto`)
- Errors section (what can fail)
- Notes on Rust-specific behavior if any

Example (for module-level doctests, not individual functions):
```rust
//! # Example
//!
//! ```ignore
//! use playwright_rs::Playwright;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!     let browser = playwright.chromium().launch().await?;
//!     let page = browser.new_page().await?;
//!
//!     // Demonstrate goto - navigates to the specified URL
//!     page.goto("https://example.com", None).await?;
//!
//!     // Returns error if:
//!     // - URL is invalid
//!     // - Navigation timeout (default 30s)
//!     // - Network error
//!     //
//!     // See: <https://playwright.dev/docs/api/class-page#page-goto>
//!
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
```

**Note**: Individual function examples are discouraged. Use module-level doctests that demonstrate multiple related APIs together.

## Documentation Testing Strategy

### Philosophy: Executable Documentation

We use a **module-level doctest approach** that ensures documentation stays synchronized with implementation:

1. **All doctests use `ignore` annotation** - They compile and run, but only when explicitly requested
2. **Module-level consolidation** - One comprehensive doctest per file for efficiency
3. **Manually runnable** - Doctests can be executed to verify they match actual implementation
4. **CI verification** - Full execution in GitHub Actions with `--ignored` flag
5. **Pre-commit compilation** - Fast compile-only checks during local development

### Why This Approach?

**Playwright-rust has unique requirements:**
- External Playwright server needed for execution
- Cross-browser testing takes time
- Documentation must reflect real, working examples
- Risk of documentation drift from implementation

**Our strategy**:
- `ignore` annotation allows selective execution without slowing normal development
- Module-level doctests are comprehensive integration tests
- CI runs full execution to catch drift
- Pre-commit hooks ensure doctests compile

### Doctest Structure

**All doctests use `ignore` and are placed at module level:**

```rust
//! Page protocol object
//!
//! Represents a web page within a browser context.
//!
//! # Example
//!
//! ```ignore
//! use playwright_core::protocol::Playwright;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!     let browser = playwright.chromium().launch().await?;
//!     let page = browser.new_page().await?;
//!
//!     // Demonstrate multiple Page APIs in one comprehensive example
//!     page.goto("https://example.com", None).await?;
//!     let title = page.title().await?;
//!     let screenshot = page.screenshot(None).await?;
//!
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```

use crate::protocol::*;

pub struct Page {
    // implementation...
}
```

**Key principles:**
- **One doctest per module** - Consolidate all examples for that module
- **Comprehensive coverage** - Demonstrate multiple APIs in realistic scenarios
- **Always use `ignore`** - Never `no_run` or no annotation
- **Full async context** - Include `#[tokio::main]` and proper error handling

### Running Doc-Tests

```bash
# Compile doctests only (what pre-commit runs)
cargo test --doc --workspace

# Execute all ignored doctests (what CI runs)
cargo test --doc --workspace -- --ignored

# Execute specific module's doctest
cargo test --doc -p playwright-core assertions -- --ignored

# Execute specific file's doctest
cargo test --doc --package playwright-core 'protocol::page::Page' -- --ignored
```

### CI/Pre-commit Integration

**GitHub Actions CI:**
```yaml
- name: Run doctests
  run: cargo test --doc --workspace -- --ignored
```

**Pre-commit hooks:**
```yaml
- name: Compile doctests
  run: cargo test --doc --workspace
```

### Best Practices for Contributors

1. **Module-level doctests only** - No individual function examples (prevents fragmentation)
2. **Always use `ignore` annotation** - Enables manual execution without slowing development
3. **Comprehensive scenarios** - Show multiple related APIs working together
4. **Real-world usage** - Examples should reflect actual use cases
5. **Test manually before pushing** - Run `cargo test --doc -p <crate> -- --ignored` to verify

### Why `ignore` Instead of `no_run`?

- **`no_run`**: Compiles but never executes → Documentation can drift from reality
- **`ignore`**: Can be executed on-demand → Guarantees documentation matches implementation
- **Manual verification**: Developers can run `--ignored` to validate examples
- **CI enforcement**: Automated execution catches drift before merge

## Versioning and Release Strategy

### Semantic Versioning

- **0.x.y** - Pre-1.0, API may change (current phase)
- **1.0.0** - Stable API, ready for production
- **2.0.0+** - Major version aligns with Playwright version if possible

### Release Milestones

- **v0.1.0** - Basic browser launch, page navigation, locators (folio needs)
- **v0.2.0** - Actions, assertions, screenshots
- **v0.3.0** - Network interception, advanced features
- **v0.4.0** - Feature parity with playwright-python basics
- **v0.5.0** - Production hardening, documentation
- **v1.0.0** - Stable release

### Publishing to crates.io

**Incremental publishing:**
1. Publish `playwright-core` v0.1.0 (internal crate)
2. Publish `playwright` v0.1.0 (public API)
3. Iterate with minor versions for new features
4. Breaking changes increment to 0.x+1.0

## Testing Strategy

### Test Levels

1. **Unit Tests** (`playwright-core`)
   - Protocol serialization/deserialization
   - Connection management
   - Server lifecycle

2. **Integration Tests** (`playwright` crate)
   - End-to-end API usage
   - Cross-browser compatibility
   - Error handling

3. **Example Tests**
   - All examples should be runnable tests
   - Verify documentation code works

### Test Data

- Use Playwright's test server examples
- Minimal external dependencies
- Fast, deterministic tests

### Continuous Integration

- Run on Linux, macOS, Windows
- Test with Chromium, Firefox, and WebKit
- Run clippy, fmt, tests
- Check documentation

## Playwright Server Management

### Build-time Download

```rust
// build.rs in playwright-core
// Download Playwright server on first build

fn main() {
    let drivers_dir = Path::new("../../drivers");

    if !drivers_dir.exists() {
        println!("cargo:warning=Downloading Playwright server...");

        // Use npm to install @playwright/test
        Command::new("npm")
            .args(&["install", "-g", "@playwright/test"])
            .status()
            .expect("Failed to install Playwright");

        // Playwright will be in node_modules or global npm
    }
}
```

### Runtime Launch

```rust
// Server lifecycle managed in playwright-core/src/server.rs

pub struct PlaywrightServer {
    process: Child,
    connection: Connection,
}

impl PlaywrightServer {
    pub async fn launch() -> Result<Self> {
        // 1. Find Playwright CLI
        // 2. Launch with `playwright run-server`
        // 3. Connect via stdio
        // 4. Return server handle
    }

    pub async fn shutdown(self) -> Result<()> {
        // Graceful shutdown
    }
}
```

## API Design Patterns

### Builder Pattern for Options

```rust
// Match Playwright's option pattern
browser.launch()
    .headless(true)
    .slow_mo(100)
    .args(vec!["--no-sandbox"])
    .await?;

page.goto("https://example.com")
    .timeout(Duration::from_secs(60))
    .wait_until(WaitUntil::NetworkIdle)
    .await?;
```

### Locators (Playwright Pattern)

```rust
// Playwright uses locators for auto-waiting
let button = page.locator("button.submit");

// Actions auto-wait for element
button.click().await?;

// Assertions auto-retry
expect(button).to_be_visible().await?;
```

### Error Handling

```rust
// Use Result<T, Error> consistently
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Playwright server not found")]
    ServerNotFound,

    #[error("Navigation timeout after {0:?}")]
    NavigationTimeout(Duration),

    #[error("Element not found: {0}")]
    ElementNotFound(String),

    // ... more variants
}
```

## Contribution Guidelines

### Code Quality

- Follow Rust conventions (rustfmt, clippy)
- Write tests for all features
- Document public APIs
- No unsafe code unless justified with SAFETY comment

### Compatibility

- Match Playwright API exactly
- Don't add Rust-specific features (stay compatible)
- Use idiomatic Rust patterns where possible
- Document differences from other languages
- **Use API Compatibility Validator Agent** to verify cross-language consistency

### Pull Requests

- Small, focused changes
- Include tests
- Update documentation
- Pass CI checks

## Path to Broad Adoption

**Criteria for proposal:**
1. ✅ Follow Playwright architecture (JSON-RPC to server)
2. ⬜ API parity with playwright-python (core features)
3. ⬜ Comprehensive test suite
4. ⬜ Production usage by 3+ projects
5. ⬜ 100+ GitHub stars
6. ⬜ 5-10 active contributors
7. ⬜ Maintained for 6+ months
8. ⬜ Apache-2.0 license
9. ⬜ Good documentation

## Useful References

- **Playwright Docs**: https://playwright.dev/docs/api
- **playwright-python**: https://github.com/microsoft/playwright-python
- **Playwright Protocol**: https://github.com/microsoft/playwright/tree/main/packages/playwright-core/src/server
- **Folio Project**: Example usage driver (browser testing for media tool)

## Development Commands

```bash
# Build
cargo build

# Test (recommended - uses cargo-nextest)
cargo nextest run

# Test specific crate
cargo nextest run -p playwright-core

# Test (standard cargo, fallback)
cargo test

# Doc-tests (nextest doesn't run these)
# See "Documentation Testing Strategy" section for details
cargo test --doc

# Doc-tests for specific crate
cargo test --doc -p playwright-core

# Run example
cargo run --example basic

# Run all examples
for example in crates/playwright/examples/*.rs; do
    cargo run --package playwright --example $(basename "$example" .rs) || exit 1
done

# Check formatting
cargo fmt -- --check

# Run clippy
cargo clippy -- -D warnings

# Generate docs
cargo doc --open

# Run CI locally
pre-commit run --all-files
```

**Note:** Install cargo-nextest once globally: `cargo install cargo-nextest`
