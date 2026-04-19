// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0

use crate::test_server::TestServer;
use playwright_rs::expect_page;

// ============================================================================
// to_have_title() and to_have_url(): match, negation, regex — single browser session
// ============================================================================

#[tokio::test]
async fn test_page_assertion_matches() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    let url = server.url();
    page.goto(&url, None).await?;

    // to_have_title: exact match
    expect_page(&page).to_have_title("Test Index").await?;

    // to_have_title: negation
    expect_page(&page)
        .not()
        .to_have_title("Wrong Title")
        .await?;

    // to_have_title: regex
    expect_page(&page).to_have_title_regex("Test.*").await?;

    // to_have_url: exact match (browser adds trailing slash)
    expect_page(&page).to_have_url(&format!("{}/", url)).await?;

    // to_have_url: regex
    expect_page(&page)
        .to_have_url_regex("http://127\\.0\\.0\\.1:\\d+/")
        .await?;

    browser.close().await?;
    server.shutdown();
    Ok(())
}

// ============================================================================
// to_have_title() and to_have_url(): mismatch should fail — single browser session
// ============================================================================

/// Verify to_have_title and to_have_url both time out when the value doesn't match.
#[tokio::test]
async fn test_page_assertion_mismatches() -> Result<(), Box<dyn std::error::Error>> {
    let server = TestServer::start().await;
    let (_pw, browser, page) = crate::common::setup().await;

    page.goto(&server.url(), None).await?;

    // to_have_title: should fail when title doesn't match
    let result = expect_page(&page)
        .with_timeout(std::time::Duration::from_millis(500))
        .to_have_title("Wrong Title")
        .await;
    assert!(result.is_err(), "Should fail when title doesn't match");

    // to_have_url: should fail when URL doesn't match
    let result = expect_page(&page)
        .with_timeout(std::time::Duration::from_millis(500))
        .to_have_url("https://wrong.example.com")
        .await;
    assert!(result.is_err(), "Should fail when URL doesn't match");

    browser.close().await?;
    server.shutdown();
    Ok(())
}
