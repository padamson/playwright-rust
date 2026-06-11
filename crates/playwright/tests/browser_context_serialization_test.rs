use playwright_rs::protocol::{BrowserContextOptions, RecordHar, RecordVideo, Viewport};

#[test]
fn test_serialize_record_har_full() {
    let options = BrowserContextOptions::builder()
        .record_har(
            RecordHar::new("/tmp/test.har")
                .omit_content(true)
                .mode("minimal")
                .content("omit")
                .url_filter("**/api/**"),
        )
        .build();

    let json = serde_json::to_value(options).unwrap();
    let record_har = json.get("recordHar").unwrap();

    assert_eq!(record_har["path"], "/tmp/test.har");
    assert_eq!(record_har["omitContent"], true);
    assert_eq!(record_har["mode"], "minimal");
    assert_eq!(record_har["content"], "omit");
    assert_eq!(record_har["urlFilter"], "**/api/**");
}

#[test]
fn test_serialize_record_har_minimal() {
    let options = BrowserContextOptions::builder()
        .record_har(RecordHar::new("simple.har"))
        .build();

    let json = serde_json::to_value(options).unwrap();
    let record_har = json.get("recordHar").unwrap();

    assert_eq!(record_har["path"], "simple.har");
    assert!(record_har.get("omitContent").is_none());
    assert!(record_har.get("mode").is_none());
}

#[test]
fn test_serialize_record_video() {
    let options = BrowserContextOptions::builder()
        .record_video(RecordVideo::new("/tmp/videos").size(Viewport {
            width: 800,
            height: 600,
        }))
        .build();

    let json = serde_json::to_value(options).unwrap();
    let record_video = json.get("recordVideo").unwrap();

    assert_eq!(record_video["dir"], "/tmp/videos");
    assert_eq!(record_video["size"]["width"], 800);
    assert_eq!(record_video["size"]["height"], 600);
}

#[test]
fn test_serialize_service_workers() {
    let options = BrowserContextOptions::builder()
        .service_workers("block".to_string())
        .build();

    let json = serde_json::to_value(options).unwrap();
    assert_eq!(json["serviceWorkers"], "block");
}

#[test]
fn test_serialize_no_viewport_as_no_default_viewport() {
    let options = BrowserContextOptions::builder().no_viewport(true).build();

    let json = serde_json::to_value(&options).unwrap();

    // Must serialize as "noDefaultViewport" (what the Playwright server expects),
    // not "noViewport" (what camelCase would produce).
    assert_eq!(json["noDefaultViewport"], true);
    assert!(json.get("noViewport").is_none());
}
