use crate::common::setup;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, timeout};

#[tokio::test]
async fn test_page_on_worker_fires() {
    let (_pw, browser, page) = setup().await;

    let fired = Arc::new(Mutex::new(false));
    let fired_clone = fired.clone();

    page.on_worker(move |_worker| {
        let f = fired_clone.clone();
        Box::pin(async move {
            *f.lock().unwrap() = true;
            Ok(())
        })
    })
    .await
    .expect("on_worker registration should succeed");

    // Inline blob worker — minimal valid JS Worker
    page.evaluate::<(), serde_json::Value>(
        r#"() => {
            const blob = new Blob(['self.onmessage = () => {};'], {type: 'application/javascript'});
            const url = URL.createObjectURL(blob);
            new Worker(url);
        }"#,
        None,
    )
    .await
    .expect("evaluate should create a worker");

    // Give the event time to arrive
    let result = timeout(Duration::from_secs(5), async {
        loop {
            if *fired.lock().unwrap() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    assert!(
        result.is_ok() && result.unwrap(),
        "on_worker handler should have fired"
    );

    browser.close().await.expect("browser should close");
}

#[tokio::test]
async fn test_worker_url() {
    let (_pw, browser, page) = setup().await;

    let worker_url = Arc::new(Mutex::new(None::<String>));
    let url_clone = worker_url.clone();

    page.on_worker(move |worker| {
        let u = url_clone.clone();
        Box::pin(async move {
            *u.lock().unwrap() = Some(worker.url().to_string());
            Ok(())
        })
    })
    .await
    .expect("on_worker registration should succeed");

    page.evaluate::<(), serde_json::Value>(
        r#"() => {
            const blob = new Blob(['self.onmessage = () => {};'], {type: 'application/javascript'});
            const url = URL.createObjectURL(blob);
            new Worker(url);
        }"#,
        None,
    )
    .await
    .expect("evaluate should create a worker");

    // Wait for the URL to be captured
    let result = timeout(Duration::from_secs(5), async {
        loop {
            if worker_url.lock().unwrap().is_some() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    assert!(result.is_ok(), "on_worker handler should have fired");

    let url = worker_url.lock().unwrap().clone().unwrap();
    assert!(
        url.starts_with("blob:"),
        "worker URL should be a blob URL, got: {}",
        url
    );

    browser.close().await.expect("browser should close");
}

#[tokio::test]
async fn test_worker_evaluate() {
    let (_pw, browser, page) = setup().await;

    let worker_result: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    let result_clone = worker_result.clone();

    page.on_worker(move |worker| {
        let r = result_clone.clone();
        Box::pin(async move {
            let val = worker
                .evaluate::<serde_json::Value, serde_json::Value>("1 + 2", None)
                .await?;
            *r.lock().unwrap() = Some(val);
            Ok(())
        })
    })
    .await
    .expect("on_worker registration should succeed");

    page.evaluate::<(), serde_json::Value>(
        r#"() => {
            const blob = new Blob(['self.onmessage = () => {};'], {type: 'application/javascript'});
            const url = URL.createObjectURL(blob);
            new Worker(url);
        }"#,
        None,
    )
    .await
    .expect("evaluate should create a worker");

    let done = timeout(Duration::from_secs(5), async {
        loop {
            if worker_result.lock().unwrap().is_some() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    assert!(done.is_ok(), "worker evaluate should complete");
    let val = worker_result.lock().unwrap().clone().unwrap();
    assert_eq!(val, serde_json::json!(3), "1 + 2 should equal 3");

    browser.close().await.expect("browser should close");
}

#[tokio::test]
async fn test_worker_type_registered() {
    // Verify that launching Playwright succeeds (i.e., Worker type doesn't crash)
    // and the page can be created. This exercises the object factory code path
    // that must handle "Worker" type without panicking.
    let (_pw, browser, page) = setup().await;

    // Navigate to a page — no worker needed, just ensuring no factory panics
    let _ = page
        .goto("about:blank", None)
        .await
        .expect("navigation should succeed");

    browser.close().await.expect("browser should close");
}

#[tokio::test]
async fn test_context_on_serviceworker_handler_registered() {
    // Verify that on_serviceworker can be registered without errors.
    // (Full SW tests require HTTPS + registered service worker, which is out of scope here.)
    let (_pw, browser, context) = crate::common::setup_context().await;

    context
        .on_serviceworker(|_worker| Box::pin(async move { Ok(()) }))
        .await
        .expect("on_serviceworker registration should succeed");

    browser.close().await.expect("browser should close");
}

/// Test that page.workers() returns the active web workers in the page.
#[tokio::test]
async fn test_page_workers() {
    let (_pw, browser, page) = setup().await;

    // Initially no workers
    assert!(
        page.workers().is_empty(),
        "page.workers() should be empty before creating any workers"
    );

    // Create a blob worker
    page.evaluate::<(), serde_json::Value>(
        r#"() => {
            const blob = new Blob(['self.onmessage = () => {};'], {type: 'application/javascript'});
            const url = URL.createObjectURL(blob);
            new Worker(url);
        }"#,
        None,
    )
    .await
    .expect("evaluate should create a worker");

    // Wait for the worker event to arrive
    let result = timeout(Duration::from_secs(5), async {
        loop {
            if !page.workers().is_empty() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    assert!(result.is_ok(), "page.workers() should become non-empty");

    let workers = page.workers();
    assert_eq!(workers.len(), 1, "Should have exactly 1 worker");
    assert!(
        workers[0].url().starts_with("blob:"),
        "worker URL should start with 'blob:', got: {}",
        workers[0].url()
    );

    browser.close().await.expect("browser should close");
}

/// Test that context.service_workers() returns an empty vec when no service workers are registered.
#[tokio::test]
async fn test_context_service_workers_empty() {
    // Full service worker tests require HTTPS + a real SW registration.
    // Here we just verify the accessor returns an empty Vec in normal usage.
    let (_pw, browser, context) = crate::common::setup_context().await;

    let sws = context.service_workers();
    assert!(
        sws.is_empty(),
        "service_workers() should be empty when no SW is registered"
    );

    browser.close().await.expect("browser should close");
}
