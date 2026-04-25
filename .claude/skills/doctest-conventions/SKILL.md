---
name: doctest-conventions
description: Conventions for authoring rustdoc doctests in playwright-rust — module-level placement, the `ignore` annotation, comprehensive scenarios over per-function snippets, and how doctests are exercised in CI vs pre-commit.
---

# Doctest Conventions

## Philosophy: Executable Documentation

We use a **module-level doctest approach** that ensures documentation
stays synchronized with implementation:

1. **All doctests use `ignore` annotation** — they compile and run, but
   only when explicitly requested
2. **Module-level consolidation** — one comprehensive doctest per file
3. **Manually runnable** — doctests can be executed to verify they
   match actual implementation
4. **CI verification** — full execution in GitHub Actions with `--ignored`
5. **Pre-commit compilation** — fast compile-only checks during local development

## Why `ignore` instead of `no_run`?

- **`no_run`**: compiles but never executes → documentation can drift
- **`ignore`**: can be executed on demand → guarantees doc matches impl
- Manual verification: developers can run `--ignored` to validate
- CI enforcement: automated execution catches drift before merge

## Doctest Structure

All doctests use `ignore` and are placed at module level:

````rust
//! Page protocol object
//!
//! Represents a web page within a browser context.
//!
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
//!     // Demonstrate multiple Page APIs in one realistic scenario
//!     page.goto("https://example.com", None).await?;
//!     let title = page.title().await?;
//!     let _ = page.screenshot(None).await?;
//!
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```

use crate::protocol::*;

pub struct Page {
    // implementation...
}
````

### Key rules

- **One doctest per module** — consolidate examples; no per-function snippets
- **Comprehensive coverage** — demonstrate multiple related APIs together
- **Always `ignore`** — never `no_run` or unannotated
- **Full async context** — `#[tokio::main]` and proper error handling
- **Real-world usage** — examples should reflect actual use cases

## Running doctests

```bash
# Compile doctests only (what pre-commit runs)
cargo test --doc --workspace

# Execute all ignored doctests (what CI runs)
cargo test --doc --workspace -- --ignored

# Execute specific module's doctest
cargo test --doc -p playwright-rs assertions -- --ignored

# Execute a specific item's doctest
cargo test --doc --package playwright-rs 'protocol::page::Page' -- --ignored
```

## CI / pre-commit integration

GitHub Actions:
```yaml
- name: Run doctests
  run: cargo test --doc --workspace -- --ignored
```

Pre-commit:
```yaml
- name: Compile doctests
  run: cargo test --doc --workspace
```
