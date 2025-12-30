// FilePayload Tests
//
// Tests for advanced file uploads using FilePayload struct
//
// These tests verify that:
// 1. Files can be uploaded with explicit name, mimeType, and buffer
// 2. Multiple FilePayload objects can be uploaded at once
// 3. FilePayload works alongside PathBuf-based uploads
//
// TDD approach: Tests written FIRST, then implementation

use playwright_rs::protocol::{FilePayload, Playwright};

mod common;

#[tokio::test]
async fn test_file_payload_basic() {
    common::init_tracing();
    // Test uploading a file using FilePayload
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Use evaluate to set HTML
    page.evaluate_expression("document.body.innerHTML = '<input type=\\'file\\' id=\\'upload\\'><div id=\\'result\\'></div>'")
        .await
        .expect("Failed to set content");

    // Create FilePayload
    let file_payload = FilePayload::builder()
        .name("test.txt".to_string())
        .mime_type("text/plain".to_string())
        .buffer(b"Test file content".to_vec())
        .build();

    let input = page.locator("#upload").await;
    input
        .set_input_files_payload(file_payload, None)
        .await
        .expect("Failed to upload file");

    // Verify file was uploaded
    let has_files = page
        .evaluate_value("document.getElementById('upload').files.length > 0")
        .await
        .expect("Failed to check files");
    assert_eq!(has_files, "true");

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_file_payload_multiple() {
    common::init_tracing();
    // Test uploading multiple files using FilePayload
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Create test HTML with multiple file input
    page.evaluate_expression("document.body.innerHTML = '<input type=\\'file\\' multiple id=\\'upload\\'><div id=\\'result\\'></div>'")
        .await
        .expect("Failed to set content");

    // Create multiple FilePayloads
    let file1 = FilePayload::builder()
        .name("file1.txt".to_string())
        .mime_type("text/plain".to_string())
        .buffer(b"File 1 content".to_vec())
        .build();

    let file2 = FilePayload::builder()
        .name("file2.json".to_string())
        .mime_type("application/json".to_string())
        .buffer(b"{\"key\": \"value\"}".to_vec())
        .build();

    let input = page.locator("#upload").await;
    input
        .set_input_files_payload_multiple(&[file1, file2], None)
        .await
        .expect("Failed to upload files");

    // Verify files were uploaded
    let file_count = page
        .evaluate_value("document.getElementById('upload').files.length")
        .await
        .expect("Failed to check file count");
    assert_eq!(file_count, "2");

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_file_payload_custom_mime_type() {
    common::init_tracing();
    // Test uploading a file with custom MIME type
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.evaluate_expression("document.body.innerHTML = '<input type=\\'file\\' id=\\'upload\\'>'")
        .await
        .expect("Failed to set content");

    // Create FilePayload with custom MIME type
    let file_payload = FilePayload::builder()
        .name("data.csv".to_string())
        .mime_type("text/csv".to_string())
        .buffer(b"col1,col2\nval1,val2".to_vec())
        .build();

    let input = page.locator("#upload").await;
    input
        .set_input_files_payload(file_payload, None)
        .await
        .expect("Failed to upload file");

    // Verify file was uploaded
    let has_files = page
        .evaluate_value("document.getElementById('upload').files.length > 0")
        .await
        .expect("Failed to check files");
    assert_eq!(has_files, "true");

    page.close().await.expect("Failed to close page");
    browser.close().await.expect("Failed to close browser");
}
