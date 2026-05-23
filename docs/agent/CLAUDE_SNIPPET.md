# Playwright-rs — copy-paste CLAUDE.md snippet

Drop this section into your project's `CLAUDE.md` when you depend on
[`playwright-rs`](https://crates.io/crates/playwright-rs). It gives the
coding agent enough of the API model to write idiomatic test code
instead of guessing from generic Playwright knowledge.

Source of truth for the API itself stays on
[docs.rs](https://docs.rs/playwright-rs) — this snippet is just the
"what to reach for, what to avoid" overlay.

For a richer, auto-discovered version of the same guidance, see the
companion [`playwright-rs-usage` skill](../../.claude/skills/playwright-rs-usage/).
Either approach works; pick whichever fits your repo's convention.

The snippet intentionally avoids verbatim code so it can't silently
drift from the crate API — the canonical, compile-checked examples
live in the crate-level rustdoc and the
[`examples/`](https://github.com/padamson/playwright-rust/tree/main/crates/playwright/examples)
directory.

---

## Snippet (copy from here down into your CLAUDE.md)

```markdown
## Playwright-rs

This project uses [playwright-rs](https://docs.rs/playwright-rs) for
browser automation / E2E testing. The crate-level rustdoc on docs.rs
is the canonical API tour; this section is the high-level conventions
overlay.

- **Object model:** `Playwright::launch()` → `BrowserType`
  (`.chromium()` / `.firefox()` / `.webkit()`) → `Browser` →
  `BrowserContext` → `Page` → `Locator`. Build locators with
  `page.locator(...)` or the semantic `get_by_*` helpers, then chain
  action / assertion methods.
- **Use the `locator!()` macro** for selector literals — compile-time
  validation catches typos, unbalanced brackets, and unknown engine
  prefixes. Fall back to `&str` only for selectors computed at runtime.
- **`expect()` assertions over manual polling.** They auto-retry within
  the default timeout — never sprinkle `tokio::time::sleep` between
  an action and a check.
- **Builders for options.** `goto`, `click`, `screenshot`, `fill`,
  `tracing().start`, etc. take an `Options` struct constructed with
  `..Default::default()`.
- **`Result<T>` and `async/await` on `tokio`** throughout. One error
  type: `playwright_rs::Error`.
- **No reimplemented browser protocols.** This crate is a thin
  JSON-RPC client to the upstream Playwright server; the API mirrors
  playwright-python / java / .NET semantics. When in doubt, the
  [upstream Playwright docs](https://playwright.dev/docs/api) are
  authoritative.

### Debugging failures

Wrap the test body in tracing → on failure, `playwright show-trace
trace.zip` opens a visual debugger. See
[`examples/trace_on_failure.rs`](https://github.com/padamson/playwright-rust/blob/main/crates/playwright/examples/trace_on_failure.rs)
for the canonical Rust pattern (Rust has no async `Drop`, so cleanup
is explicit).

For programmatic trace inspection (CI bots, agent feedback loops),
add [`playwright-rs-trace`](https://docs.rs/playwright-rs-trace) to
`[dev-dependencies]`.

### References

- Full API: <https://docs.rs/playwright-rs>
- Runnable examples: <https://github.com/padamson/playwright-rust/tree/main/crates/playwright/examples>
- Upstream Playwright docs: <https://playwright.dev/docs/api>
```
