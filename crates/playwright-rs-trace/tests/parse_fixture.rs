//! Parser tests against the checked-in `basic.trace.zip` fixture.
//!
//! Regenerate the fixture with:
//!
//!     cargo xtask regenerate-trace-fixture
//!
//! See `tests/fixtures/README.md` for details.

use playwright_rs_trace::{TraceEvent, TraceReader};
use std::io::Cursor;

const BASIC_FIXTURE: &[u8] = include_bytes!("fixtures/basic.trace.zip");

fn open_basic() -> TraceReader<Cursor<&'static [u8]>> {
    TraceReader::open(Cursor::new(BASIC_FIXTURE)).expect("open basic fixture")
}

#[test]
fn opens_basic_fixture_and_reads_context() {
    let reader = open_basic();
    let ctx = reader.context();
    assert_eq!(ctx.version, 8, "trace v8 expected");
    assert_eq!(ctx.browser_name, "chromium");
    assert!(
        !ctx.playwright_version.is_empty(),
        "playwright_version should be populated"
    );
}

#[test]
fn raw_events_iterates_lossless() {
    let mut reader = open_basic();
    let raw_events: Vec<_> = reader
        .raw_events()
        .expect("raw_events stream")
        .collect::<Result<_, _>>()
        .expect("raw events");
    assert!(
        raw_events.len() >= 5,
        "expected several events from a non-empty trace, got {}",
        raw_events.len(),
    );

    // First event in `trace.trace` is always `context-options` for v8.
    assert_eq!(raw_events[0].kind(), Some("context-options"));

    // Every event preserves its `type` field.
    for ev in &raw_events {
        assert!(ev.kind().is_some(), "every raw event should carry a `type`",);
    }
}

#[test]
fn typed_events_includes_known_kinds() {
    let mut reader = open_basic();
    let events: Vec<_> = reader
        .events()
        .expect("events stream")
        .collect::<Result<_, _>>()
        .expect("typed events");

    let mut saw_context = false;
    let mut saw_before = false;
    let mut saw_after = false;
    let mut saw_console_hi = false;

    for ev in &events {
        match ev {
            TraceEvent::ContextOptions(_) => saw_context = true,
            TraceEvent::Before(_) => saw_before = true,
            TraceEvent::After(_) => saw_after = true,
            TraceEvent::Console(c) if c.text == "hi" => saw_console_hi = true,
            _ => {}
        }
    }

    assert!(saw_context, "expected ContextOptions");
    assert!(saw_before, "expected at least one Before event");
    assert!(saw_after, "expected at least one After event");
    assert!(
        saw_console_hi,
        "expected a Console event with text \"hi\" from the recorded onclick handler",
    );
}

#[test]
fn actions_reassemble_a_click() {
    let mut reader = open_basic();
    let actions: Vec<_> = reader
        .actions()
        .expect("actions stream")
        .collect::<Result<_, _>>()
        .expect("reassembled actions");

    assert!(!actions.is_empty(), "expected at least one action");

    // The fixture records a click on `#b`. Find it.
    let click = actions
        .iter()
        .find(|a| a.method == "click")
        .expect("expected a click action in the fixture");
    assert!(
        click.params.get("selector").is_some(),
        "click action's params should carry the selector",
    );
    assert!(
        click.end_time.is_some(),
        "click should have completed (end_time set)",
    );
    assert!(
        click.error.is_none(),
        "click should not have errored: {:?}",
        click.error,
    );
}

#[test]
fn unknown_event_via_synthetic_zip() {
    // Forward-compat contract: events with a `type` we don't model
    // surface as `TraceEvent::Unknown` carrying the original payload,
    // never silently dropped. Build a minimal trace zip exercising
    // this without depending on the fixture content.
    let zip_bytes = build_synthetic_trace(&[
        r#"{"type":"context-options","version":8,"browserName":"chromium","playwrightVersion":"1.59.1"}"#,
        r#"{"type":"future-thing-not-modelled","customField":42,"text":"hello"}"#,
    ]);

    let mut reader = TraceReader::open(Cursor::new(zip_bytes)).expect("open synthetic trace");
    let events: Vec<_> = reader
        .events()
        .expect("events")
        .collect::<Result<_, _>>()
        .expect("typed events");

    assert!(matches!(events[0], TraceEvent::ContextOptions(_)));
    match &events[1] {
        TraceEvent::Unknown(re) => {
            assert_eq!(re.kind(), Some("future-thing-not-modelled"));
            assert_eq!(
                re.as_value().get("customField").and_then(|v| v.as_i64()),
                Some(42),
            );
            assert_eq!(
                re.as_value().get("text").and_then(|v| v.as_str()),
                Some("hello"),
            );
        }
        other => panic!("expected Unknown for unmodelled kind, got {other:?}"),
    }
}

/// Build a minimal `.trace.zip` containing a single `trace.trace` entry
/// holding the given JSONL lines. Used for synthetic forward-compat
/// tests; production code reads zips produced by Playwright itself.
fn build_synthetic_trace(jsonl_lines: &[&str]) -> Vec<u8> {
    use std::io::Write as _;
    use zip::CompressionMethod;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("trace.trace", opts).expect("start file");
        for line in jsonl_lines {
            zip.write_all(line.as_bytes()).expect("write line");
            zip.write_all(b"\n").expect("write newline");
        }
        zip.finish().expect("finish zip");
    }
    buf
}
