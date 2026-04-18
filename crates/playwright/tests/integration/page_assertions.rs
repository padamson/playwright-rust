// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0

use crate::test_server::TestServer;
use playwright_rs::expect_page;

// ============================================================================
// to_have_title(): match, negation, regex — single browser session
// ============================================================================

#[tokio::test]
async fn test_to_have_title() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&server.url(), None).await?;

    // Exact match
    expect_page(&page).to_have_title("Test Index").await?;

    // Negation
    expect_page(&page)
        .not()
        .to_have_title("Wrong Title")
        .await?;

    // Regex
    expect_page(&page).to_have_title_regex("Test.*").await?;

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// Verify to_have_title times out when the title doesn't match.
#[tokio::test]
async fn test_to_have_title_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&server.url(), None).await?;

    let result = expect_page(&page)
        .with_timeout(std::time::Duration::from_millis(500))
        .to_have_title("Wrong Title")
        .await;

    assert!(result.is_err(), "Should fail when title doesn't match");

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// to_have_url(): match, regex — single browser session
// ============================================================================

#[tokio::test]
async fn test_to_have_url() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let url = server.url();
    page.goto(&url, None).await?;

    // Exact match (browser adds trailing slash)
    expect_page(&page).to_have_url(&format!("{}/", url)).await?;

    // Regex
    expect_page(&page)
        .to_have_url_regex("http://127\\.0\\.0\\.1:\\d+/")
        .await?;

    browser.close().await?;
    server.shutdown();
    Ok(())
}

/// Verify to_have_url times out when the URL doesn't match.
#[tokio::test]
async fn test_to_have_url_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&server.url(), None).await?;

    let result = expect_page(&page)
        .with_timeout(std::time::Duration::from_millis(500))
        .to_have_url("https://wrong.example.com")
        .await;

    assert!(result.is_err(), "Should fail when URL doesn't match");

    browser.close().await?;
    server.shutdown();
    Ok(())
}
