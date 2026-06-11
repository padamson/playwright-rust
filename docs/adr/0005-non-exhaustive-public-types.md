# ADR 0005: `#[non_exhaustive]` on public option and data types

**Status:** Accepted

**Date:** 2026-06-10

**Related Documents:**
- v1.0 gap analysis: [v1.0-gap-analysis.md](../implementation-plans/v1.0-gap-analysis.md)
- Roadmap (1.0 after dogfooding): [roadmap.md](../roadmap.md)

---

## Context and Problem Statement

The crate tracks upstream Playwright, which adds option fields and enum
values in nearly every release. In the 1.60 cycle alone, upstream added
`description` to `get_by_role` options, `boxes` to ARIA-snapshot
options, `style` to highlight options, and `noDefaults` to
`connect_over_cdp` options.

Before this decision the public surface had ~93 structs with all-`pub`
fields and 26 exhaustively matchable enums, with only 2 uses of
`#[non_exhaustive]`. Under Rust's semver rules, adding a field to an
exhaustively constructible struct, or a variant to an exhaustively
matchable enum, is a breaking change. After a 1.0 release, every
upstream option addition would force a major version bump, or force the
crate to stop tracking upstream.

This had to be fixed before 1.0 freezes the contract, and ideally
inside the v0.14.0 pre-release window, which already carries one
breaking change.

## Decision

Apply `#[non_exhaustive]` to all public structs with `pub` fields and
all public enums in `protocol/`, `api/`, `assertions.rs`, and
`error.rs` (101 types), with two classes of exception:

1. **Geometrically closed value types stay exhaustive** because their
   field sets cannot grow and literal construction is the ergonomic
   point: `Position`, `ScreenshotClip`, `PdfMargin`, `ScreencastSize`,
   `Viewport`, `DeviceViewport`, `Geolocation`.
2. **`server/` is out of scope.** Its types are internal plumbing that
   happens to be `pub` (our own integration tests use it white-box).
   Whether `server` should remain public at 1.0 is an open question for
   a future ADR; marking its types `non_exhaustive` piecemeal would
   paper over that larger decision.

Construction moves to one of three supported patterns, in order of
preference:

- **Existing hand-written builders** (`ScreenshotOptions::builder()`,
  `ClickOptions::builder()`, ...) where they already exist.
- **Chainable consuming-`self` setters** added to the high-traffic
  builderless option structs (`GetByRoleOptions`, `FilterOptions`,
  `TracingStartOptions`, `Cookie::new(name, value)`,
  `ProxySettings::new(server)`, ...), following the precedent set by
  `ConnectOverCdpOptions`. Fields stay `pub`, so reading and mutating
  remain available.
- **`Default` + field mutation** as the universal fallback:
  `let mut o = X::default(); o.field = Some(v);`. `non_exhaustive`
  forbids literal construction but not field access.

Setters can be added to any remaining struct later; that is an
additive, non-breaking change.

## Considered Alternatives

- **Keep struct literals, rely on the `..Default::default()`
  convention.** Adding a field does not break callers who use
  functional-update syntax, but nothing stops downstream from writing
  exhaustive literals, and semver tooling (`cargo-semver-checks`)
  correctly flags every field addition as breaking. Convention without
  enforcement fails exactly when it matters.
- **Private fields behind builders everywhere.** Strongest
  encapsulation, but removes field *read* access (useful on returned
  data like cookies and coverage entries), and would have required
  hand-writing builders for ~60 structs at once. `non_exhaustive`
  achieves the same forward-compatibility with `pub` fields kept.
- **A builder-derive dependency (e.g. `bon`, `typed-builder`).** Less
  hand-written code, but adds a proc-macro dependency to a crate that
  deliberately trims its tree (see the reqwest-to-ureq change), and
  each new dependency carries cargo-vet burden.

## Consequences

- Downstream construction of option/data types must use builders,
  setters, or `Default` + mutation. The `..Default::default()` literal
  idiom no longer compiles for these types. This is a one-time breaking
  change shipped in v0.14.0 alongside the `Locator::highlight`
  signature change.
- Post-1.0, tracking upstream Playwright option/field additions becomes
  semver-minor, enforced by the compiler rather than by review
  discipline.
- Downstream `match` on public enums (including `Error`) requires a
  wildcard arm, matching std practice (`std::io::ErrorKind`).
- Our integration tests, examples, and doctests compile as separate
  crates, so they prove the migration story; they were all migrated as
  part of this change (~85 construction sites).
