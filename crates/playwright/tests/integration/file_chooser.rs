use playwright_rs::protocol::Playwright;
use std::fs;
use std::io::Write;

// ============================================================================
// Helper: HTML for file chooser tests
// The test server has /upload.html, but we use set_content for more control.
// ============================================================================

/// Single file chooser HTML
fn single_chooser_html() -> &'static str {
    r#"<!DOCTYPE html>
<html>
<head><title>File Chooser Test</title></head>
<body>
  <input type="file" id="single-file" />
  <input type="file" id="multi-file" multiple />
  <div id="file-info"></div>
  <script>
    document.getElementById('single-file').addEventListener('change', (e) => {
      const files = Array.from(e.target.files).map(f => f.name).join(', ');
      document.getElementById('file-info').textContent = 'Files: ' + files;
    });
    document.getElementById('multi-file').addEventListener('change', (e) => {
      const files = Array.from(e.target.files).map(f => f.name).join(', ');
      document.getElementById('file-info').textContent = 'Files: ' + files;
    });
  </script>
</body>
</html>"#
}

// ============================================================================
// Test 1: on_filechooser handler fires on single-file input click
// ============================================================================

#[tokio::test]
async fn test_on_filechooser_single_fires() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(single_chooser_html(), None)
        .await
        .expect("Failed to set content");

    let waiter = page
        .expect_file_chooser(Some(5000.0))
        .await
        .expect("Failed to create waiter");

    page.locator("#single-file")
        .await
        .click(None)
        .await
        .expect("Failed to click file input");

    let chooser = waiter.wait().await.expect("FileChooser event did not fire");

    assert!(
        !chooser.is_multiple(),
        "Expected is_multiple=false for single file input"
    );

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Test 2: on_filechooser handler captures is_multiple=true for multiple input
// ============================================================================

#[tokio::test]
async fn test_on_filechooser_multiple_flag() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(single_chooser_html(), None)
        .await
        .expect("Failed to set content");

    let waiter = page
        .expect_file_chooser(Some(5000.0))
        .await
        .expect("Failed to create waiter");

    page.locator("#multi-file")
        .await
        .click(None)
        .await
        .expect("Failed to click multi-file input");

    let chooser = waiter.wait().await.expect("FileChooser event did not fire");

    assert!(
        chooser.is_multiple(),
        "Expected is_multiple=true for multiple file input"
    );

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Test 3: FileChooser.set_files() sets the file on the input
// ============================================================================

#[tokio::test]
async fn test_filechooser_set_files() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(single_chooser_html(), None)
        .await
        .expect("Failed to set content");

    // Create a temp file to upload
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("playwright_fc_test.txt");
    {
        let mut f = fs::File::create(&test_file).expect("Failed to create temp file");
        f.write_all(b"hello from filechooser test")
            .expect("Failed to write");
    }

    let waiter = page
        .expect_file_chooser(Some(5000.0))
        .await
        .expect("Failed to create waiter");

    page.locator("#single-file")
        .await
        .click(None)
        .await
        .expect("Failed to click");

    let chooser = waiter.wait().await.expect("FileChooser event did not fire");

    chooser
        .set_files(std::slice::from_ref(&test_file))
        .await
        .expect("set_files returned an error");

    // Auto-retry text assertion until the JS change handler updates the DOM
    let info = page.locator("#file-info").await;
    playwright_rs::expect(info.clone())
        .to_contain_text("playwright_fc_test.txt")
        .await
        .expect("DOM #file-info did not update with uploaded filename");

    // Cleanup
    let _ = fs::remove_file(&test_file);
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Test 4: FileChooser.page() returns correct back-reference
// ============================================================================

#[tokio::test]
async fn test_filechooser_page_back_reference() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(single_chooser_html(), None)
        .await
        .expect("Failed to set content");

    let waiter = page
        .expect_file_chooser(Some(5000.0))
        .await
        .expect("Failed to create waiter");

    page.locator("#single-file")
        .await
        .click(None)
        .await
        .expect("Failed to click");

    let chooser = waiter.wait().await.expect("FileChooser event did not fire");

    // Access the page — just verify it doesn't panic
    let _p = chooser.page();

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Test 5: expect_file_chooser() resolves when file chooser opens
// ============================================================================

#[tokio::test]
async fn test_expect_file_chooser() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(single_chooser_html(), None)
        .await
        .expect("Failed to set content");

    // Set up the waiter BEFORE triggering the action
    let waiter = page
        .expect_file_chooser(Some(5000.0))
        .await
        .expect("Failed to create waiter");

    // Click file input to trigger the file chooser event
    page.locator("#single-file")
        .await
        .click(None)
        .await
        .expect("Failed to click");

    // Resolve the waiter
    let chooser = waiter.wait().await.expect("expect_file_chooser timed out");

    // Verify we got a valid FileChooser
    assert!(!chooser.is_multiple(), "Expected is_multiple=false");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Test 6: expect_file_chooser() with set_files
// ============================================================================

#[tokio::test]
async fn test_expect_file_chooser_set_files() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content(single_chooser_html(), None)
        .await
        .expect("Failed to set content");

    // Create temp file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("playwright_expect_fc_test.txt");
    {
        let mut f = fs::File::create(&test_file).expect("Failed to create temp file");
        f.write_all(b"expect_file_chooser test content")
            .expect("Failed to write");
    }

    // Set up waiter before triggering
    let waiter = page
        .expect_file_chooser(Some(5000.0))
        .await
        .expect("Failed to create waiter");

    // Trigger file chooser by clicking
    page.locator("#single-file")
        .await
        .click(None)
        .await
        .expect("Failed to click");

    let chooser = waiter.wait().await.expect("expect_file_chooser timed out");

    // Set files via the chooser
    chooser
        .set_files(std::slice::from_ref(&test_file))
        .await
        .expect("set_files failed");

    // Auto-retry text assertion until the JS change handler updates the DOM
    let info = page.locator("#file-info").await;
    playwright_rs::expect(info.clone())
        .to_contain_text("playwright_expect_fc_test.txt")
        .await
        .expect("DOM #file-info did not update with uploaded filename");

    // Cleanup
    let _ = fs::remove_file(&test_file);
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Test 7: Cross-browser smoke test (Firefox + WebKit)
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_filechooser_cross_browser_smoke() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Test Firefox
    {
        let browser = playwright
            .firefox()
            .launch()
            .await
            .expect("Failed to launch Firefox");
        let page = browser.new_page().await.expect("Failed to create page");
        page.set_content(single_chooser_html(), None)
            .await
            .expect("Failed to set content");

        let waiter = page
            .expect_file_chooser(Some(5000.0))
            .await
            .expect("Failed to create waiter");

        page.locator("#single-file")
            .await
            .click(None)
            .await
            .expect("Failed to click");

        let chooser = waiter
            .wait()
            .await
            .expect("Firefox: expect_file_chooser timed out");
        assert!(
            !chooser.is_multiple(),
            "Firefox: expected is_multiple=false"
        );

        browser.close().await.expect("Failed to close Firefox");
    }

    // Test WebKit
    {
        let browser = playwright
            .webkit()
            .launch()
            .await
            .expect("Failed to launch WebKit");
        let page = browser.new_page().await.expect("Failed to create page");
        page.set_content(single_chooser_html(), None)
            .await
            .expect("Failed to set content");

        let waiter = page
            .expect_file_chooser(Some(5000.0))
            .await
            .expect("Failed to create waiter");

        page.locator("#single-file")
            .await
            .click(None)
            .await
            .expect("Failed to click");

        let chooser = waiter
            .wait()
            .await
            .expect("WebKit: expect_file_chooser timed out");
        assert!(!chooser.is_multiple(), "WebKit: expected is_multiple=false");

        browser.close().await.expect("Failed to close WebKit");
    }
}
