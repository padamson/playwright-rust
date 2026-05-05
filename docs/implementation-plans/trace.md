# `playwright-rs-trace` Plan

**Last updated:** 2026-05-04
**Tracking issue:** [#80](https://github.com/padamson/playwright-rust/issues/80) (closed; planning lives here now)
**Crate:** [`crates/playwright-rs-trace/`](../../crates/playwright-rs-trace/)
**Targets:** independent of `playwright-rs` versions; first publish ships alongside v0.13.0 (see [#86](https://github.com/padamson/playwright-rust/issues/86))
**Downstream:** [#82](https://github.com/padamson/playwright-rust/issues/82) — WASM trace viewer

## Goal

A pure-Rust programmatic parser for Playwright trace zip files (format
v8, matching Playwright 1.59.x). **No language ecosystem ships such a
library today** — JS has the trace-viewer UI but no documented API; the
underlying `@playwright/trace-viewer` package isn't intended for
external consumption.

Shipping this crate gives every Rust CI tool, dashboard, agent learning
loop, and post-mortem analyzer a single-good-option for trace
introspection. It also lays the foundation for a future browser-hosted
WASM trace viewer ([#82](https://github.com/padamson/playwright-rust/issues/82)),
because the parser is `std + WASM`-friendly and depends on nothing
else from the workspace at runtime.

## Scope

The trace-format research is one pass — see "Trace format reference"
below. Each slice ships its own piece of the public API and is filed
as a sub-issue **when it's the next-up work**, not all upfront. Slices
may merge, split, or reorder as ergonomic issues surface — the only
load-bearing decision is the streaming-iterator commitment from slice
1, which the WASM viewer depends on.

## Trace format reference

A `.trace.zip` archive contains:

- `trace.trace` — JSONL stream of action / event chunks
- `trace.network` — JSONL stream of HAR-like resource-snapshot entries
- `resources/<sha1>` — binary payloads (response bodies, screencast
  JPEG frames, font / CSS overrides for snapshots)

`trace.trace` events (discriminated by a `type` field):

- `context-options` — once per context: `version` (8 currently),
  `browserName`, `playwrightVersion`, `platform`, `options`, `wallTime`,
  `monotonicTime`, `contextId`, `sdkLanguage`, `testIdAttributeName`
- `before` — action start: `callId`, `startTime`, `class`, `method`,
  `params`, `title`, `pageId`, `beforeSnapshot`, `stepId`, optional
  `parentId`
- `input` — optional input-coords / input-snapshot reference
- `log` — message + time, attached to a `callId`
- `after` — action end: `callId`, `endTime`, `result`, `error`,
  `afterSnapshot`, optional `point`
- `console` — browser console output: `type`, `text`, `args[]`,
  `location`, `time`, `pageId`
- `event` — system events (dialog, download, page open/close):
  `class`, `method`, `params`, `time`, `pageId?`
- `frame-snapshot` — DOM snapshot: `callId`, `snapshotName`, `pageId`,
  `frameId`, `frameUrl`, `doctype`, `html`, `viewport`, timestamps,
  `resourceOverrides[]`
- `screencast-frame` — `pageId`, `sha1`, `width`, `height`,
  `timestamp` (JPEG in `resources/`)

`trace.network` is exclusively `resource-snapshot` events (HAR-like):
`_frameref`, `_monotonicTime`, `request: {url, method, headers,
postData?: {_sha1}}`, `response: {status, headers, content: {mimeType,
_sha1}}`.

Notable mechanics:

- **Two-phase action recording.** `before` + (optional `input`) +
  (zero-or-more `log`) + `after`, all sharing `callId`. Reassembly
  produces a logical "Action".
- **Snapshots referenced by name.** `before@<callId>`,
  `input@<callId>`, `after@<callId>` are identifiers in events; no
  explicit index — must build a map.
- **SHA-1-deduped resources.** A font used in 10 snapshots is stored
  once.
- **Action tree via `parentId`.** Internal calls (`page.click` →
  `locator.click`) form a parent/child chain.
- **Format v8 stable across Playwright 1.59.x.**

The driver bundles the trace-viewer source under
`drivers/playwright-*/package/lib/server/trace/` — that's the
authoritative reference for the chunk schema and zip layout.

## Slice tracking

Slice status lives in this doc — no GitHub sub-issues. Each slice's
section flips from "active" to "shipped" with the landing commit hash
when complete.

### Slice 1 — `TraceReader::open`, JSONL streaming, action reassembly ✅

**Status:** shipped in commit [2345cf6](https://github.com/padamson/playwright-rust/commit/2345cf6).

Public API on `TraceReader<R: Read + Seek>`:

- `TraceReader::open(reader) -> Result<Self>` — opens a zip, parses
  the first `context-options` event eagerly so callers see metadata
  without consuming the rest of the stream.
- `TraceReader::context() -> &ContextOptions`.
- `TraceReader::raw_events()` — lossless `RawEvent` per JSONL line;
  the full JSON object is preserved so callers can implement their own
  dispatch on event kinds the parser doesn't model.
- `TraceReader::events()` — typed `TraceEvent` enum (variants for
  every modelled kind). Unknown kinds — and known kinds with payloads
  we fail to deserialize, e.g. a future Playwright field addition —
  surface as `TraceEvent::Unknown(RawEvent)` so nothing is silently
  dropped.
- `TraceReader::actions()` — reassembles `before` + optional `input`
  + zero-or-more `log` + `after` events into a logical `Action`.
  Emitted in `after`-arrival order (callers wanting strict
  chronological order should `.collect::<Vec<_>>().sort_by_key(…)`).
  Truncated actions emit at end-of-stream.
- Free function `playwright_rs_trace::open(path)` for the file-on-disk
  case.

Sister change: `xtask` workspace member (`publish = false`) for
fixture regeneration. `cargo xtask regenerate-trace-fixture` drives a
real Chromium session through `playwright-rs::Tracing` to refresh the
deterministic test fixture. Future build automation (release prep,
doc-gen, supply-chain wrappers) gets a natural home here.

### Slice 2 — `trace.network` parsing → `NetworkEntry` streaming iterator (next-up)

**Why:** trace files record HTTP traffic alongside actions, and any
post-mortem analysis of a Playwright run needs both. Without slice 2,
`playwright-rs-trace` can tell you what the user clicked but not what
the page fetched.

**What:** add `TraceReader::network()` returning a streaming iterator
of `NetworkEntry`. Each entry is a HAR-like resource snapshot
(request + response pair) read from the second JSONL stream in the
zip. The HAR shape is preserved verbatim — slice 6 layers query
helpers (failed-request filters, redirect-chain joins) on top.

**Wire format** (from driver source: `lib/server/trace/recorder/tracing.js`
+ `lib/server/har/harTracer.js`): each line is
`{"type": "resource-snapshot", "snapshot": <HAR entry>}`. The HAR
entry carries Playwright extensions: `_frameref`, `pageref`,
`_monotonicTime`, and `_sha1` references on `postData` and
`content` that the slice 3 resource loader resolves.

**Decisions pinned:**

1. **Single iterator, not raw + typed.** `trace.network` only has one
   event kind. Forward-compat comes from preserving the unmodelled
   HAR fields on the entry verbatim, not from a raw-vs-typed split.
2. **No `all_events_chronological()` helper.** Callers who want a
   merged time-ordered view can collect-and-sort themselves. Add a
   builtin merge in slice 6 if query helpers actually need it.
3. **Local-server fixture regeneration.** `data:` URLs don't generate
   network entries. Stand up a tiny localhost server in the xtask
   binary and have the page navigate to it. Hermetic, matches the
   `tests/integration/test_server.rs` pattern in `playwright-rs`.
4. **Missing `trace.network` is an error; empty `trace.network` is
   not.** Trace zips from `playwright-rs::Tracing` always include the
   entry; an absent file means a corrupt or non-Playwright zip.

**Tests:** parse-against-synthetic-zip for the HAR shape and
forward-compat error path; parse-against-real-fixture once the xtask
local-server regen lands.

**Out of scope:** resource loading (`_sha1` → bytes; slice 3),
filter helpers (slice 6), HAR / W3C conversion (per the umbrella's
out-of-scope list), redirect-chain joins (slice 6).

### Slice 3 — Resource loader (sketched)

`TraceReader::resource(sha1: &str) -> Result<Vec<u8>>`, reading from
`resources/<sha1>` lazily. Considered an LRU; defer until profiling
shows it matters. Pulls in nothing new — the existing `zip` dep covers
random-access reads.

Open question: signature `Result<Vec<u8>>` vs `Result<Bytes>`. `Bytes`
matches the `ScreencastFrame::data` precedent in the main crate but
adds a direct `bytes` dep. Decide when this slice is filed.

### Slice 4 — Snapshot indexing & retrieval (sketched)

Index `frame-snapshot` events by `(call_id, kind)` so
`Action::before_snapshot()` / `::after_snapshot()` return
`&FrameSnapshot`. Resource overrides resolve through slice 3.

### Slice 5 — Action tree reconstruction (sketched)

Post-process `parent_id` into a tree; add `TraceReader::roots() ->
Vec<&Action>` and `Action::children`. Orphans (parent missing) become
roots — document explicitly.

### Slice 6 — Query helpers (sketched)

`trace.find_clicks_on(selector)`, `trace.failures()`,
`trace.console_errors()`. Selector comparison starts as string
equality; deeper matching via `playwright-rs-macros` reuse later.

### Slice 7 — WASM compatibility pass (sketched)

Verify `wasm32-unknown-unknown` builds. Likely needs a `std` /
`wasm-bindgen` feature split, possibly a `getrandom` feature flip,
audit `zip` features for any `mio` pulls. The streaming-iterator
commitment from slice 1 already supports the partial-render need;
this slice ratifies it.

### Slice 8 — Console + screencast streams (sketched)

`TraceReader::console()` and `TraceReader::screencast_frames(page_id)`.
JPEG bytes via the slice 3 resource loader. No new deps.

## Out of scope

- **Trace generation.** Already handled by `playwright-rs::Tracing`.
- **UI rendering.** Lives in the WASM viewer crate
  ([#82](https://github.com/padamson/playwright-rust/issues/82)).
- **HAR / W3C trace format conversion.** Separate concern; could be a
  third crate later.
- **Trace anonymisation / rewriting.** A different consumer of the
  parser, not the parser itself.

## Coupling

- `playwright-rs-trace` is independent of the main `playwright-rs` at
  runtime. They share Cargo workspace membership, that's it.
- Independent versioning: trace-crate bumps don't force a
  `playwright-rs` bump.
- Streaming iterator from slice 1 is the only API decision the WASM
  viewer ([#82](https://github.com/padamson/playwright-rust/issues/82))
  depends on. Everything else can flex.

## Critical files

- [`crates/playwright-rs-trace/`](../../crates/playwright-rs-trace/) — the crate
- [`crates/playwright-rs-trace/CHANGELOG.md`](../../crates/playwright-rs-trace/CHANGELOG.md)
- [`crates/playwright-rs-trace/tests/fixtures/`](../../crates/playwright-rs-trace/tests/fixtures/) — `basic.trace.zip` + regenerate doc
- [`crates/xtask/src/main.rs`](../../crates/xtask/src/main.rs) — fixture regeneration entry point
- [`drivers/playwright-1.59.1-mac-arm64/package/lib/server/trace/`](../../drivers/) — authoritative format reference (read-only; no changes here)
