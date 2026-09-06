//! Parser tests against the checked-in `basic.trace.zip` fixture.
//!
//! Regenerate the fixture with:
//!
//!     cargo xtask regenerate-trace-fixture
//!
//! See `tests/fixtures/README.md` for details.

use playwright_rs_trace::{ActionPhase, NetworkEntry, TraceError, TraceEvent, TraceReader};
use std::io::Cursor;

const BASIC_FIXTURE: &[u8] = include_bytes!("fixtures/basic.trace.zip");

/// A trace from the last driver that wrote v8. Kept so the v8 paths stay
/// covered by a real recording once the bundled driver moved past them.
const BASIC_V8_FIXTURE: &[u8] = include_bytes!("fixtures/basic-v8.trace.zip");

/// Driver the fixture was recorded with. `cargo xtask verify-driver-version`
/// anchors this to `build.rs`'s `PLAYWRIGHT_VERSION`, so bumping the driver
/// without regenerating the fixture fails a gate instead of quietly leaving
/// the parser untested against the format it will actually meet.
const FIXTURE_DRIVER: &str = "playwright@1.63.0";

fn open_basic() -> TraceReader<Cursor<&'static [u8]>> {
    TraceReader::open(Cursor::new(BASIC_FIXTURE)).expect("open basic fixture")
}

#[test]
fn opens_basic_fixture_and_reads_context() {
    let reader = open_basic();
    let ctx = reader.context();
    assert_eq!(ctx.version, 9, "playwright@1.63.0 writes trace v9");
    assert_eq!(ctx.browser_name, "chromium");
    let expected_driver = FIXTURE_DRIVER
        .strip_prefix("playwright@")
        .expect("FIXTURE_DRIVER is written in playwright@<version> form");
    assert_eq!(
        ctx.playwright_version, expected_driver,
        "fixture was recorded with a different driver; \
         run `cargo xtask regenerate-trace-fixture`"
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
    let mut phases = Vec::new();
    let mut blobs = Vec::new();

    for ev in &events {
        match ev {
            TraceEvent::ContextOptions(_) => saw_context = true,
            TraceEvent::Before(_) => saw_before = true,
            TraceEvent::After(_) => saw_after = true,
            TraceEvent::Console(c) if c.text == "hi" => {
                assert_eq!(c.level, "log", "the console level is read from the trace");
                saw_console_hi = true;
            }
            TraceEvent::FrameSnapshot(f) => {
                assert!(f.snapshot_name.is_none(), "v9 names no snapshots");
                assert!(f.html.is_array(), "the DOM is the viewer's array encoding");
                phases.push(f.phase);
            }
            TraceEvent::ScreencastFrame(f) => blobs.push(f.file.clone()),
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
    assert!(
        phases.contains(&ActionPhase::Before) && phases.contains(&ActionPhase::After),
        "the goto and click each snapshot before and after: {phases:?}"
    );
    assert!(
        !blobs.is_empty(),
        "screenshots(true) recorded screencast frames"
    );

    // A blob reference is a path that opens in the archive, which is the
    // whole point of normalizing it.
    for file in &blobs {
        assert!(
            file.starts_with("screencast/"),
            "v9 keeps frames under screencast/: {file}"
        );
        let bytes = reader
            .blob(file)
            .unwrap_or_else(|e| panic!("blob {file} should open: {e}"));
        assert!(!bytes.is_empty(), "screencast frame {file} is empty");
    }
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

/// The v8 paths, against a real recording rather than synthetic lines: the
/// driver stopped writing this format, so nothing else would exercise them.
#[test]
fn the_v8_fixture_parses_with_names_and_derived_phases() {
    let mut reader = TraceReader::open(Cursor::new(BASIC_V8_FIXTURE)).expect("open the v8 fixture");
    assert_eq!(reader.context().version, 8);

    let events: Vec<_> = reader
        .events()
        .expect("events")
        .collect::<Result<_, _>>()
        .expect("typed events");

    let mut snapshots = Vec::new();
    let mut blobs = Vec::new();
    for ev in &events {
        match ev {
            TraceEvent::FrameSnapshot(f) => snapshots.push((
                f.snapshot_name.clone().expect("v8 names every snapshot"),
                f.phase,
            )),
            TraceEvent::ScreencastFrame(f) => blobs.push(f.file.clone()),
            _ => {}
        }
    }

    assert!(
        snapshots
            .iter()
            .any(|(name, phase)| name.starts_with("before@") && *phase == ActionPhase::Before),
        "the phase is read back from the v8 name: {snapshots:?}"
    );
    assert!(
        snapshots
            .iter()
            .any(|(name, phase)| name.starts_with("input@") && *phase == ActionPhase::Action),
        "an input snapshot is the action phase: {snapshots:?}"
    );

    // v8 wrote the entry name; normalized, it opens like any other blob.
    for file in &blobs {
        assert!(
            file.starts_with("resources/"),
            "v8 kept frames under resources/: {file}"
        );
        assert!(!reader.blob(file).expect("blob opens").is_empty());
    }

    // The v8-only action linkage still resolves.
    let mut reader = TraceReader::open(Cursor::new(BASIC_V8_FIXTURE)).expect("reopen");
    let actions: Vec<_> = reader
        .actions()
        .expect("actions")
        .collect::<Result<_, _>>()
        .expect("action stream");
    assert!(
        actions.iter().any(|a| a
            .before_snapshot
            .as_deref()
            .is_some_and(|n| n.starts_with("before@"))),
        "v8 actions name their snapshots"
    );
}

/// A `context-options` line declaring `version`, as the driver writes it.
fn context_line(version: u32) -> String {
    format!(
        r#"{{"type":"context-options","version":{version},"browserName":"chromium","playwrightVersion":"1.63.0"}}"#
    )
}

/// v8 wrote a blob's entry name, v9 writes its archive path; both resolve to the path.
#[test]
fn screencast_blob_resolves_to_its_archive_path_on_v8_and_v9() {
    let cases = [
        (
            8,
            r#"{"type":"screencast-frame","pageId":"p","sha1":"page@1-2.jpeg","width":2,"height":2,"timestamp":1.0}"#,
            "resources/page@1-2.jpeg",
        ),
        (
            9,
            r#"{"type":"screencast-frame","pageId":"p","file":"screencast/page@1-2.jpeg","width":2,"height":2,"timestamp":1.0}"#,
            "screencast/page@1-2.jpeg",
        ),
    ];

    for (version, frame, expected) in cases {
        let zip_bytes = build_synthetic_trace(&[&context_line(version), frame]);
        let mut reader =
            TraceReader::open(Cursor::new(zip_bytes)).expect("a supported version opens");
        assert_eq!(reader.context().version, version);

        let events: Vec<_> = reader
            .events()
            .expect("events")
            .collect::<Result<_, _>>()
            .expect("typed events");
        match &events[1] {
            TraceEvent::ScreencastFrame(f) => assert_eq!(f.file, expected, "trace v{version}"),
            other => panic!("expected a screencast frame, got {other:?}"),
        }
    }
}

#[test]
fn a_future_trace_version_is_refused() {
    let zip_bytes = build_synthetic_trace(&[&context_line(10)]);

    let Err(error) = TraceReader::open(Cursor::new(zip_bytes)) else {
        panic!("v10 is newer than this parser and must be refused");
    };
    assert!(
        matches!(&error, TraceError::UnsupportedVersion { found: 10, supported } if *supported == (8..=9)),
        "got {error:?}"
    );
}

#[test]
fn unknown_event_via_synthetic_zip() {
    // Forward-compat contract: events with a `type` we don't model
    // surface as `TraceEvent::Unknown` carrying the original payload,
    // never silently dropped. Build a minimal trace zip exercising
    // this without depending on the fixture content.
    let zip_bytes = build_synthetic_trace(&[
        &context_line(8),
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

///synthetic `trace.network` line parses into the typed
/// `NetworkEntry` shape. Exercises every modelled field at once,
/// including the HAR-extension fields (`_frameref`, `_monotonicTime`,
/// `_sha1`) and confirms the `raw_snapshot` carries the fields we
/// don't model individually (`cookies`, `timings`, `cache`, …).
#[test]
fn network_parses_synthetic_resource_snapshot() {
    // One JSONL line in `trace.network` — wraps the HAR entry in a
    // `{"type": "resource-snapshot", "snapshot": ...}` envelope.
    let line = r#"{"type":"resource-snapshot","snapshot":{
        "pageref":"page-guid-1",
        "_frameref":"frame-guid-1",
        "_monotonicTime":12345.6,
        "startedDateTime":"2026-05-04T12:00:00.000Z",
        "time":42.5,
        "request":{
            "method":"POST",
            "url":"http://127.0.0.1:5555/api",
            "httpVersion":"HTTP/1.1",
            "headers":[
                {"name":"content-type","value":"application/json"},
                {"name":"x-test","value":"first"},
                {"name":"x-test","value":"second"}
            ],
            "headersSize":64,
            "bodySize":11,
            "postData":{"_sha1":"req-body-hash"}
        },
        "response":{
            "status":200,
            "statusText":"OK",
            "httpVersion":"HTTP/1.1",
            "headers":[{"name":"content-type","value":"text/plain"}],
            "headersSize":32,
            "bodySize":5,
            "redirectURL":"",
            "content":{"size":5,"mimeType":"text/plain","_sha1":"resp-body-hash"}
        },
        "cookies":[],
        "timings":{"send":1.0,"wait":40.0,"receive":1.5},
        "cache":{}
    }}"#;
    // JsonLines tolerates whitespace, but trace.network is real JSONL —
    // collapse to a single line so the synthetic zip mirrors the wire.
    let line = compact_json(line);
    let zip_bytes = build_synthetic_network_zip(&[&line]);
    let mut reader = TraceReader::open(Cursor::new(zip_bytes)).expect("open synthetic trace");
    let entries: Vec<NetworkEntry> = reader
        .network()
        .expect("network()")
        .collect::<Result<_, _>>()
        .expect("entries parse");

    assert_eq!(entries.len(), 1);
    let e = &entries[0];

    assert_eq!(e.frame_ref.as_deref(), Some("frame-guid-1"));
    assert_eq!(e.page_ref.as_deref(), Some("page-guid-1"));
    assert_eq!(e.monotonic_time, Some(12345.6));
    assert_eq!(e.started_date_time, "2026-05-04T12:00:00.000Z");
    assert_eq!(e.time, Some(42.5));

    assert_eq!(e.request.method, "POST");
    assert_eq!(e.request.url, "http://127.0.0.1:5555/api");
    assert_eq!(e.request.http_version, "HTTP/1.1");
    assert_eq!(e.request.headers.len(), 3, "duplicate x-test preserved");
    assert_eq!(e.request.headers[1].name, "x-test");
    assert_eq!(e.request.headers[1].value, "first");
    assert_eq!(e.request.headers[2].value, "second");
    assert_eq!(e.request.headers_size, Some(64));
    assert_eq!(e.request.body_size, Some(11));
    assert_eq!(
        e.request.post_data.as_ref().map(|p| p.file.as_str()),
        Some("resources/req-body-hash"),
    );

    assert_eq!(e.response.status, Some(200));
    assert_eq!(e.response.status_text, "OK");
    assert_eq!(e.response.headers.len(), 1);
    assert_eq!(e.response.headers_size, Some(32));
    assert_eq!(e.response.body_size, Some(5));
    assert_eq!(e.response.redirect_url, None, "empty redirectURL → None");
    assert_eq!(e.response.content.size, Some(5));
    assert_eq!(e.response.content.mime_type, "text/plain");
    assert_eq!(
        e.response.content.file.as_deref(),
        Some("resources/resp-body-hash")
    );

    // raw_snapshot preserves fields we don't model.
    assert!(
        e.raw_snapshot.get("cookies").is_some(),
        "raw_snapshot must carry unmodelled HAR fields like `cookies`",
    );
    assert!(e.raw_snapshot.get("timings").is_some());
    assert!(e.raw_snapshot.get("cache").is_some());
}

///real-HTTP fixture parses end-to-end. The xtask
/// fixture-regen recipe stands up a localhost server, navigates to it,
/// and clicks a button — the click is recorded as before, and the
/// navigation produces at least one resource-snapshot in
/// `trace.network`. This test asserts the real-Playwright wire format
/// matches our types.
#[test]
fn fixture_with_real_http_request_parses() {
    let mut reader = open_basic();
    let entries: Vec<NetworkEntry> = reader
        .network()
        .expect("network()")
        .collect::<Result<_, _>>()
        .expect("entries parse");

    assert!(
        !entries.is_empty(),
        "regenerated fixture must record at least one HTTP request",
    );

    // Find the navigation request to the local server. The xtask
    // recipe uses `http://127.0.0.1:<port>/` as the entry URL.
    let nav = entries
        .iter()
        .find(|e| e.request.method == "GET" && e.request.url.starts_with("http://127.0.0.1:"))
        .expect("expected a GET to the local fixture server");

    assert_eq!(
        nav.response.status,
        Some(200),
        "fixture server should return 200 for the entry page",
    );
    assert!(
        !nav.request.headers.is_empty(),
        "request headers should be populated by the browser",
    );
    assert!(
        !nav.response.headers.is_empty(),
        "response headers should be populated by the server",
    );
    assert!(
        nav.frame_ref.is_some(),
        "Playwright records `_frameref` by default",
    );
    assert!(
        nav.monotonic_time.is_some(),
        "Playwright records `_monotonicTime` by default",
    );
}

///forward-compat error path: a `trace.network` line whose
/// `type` we don't recognise should yield `Err(...)` rather than
/// silently skipping. Slice-2 deliberately keeps this strict (no
/// "tolerant" mode); when Playwright adds a new network event kind,
/// we'd rather fail loudly than swallow it.
#[test]
fn network_unknown_event_type_yields_error() {
    let line = r#"{"type":"future-network-event","payload":{"x":1}}"#;
    let zip_bytes = build_synthetic_network_zip(&[line]);
    let mut reader = TraceReader::open(Cursor::new(zip_bytes)).expect("open synthetic trace");
    let results: Vec<_> = reader.network().expect("network()").collect();

    assert_eq!(results.len(), 1, "single line = single Result item");
    let err = results
        .into_iter()
        .next()
        .unwrap()
        .expect_err("unknown event type should produce an error, not be silently skipped");
    let msg = err.to_string();
    assert!(
        msg.contains("future-network-event") || msg.contains("resource-snapshot"),
        "error should name the unexpected kind or the expected one; got: {msg}",
    );
}

///sentinel mapping: HAR-spec `-1` for size / status / time
/// becomes `None` in the public API rather than leaking the magic
/// value to callers. Empty `redirectURL` → `None` similarly.
#[test]
fn network_minus_one_sentinels_become_none() {
    let line = r#"{"type":"resource-snapshot","snapshot":{
        "startedDateTime":"2026-05-04T12:00:00.000Z",
        "time":-1,
        "request":{
            "method":"GET",
            "url":"http://example.com/",
            "httpVersion":"HTTP/1.1",
            "headers":[],
            "headersSize":-1,
            "bodySize":-1
        },
        "response":{
            "status":-1,
            "statusText":"",
            "httpVersion":"HTTP/1.1",
            "headers":[],
            "headersSize":-1,
            "bodySize":-1,
            "redirectURL":"",
            "content":{"size":-1,"mimeType":"x-unknown"}
        }
    }}"#;
    let line = compact_json(line);
    let zip_bytes = build_synthetic_network_zip(&[&line]);
    let mut reader = TraceReader::open(Cursor::new(zip_bytes)).expect("open");
    let entry = reader
        .network()
        .expect("network()")
        .next()
        .expect("one entry")
        .expect("parses");

    assert_eq!(entry.time, None);
    assert_eq!(entry.request.headers_size, None);
    assert_eq!(entry.request.body_size, None);
    assert_eq!(entry.response.status, None);
    assert_eq!(entry.response.headers_size, None);
    assert_eq!(entry.response.body_size, None);
    assert_eq!(entry.response.redirect_url, None);
    assert_eq!(entry.response.content.size, None);
}

///optional fields absent: snapshots that don't carry
/// `_frameref` / `pageref` / `_monotonicTime` / `postData` /
/// `content._sha1` parse as `None` for those fields. Exercises the
/// `serde(default)` paths and the `Option<RequestPostData>` /
/// `Option<String>` shapes.
#[test]
fn network_absent_optional_fields_become_none() {
    let line = r#"{"type":"resource-snapshot","snapshot":{
        "startedDateTime":"2026-05-04T12:00:00.000Z",
        "time":1.0,
        "request":{
            "method":"GET",
            "url":"http://example.com/",
            "httpVersion":"HTTP/1.1",
            "headers":[],
            "headersSize":50,
            "bodySize":0
        },
        "response":{
            "status":204,
            "statusText":"No Content",
            "httpVersion":"HTTP/1.1",
            "headers":[],
            "headersSize":40,
            "bodySize":0,
            "redirectURL":"",
            "content":{"size":0,"mimeType":""}
        }
    }}"#;
    let line = compact_json(line);
    let zip_bytes = build_synthetic_network_zip(&[&line]);
    let mut reader = TraceReader::open(Cursor::new(zip_bytes)).expect("open");
    let entry = reader
        .network()
        .expect("network()")
        .next()
        .expect("one entry")
        .expect("parses");

    // _frameref, pageref, _monotonicTime: all absent in the JSON.
    assert_eq!(entry.frame_ref, None);
    assert_eq!(entry.page_ref, None);
    assert_eq!(entry.monotonic_time, None);

    // postData: absent in the JSON (GET request).
    assert!(entry.request.post_data.is_none());

    // content._sha1: absent (response with no body, e.g. 204).
    assert_eq!(entry.response.content.file, None);

    // status_text is not Optional — 204 still has "No Content".
    assert_eq!(entry.response.status, Some(204));
    assert_eq!(entry.response.status_text, "No Content");
}

///empty-stream path: a `trace.network` entry that exists
/// but contains no lines (typical for traces driven entirely against
/// `data:` URLs) should yield an iterator with zero items, not error.
#[test]
fn network_returns_empty_iterator_when_no_requests() {
    let zip_bytes = build_synthetic_network_zip(&[]);
    let mut reader = TraceReader::open(Cursor::new(zip_bytes)).expect("open synthetic trace");
    let entries: Vec<NetworkEntry> = reader
        .network()
        .expect("network() should succeed on an empty trace.network")
        .collect::<Result<_, _>>()
        .expect("no parse errors expected on empty input");
    assert_eq!(entries.len(), 0);
}

/// Compact a multi-line JSON literal to a single line preserving any
/// whitespace inside JSON string values. `split_whitespace().collect()`
/// would mangle internal strings (e.g. `"No Content"` → `"NoContent"`),
/// so route through serde_json's parser/serializer instead.
fn compact_json(s: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(s).expect("compact_json: invalid JSON input");
    v.to_string()
}

/// Build a minimal `.trace.zip` containing both a `trace.trace` and a
/// `trace.network` entry. Network lines populate `trace.network`; the
/// `trace.trace` entry carries only the required `context-options`
/// header so `TraceReader::open` succeeds.
fn build_synthetic_network_zip(network_lines: &[&str]) -> Vec<u8> {
    use std::io::Write as _;
    use zip::CompressionMethod;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut zip = ZipWriter::new(cursor);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("trace.trace", opts).expect("start trace");
        zip.write_all(format!("{}\n", context_line(8)).as_bytes())
            .expect("write trace");

        zip.start_file("trace.network", opts)
            .expect("start network");
        for line in network_lines {
            zip.write_all(line.as_bytes()).expect("write network line");
            zip.write_all(b"\n").expect("write newline");
        }

        zip.finish().expect("finish zip");
    }
    buf
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
