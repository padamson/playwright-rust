// Integration tests for select and file upload interactions
//
// Following TDD: Write tests first (Red), then implement (Green)
//
// Tests cover:
// - select_option() with value/label/index
// - select_option() for multiple selections
// - set_input_files() with single file
// - set_input_files() with multiple files
// - set_input_files() for clearing files
//
// Performance Optimization (Phase 6):
// - Combined related tests to minimize browser launches
// - Removed redundant cross-browser tests (Rust bindings use same protocol for all browsers)
// - Expected speedup: ~69% (13 tests → 4 tests)

mod test_server;

use playwright_rs::protocol::{Playwright, SelectOption};
use std::fs;
use std::io::Write;
use test_server::TestServer;

// ============================================================================
// select_option() Tests
// ============================================================================

#[tokio::test]
async fn test_select_option_methods() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/select.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Select option by value
    let select = page.locator("#single-select").await;
    let selected = select
        .select_option("banana", None)
        .await
        .expect("Failed to select option");
    assert_eq!(selected, vec!["banana"]);

    // Test 2: Select option by label
    let selected = select
        .select_option(SelectOption::Label("Banana".to_string()), None)
        .await
        .expect("Failed to select option by label");
    assert_eq!(selected, vec!["banana"]);

    // Test 3: Select option by index (0-based, index 3 = "Cherry")
    let selected = select
        .select_option(SelectOption::Index(3), None)
        .await
        .expect("Failed to select option by index");
    assert_eq!(selected, vec!["cherry"]);

    // Test 4: Select option by index when options have no value attribute
    let select_no_value = page.locator("#select-by-index").await;
    let selected = select_no_value
        .select_option(SelectOption::Index(1), None)
        .await
        .expect("Failed to select by index");
    // When no value attribute, the text content becomes the value
    assert_eq!(selected, vec!["Second"]);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// select_option_multiple() Tests
// ============================================================================

#[tokio::test]
async fn test_select_multiple_options() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/select.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Test 1: Select multiple options with string values
    let select = page.locator("#multi-select").await;
    let selected = select
        .select_option_multiple(&["red", "blue"], None)
        .await
        .expect("Failed to select multiple options");
    assert_eq!(selected, vec!["red", "blue"]);

    // Test 2: Select multiple options using SelectOption variants (mixed types)
    let options = vec![
        SelectOption::Value("red".to_string()),
        SelectOption::Label("Blue".to_string()),
    ];
    let selected = select
        .select_option_multiple(&options, None)
        .await
        .expect("Failed to select multiple options with mixed types");
    assert_eq!(selected, vec!["red", "blue"]);

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// File Upload Tests
// ============================================================================

#[tokio::test]
async fn test_file_upload_methods() {
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/upload.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let temp_dir = std::env::temp_dir();

    // Test 1: Upload single file
    let test_file = temp_dir.join("playwright_test_file.txt");
    let mut file = fs::File::create(&test_file).expect("Failed to create test file");
    file.write_all(b"Test file content")
        .expect("Failed to write to test file");

    let input = page.locator("#single-file").await;
    input
        .set_input_files(&test_file, None)
        .await
        .expect("Failed to set input file");

    // Verify file was uploaded by checking the displayed info
    let info = page.locator("#file-info").await;
    let text = info.text_content().await.expect("Failed to get text");
    assert!(text.unwrap().contains("playwright_test_file.txt"));

    // Test 2: Upload multiple files
    let test_file1 = temp_dir.join("playwright_test_file1.txt");
    let test_file2 = temp_dir.join("playwright_test_file2.txt");

    let mut file1 = fs::File::create(&test_file1).expect("Failed to create test file 1");
    file1
        .write_all(b"Test file 1 content")
        .expect("Failed to write to test file 1");

    let mut file2 = fs::File::create(&test_file2).expect("Failed to create test file 2");
    file2
        .write_all(b"Test file 2 content")
        .expect("Failed to write to test file 2");

    let multi_input = page.locator("#multi-file").await;
    multi_input
        .set_input_files_multiple(&[&test_file1, &test_file2], None)
        .await
        .expect("Failed to set multiple input files");

    // Verify files were uploaded
    let info = page.locator("#file-info").await;
    let text = info.text_content().await.expect("Failed to get text");
    let text_content = text.unwrap();
    assert!(text_content.contains("playwright_test_file1.txt"));
    assert!(text_content.contains("playwright_test_file2.txt"));

    // Test 3: Clear file input by passing empty array
    input
        .set_input_files_multiple(&[], None)
        .await
        .expect("Failed to clear input files");

    // Cleanup
    fs::remove_file(test_file).expect("Failed to remove test file");
    fs::remove_file(test_file1).expect("Failed to remove test file 1");
    fs::remove_file(test_file2).expect("Failed to remove test file 2");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

#[tokio::test]
async fn test_cross_browser_smoke() {
    // Smoke test to verify select and upload work in Firefox and WebKit
    // (Rust bindings use the same protocol layer for all browsers,
    //  so we don't need exhaustive cross-browser testing for each feature)

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Test Firefox - select options
    let firefox = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch Firefox");
    let firefox_page = firefox.new_page().await.expect("Failed to create page");

    firefox_page
        .goto(&format!("{}/select.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let firefox_select = firefox_page.locator("#single-select").await;
    let selected = firefox_select
        .select_option("cherry", None)
        .await
        .expect("Failed to select option");
    assert_eq!(selected, vec!["cherry"]);

    let selected = firefox_select
        .select_option(SelectOption::Label("Apple".to_string()), None)
        .await
        .expect("Failed to select option by label in Firefox");
    assert_eq!(selected, vec!["apple"]);

    firefox.close().await.expect("Failed to close Firefox");

    // Test WebKit - select and file upload
    let webkit = playwright
        .webkit()
        .launch()
        .await
        .expect("Failed to launch WebKit");
    let webkit_page = webkit.new_page().await.expect("Failed to create page");

    webkit_page
        .goto(&format!("{}/select.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let webkit_select = webkit_page.locator("#single-select").await;
    let selected = webkit_select
        .select_option(SelectOption::Index(2), None)
        .await
        .expect("Failed to select option by index in WebKit");
    assert_eq!(selected, vec!["banana"]);

    // Test file upload in WebKit
    webkit_page
        .goto(&format!("{}/upload.html", server.url()), None)
        .await
        .expect("Failed to navigate");

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("playwright_webkit_test.txt");
    let mut file = fs::File::create(&test_file).expect("Failed to create test file");
    file.write_all(b"WebKit test content")
        .expect("Failed to write to test file");

    let webkit_input = webkit_page.locator("#single-file").await;
    webkit_input
        .set_input_files(&test_file, None)
        .await
        .expect("Failed to set input file");

    // Cleanup
    fs::remove_file(test_file).expect("Failed to remove test file");
    webkit.close().await.expect("Failed to close WebKit");
    server.shutdown();
}
