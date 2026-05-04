# `tests/ui/` — compile-fail fixtures for `playwright-rs-macros`

This folder holds **compile-fail tests** for the `locator!` proc-macro,
driven by the [`trybuild`](https://docs.rs/trybuild) crate. Each pair
of files (`<name>.rs` + `<name>.stderr`) asserts a single bad usage:

- `<name>.rs` — a Rust program that exercises one invalid call to
  `locator!(...)`. trybuild compiles this file and expects it to fail.
- `<name>.stderr` — the **exact compiler output** we expect, captured
  byte-for-byte. Diffs against the actual rustc stderr; mismatches
  fail the test.

The harness lives at [`../locator_compile_fail.rs`](../locator_compile_fail.rs).
It's a single `#[test]` function listing each `<name>.rs` to compile.
trybuild handles the spawning of rustc, capturing stderr, and diffing.

## Why pin the diagnostic this strictly

Two regression guards in one fixture:

1. **Message guard** — if a refactor accidentally degrades the error
   text (e.g. `"selector is empty or whitespace-only"` →
   `"invalid input"`), the diff fails the test.
2. **Span guard** — the `^^^^^` underline in `.stderr` is part of the
   diagnostic. If a refactor accidentally points the underline at the
   wrong token, the diff fails. This protects users from getting
   error messages that gesture at the wrong piece of code.

## Adding a new compile-fail case

1. Create `tests/ui/<name>.rs` — a Rust program that should fail to
   compile, exercising one specific bad usage.
2. Add `t.compile_fail("tests/ui/<name>.rs");` to the harness in
   [`../locator_compile_fail.rs`](../locator_compile_fail.rs).
3. Run `TRYBUILD=overwrite cargo nextest run -p playwright-rs-macros`.
   trybuild auto-generates `tests/ui/<name>.stderr` from the actual
   rustc output.
4. **Eyeball the generated `.stderr`** to make sure the diagnostic is
   the one you intended (right message, right span).
5. Commit both files together.

## Updating an existing fixture after changing the macro

Same regenerate workflow — `TRYBUILD=overwrite` rewrites every fixture
in this folder. Always review the diff before committing; an
unintentional message change is exactly what the fixtures are designed
to catch.

## Background

`trybuild` is the standard pattern for this in the Rust ecosystem.
Serde, tokio, futures, axum, and most other libraries with non-trivial
proc-macros use it the same way.
