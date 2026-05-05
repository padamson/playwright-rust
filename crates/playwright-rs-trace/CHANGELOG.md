# Changelog — playwright-rs-trace

All notable changes to this crate are documented here. The crate is
versioned **independently** of `playwright-rs` so the parser can evolve
at its own pace.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
