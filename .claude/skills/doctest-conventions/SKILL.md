---
name: doctest-conventions
description: Conventions for authoring rustdoc doctests in playwright-rust — the `no_run` annotation, module-level placement, hidden scaffolding lines, and how doctests are exercised in CI vs pre-commit.
metadata:
  internal: true
---

# Doctest Conventions

## Philosophy: documentation that cannot rot

Every rustdoc example is **compile-checked on every `cargo test --doc`**
(pre-commit and CI). Browser automation can't *execute* in a doctest
harness, so examples are compiled but not run — same contract as the
agent-doc snippets (`verify-agent-docs`) and landing-page snippets
(`verify-site-snippets`).

## The annotation: `no_run`

```text
no_run  — compiled every time, never executed.  ← use this
ignore  — never compiled, never executed; rustdoc only highlights it.
(none)  — compiled AND executed; only for pure, browser-free examples.
```

**Use ` ```no_run ` for anything that would launch a browser or talk to
the Playwright server.** Plain ` ```rust ` is fine for pure logic
(selector builders, option construction) that can really execute.

**Never use ` ```ignore `** except when the code genuinely cannot
compile in this crate — the only sanctioned case is the macros/trace
crates' module docs showing downstream `playwright-rs` usage, which
would need a cyclic dev-dependency to compile.

> **History.** Until 2026-06, the convention was `ignore` + a CI step
> running `cargo test --doc -- --ignored`, believed to execute them.
> It never did: rustdoc surfaces `ignore` blocks as empty always-pass
> stubs (83 "passed" in 0.00s). The examples had zero protection and
> ~10 had silently rotted. `no_run` is what the convention always
> intended.

## Structure

Prefer **one comprehensive module-level doctest** (`//!`) per protocol
file, demonstrating related APIs in a realistic scenario. Per-function
examples are allowed where they pull their weight — they must compile
like everything else.

Use **hidden scaffolding lines** (`# `-prefixed) so the rendered docs
show only the instructive lines while the compiler sees a complete
program:

````rust
/// # Example
///
/// ```no_run
/// # use playwright_rs::Playwright;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let pw = Playwright::launch().await?;
/// # let page = pw.chromium().launch().await?.new_page().await?;
/// page.goto("https://example.com", None).await?;
/// let title = page.title().await?;
/// # Ok(())
/// # }
/// ```
````

Module-level (`//!`) doctests use `//! # ` for hidden lines. A full
`#[tokio::main] async fn main()` is also fine when showing the whole
program *is* the point.

## Running doctests

```bash
# Compile-check all doctests (pre-commit AND CI run exactly this)
cargo test --doc --workspace

# Pure examples without no_run also execute in the same pass
```

## CI / pre-commit integration

Both run the same command — there is no separate execution lane:

```yaml
- name: Compile doc-tests
  run: cargo test --doc --workspace
```
