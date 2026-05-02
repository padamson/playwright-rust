use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn test_debugger_accessor_returns_object() {
    let (_pw, _browser, context) = crate::common::setup_context().await;
    let dbg = context
        .debugger()
        .await
        .expect("context.debugger() should resolve");
    assert!(!dbg.is_paused());
    assert!(dbg.paused_details().is_none());
    context.close().await.expect("context close failed");
}

#[tokio::test]
async fn test_debugger_pause_resume_around_action() {
    // Drive a real action through the pause point: arm with
    // request_pause(), spawn an action that will hit the pause, observe
    // the paused-state-changed event, then resume and confirm the
    // action completes.
    let (_pw, _browser, context) = crate::common::setup_context().await;
    let dbg = context.debugger().await.expect("debugger accessor failed");

    let event_count = Arc::new(AtomicUsize::new(0));
    let counter = event_count.clone();
    dbg.on_paused_state_changed(move |_details| {
        let counter = counter.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    let page = context.new_page().await.expect("new page");
    page.set_content("<button id='b'>x</button>", None)
        .await
        .expect("set content");

    dbg.request_pause().await.expect("request_pause");

    // Spawn an action that will hit the pause.
    let page_for_click = page.clone();
    let click_task = tokio::spawn(async move {
        let btn = page_for_click.locator("#b").await;
        btn.click(None).await
    });

    // Wait for the debugger to actually pause.
    let mut waited = std::time::Duration::ZERO;
    while !dbg.is_paused() && waited < std::time::Duration::from_secs(5) {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        waited += std::time::Duration::from_millis(50);
    }
    assert!(dbg.is_paused(), "debugger should be paused within 5s");
    let details = dbg
        .paused_details()
        .expect("paused_details should be Some while paused");
    assert!(
        !details.title.is_empty(),
        "paused title should be populated"
    );

    dbg.resume()
        .await
        .expect("resume while paused should succeed");

    // The click action should now complete.
    let click_result = tokio::time::timeout(std::time::Duration::from_secs(5), click_task)
        .await
        .expect("click did not finish within 5s of resume")
        .expect("click task panicked");
    click_result.expect("click should succeed after resume");

    // The handler should have fired at least twice: paused + resumed.
    assert!(
        event_count.load(Ordering::SeqCst) >= 2,
        "expected at least 2 paused-state-changed events (paused + resumed), got {}",
        event_count.load(Ordering::SeqCst)
    );

    context.close().await.expect("context close failed");
}
