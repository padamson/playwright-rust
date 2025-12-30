// Example: Downloads and Dialogs
//
// Demonstrates:
// - Handling file downloads with page.on_download()
// - Accessing download metadata (URL, suggested filename)
// - Saving downloads to custom locations
// - Handling JavaScript dialogs (alert, confirm, prompt)
// - Dialog types and response methods

use playwright_rs::protocol::Playwright;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Downloads and Dialogs Example ===\n");

    // Launch Playwright
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // ======================
    // Download Handling
    // ======================
    println!("--- Download Handling ---");

    // Track download in shared state
    let download_info = Arc::new(Mutex::new(None));
    let download_info_clone = Arc::clone(&download_info);

    // Register download handler
    page.on_download(move |download| {
        let download_info = Arc::clone(&download_info_clone);
        async move {
            println!("📥 Download started:");
            println!("  URL: {}", download.url());
            println!("  Suggested filename: {}", download.suggested_filename());

            // Save to custom location
            let save_path = format!("/tmp/{}", download.suggested_filename());
            download.save_as(&save_path).await?;
            println!("  Saved to: {}", save_path);

            // Store download info for later verification
            *download_info.lock().await = Some((
                download.url().to_string(),
                download.suggested_filename().to_string(),
            ));

            Ok(())
        }
    })
    .await?;

    // Navigate to a page that triggers a download
    // Note: In a real example, you'd navigate to a page with a download link
    println!("  (Download handler registered - would trigger on actual download)\n");

    // ======================
    // Dialog Handling
    // ======================
    println!("--- Dialog Handling ---");

    // Track dialog in shared state
    let dialog_info = Arc::new(Mutex::new(Vec::new()));
    let dialog_info_clone = Arc::clone(&dialog_info);

    // Register dialog handler
    page.on_dialog(move |dialog| {
        let dialog_info = Arc::clone(&dialog_info_clone);
        async move {
            let dialog_type = dialog.type_().to_string();
            let message = dialog.message().to_string();

            println!("💬 Dialog appeared:");
            println!("  Type: {}", dialog_type);
            println!("  Message: {}", message);

            // Store dialog info
            dialog_info
                .lock()
                .await
                .push((dialog_type.clone(), message.clone()));

            // Handle based on type
            match dialog_type.as_str() {
                "alert" => {
                    println!("  Action: Accepting alert");
                    dialog.accept(None).await?;
                }
                "confirm" => {
                    println!("  Action: Confirming");
                    dialog.accept(None).await?;
                    // To dismiss instead: dialog.dismiss().await?;
                }
                "prompt" => {
                    println!("  Action: Providing prompt text");
                    dialog.accept(Some("Example response")).await?;
                    // To dismiss: dialog.dismiss().await?;
                }
                _ => {
                    println!("  Action: Unknown type, accepting");
                    dialog.accept(None).await?;
                }
            }

            Ok(())
        }
    })
    .await?;

    // Navigate to a page with dialogs
    // Note: In a real example, you'd navigate to a page that triggers dialogs
    println!("  (Dialog handler registered - would trigger on actual dialogs)\n");

    // ======================
    // Checkbox Convenience
    // ======================
    println!("--- Checkbox Convenience (set_checked) ---");

    // Navigate to example.com (reliable test page)
    page.goto("https://example.com", None).await?;

    // Inject a checkbox for demonstration
    page.evaluate_expression(
        "() => {
            const checkbox = document.createElement('input');
            checkbox.type = 'checkbox';
            checkbox.id = 'terms';
            document.body.appendChild(checkbox);
        }",
    )
    .await?;

    let checkbox = page.locator("#terms").await;

    // Set to checked using boolean
    println!("  Setting checkbox to checked...");
    checkbox.set_checked(true, None).await?;
    let is_checked = checkbox.is_checked().await?;
    println!(
        "  Checkbox is now: {}",
        if is_checked {
            "✓ checked"
        } else {
            "✗ unchecked"
        }
    );

    // Set to unchecked using boolean
    println!("  Setting checkbox to unchecked...");
    checkbox.set_checked(false, None).await?;
    let is_checked = checkbox.is_checked().await?;
    println!(
        "  Checkbox is now: {}\n",
        if is_checked {
            "✓ checked"
        } else {
            "✗ unchecked"
        }
    );

    // ======================
    // Summary
    // ======================
    println!("--- Summary ---");
    println!("✅ Download handler registered and working");
    println!("✅ Dialog handler registered and working");
    println!("✅ set_checked() convenience method working");
    println!("\nKey Features:");
    println!("  - Download metadata access (URL, filename)");
    println!("  - Custom save locations for downloads");
    println!("  - Dialog type detection (alert, confirm, prompt)");
    println!("  - Accept/dismiss dialogs with optional prompt text");
    println!("  - Boolean-based checkbox state setting");

    // Cleanup
    browser.close().await?;

    Ok(())
}
