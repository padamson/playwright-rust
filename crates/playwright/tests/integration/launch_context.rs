use crate::test_server::TestServer;
use playwright_rs::Error;
use playwright_rs::api::IgnoreDefaultArgs;
use playwright_rs::protocol::{BrowserContextOptions, Playwright, Viewport};
use tempfile::TempDir;

#[tokio::test]
async fn test_launch_persistent_context_basic() {
    crate::common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_basic: Starting");

    let server = TestServer::start().await;

    // Create temporary directory for user data
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

    // Launch Playwright
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();

    // Launch persistent context with basic options
    tracing::debug!("[TEST] Launching persistent context...");
    let context = chromium
        .launch_persistent_context(&user_data_dir)
        .await
        .expect("Failed to launch persistent context");

    // Verify context was created
    tracing::debug!("[TEST] Context created successfully");

    // Create a page and verify it works
    let page = context.new_page().await.expect("Failed to create page");
    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    // Cleanup
    context.close().await.expect("Failed to close context");
    server.shutdown();
    tracing::debug!("[TEST] test_launch_persistent_context_basic: Complete");
}

#[tokio::test]
async fn test_launch_persistent_context_with_options() {
    crate::common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_with_options: Starting");

    let server = TestServer::start().await;

    // Create temporary directory for user data
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();

    // Create options with viewport and headless
    let options = BrowserContextOptions::builder()
        .viewport(Viewport {
            width: 1280,
            height: 720,
        })
        .build();

    // Launch persistent context with options
    let context = chromium
        .launch_persistent_context_with_options(&user_data_dir, options)
        .await
        .expect("Failed to launch persistent context with options");

    // Verify context works with options
    let page = context.new_page().await.expect("Failed to create page");
    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    // Cleanup
    context.close().await.expect("Failed to close context");
    server.shutdown();
    tracing::debug!("[TEST] test_launch_persistent_context_with_options: Complete");
}

#[tokio::test]
async fn test_launch_persistent_context_app_mode() {
    crate::common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_app_mode: Starting");

    let server = TestServer::start().await;

    // Create temporary directory for user data
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();

    // Launch with app mode args
    let options = BrowserContextOptions::builder()
        .args(vec![format!("--app={}", server.url())])
        .headless(true) // App mode works in headless
        .build();

    let context = chromium
        .launch_persistent_context_with_options(&user_data_dir, options)
        .await
        .expect("Failed to launch persistent context in app mode");

    // Verify context was created
    // Note: In app mode, browser opens directly to the URL
    let _page = context.new_page().await.expect("Failed to create page");

    // Cleanup
    context.close().await.expect("Failed to close context");
    server.shutdown();
    tracing::debug!("[TEST] test_launch_persistent_context_app_mode: Complete");
}

#[tokio::test]
async fn test_launch_persistent_context_storage_persistence() {
    crate::common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_storage_persistence: Starting");

    // Use a single test server — both sessions navigate to the same origin
    // so localStorage persists across context restarts
    let server = TestServer::start().await;
    let url = server.url();

    // Create temporary directory for user data
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();

    // First session: set local storage
    {
        let context = chromium
            .launch_persistent_context(&user_data_dir)
            .await
            .expect("Failed to launch first persistent context");

        let page = context.new_page().await.expect("Failed to create page");
        page.goto(&url, None).await.expect("Failed to navigate");

        // Set local storage value
        page.evaluate_expression("localStorage.setItem('test_key', 'test_value')")
            .await
            .expect("Failed to set local storage");

        context.close().await.expect("Failed to close context");
    }

    // Second session: verify local storage persisted
    {
        let context = chromium
            .launch_persistent_context(&user_data_dir)
            .await
            .expect("Failed to launch second persistent context");

        let page = context.new_page().await.expect("Failed to create page");
        page.goto(&url, None).await.expect("Failed to navigate");

        // Retrieve local storage value
        let stored_value = page
            .evaluate_value("localStorage.getItem('test_key')")
            .await
            .expect("Failed to get local storage");

        assert_eq!(stored_value, "test_value", "Storage did not persist");

        context.close().await.expect("Failed to close context");
    }

    server.shutdown();
    tracing::debug!("[TEST] test_launch_persistent_context_storage_persistence: Complete");
}

#[tokio::test]
async fn launch_persistent_context_creates_a_missing_user_data_dir() {
    crate::common::init_tracing();
    let playwright = Playwright::launch().await.expect("launch Playwright");
    let temp = tempfile::tempdir().expect("create temp dir");
    let user_data_dir = temp.path().join("profiles").join("fresh");

    let context = playwright
        .chromium()
        .launch_persistent_context(user_data_dir.to_str().expect("utf-8 temp path"))
        .await
        .expect("launch_persistent_context with a not-yet-existing directory");

    assert!(
        user_data_dir.is_dir(),
        "the driver should create the user data dir at {}",
        user_data_dir.display()
    );
    context.close().await.expect("close context");
}

#[tokio::test]
async fn launch_persistent_context_fails_when_the_user_data_dir_cannot_be_created() {
    crate::common::init_tracing();
    let playwright = Playwright::launch().await.expect("launch Playwright");
    let temp = tempfile::tempdir().expect("create temp dir");
    // A regular file where a parent directory is needed fails the mkdir on
    // every platform, unlike a root-level path that Windows can create.
    let blocker = temp.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").expect("write blocker file");
    let user_data_dir = blocker.join("profile");

    let err = playwright
        .chromium()
        .launch_persistent_context(user_data_dir.to_str().expect("utf-8 temp path"))
        .await
        .expect_err("a user data dir under a regular file cannot be created");

    assert!(
        matches!(&err, Error::ProtocolError(_)),
        "expected the driver's mkdir failure as ProtocolError, got {err:?}"
    );
}

#[tokio::test]
#[ignore = "launches Firefox and WebKit; runs in CI's --run-ignored lane"]
async fn test_launch_persistent_context_cross_browser() {
    crate::common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_cross_browser: Starting");

    let server = TestServer::start().await;
    let url = server.url();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Test Chromium
    {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

        let chromium = playwright.chromium();
        let context = chromium
            .launch_persistent_context(&user_data_dir)
            .await
            .expect("Failed to launch Chromium persistent context");

        let page = context.new_page().await.expect("Failed to create page");
        page.goto(&url, None)
            .await
            .expect("Failed to navigate in Chromium");

        context.close().await.expect("Failed to close Chromium");
        tracing::info!("✓ Chromium persistent context works");
    }

    // Test Firefox
    {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

        let firefox = playwright.firefox();
        let context = firefox
            .launch_persistent_context(&user_data_dir)
            .await
            .expect("Failed to launch Firefox persistent context");

        let page = context.new_page().await.expect("Failed to create page");
        page.goto(&url, None)
            .await
            .expect("Failed to navigate in Firefox");

        context.close().await.expect("Failed to close Firefox");
        tracing::info!("✓ Firefox persistent context works");
    }

    // Test WebKit (works on macOS/Linux since Playwright 1.58.2).
    //
    // Known Windows limitation (issue #39): WebKit persistent context fails on
    // native Windows with "Initial load failed" in
    // wkPage.js:handleProvisionalLoadFailed. Upstream has no active remedy:
    // playwright#36936 (WebKit crashes on Win10/Server 16) was closed by triage
    // for low engagement, not fixed, and the proposed `webkit-wsl` replacement
    // (playwright#37036) was closed not-planned. Stays gated on Windows until
    // upstream is re-engaged or WebKit Windows builds stabilize.
    if !cfg!(target_os = "windows") {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

        let webkit = playwright.webkit();
        let context = webkit
            .launch_persistent_context(&user_data_dir)
            .await
            .expect("Failed to launch WebKit persistent context");

        let page = context.new_page().await.expect("Failed to create page");
        page.goto(&url, None)
            .await
            .expect("Failed to navigate in WebKit");

        context.close().await.expect("Failed to close WebKit");
        tracing::info!("✓ WebKit persistent context works");
    } else {
        tracing::warn!("Skipping WebKit persistent context test on Windows (issue #39)");
    }

    server.shutdown();
    tracing::debug!("[TEST] test_launch_persistent_context_cross_browser: Complete");
}

#[tokio::test]
async fn test_launch_persistent_context_ignore_default_args_bool() {
    crate::common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_ignore_default_args_bool: Starting");

    let server = TestServer::start().await;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();

    // Launch with ignore_default_args(false) — uses default args (no-op but tests the path)
    // Note: Bool(true) removes ALL default args which can cause browser launch failures
    // in CI environments; the protocol normalization is tested via unit tests.
    let options = BrowserContextOptions::builder()
        .ignore_default_args(IgnoreDefaultArgs::Bool(false))
        .build();

    let context = chromium
        .launch_persistent_context_with_options(&user_data_dir, options)
        .await
        .expect("Failed to launch persistent context with ignore_default_args(false)");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    context.close().await.expect("Failed to close context");
    server.shutdown();
    tracing::debug!("[TEST] test_launch_persistent_context_ignore_default_args_bool: Complete");
}

#[tokio::test]
async fn test_launch_persistent_context_ignore_default_args_array() {
    crate::common::init_tracing();
    tracing::debug!("[TEST] test_launch_persistent_context_ignore_default_args_array: Starting");

    let server = TestServer::start().await;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let user_data_dir = temp_dir.path().to_str().unwrap().to_string();

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let chromium = playwright.chromium();

    // Launch with ignore_default_args filtering specific args
    let options = BrowserContextOptions::builder()
        .ignore_default_args(IgnoreDefaultArgs::Array(vec![
            "--disable-popup-blocking".to_string(),
        ]))
        .build();

    let context = chromium
        .launch_persistent_context_with_options(&user_data_dir, options)
        .await
        .expect("Failed to launch persistent context with ignore_default_args(array)");

    let page = context.new_page().await.expect("Failed to create page");
    page.goto(&server.url(), None)
        .await
        .expect("Failed to navigate");

    context.close().await.expect("Failed to close context");
    server.shutdown();
    tracing::debug!("[TEST] test_launch_persistent_context_ignore_default_args_array: Complete");
}

#[tokio::test]
async fn test_launch_with_artifacts_dir() {
    crate::common::init_tracing();

    use playwright_rs::api::LaunchOptions;

    let pw = Playwright::launch()
        .await
        .expect("Failed to launch playwright");

    // Create a fresh artifacts dir for this run; verify Playwright accepts
    // the option and the launch succeeds. We can't assert exact artifact
    // file placement (Playwright may write into subdirectories) — the
    // contract here is "the option is plumbed through to Node and not
    // rejected as an invalid arg".
    let temp = std::env::temp_dir().join(format!(
        "pw-rust-artifacts-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp).expect("Failed to create temp artifacts dir");

    let options = LaunchOptions::new().artifacts_dir(temp.to_string_lossy().into_owned());
    let browser = pw
        .chromium()
        .launch_with_options(options)
        .await
        .expect("Failed to launch chromium with artifacts_dir");

    // Sanity-check we can do a basic operation with the resulting browser.
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");
    page.goto(
        "data:text/html,<html><body>artifacts test</body></html>",
        None,
    )
    .await
    .expect("Failed to navigate");

    browser.close().await.expect("Failed to close browser");

    // Cleanup the directory; ignore errors (Playwright may have left files).
    let _ = std::fs::remove_dir_all(&temp);
}
