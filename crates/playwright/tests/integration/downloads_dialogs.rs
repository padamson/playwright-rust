use playwright_rs::protocol::Playwright;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

// ============================================================================
// Helper
// ============================================================================

/// Await a `Notify` with a timeout, failing the test on expiry.
async fn notified_or_timeout(notify: &Notify, ms: u64, what: &str) {
    tokio::time::timeout(Duration::from_millis(ms), notify.notified())
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
}

// ============================================================================
// Download Methods
// ============================================================================

/// Test download event handling and save functionality
///
/// Verifies that:
/// 1. Download event is fired when download is triggered
/// 2. Download object provides URL and suggested filename
/// 3. Download can be saved to disk
#[tokio::test]
async fn test_download_methods() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    // Test 1: Basic download event handling
    let download_captured = Arc::new(Mutex::new(None));
    let download_captured_clone = download_captured.clone();

    page.on_download(move |download| {
        let captured = download_captured_clone.clone();
        async move {
            *captured.lock().unwrap() = Some(download);
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;

    page.evaluate_expression(
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

    let locator = page.locator("#download-link").await;
    let waiter = page.expect_download(Some(5000.0)).await?;
    locator.click(None).await?;
    waiter.wait().await.expect("download event did not fire");

    let download_opt = download_captured.lock().unwrap().take();
    assert!(download_opt.is_some(), "Download event should have fired");

    let download = download_opt.unwrap();

    assert!(
        download.url().contains("data:text/plain"),
        "Download URL should be the data URL"
    );
    assert_eq!(
        download.suggested_filename(),
        "test.txt",
        "Suggested filename should be 'test.txt'"
    );

    // Test 2: Save download to file
    let download_captured2 = Arc::new(Mutex::new(None));
    let download_captured2_clone = download_captured2.clone();

    page.on_download(move |download| {
        let captured = download_captured2_clone.clone();
        async move {
            *captured.lock().unwrap() = Some(download);
            Ok(())
        }
    })
    .await?;

    page.evaluate_expression(
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

    let locator = page.locator("#dl").await;
    let waiter2 = page.expect_download(Some(5000.0)).await?;
    locator.click(None).await?;
    waiter2.wait().await.expect("download event did not fire");

    let download_opt = download_captured2.lock().unwrap().take();
    assert!(download_opt.is_some());
    let download = download_opt.unwrap();

    let temp_dir = std::env::temp_dir();
    let save_path = temp_dir.join("playwright_test_download.txt");
    let _ = std::fs::remove_file(&save_path);

    download.save_as(&save_path).await?;

    assert!(
        save_path.exists(),
        "Downloaded file should exist at save path"
    );

    std::fs::remove_file(&save_path)?;

    browser.close().await?;
    Ok(())
}

// ============================================================================
// Dialog Alert Methods
// ============================================================================

/// Test alert dialog handling
///
/// Verifies that:
/// 1. Alert dialog event is fired
/// 2. Dialog type is "alert"
/// 3. Dialog message is captured
/// 4. Dialog can be accepted
#[tokio::test]
async fn test_dialog_alert_methods() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let dialog_info = Arc::new(Mutex::new(None));
    let dialog_info_clone = dialog_info.clone();
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();

    page.on_dialog(move |dialog| {
        let info = dialog_info_clone.clone();
        let n = notify2.clone();
        async move {
            let type_ = dialog.type_().to_string();
            let message = dialog.message().to_string();
            *info.lock().unwrap() = Some((type_, message));
            let result = dialog.accept(None).await;
            n.notify_one();
            result
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;

    page.evaluate_expression(
        r#"
        const button = document.createElement('button');
        button.onclick = () => alert('Hello from alert!');
        button.textContent = 'Alert';
        document.body.appendChild(button);
        "#,
    )
    .await?;

    let locator = page.locator("button").await;
    locator.click(None).await?;

    notified_or_timeout(&notify, 5000, "alert dialog").await;

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

// ============================================================================
// Dialog Confirm Methods
// ============================================================================

/// Test confirm dialog handling - accept and dismiss
///
/// Verifies that:
/// 1. Confirm dialog event is fired
/// 2. Dialog type is "confirm"
/// 3. Dialog can be accepted (returns true)
/// 4. Dialog can be dismissed (returns false)
#[tokio::test]
async fn test_dialog_confirm_methods() -> Result<(), Box<dyn std::error::Error>> {
    let (pw, browser1, page1) = crate::common::setup().await;

    let dialog_info = Arc::new(Mutex::new(None));
    let dialog_info_clone = dialog_info.clone();
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();

    page1
        .on_dialog(move |dialog| {
            let info = dialog_info_clone.clone();
            let n = notify2.clone();
            async move {
                *info.lock().unwrap() = Some(dialog.type_().to_string());
                let result = dialog.accept(None).await;
                n.notify_one();
                result
            }
        })
        .await?;

    let _ = page1.goto("about:blank", None).await;

    page1
        .evaluate_expression(
            r#"
        const button = document.createElement('button');
        button.onclick = () => { window.confirmResult = confirm('Continue?'); };
        button.textContent = 'Confirm';
        document.body.appendChild(button);
        "#,
        )
        .await?;

    let locator = page1.locator("button").await;
    locator.click(None).await?;

    notified_or_timeout(&notify, 5000, "confirm dialog").await;

    let dialog_type = dialog_info.lock().unwrap().take();
    assert_eq!(
        dialog_type,
        Some("confirm".to_string()),
        "Dialog type should be 'confirm'"
    );

    let result = page1.evaluate_value("window.confirmResult").await?;
    assert_eq!(result, "true", "confirm() should return true when accepted");

    browser1.close().await?;

    // Test 2: Confirm dismiss (needs separate browser to avoid handler conflicts)
    let browser2 = pw.chromium().launch().await?;
    let page2 = browser2.new_page().await?;

    let notify3 = Arc::new(Notify::new());
    let notify4 = notify3.clone();

    page2
        .on_dialog(move |dialog| {
            let n = notify4.clone();
            async move {
                let result = dialog.dismiss().await;
                n.notify_one();
                result
            }
        })
        .await?;

    let _ = page2.goto("about:blank", None).await;

    page2
        .evaluate_expression(
            r#"
        const button = document.createElement('button');
        button.onclick = () => { window.confirmResult = confirm('Really?'); };
        button.textContent = 'Confirm';
        document.body.appendChild(button);
        "#,
        )
        .await?;

    let locator = page2.locator("button").await;
    locator.click(None).await?;

    notified_or_timeout(&notify3, 5000, "confirm dismiss dialog").await;

    let result = page2.evaluate_value("window.confirmResult").await?;
    assert_eq!(
        result, "false",
        "confirm() should return false when dismissed"
    );

    browser2.close().await?;
    Ok(())
}

// ============================================================================
// Dialog Prompt Methods
// ============================================================================

/// Test prompt dialog handling with input and dismiss
///
/// Verifies that:
/// 1. Prompt dialog event is fired
/// 2. Dialog type is "prompt"
/// 3. Default value is captured
/// 4. Custom input can be provided (returns input text)
/// 5. Prompt can be dismissed (returns null)
#[tokio::test]
async fn test_dialog_prompt_methods() -> Result<(), Box<dyn std::error::Error>> {
    let (pw, browser1, page1) = crate::common::setup().await;

    let dialog_data = Arc::new(Mutex::new(None));
    let dialog_data_clone = dialog_data.clone();
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();

    page1
        .on_dialog(move |dialog| {
            let data = dialog_data_clone.clone();
            let n = notify2.clone();
            async move {
                let type_ = dialog.type_().to_string();
                let message = dialog.message().to_string();
                let default = dialog.default_value().to_string();
                *data.lock().unwrap() = Some((type_, message, default));
                let result = dialog.accept(Some("Custom Input")).await;
                n.notify_one();
                result
            }
        })
        .await?;

    let _ = page1.goto("about:blank", None).await;

    page1
        .evaluate_expression(
            r#"
        const button = document.createElement('button');
        button.onclick = () => { window.promptResult = prompt('Enter text:', 'DefaultValue'); };
        button.textContent = 'Prompt';
        document.body.appendChild(button);
        "#,
        )
        .await?;

    let locator = page1.locator("button").await;
    locator.click(None).await?;

    notified_or_timeout(&notify, 5000, "prompt dialog").await;

    let data_opt = dialog_data.lock().unwrap().take();
    assert!(data_opt.is_some(), "Dialog event should have fired");

    let (dialog_type, message, default) = data_opt.unwrap();
    assert_eq!(dialog_type, "prompt", "Dialog type should be 'prompt'");
    assert_eq!(message, "Enter text:", "Dialog message should match");
    assert_eq!(
        default, "DefaultValue",
        "Default value should be 'DefaultValue'"
    );

    let result = page1.evaluate_value("window.promptResult").await?;
    assert_eq!(
        result, "Custom Input",
        "prompt() should return the custom input text"
    );

    browser1.close().await?;

    // Test 2: Prompt dismiss (needs separate browser to avoid handler conflicts)
    let browser2 = pw.chromium().launch().await?;
    let page2 = browser2.new_page().await?;

    let notify3 = Arc::new(Notify::new());
    let notify4 = notify3.clone();

    page2
        .on_dialog(move |dialog| {
            let n = notify4.clone();
            async move {
                let result = dialog.dismiss().await;
                n.notify_one();
                result
            }
        })
        .await?;

    let _ = page2.goto("about:blank", None).await;

    page2
        .evaluate_expression(
            r#"
        const button = document.createElement('button');
        button.onclick = () => { window.promptResult = prompt('More text:'); };
        button.textContent = 'Prompt';
        document.body.appendChild(button);
        "#,
        )
        .await?;

    let locator = page2.locator("button").await;
    locator.click(None).await?;

    notified_or_timeout(&notify3, 5000, "prompt dismiss dialog").await;

    let result = page2.evaluate_value("window.promptResult").await?;
    assert_eq!(result, "null", "prompt() should return null when dismissed");

    browser2.close().await?;
    Ok(())
}

// ============================================================================
// Cross-browser Smoke Test
// ============================================================================

/// Test cross-browser support for downloads and dialogs
///
/// Verifies that both downloads and dialogs work in Firefox and WebKit
/// (Rust bindings use the same protocol layer for all browsers,
///  so we don't need exhaustive cross-browser testing for each method)
#[tokio::test]
#[ignore]
async fn test_cross_browser_smoke() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::init_tracing();
    let playwright = Playwright::launch().await?;

    // Test Firefox - dialog
    let firefox = playwright.firefox().launch().await?;
    let firefox_page = firefox.new_page().await?;

    let dialog_handled = Arc::new(Mutex::new(false));
    let dialog_handled_clone = dialog_handled.clone();
    let notify = Arc::new(Notify::new());
    let notify2 = notify.clone();

    firefox_page
        .on_dialog(move |dialog| {
            let handled = dialog_handled_clone.clone();
            let n = notify2.clone();
            async move {
                *handled.lock().unwrap() = true;
                let result = dialog.accept(None).await;
                n.notify_one();
                result
            }
        })
        .await?;

    let _ = firefox_page.goto("about:blank", None).await;

    firefox_page
        .evaluate_expression(
            r#"
        const button = document.createElement('button');
        button.onclick = () => alert('Test');
        button.textContent = 'Alert';
        document.body.appendChild(button);
        "#,
        )
        .await?;

    let locator = firefox_page.locator("button").await;
    locator.click(None).await?;

    notified_or_timeout(&notify, 5000, "Firefox dialog").await;

    assert!(
        *dialog_handled.lock().unwrap(),
        "Dialog should be handled in Firefox"
    );

    firefox.close().await?;

    // Test WebKit - download
    let webkit = playwright.webkit().launch().await?;
    let webkit_page = webkit.new_page().await?;

    let download_captured = Arc::new(Mutex::new(false));
    let download_captured_clone = download_captured.clone();

    webkit_page
        .on_download(move |_download| {
            let captured = download_captured_clone.clone();
            async move {
                *captured.lock().unwrap() = true;
                Ok(())
            }
        })
        .await?;

    let _ = webkit_page.goto("about:blank", None).await;

    webkit_page
        .evaluate_expression(
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

    let locator = webkit_page.locator("#dl").await;
    let waiter = webkit_page.expect_download(Some(5000.0)).await?;
    locator.click(None).await?;
    waiter
        .wait()
        .await
        .expect("WebKit download event did not fire");

    assert!(
        *download_captured.lock().unwrap(),
        "Download should be captured in WebKit"
    );

    webkit.close().await?;
    Ok(())
}
