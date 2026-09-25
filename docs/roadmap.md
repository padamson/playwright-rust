# Roadmap

This page records where `playwright-rs` is going and what has to be true
before 1.0.0 ships.

## Where things stand

Full playwright-python API parity was reached in 0.12.0, and every driver
bump since has closed the new upstream surface within the same release
cycle. The crate is pre-1.0 and in use by several downstream projects on
the release channel; their findings, not a feature list, are what drive
the remaining work.

## What 1.0.0 means

At 1.0.0 the public API freezes, and any breaking change after that needs
a major version. Until then, renames and removals are still allowed and
are recorded as BREAKING in the CHANGELOG.

Version 1.0.0 is cut when all of these hold:

1. **Driver currency.** The bundled driver is within one minor of the
   latest upstream release, the vendored protocol spec matches it, and the
   gap analysis reports the surface for that driver closed.
2. **A quiet release cycle.** Every downstream project is on the latest
   release with green CI, and one full cycle (bump, consumers adopt,
   findings answered) has passed with no change to the public API.
3. **Stability.** The flaky-test tracking issue is empty for a full cycle,
   and the two stress tests kept under `#[ignore]` for environmental
   variance are either stabilized or replaced.
4. **Documentation held by gates.** The README, the shipped skill, the
   landing-page snippets, and every rustdoc example are verified by the
   existing xtask and doctest checks, and a short migration note covers
   anyone still on 0.x.

New public surface before 1.0 comes from two places only: a driver bump
(parity with what upstream added) or a consumer finding. Nothing is added
on speculation.

## After 1.0

Driver bumps keep the same rhythm: each upstream minor maps to a minor
release of this crate, and a security advisory against a transitive
dependency gets a patch release. Candidates that are unscheduled and taken
only when a consumer asks for them: a sync API wrapper, test-runner
ergonomics (an attribute macro, a shared browser server for
process-per-test runners), and component testing for Rust web frameworks.

## Deliberately not planned

- **A native Rust driver replacing the Node server.** The driver is
  Playwright's server (actionability, auto-wait, selector engines, routing,
  tracing), not a wire protocol, and a Rust rewrite of it is a different
  product with a different parity story. The point of this crate is the
  official architecture, which gives feature parity with every upstream
  release for free; see [WHY.md](../WHY.md) and
  [ADR 0001](adr/0001-protocol-architecture.md).
- **Protocol code generation from the spec.** The official ports hand-write
  their APIs for ergonomics, and so does this one. Revisited only if
  tracking upstream releases becomes painful.

## Guiding principles

Match the official bindings' API and semantics; diverge only for idiomatic
Rust where compatibility allows. Every feature works on Chromium, Firefox
and WebKit. Tests come first, and documentation that cannot be verified is
documentation that will rot.

## Where to look for the rest

- Current version: the crates.io badge in the [README](../README.md).
- What shipped: each crate's CHANGELOG
  ([playwright-rs](../crates/playwright/CHANGELOG.md),
  [playwright-rs-macros](../crates/playwright-rs-macros/CHANGELOG.md),
  [playwright-rs-trace](../crates/playwright-rs-trace/CHANGELOG.md)).
- Coverage against upstream Playwright: the
  [gap analysis](implementation-plans/v1.0-gap-analysis.md).
- Why the architecture is what it is: [WHY.md](../WHY.md) and the
  [ADRs](adr/).
