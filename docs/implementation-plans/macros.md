# `playwright-rs-macros` Plan

**Last updated:** 2026-05-04
**Tracking issue:** [#81](https://github.com/padamson/playwright-rust/issues/81) (closed; planning lives here now)
**Crate:** [`crates/playwright-rs-macros/`](../../crates/playwright-rs-macros/)
**Targets:** independent of `playwright-rs` versions; first publish ships alongside v0.13.0 (see [#86](https://github.com/padamson/playwright-rust/issues/86))

## Goal

A proc-macro crate exposing compile-time-validated selector and locator
macros. JS / Python / Java / .NET cannot validate selector strings at
compile time — they have no compile time. **Rust's procedural macros
can.** This crate is the differentiating compile-time-only check that
catches typos and structural errors before `cargo build` finishes.

It's also a "demo well, ship cheap" feature: small implementation
effort, instantly grokkable by Rust users, easy to feature in marketing
material, and decoupled from the moving Playwright API surface.

## Scope

This document is the analysis-and-roadmap reference. Each "land this"
item below corresponds to a sub-issue (filed when the item is the
next-up work, not all upfront — keeps GitHub-issue noise low).

The crate is consumed by `playwright-rs` as a default-on `macros`
feature. Users see `playwright_rs::locator!` rather than depending on
the macros crate directly.

## Why a separate crate

Procedural macros must live in a `proc-macro = true` crate — Rust's
compilation model requires it. The crate is therefore unavoidable,
not optional. We layer it under a feature flag so consumers who don't
want the proc-macro toolchain dependency can opt out.

## Slice tracking

Slice 1 shipped in commit
[42303ec](https://github.com/padamson/playwright-rust/commit/42303ec).
Subsequent slices are filed as sub-issues of [#80](https://github.com/padamson/playwright-rust/issues/80)-style umbrella **in this crate's own tracking issue when it next has work.**
Until then, the items below are the unfiled roadmap.

### Slice 1 — `locator!()` baseline ✅

**Status:** shipped in 42303ec.

What ships:

- `locator!("…")` proc-macro: validates a Playwright selector string at
  compile time, expands to the validated `&'static str` so
  `page.locator(locator!("#submit"))` is identical to
  `page.locator("#submit")` at runtime.
- Validation rules (conservative — easy to widen later, hard to
  narrow):
  1. Reject empty / whitespace-only.
  2. Reject unbalanced `[]`, `()`, `{}` (with quote-aware skipping so
     brackets inside attribute values like `[aria-label='go [back]']`
     don't break the balance check).
  3. Reject unknown engine prefixes — `css=`, `xpath=`, `text=`,
     `role=`, `id=`, `data-testid=`, `nth=`, and the `internal:*=`
     namespace are recognised.
- 7 happy-path unit tests + 5 [`trybuild`](https://docs.rs/trybuild)
  compile-fail fixtures pinning both diagnostic messages and
  source-span highlighting (see
  [`tests/ui/README.md`](../../crates/playwright-rs-macros/tests/ui/README.md)).
- Default-on `macros` feature in `playwright-rs`; opt out with
  `default-features = false`.
- Workflow / release plumbing: workspace member, vet exemptions for
  `trybuild` and its transitive deps, per-crate CHANGELOG, dependency-
  ordered publish in `release.yml`.

### Slice 2 — ARIA-role enum check (sketched)

The crate already accepts `role=button` because the `role=` engine
prefix is recognised, but the role *name* is opaque. `playwright-rs`
already exposes `AriaRole` (the enumeration of valid ARIA roles); the
macro can validate the name at compile time against that enum.

Public-API-affecting? No — the macro returns the same string. The
diagnostic improves: `locator!("role=spaceship")` (not a real ARIA
role) becomes a compile-error pointing at the unrecognised role
name, listing the valid alternatives.

Open questions:

- Crate dep graph: `playwright-rs-macros` depending on `playwright-rs`
  for `AriaRole` is a cycle (`playwright-rs` already deps on macros).
  Resolution: extract the role list into a tiny `playwright-rs-aria`
  data crate, OR generate the role list at macros build time from a
  shared text file.
- Should `role=button[name="..."]` syntax be parsed too? Playwright
  supports it; the body of the role= prefix has its own grammar.

### Slice 3 — full CSS grammar (sketched)

Replace the bracket-balance heuristic with a real CSS selector parser
(via `cssparser` or `selectors`). Catches:

- Invalid pseudo-class spellings (`:hovr` → error).
- Malformed attribute selectors (`[ ]`, `[x=]`, `[x="`).
- Combinator typos (`>>>`, `++`).

Trade-off: pulls a heavy dep (`cssparser` is the Servo selector
parser). Gate behind a feature flag (`strict-css`) so users who don't
want the build-time cost can stay on the lightweight balance check.

### Slice 4 — `>>` chain validation (sketched)

Playwright lets users chain selectors with `>>`. Each side can be a
different engine: `div.container >> text=Submit`. The macro currently
treats the chain as one big string; it should split on `>>` and
validate each segment individually with the appropriate engine
prefix.

### Slice 5 — `get_by_role!()` stretch (sketched)

Macro form of `page.get_by_role(role, options)` that takes the role
plus name + options as macro arguments and emits the runtime call.
Compile-time-checks the role name (slice 2) and the option keys.
Useful for codegen scenarios where the role is a literal.

## Out of scope

- **Type-state lifecycle macros** (encoding `Page<Loaded>`, etc.). v2.0-
  territory API rework; not a feature flag.
- **Code generation from existing Page DOM** (recorder mode). That's
  the `playwright-rs-cli` story ([#77](https://github.com/padamson/playwright-rust/issues/77)).
- **Test-id transformations**. Playwright's `getByTestId` already
  handles the `data-testid` attribute name; the macro just needs to
  recognise the engine prefix.

## Coupling

- `playwright-rs-macros` is consumed by `playwright-rs` (gated on the
  `macros` feature, default-on). Users see `playwright_rs::locator!`,
  not the underlying crate.
- Independent versioning: macro bumps don't force a `playwright-rs`
  bump as long as the macro's public API surface is compatible.
- The crate is intentionally **runtime-zero-cost**: every macro should
  emit code identical to the hand-written runtime form, with the
  validation a one-time compile-time check. Don't add macros that
  introduce wrappers, type indirections, or runtime overhead — that
  belongs in `playwright-rs` proper.

## Critical files

- [`crates/playwright-rs-macros/src/lib.rs`](../../crates/playwright-rs-macros/src/lib.rs) — proc-macro definitions
- [`crates/playwright-rs-macros/Cargo.toml`](../../crates/playwright-rs-macros/Cargo.toml) — `proc-macro = true`, deps
- [`crates/playwright-rs-macros/tests/ui/`](../../crates/playwright-rs-macros/tests/ui/) — trybuild compile-fail fixtures (see the directory's `README.md` for the workflow)
- [`crates/playwright/Cargo.toml`](../../crates/playwright/Cargo.toml) — `macros` feature gate + dep declaration
- [`crates/playwright/src/lib.rs`](../../crates/playwright/src/lib.rs) — re-export under `#[cfg(feature = "macros")]`
- [`crates/playwright-rs-macros/CHANGELOG.md`](../../crates/playwright-rs-macros/CHANGELOG.md) — per-crate release notes
