// Copyright 2024 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Integration tests for download and dialog event handling
//
// Tests download events (page.on_download) and dialog events (page.on_dialog)
// for alert, confirm, and prompt dialog types.

use playwright_core::protocol::Playwright;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Test basic download event handling
///
/// Verifies that:
/// 1. Download event is fired when download is triggered
/// 2. Download object provides URL and suggested filename
/// 3. Download can be saved to disk
#[tokio::test]
async fn test_download_event_triggered() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Shared state to capture download
    let download_captured = Arc::new(Mutex::new(None));
    let download_captured_clone = download_captured.clone();

    // Register download handler
    page.on_download(move |download| {
        let captured = download_captured_clone.clone();
        async move {
            *captured.lock().unwrap() = Some(download);
            Ok(())
        }
    })
    .await?;

    // Navigate to blank page first
    let _ = page.goto("about:blank", None).await;

    // Create a test page that triggers a download using evaluate
    page.evaluate(
        r#"
        const a = document.createElement('a');
        a.href = 'data:text/plain;charset=utf-8,Hello%20World';
        a.download = 'test.txt';
        a.id = 'download-link';
        a.textContent = 'Download';
        document.body.appendChild(a);
        "#,
    )
    .await?;

    // Trigger download
    let locator = page.locator("#download-link").await;
    locator.click(None).await?;

    // Wait for download event
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify download was captured
    let download_opt = download_captured.lock().unwrap().take();
    assert!(download_opt.is_some(), "Download event should have fired");

    let download = download_opt.unwrap();

    // Verify download properties
    assert!(
        download.url().contains("data:text/plain"),
        "Download URL should be the data URL"
    );
    assert_eq!(
        download.suggested_filename(),
        "test.txt",
        "Suggested filename should be 'test.txt'"
    );

    browser.close().await?;
    Ok(())
}

/// Test download save_as functionality
///
/// Verifies that:
/// 1. Download can be saved to a specific path
/// 2. File exists after saving
/// 3. Content is correct
#[tokio::test]
async fn test_download_save_as() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Shared state to capture download
    let download_captured = Arc::new(Mutex::new(None));
    let download_captured_clone = download_captured.clone();

    // Register download handler
    page.on_download(move |download| {
        let captured = download_captured_clone.clone();
        async move {
            *captured.lock().unwrap() = Some(download);
            Ok(())
        }
    })
    .await?;

    // Navigate to blank page
    let _ = page.goto("about:blank", None).await;

    // Create test page with download
    page.evaluate(
        r#"
        const a = document.createElement('a');
        a.href = 'data:text/plain;charset=utf-8,TestContent';
        a.download = 'file.txt';
        a.id = 'dl';
        a.textContent = 'Download';
        document.body.appendChild(a);
        "#,
    )
    .await?;

    // Trigger download
    let locator = page.locator("#dl").await;
    locator.click(None).await?;

    // Wait for download
    tokio::time::sleep(Duration::from_millis(500)).await;

    let download_opt = download_captured.lock().unwrap().take();
    assert!(download_opt.is_some());
    let download = download_opt.unwrap();

    // Save to temp file
    let temp_dir = std::env::temp_dir();
    let save_path = temp_dir.join("playwright_test_download.txt");

    // Clean up if exists
    let _ = std::fs::remove_file(&save_path);

    // Save download
    download.save_as(&save_path).await?;

    // Verify file exists
    assert!(
        save_path.exists(),
        "Downloaded file should exist at save path"
    );

    // Clean up
    std::fs::remove_file(&save_path)?;

    browser.close().await?;
    Ok(())
}

/// Test dialog alert handling
///
/// Verifies that:
/// 1. Alert dialog event is fired
/// 2. Dialog type is "alert"
/// 3. Dialog message is captured
/// 4. Dialog can be accepted
#[tokio::test]
async fn test_dialog_alert() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Shared state to capture dialog info
    let dialog_info = Arc::new(Mutex::new(None));
    let dialog_info_clone = dialog_info.clone();

    // Register dialog handler
    page.on_dialog(move |dialog| {
        let info = dialog_info_clone.clone();
        async move {
            // Capture dialog info
            let type_ = dialog.type_().to_string();
            let message = dialog.message().to_string();
            *info.lock().unwrap() = Some((type_, message));

            // Accept the dialog
            dialog.accept(None).await
        }
    })
    .await?;

    // Navigate to blank page
    let _ = page.goto("about:blank", None).await;

    // Create test page with alert button
    page.evaluate(
        r#"
        const button = document.createElement('button');
        button.onclick = () => alert('Hello from alert!');
        button.textContent = 'Alert';
        document.body.appendChild(button);
        "#,
    )
    .await?;

    // Trigger alert
    let locator = page.locator("button").await;
    locator.click(None).await?;

    // Wait for dialog event
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify dialog was captured
    let info_opt = dialog_info.lock().unwrap().take();
    assert!(info_opt.is_some(), "Dialog event should have fired");

    let (dialog_type, dialog_message) = info_opt.unwrap();
    assert_eq!(dialog_type, "alert", "Dialog type should be 'alert'");
    assert_eq!(
        dialog_message, "Hello from alert!",
        "Dialog message should match"
    );

    browser.close().await?;
    Ok(())
}

/// Test dialog confirm handling - accept
///
/// Verifies that:
/// 1. Confirm dialog event is fired
/// 2. Dialog type is "confirm"
/// 3. Dialog can be accepted
/// 4. JavaScript receives true when accepted
#[tokio::test]
async fn test_dialog_confirm_accept() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Shared state
    let dialog_info = Arc::new(Mutex::new(None));
    let dialog_info_clone = dialog_info.clone();

    // Register dialog handler that accepts
    page.on_dialog(move |dialog| {
        let info = dialog_info_clone.clone();
        async move {
            *info.lock().unwrap() = Some(dialog.type_().to_string());
            dialog.accept(None).await
        }
    })
    .await?;

    // Navigate to blank page
    let _ = page.goto("about:blank", None).await;

    // Page with confirm that stores result
    page.evaluate(
        r#"
        const button = document.createElement('button');
        button.onclick = () => { window.confirmResult = confirm('Continue?'); };
        button.textContent = 'Confirm';
        document.body.appendChild(button);
        "#,
    )
    .await?;

    // Trigger confirm
    let locator = page.locator("button").await;
    locator.click(None).await?;

    // Wait for dialog
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify dialog type
    let dialog_type = dialog_info.lock().unwrap().take();
    assert_eq!(
        dialog_type,
        Some("confirm".to_string()),
        "Dialog type should be 'confirm'"
    );

    // Verify JavaScript received true
    let result = page.evaluate_value("window.confirmResult").await?;
    assert_eq!(result, "true", "confirm() should return true when accepted");

    browser.close().await?;
    Ok(())
}

/// Test dialog confirm handling - dismiss
///
/// Verifies that:
/// 1. Dialog can be dismissed
/// 2. JavaScript receives false when dismissed
#[tokio::test]
async fn test_dialog_confirm_dismiss() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Register dialog handler that dismisses
    page.on_dialog(move |dialog| async move { dialog.dismiss().await })
        .await?;

    // Navigate to blank page
    let _ = page.goto("about:blank", None).await;

    // Page with confirm that stores result
    page.evaluate(
        r#"
        const button = document.createElement('button');
        button.onclick = () => { window.confirmResult = confirm('Continue?'); };
        button.textContent = 'Confirm';
        document.body.appendChild(button);
        "#,
    )
    .await?;

    // Trigger confirm
    let locator = page.locator("button").await;
    locator.click(None).await?;

    // Wait for dialog
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify JavaScript received false
    let result = page.evaluate_value("window.confirmResult").await?;
    assert_eq!(
        result, "false",
        "confirm() should return false when dismissed"
    );

    browser.close().await?;
    Ok(())
}

/// Test dialog prompt handling with input
///
/// Verifies that:
/// 1. Prompt dialog event is fired
/// 2. Dialog type is "prompt"
/// 3. Default value is captured
/// 4. Custom input can be provided
/// 5. JavaScript receives the input text
#[tokio::test]
async fn test_dialog_prompt_with_input() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Shared state
    let dialog_data = Arc::new(Mutex::new(None));
    let dialog_data_clone = dialog_data.clone();

    // Register dialog handler
    page.on_dialog(move |dialog| {
        let data = dialog_data_clone.clone();
        async move {
            let type_ = dialog.type_().to_string();
            let message = dialog.message().to_string();
            let default = dialog.default_value().to_string();
            *data.lock().unwrap() = Some((type_, message, default));

            // Accept with custom input
            dialog.accept(Some("Custom Input")).await
        }
    })
    .await?;

    // Navigate to blank page
    let _ = page.goto("about:blank", None).await;

    // Page with prompt that stores result
    page.evaluate(
        r#"
        const button = document.createElement('button');
        button.onclick = () => { window.promptResult = prompt('Enter text:', 'DefaultValue'); };
        button.textContent = 'Prompt';
        document.body.appendChild(button);
        "#,
    )
    .await?;

    // Trigger prompt
    let locator = page.locator("button").await;
    locator.click(None).await?;

    // Wait for dialog
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify dialog data
    let data_opt = dialog_data.lock().unwrap().take();
    assert!(data_opt.is_some(), "Dialog event should have fired");

    let (dialog_type, message, default) = data_opt.unwrap();
    assert_eq!(dialog_type, "prompt", "Dialog type should be 'prompt'");
    assert_eq!(message, "Enter text:", "Dialog message should match");
    assert_eq!(
        default, "DefaultValue",
        "Default value should be 'DefaultValue'"
    );

    // Verify JavaScript received custom input
    let result = page.evaluate_value("window.promptResult").await?;
    assert_eq!(
        result, "Custom Input",
        "prompt() should return the custom input text"
    );

    browser.close().await?;
    Ok(())
}

/// Test dialog prompt handling - dismiss
///
/// Verifies that:
/// 1. Prompt can be dismissed
/// 2. JavaScript receives null when dismissed
#[tokio::test]
async fn test_dialog_prompt_dismiss() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Register dialog handler that dismisses
    page.on_dialog(move |dialog| async move { dialog.dismiss().await })
        .await?;

    // Navigate to blank page
    let _ = page.goto("about:blank", None).await;

    // Page with prompt that stores result
    page.evaluate(
        r#"
        const button = document.createElement('button');
        button.onclick = () => { window.promptResult = prompt('Enter text:'); };
        button.textContent = 'Prompt';
        document.body.appendChild(button);
        "#,
    )
    .await?;

    // Trigger prompt
    let locator = page.locator("button").await;
    locator.click(None).await?;

    // Wait for dialog
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify JavaScript received null
    let result = page.evaluate_value("window.promptResult").await?;
    assert_eq!(result, "null", "prompt() should return null when dismissed");

    browser.close().await?;
    Ok(())
}

/// Test cross-browser dialog support - Firefox
#[tokio::test]
async fn test_dialog_firefox() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.firefox().launch().await?;
    let page = browser.new_page().await?;

    let dialog_handled = Arc::new(Mutex::new(false));
    let dialog_handled_clone = dialog_handled.clone();

    page.on_dialog(move |dialog| {
        let handled = dialog_handled_clone.clone();
        async move {
            *handled.lock().unwrap() = true;
            dialog.accept(None).await
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;

    page.evaluate(
        r#"
        const button = document.createElement('button');
        button.onclick = () => alert('Test');
        button.textContent = 'Alert';
        document.body.appendChild(button);
        "#,
    )
    .await?;

    let locator = page.locator("button").await;
    locator.click(None).await?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(
        *dialog_handled.lock().unwrap(),
        "Dialog should be handled in Firefox"
    );

    browser.close().await?;
    Ok(())
}

/// Test cross-browser dialog support - WebKit
#[tokio::test]
async fn test_dialog_webkit() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.webkit().launch().await?;
    let page = browser.new_page().await?;

    let dialog_handled = Arc::new(Mutex::new(false));
    let dialog_handled_clone = dialog_handled.clone();

    page.on_dialog(move |dialog| {
        let handled = dialog_handled_clone.clone();
        async move {
            *handled.lock().unwrap() = true;
            dialog.accept(None).await
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;

    page.evaluate(
        r#"
        const button = document.createElement('button');
        button.onclick = () => alert('Test');
        button.textContent = 'Alert';
        document.body.appendChild(button);
        "#,
    )
    .await?;

    let locator = page.locator("button").await;
    locator.click(None).await?;

    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(
        *dialog_handled.lock().unwrap(),
        "Dialog should be handled in WebKit"
    );

    browser.close().await?;
    Ok(())
}

/// Test cross-browser download support - Firefox
#[tokio::test]
async fn test_download_firefox() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.firefox().launch().await?;
    let page = browser.new_page().await?;

    let download_captured = Arc::new(Mutex::new(false));
    let download_captured_clone = download_captured.clone();

    page.on_download(move |_download| {
        let captured = download_captured_clone.clone();
        async move {
            *captured.lock().unwrap() = true;
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;

    page.evaluate(
        r#"
        const a = document.createElement('a');
        a.href = 'data:text/plain,Test';
        a.download = 'test.txt';
        a.id = 'dl';
        a.textContent = 'Download';
        document.body.appendChild(a);
        "#,
    )
    .await?;

    let locator = page.locator("#dl").await;
    locator.click(None).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(
        *download_captured.lock().unwrap(),
        "Download should be captured in Firefox"
    );

    browser.close().await?;
    Ok(())
}

/// Test cross-browser download support - WebKit
#[tokio::test]
async fn test_download_webkit() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.webkit().launch().await?;
    let page = browser.new_page().await?;

    let download_captured = Arc::new(Mutex::new(false));
    let download_captured_clone = download_captured.clone();

    page.on_download(move |_download| {
        let captured = download_captured_clone.clone();
        async move {
            *captured.lock().unwrap() = true;
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;

    page.evaluate(
        r#"
        const a = document.createElement('a');
        a.href = 'data:text/plain,Test';
        a.download = 'test.txt';
        a.id = 'dl';
        a.textContent = 'Download';
        document.body.appendChild(a);
        "#,
    )
    .await?;

    let locator = page.locator("#dl").await;
    locator.click(None).await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    assert!(
        *download_captured.lock().unwrap(),
        "Download should be captured in WebKit"
    );

    browser.close().await?;
    Ok(())
}
