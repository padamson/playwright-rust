# Changelog — playwright-rs-trace

All notable changes to this crate are documented here. The crate is
versioned **independently** of `playwright-rs` so the parser can evolve
at its own pace.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Slice 1 of [#80](https://github.com/padamson/playwright-rust/issues/80) — `TraceReader`, streaming events, action reassembly.**
  - `TraceReader::open(reader)` opens a Playwright trace zip and parses
    the `context-options` event eagerly so callers can read
    `reader.context()` before iterating.
  - `TraceReader::raw_events()` — lossless streaming iterator over every
    JSONL line in `trace.trace`, yielding `RawEvent` (the full JSON
    object). Forward-compatibility escape hatch for callers who want to
    dispatch on event kinds the parser doesn't model yet.
  - `TraceReader::events()` — typed streaming iterator yielding
    `TraceEvent` (enum tagged on `type`). Known kinds become typed
    variants; anything else (including kinds whose schema we fail to
    deserialize, e.g. a future field addition) surfaces as
    `TraceEvent::Unknown(RawEvent)` so nothing is silently dropped.
  - `TraceReader::actions()` — streaming iterator that reassembles
    `before` + optional `input` + zero-or-more `log` + `after` events
    into a logical `Action`. Truncated actions (no matching `after`) are
    emitted at end-of-stream rather than discarded — useful for
    crashed-mid-action diagnostics.
  - Free function `playwright_rs_trace::open(path)` for the common
    file-on-disk case.

  Workflow / release plumbing parallels `playwright-rs-macros`:
  workspace member, independent versioning, per-crate CHANGELOG, vet
  exemptions for new transitives. Test fixtures regenerated via
  `cargo xtask regenerate-trace-fixture` (see
  [`tests/fixtures/README.md`](tests/fixtures/README.md)).
