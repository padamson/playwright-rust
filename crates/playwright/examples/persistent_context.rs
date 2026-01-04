// Example: Persistent Browser Context and App Mode
//
// Demonstrates:
// 1. Launching a browser with persistent storage (user data directory)
// 2. Using app mode to create a standalone application window
// 3. Preserving cookies and local storage across sessions
// 4. Simulating authenticated sessions with storage state
//
// See: https://playwright.dev/docs/api/class-browsertype#browser-type-launch-persistent-context

use playwright_rs::protocol::{BrowserContextOptions, Playwright, Viewport};
use std::error::Error;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter("playwright=debug,persistent_context=info")
        .init();

    // Create temporary directory for user data (in production, use a permanent path)
    let temp_dir = TempDir::new()?;
    let user_data_dir = temp_dir.path().to_str().unwrap();

    println!("=== Example 1: Basic Persistent Context ===");
    example_basic_persistent_context(user_data_dir).await?;

    println!("\n=== Example 2: App Mode (Standalone Window) ===");
    example_app_mode(user_data_dir).await?;

    println!("\n=== Example 3: Storage Persistence ===");
    example_storage_persistence(user_data_dir).await?;

    println!("\n=== Example 4: Custom Options ===");
    example_custom_options(user_data_dir).await?;

    println!("\nAll examples completed successfully!");
    Ok(())
}

/// Example 1: Launch a basic persistent context
async fn example_basic_persistent_context(user_data_dir: &str) -> Result<(), Box<dyn Error>> {
    let playwright = Playwright::launch().await?;
    let chromium = playwright.chromium();

    // Launch persistent context - browser and context created together
    let context = chromium.launch_persistent_context(user_data_dir).await?;

    // Create a page and navigate
    let page = context.new_page().await?;
    page.goto("https://example.com", None).await?;

    println!("  ✓ Navigated to example.com with persistent context");

    // Close context (also closes browser)
    context.close().await?;

    Ok(())
}

/// Example 2: Launch in app mode (standalone window)
async fn example_app_mode(user_data_dir: &str) -> Result<(), Box<dyn Error>> {
    let playwright = Playwright::launch().await?;
    let chromium = playwright.chromium();

    // Create options with app mode
    let options = BrowserContextOptions::builder()
        .args(vec!["--app=https://example.com".to_string()])
        .headless(false) // App mode typically runs headed
        .viewport(Viewport {
            width: 1280,
            height: 720,
        })
        .build();

    // Launch as standalone app window
    let context = chromium
        .launch_persistent_context_with_options(user_data_dir, options)
        .await?;

    println!("  ✓ Launched browser in app mode (standalone window)");
    println!("  Note: Browser opens directly to the URL without address bar");

    // IMPORTANT: In app mode, Playwright creates an initial page automatically.
    // Use context.pages()[0] to access it - don't create a new page!
    let pages = context.pages();
    if !pages.is_empty() {
        let initial_page = &pages[0];
        println!(
            "  ✓ Accessed initial app mode page at: {}",
            initial_page.url()
        );
    } else {
        println!("  ⚠ No initial page found (this shouldn't happen in app mode)");
    }

    // In production, you might keep the app running
    // For this example, we'll close it
    context.close().await?;

    Ok(())
}

/// Example 3: Demonstrate storage persistence
async fn example_storage_persistence(user_data_dir: &str) -> Result<(), Box<dyn Error>> {
    let playwright = Playwright::launch().await?;
    let chromium = playwright.chromium();

    // Session 1: Set some data in local storage
    {
        let context = chromium.launch_persistent_context(user_data_dir).await?;
        let page = context.new_page().await?;
        page.goto("https://example.com", None).await?;

        // Store some data
        page.evaluate_expression(
            "localStorage.setItem('user_pref', 'dark_mode'); localStorage.setItem('last_visit', Date.now().toString());",
        )
        .await?;

        println!("  ✓ Session 1: Stored user preferences");

        context.close().await?;
    }

    // Session 2: Retrieve the stored data
    {
        let context = chromium.launch_persistent_context(user_data_dir).await?;
        let page = context.new_page().await?;
        page.goto("https://example.com", None).await?;

        // Retrieve stored data
        let user_pref = page
            .evaluate_value("localStorage.getItem('user_pref')")
            .await?;
        let last_visit = page
            .evaluate_value("localStorage.getItem('last_visit')")
            .await?;

        println!("  ✓ Session 2: Retrieved stored data");
        println!("    - User preference: {}", user_pref);
        println!("    - Last visit timestamp: {}", last_visit);
        println!("  ✓ Storage persisted across browser sessions!");

        context.close().await?;
    }

    Ok(())
}

/// Example 4: Custom options (viewport, locale, timezone, etc.)
async fn example_custom_options(user_data_dir: &str) -> Result<(), Box<dyn Error>> {
    let playwright = Playwright::launch().await?;
    let chromium = playwright.chromium();

    // Create context with custom settings
    let options = BrowserContextOptions::builder()
        .viewport(Viewport {
            width: 1920,
            height: 1080,
        })
        .locale("en-US".to_string())
        .timezone_id("America/New_York".to_string())
        .headless(true)
        .args(vec!["--no-sandbox".to_string()])
        .build();

    let context = chromium
        .launch_persistent_context_with_options(user_data_dir, options)
        .await?;

    let page = context.new_page().await?;
    page.goto("https://example.com", None).await?;

    println!("  ✓ Launched with custom options:");
    println!("    - Viewport: 1920x1080");
    println!("    - Locale: en-US");
    println!("    - Timezone: America/New_York");
    println!("    - Headless mode with custom args");

    context.close().await?;

    Ok(())
}

// Additional use cases:
//
// 1. Authentication state preservation:
//    - Launch persistent context with user data directory
//    - Log in once (cookies/tokens stored automatically)
//    - All subsequent sessions are authenticated
//
// 2. Standalone applications:
//    - Use --app=URL to create app-like experience
//    - Combine with persistent context for stateful apps
//    - Great for Electron-style applications
//
// 3. Testing with real user profiles:
//    - Point user_data_dir to actual Chrome/Edge profile
//    - Test with real cookies, extensions, settings
//    - WARNING: Use separate automation profile, not your main browser
//
// 4. Cross-session testing:
//    - Test workflows that span multiple browser sessions
//    - Verify data persistence and session management
//    - Test offline/online transitions
