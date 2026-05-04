# playwright-rs-macros

Compile-time-validated selector macros for [playwright-rs].

The companion proc-macro crate. Most users get this transitively via
`playwright-rs` (the `macros` feature is on by default) and never depend
on it directly:

```rust,ignore
use playwright_rs::locator;

let locator = page.locator(locator!("#submit-button")).await;
```

The `locator!()` macro validates the selector string at compile time —
typos, unbalanced brackets, and unknown engine prefixes are caught
before `cargo build` finishes. At runtime it expands to the same
`&'static str` the validated literal already represents, so there is
no runtime cost over `page.locator("#submit-button")`.

See the [playwright-rs] crate for usage and the project README for the
broader story.

[playwright-rs]: https://crates.io/crates/playwright-rs
