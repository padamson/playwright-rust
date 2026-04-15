// Tests for WebError and BrowserContext::on_weberror

use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_context_on_weberror() {
    let (_pw, browser, context) = crate::common::setup_context().await;

    let errors: Arc<Mutex<Vec<(String, bool)>>> = Arc::new(Mutex::new(vec![]));
    let errors2 = errors.clone();

    context
        .on_weberror(move |web_error| {
            let errs = errors2.clone();
            async move {
                let msg = web_error.error().to_string();
                let has_page = web_error.page().is_some();
                errs.lock().unwrap().push((msg, has_page));
                Ok(())
            }
        })
        .await
        .expect("Failed to register on_weberror handler");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto("about:blank", None)
        .await
        .expect("Failed to navigate");

    // Throw an uncaught error asynchronously so it escapes the evaluate call
    let _ = page
        .evaluate_expression("setTimeout(() => { throw new Error('test error') }, 0)")
        .await;

    // Give the async event a moment to fire
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    {
        let errs = errors.lock().unwrap();
        assert_eq!(
            errs.len(),
            1,
            "on_weberror handler should fire once, got: {:?}",
            errs
        );
        assert!(
            errs[0].0.contains("test error"),
            "error message should contain 'test error', got: {:?}",
            errs[0].0
        );
        assert!(errs[0].1, "WebError.page() should be Some, but got None");
    }

    browser.close().await.expect("Failed to close browser");
}
