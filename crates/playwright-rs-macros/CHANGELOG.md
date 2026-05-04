# Changelog — playwright-rs-macros

All notable changes to this crate are documented here. The crate is
versioned **independently** of `playwright-rs` so the proc-macro
implementation can evolve at its own pace; only the `playwright-rs`
dependency line in `crates/playwright/Cargo.toml` ties them together.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **`locator!()` macro** — compile-time-validated Playwright selector.
  Takes a string literal, validates it at compile time (rejects empty
  or whitespace-only input, unbalanced `[]`/`()`/`{}`, and unknown
  engine prefixes — `css=`, `xpath=`, `text=`, `role=`, `id=`,
  `data-testid=`, `nth=`, and the `internal:*=` namespace are
  recognized), and expands to the validated `&'static str` so calls
  like `page.locator(locator!("#submit"))` carry zero runtime cost
  versus `page.locator("#submit")`. Brackets inside quoted attribute
  values (`[aria-label='go [back]']`) are correctly skipped during the
  balance check.

  Compile-fail tests under `tests/ui/` pin both the diagnostic message
  and source-span highlighting via [`trybuild`](https://docs.rs/trybuild)
  — see `tests/ui/README.md` for the workflow when extending coverage.

  Closes [#81](https://github.com/padamson/playwright-rust/issues/81).
