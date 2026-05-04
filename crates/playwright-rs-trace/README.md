# playwright-rs-trace

Programmatic parser for [Playwright][pw] trace zip files (format v8,
matching Playwright 1.59.x).

The Playwright JS ecosystem ships a trace-viewer UI but no documented
parsing API. This crate fills that gap for Rust:

```rust,ignore
use playwright_rs_trace::{open, TraceEvent};

let mut reader = open("trace.zip")?;
println!("trace v{} from {}", reader.context().version, reader.context().browser_name);

for action in reader.actions() {
    let action = action?;
    if action.error.is_some() {
        eprintln!("failed: {}.{} ({:?})", action.class, action.method, action.error);
    }
}
```

The reader is a **streaming iterator** — events / actions are yielded
lazily as the underlying zip stream is read, so a large trace doesn't
need to fit in memory before processing begins. This is the property
the future browser-hosted WASM trace viewer (issue [#82][issue-82])
needs to render partial traces.

## Forward compatibility

The parser is conservative about what it knows. Every event is preserved
losslessly via `TraceReader::raw_events()`; only the `events()` and
`actions()` paths attempt typed deserialization. Event kinds we don't
model (or known kinds whose payload schema changes in a future
Playwright minor version) surface as
[`TraceEvent::Unknown(RawEvent)`][unknown] — never silently dropped.

If you need to handle a kind we haven't modelled yet, dispatch on
[`RawEvent::kind()`][raw-kind] and read [`as_value()`][raw-value] for
the full JSON.

## Status

Slice 1 of issue [#80][issue-80] — open zip, stream events, reassemble
actions. Slices 2+ (network entries, snapshot indexing, resource
loading, action tree, query helpers, WASM compatibility, console +
screencast) land iteratively as the slice 1 ergonomics settle.

## Test fixtures

The integration tests parse a checked-in `tests/fixtures/basic.trace.zip`.
Regenerate when the trace format or fixture content changes:

```bash
cargo xtask regenerate-trace-fixture
```

See [`tests/fixtures/README.md`](tests/fixtures/README.md) for details.

## License

Apache-2.0. See [LICENSE](../../LICENSE) (workspace root).

[pw]: https://playwright.dev/
[issue-80]: https://github.com/padamson/playwright-rust/issues/80
[issue-82]: https://github.com/padamson/playwright-rust/issues/82
[unknown]: https://docs.rs/playwright-rs-trace/latest/playwright_rs_trace/enum.TraceEvent.html#variant.Unknown
[raw-kind]: https://docs.rs/playwright-rs-trace/latest/playwright_rs_trace/struct.RawEvent.html#method.kind
[raw-value]: https://docs.rs/playwright-rs-trace/latest/playwright_rs_trace/struct.RawEvent.html#method.as_value
