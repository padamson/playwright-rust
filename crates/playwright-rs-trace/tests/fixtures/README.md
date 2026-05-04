# Trace fixtures

The `*.trace.zip` files in this directory are deterministic Playwright
trace recordings that drive `playwright-rs-trace`'s parse tests. They
are checked into the repository so the test suite is hermetic — neither
the test job nor a `cargo nextest run` invocation needs to launch a
browser.

## Regenerate

When the trace format changes (Playwright minor version bump that
shifts the format from v8) or when fixture content needs updating,
regenerate via:

```bash
cargo xtask regenerate-trace-fixture
```

This drives a real Chromium session through `playwright-rs::Tracing`
and writes the resulting zip back to this directory. The xtask binary
is a sibling workspace member (`crates/xtask/`) — see its
[`src/main.rs`](../../../xtask/src/main.rs) for what's recorded.

Commit the regenerated fixture alongside whatever change required it.

## Why xtask, not a unit-test fixture builder

A regular `#[test]` that builds the fixture would force the trace
crate's test job to launch browsers in CI — a significant slowdown
and a portability hazard. Putting the regen step in `xtask` keeps the
trace-crate tests fast and offline; the developer regenerates locally
when needed.
