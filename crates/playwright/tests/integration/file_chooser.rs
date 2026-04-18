// Integration tests for FileChooser event handling
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - on_filechooser() handler fires when file input is clicked
// - FileChooser.is_multiple() returns correct value
// - FileChooser.set_files() sets files on the input
// - FileChooser.page() returns the correct back-reference
// - expect_file_chooser() waiter resolves on file chooser event
//
// FileChooser is NOT a ChannelOwner — it's a plain struct constructed from
// event params (element GUID + isMultiple). The "fileChooser" event is sent
// on the Page channel.
//
// See: <https://playwright.dev/docs/api/class-filechooser>

use playwright_rs::protocol::{FileChooser, Playwright};
use std::fs;
use std::io::Write;
use std::sync::{Arc, Mutex};

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

    // Track whether handler fired and capture is_multiple
    let fired = Arc::new(Mutex::new(false));
    let is_multiple_captured = Arc::new(Mutex::new(None::<bool>));

    let fired_clone = fired.clone();
    let is_multiple_clone = is_multiple_captured.clone();

    page.on_filechooser(move |chooser: FileChooser| {
        let fired_inner = fired_clone.clone();
        let is_multiple_inner = is_multiple_clone.clone();
        async move {
            *fired_inner.lock().unwrap() = true;
            *is_multiple_inner.lock().unwrap() = Some(chooser.is_multiple());
            Ok(())
        }
    })
    .await
    .expect("Failed to register filechooser handler");

    // Click the single file input to trigger the file chooser
    page.locator("#single-file")
        .await
        .click(None)
        .await
        .expect("Failed to click file input");

    // Give the async handler time to fire
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    assert!(*fired.lock().unwrap(), "FileChooser handler did not fire");
    assert_eq!(
        *is_multiple_captured.lock().unwrap(),
        Some(false),
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

    let is_multiple_captured = Arc::new(Mutex::new(None::<bool>));
    let is_multiple_clone = is_multiple_captured.clone();

    page.on_filechooser(move |chooser: FileChooser| {
        let is_multiple_inner = is_multiple_clone.clone();
        async move {
            *is_multiple_inner.lock().unwrap() = Some(chooser.is_multiple());
            Ok(())
        }
    })
    .await
    .expect("Failed to register filechooser handler");

    // Click the multiple-file input
    page.locator("#multi-file")
        .await
        .click(None)
        .await
        .expect("Failed to click multi-file input");

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    assert_eq!(
        *is_multiple_captured.lock().unwrap(),
        Some(true),
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

    let test_file_clone = test_file.clone();
    let set_files_result = Arc::new(Mutex::new(None::<Result<(), String>>));
    let result_clone = set_files_result.clone();

    page.on_filechooser(move |chooser: FileChooser| {
        let file = test_file_clone.clone();
        let result = result_clone.clone();
        async move {
            let r = chooser.set_files(&[file]).await;
            *result.lock().unwrap() = Some(r.map_err(|e| e.to_string()));
            Ok(())
        }
    })
    .await
    .expect("Failed to register handler");

    // Click the single file input
    page.locator("#single-file")
        .await
        .click(None)
        .await
        .expect("Failed to click");

    // Wait for handler to complete
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    // Check set_files returned Ok
    let result = set_files_result.lock().unwrap().clone();
    assert!(
        result.is_some(),
        "set_files was not called (handler did not fire)"
    );
    assert!(result.unwrap().is_ok(), "set_files returned an error");

    // Verify the file info div shows the filename
    let info_text = page
        .locator("#file-info")
        .await
        .text_content()
        .await
        .expect("Failed to get text content");
    assert!(
        info_text
            .as_deref()
            .unwrap_or("")
            .contains("playwright_fc_test.txt"),
        "Expected file name in #file-info, got: {:?}",
        info_text
    );

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

    // Navigate to a recognizable URL via set_content
    // (page URL will be "about:blank" after set_content since it doesn't navigate)
    // We just verify chooser.page() returns a Page (non-panicking clone)
    let page_guid_captured = Arc::new(Mutex::new(None::<String>));
    let captured_clone = page_guid_captured.clone();

    page.on_filechooser(move |chooser: FileChooser| {
        let captured = captured_clone.clone();
        async move {
            // Access the page — just verify it doesn't panic
            let _p = chooser.page();
            *captured.lock().unwrap() = Some("ok".to_string());
            Ok(())
        }
    })
    .await
    .expect("Failed to register handler");

    page.locator("#single-file")
        .await
        .click(None)
        .await
        .expect("Failed to click");

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    assert_eq!(
        *page_guid_captured.lock().unwrap(),
        Some("ok".to_string()),
        "Handler did not fire or chooser.page() panicked"
    );

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

    // Verify file name appears in the page
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let info_text = page
        .locator("#file-info")
        .await
        .text_content()
        .await
        .expect("Failed to get text content");
    assert!(
        info_text
            .as_deref()
            .unwrap_or("")
            .contains("playwright_expect_fc_test.txt"),
        "Expected file name in #file-info, got: {:?}",
        info_text
    );

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
