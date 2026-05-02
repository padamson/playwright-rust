use playwright_rs::protocol::{
    ChapterOptions, ScreencastStartOptions, ShowActionsOptions, ShowOverlayOptions,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn test_screencast_streams_frames_to_handler() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(
        "<body style='background:#06f;color:#fff;font-size:48px'>frame</body>",
        None,
    )
    .await
    .expect("Failed to set content");

    let frames = Arc::new(AtomicUsize::new(0));
    let bytes_seen = Arc::new(parking_lot::Mutex::new(0usize));
    let counter = frames.clone();
    let bytes = bytes_seen.clone();
    page.screencast().on_frame(move |frame| {
        let counter = counter.clone();
        let bytes = bytes.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            *bytes.lock() = frame.data.len();
            Ok(())
        }
    });

    page.screencast()
        .start(ScreencastStartOptions::default())
        .await
        .expect("screencast start failed");

    let _ = page
        .evaluate_value("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
        .await;

    let mut waited = std::time::Duration::ZERO;
    while frames.load(Ordering::SeqCst) == 0 && waited < std::time::Duration::from_secs(10) {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        waited += std::time::Duration::from_millis(100);
    }

    page.screencast()
        .stop()
        .await
        .expect("screencast stop failed");

    assert!(
        frames.load(Ordering::SeqCst) > 0,
        "expected at least one screencast frame within 10s"
    );
    assert!(
        *bytes_seen.lock() > 0,
        "frame data should be a non-empty JPEG byte buffer"
    );

    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_screencast_records_to_path() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content("<body>recorded</body>", None)
        .await
        .expect("Failed to set content");

    let path = std::env::temp_dir().join(format!(
        "pw-rust-screencast-{}.webm",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&path);

    page.screencast()
        .start(ScreencastStartOptions {
            path: Some(path.clone()),
            ..Default::default()
        })
        .await
        .expect("screencast start with path failed");

    let _ = page
        .evaluate_value("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
        .await;

    page.screencast()
        .stop()
        .await
        .expect("screencast stop failed");

    let metadata =
        std::fs::metadata(&path).expect("recorded screencast file should exist after stop");
    assert!(
        metadata.len() > 0,
        "recorded screencast should be non-empty"
    );

    let _ = std::fs::remove_file(&path);
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_screencast_overlay_and_chapter_calls_succeed() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content("<body>overlays</body>", None)
        .await
        .expect("Failed to set content");

    let screencast = page.screencast();
    screencast
        .start(ScreencastStartOptions::default())
        .await
        .expect("start failed");

    screencast
        .show_actions(ShowActionsOptions {
            duration: Some(500.0),
            ..Default::default()
        })
        .await
        .expect("show_actions failed");
    screencast
        .hide_actions()
        .await
        .expect("hide_actions failed");

    screencast
        .show_chapter(
            "Phase 1",
            ChapterOptions {
                description: Some("setting up".into()),
                duration: Some(500.0),
            },
        )
        .await
        .expect("show_chapter failed");

    let id = screencast
        .show_overlay(
            "<div style='color:white;background:black;padding:8px'>hi</div>",
            ShowOverlayOptions {
                duration: Some(500.0),
            },
        )
        .await
        .expect("show_overlay failed");
    assert!(!id.0.is_empty(), "overlay id should be non-empty");

    screencast
        .set_overlay_visible(false)
        .await
        .expect("set_overlay_visible(false) failed");
    screencast
        .set_overlay_visible(true)
        .await
        .expect("set_overlay_visible(true) failed");

    screencast
        .remove_overlay(id)
        .await
        .expect("remove_overlay failed");

    screencast.stop().await.expect("stop failed");
    browser.close().await.expect("Failed to close browser");
}
