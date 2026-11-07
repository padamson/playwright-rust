# Changelog — playwright-rs-macros

All notable changes to this crate are documented here. The crate is
versioned **independently** of `playwright-rs` so the proc-macro
implementation can evolve at its own pace; only the `playwright-rs`
dependency line in `crates/playwright/Cargo.toml` ties them together.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **MSRV is now Rust 1.95** (was 1.88), following the workspace, which moved for playwright-rs's parking_lot 0.13. Nothing in this crate needs it.

## [0.1.2] - 2026-10-11

### Added

- **`aria-ref=` is a known selector engine**, so `locator!("aria-ref=e2")` compiles. It is the engine `Page::get_by_ref` uses to find an element an `ai`-mode aria snapshot names, and the macro rejected it as unknown.

## [0.1.1] - 2026-08-02

### Changed

- **`syn` bumped to 3.0.** Build-time only: `syn` does not appear in this
  crate's public API, so the macro's behavior and diagnostics are
  unchanged. Publishing this lets dependents that have already moved to
  `syn` 3 build one copy instead of two.

## [0.1.0] - 2026-05-23

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

[Unreleased]: https://github.com/padamson/playwright-rust/compare/macros-v0.1.2...HEAD
[0.1.2]: https://github.com/padamson/playwright-rust/compare/macros-v0.1.1...macros-v0.1.2
[0.1.1]: https://github.com/padamson/playwright-rust/compare/macros-v0.1.0...macros-v0.1.1
[0.1.0]: https://github.com/padamson/playwright-rust/releases/tag/macros-v0.1.0
