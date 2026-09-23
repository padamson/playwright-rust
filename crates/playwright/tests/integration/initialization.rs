use playwright_rs::server::{
    channel_owner::ChannelOwner, connection::Connection, playwright_server::PlaywrightServer,
    transport::PipeTransport,
};
use std::sync::Arc;

/// Test the complete initialization flow with a real Playwright server
#[tokio::test]
async fn test_initialize_playwright_with_real_server() {
    crate::common::init_tracing();

    // 1. Launch server
    let mut server = PlaywrightServer::launch()
        .await
        .expect("launch the Playwright server");

    // 2. Create transport from stdio pipes
    let stdin = server.process.stdin.take().expect("Failed to take stdin");
    let stdout = server.process.stdout.take().expect("Failed to take stdout");

    let (transport, message_rx) = PipeTransport::new(stdin, stdout);
    let (sender, receiver) = transport.into_parts();

    // 3. Create connection
    let connection: Arc<Connection> = Arc::new(Connection::new(sender, receiver, message_rx));

    // 4. Spawn connection message loop
    let conn_for_loop = Arc::clone(&connection);
    tokio::spawn(async move {
        conn_for_loop.run().await;
    });

    // 5. Initialize Playwright
    let playwright_obj = connection
        .initialize_playwright()
        .await
        .expect("initialize Playwright");

    // 6. Downcast to Playwright type
    use playwright_rs::protocol::Playwright;
    let playwright = playwright_obj
        .as_any()
        .downcast_ref::<Playwright>()
        .expect("Failed to downcast to Playwright");

    // 7. Verify Playwright object exists
    assert_eq!(playwright.guid(), "Playwright");

    // 8. Verify browser types are accessible
    let chromium = playwright.chromium();
    assert_eq!(chromium.name(), "chromium");
    assert!(!chromium.executable_path().is_empty());

    let firefox = playwright.firefox();
    assert_eq!(firefox.name(), "firefox");
    assert!(!firefox.executable_path().is_empty());

    let webkit = playwright.webkit();
    assert_eq!(webkit.name(), "webkit");
    assert!(!webkit.executable_path().is_empty());

    // Clean up
    let _ = server.shutdown().await;

    tracing::info!("✓ Server launched successfully");
    tracing::info!("✓ Connection created successfully");
    tracing::info!("✓ Playwright initialized successfully");
    tracing::info!("✓ All three browser types accessible");
}
