# Changelog — playwright-rs-trace

All notable changes to this crate are documented here. The crate is
versioned **independently** of `playwright-rs` so the parser can evolve
at its own pace.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **BREAKING: trace format v9 (Playwright 1.63) parses, and blob references are archive paths named `file`.** The next release is 0.2.0. v9 writes every blob reference as a whole path inside the archive (`resources/…`, `screencast/…`) where v8 wrote an entry name under `resources/`; the parser reads both formats and normalizes both spellings, so the `file` field on `ScreencastFrameEvent`, `ResourceOverride`, `RequestPostData`, and `ResponseContent` (formerly `sha1`) opens directly in the archive. Callers that prefixed `resources/` themselves should drop that step. `TraceError::UnsupportedVersion` now carries the `supported` range instead of a single `expected` version.
- **`TraceReader::blob()` opens a blob the trace refers to**, by the archive path an event carries in its `file` field, so reading a screencast frame or a response body no longer means opening the archive a second time.
- **`ConsoleEvent::level` now carries the console level.** It was read from the event's own `type` discriminator, so it was always empty; the driver writes it as `messageType`.
- **`ResourceOverride::reference` is a number**, as the trace writes it. A snapshot carrying one, which happens when a page mutates a linked stylesheet, previously failed to parse and surfaced as `Unknown`.
- **`TraceEvent::FrameSnapshot` now deserializes.** The event's payload is nested under a `snapshot` key and its DOM is an array, which the typed struct never matched, so every DOM snapshot surfaced as `Unknown`. `FrameSnapshotEvent` gained `phase` (`before`, `action`, `after`) and its `snapshot_name` is optional, since v9 no longer names snapshots. The phase is set on both formats, read back from the snapshot name on v8, so `call_id` plus `phase` links a snapshot to its action whatever the format; `Action::before_snapshot` and `after_snapshot` stay v8-only.

## [0.1.3] - 2026-08-17

### Changed

- **Trace fixture regenerated against the Playwright 1.62.1 driver, confirming trace format version 8 still applies.** The checked-in fixture was recorded with 1.61.1, so the parse tests were not exercising the format produced by the driver `playwright-rs` bundles on `main`. Version 8 holds across both, so `SUPPORTED_VERSION` is unchanged; the difference is that the suite now proves it instead of assuming it. A format bump would otherwise have surfaced as this crate rejecting every trace its companion crate records.

## [0.1.2] - 2026-08-02

### Fixed

- **The README quick-start example did not compile.** `TraceReader::actions()`
  returns a `Result`, and the example iterated it directly without `?`. Since
  the README is what crates.io renders, this was the first code a prospective
  user would copy. Both it and the crate-level example are now compiled by
  `cargo test --doc` (they were marked ` ```ignore `, which rustdoc never
  compiles), so neither can rot again.

## [0.1.1] - 2026-07-24

### Changed

- **Confirmed trace-format v8 compatibility with the Playwright 1.61 driver.**
  The checked-in fixture was regenerated with the 1.61.1 driver (previously a
  1.59.1 trace, predating two driver bumps); 1.61 still emits format version
  8, so the parser is unchanged. The crate docs state the verified driver
  line again, which had been reduced to a bare "trace format v8" while the
  claim was unverified.

## [0.1.0] - 2026-05-23

### Added

- **`TraceReader` — open a Playwright trace zip, stream events, reassemble actions.**
  - `TraceReader::open(reader)` parses the `context-options` event
    eagerly so callers can read `reader.context()` before iterating.
  - `TraceReader::raw_events()` — lossless iterator over every JSONL
    line in `trace.trace`, yielding `RawEvent` (the full JSON object).
    Forward-compat escape hatch for callers dispatching on event kinds
    the parser doesn't model yet.
  - `TraceReader::events()` — typed iterator yielding `TraceEvent`.
    Known kinds become typed variants; anything else surfaces as
    `TraceEvent::Unknown(RawEvent)` so nothing is silently dropped.
  - `TraceReader::actions()` — reassembles `before` + optional `input`
    + zero-or-more `log` + `after` events into a logical `Action`.
    Truncated actions are emitted at end-of-stream rather than
    discarded.
  - Free function `playwright_rs_trace::open(path)` for the
    file-on-disk case.

- **`TraceReader::network()` — `trace.network` parsing → `NetworkEntry` iterator.**
  - One entry per recorded HTTP request/response pair (HAR-like
    resource snapshot). Empty `trace.network` (typical for traces
    driven against `data:` URLs) yields zero items.
  - HAR `-1` "unknown" sentinels are mapped to `None` at parse time —
    the public types use `Option<u64>` / `Option<u16>` / `Option<f64>`
    on `time`, `headers_size`, `body_size`, `status`, `content.size`,
    so callers don't have to know the convention. Empty `redirectURL`
    likewise → `None`.
  - HAR fields not modelled individually (`cookies`, `timings`,
    `cache`, `queryString`, `_transferSize`, …) are preserved on
    `NetworkEntry::raw_snapshot: serde_json::Value`.
  - Unknown event kinds in `trace.network` yield an error rather than
    being silently skipped — the stream is single-purpose.

- **`xtask` workspace member with `regenerate-trace-fixture`
  subcommand.** Drives a real Chromium session through
  `playwright-rs::Tracing` — including a localhost `axum` server so
  the navigation produces a real `resource-snapshot` — to refresh
  the deterministic test fixture under `tests/fixtures/`. New
  `.cargo/config.toml` aliases `cargo xtask`.

[Unreleased]: https://github.com/padamson/playwright-rust/compare/trace-v0.1.3...HEAD
[0.1.3]: https://github.com/padamson/playwright-rust/compare/trace-v0.1.2...trace-v0.1.3
[0.1.2]: https://github.com/padamson/playwright-rust/compare/trace-v0.1.1...trace-v0.1.2
[0.1.1]: https://github.com/padamson/playwright-rust/compare/trace-v0.1.0...trace-v0.1.1
[0.1.0]: https://github.com/padamson/playwright-rust/releases/tag/trace-v0.1.0
