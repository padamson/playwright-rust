// Integration tests for BrowserContext runtime setter methods and Page methods.
//
// Tests cover:
// - context.cookies() - retrieve cookies
// - context.clear_cookies() - clear cookies with optional filters
// - context.set_extra_http_headers() - set headers sent with every request
// - context.grant_permissions() - grant browser permissions
// - context.clear_permissions() - clear all permissions
// - context.set_geolocation() - set or clear geolocation
// - context.set_offline() - toggle offline mode at runtime
// - page.bring_to_front() - bring page to front
// - page.viewport_size() - get current viewport dimensions
//
// TDD approach: Tests written FIRST, then implementation.

use crate::test_server::TestServer;
use playwright_rs::protocol::{
    BrowserContextOptions, ClearCookiesOptions, Cookie, Geolocation, GrantPermissionsOptions,
    Playwright, Viewport,
};

// ============================================================================
// context.cookies()
// ============================================================================

#[tokio::test]
async fn test_context_cookies_retrieve() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Add a cookie via add_cookies
    let cookie = Cookie {
        name: "test_cookie".to_string(),
        value: "test_value".to_string(),
        domain: "example.com".to_string(),
        path: "/".to_string(),
        expires: -1.0,
        http_only: false,
        secure: false,
        same_site: Some("Lax".to_string()),
    };
    context
        .add_cookies(&[cookie])
        .await
        .expect("Failed to add cookies");

    // Retrieve cookies
    let cookies = context.cookies(None).await.expect("Failed to get cookies");

    // Verify our cookie is present
    let found = cookies.iter().find(|c| c.name == "test_cookie");
    assert!(found.is_some(), "Cookie should be found");
    let found = found.unwrap();
    assert_eq!(found.value, "test_value");
    assert_eq!(found.domain, "example.com");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_cookies_with_url_filter() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Add cookies for two different domains
    let cookie1 = Cookie {
        name: "alpha_cookie".to_string(),
        value: "alpha_value".to_string(),
        domain: "example.com".to_string(),
        path: "/".to_string(),
        expires: -1.0,
        http_only: false,
        secure: false,
        same_site: None,
    };
    let cookie2 = Cookie {
        name: "beta_cookie".to_string(),
        value: "beta_value".to_string(),
        domain: "playwright.dev".to_string(),
        path: "/".to_string(),
        expires: -1.0,
        http_only: false,
        secure: false,
        same_site: None,
    };
    context
        .add_cookies(&[cookie1, cookie2])
        .await
        .expect("Failed to add cookies");

    // Filter by URL - only example.com cookies
    let cookies = context
        .cookies(Some(&["https://example.com"]))
        .await
        .expect("Failed to get cookies");

    let has_alpha = cookies.iter().any(|c| c.name == "alpha_cookie");
    let has_beta = cookies.iter().any(|c| c.name == "beta_cookie");
    assert!(has_alpha, "Should have example.com cookie");
    assert!(
        !has_beta,
        "Should NOT have playwright.dev cookie when filtering by example.com"
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_cookies_empty_initially() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // New context should have no cookies
    let cookies = context.cookies(None).await.expect("Failed to get cookies");
    assert!(cookies.is_empty(), "New context should have no cookies");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// context.clear_cookies()
// ============================================================================

#[tokio::test]
async fn test_context_clear_cookies_all() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Add some cookies
    let cookies = vec![
        Cookie {
            name: "cookie_one".to_string(),
            value: "value_one".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: None,
        },
        Cookie {
            name: "cookie_two".to_string(),
            value: "value_two".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: None,
        },
    ];
    context
        .add_cookies(&cookies)
        .await
        .expect("Failed to add cookies");

    // Verify cookies were added
    let before = context.cookies(None).await.expect("Failed to get cookies");
    assert_eq!(before.len(), 2, "Should have 2 cookies before clear");

    // Clear all cookies
    context
        .clear_cookies(None)
        .await
        .expect("Failed to clear cookies");

    // Verify cookies are gone
    let after = context.cookies(None).await.expect("Failed to get cookies");
    assert!(
        after.is_empty(),
        "Should have no cookies after clear_cookies"
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_clear_cookies_with_name_filter() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Add two cookies
    let cookies = vec![
        Cookie {
            name: "keep_me".to_string(),
            value: "keep".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: None,
        },
        Cookie {
            name: "delete_me".to_string(),
            value: "delete".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: None,
        },
    ];
    context
        .add_cookies(&cookies)
        .await
        .expect("Failed to add cookies");

    // Clear only the "delete_me" cookie by name
    let options = ClearCookiesOptions {
        name: Some("delete_me".to_string()),
        domain: None,
        path: None,
    };
    context
        .clear_cookies(Some(options))
        .await
        .expect("Failed to clear cookies by name");

    // Verify "keep_me" remains but "delete_me" is gone
    let after = context.cookies(None).await.expect("Failed to get cookies");
    let has_keep = after.iter().any(|c| c.name == "keep_me");
    let has_delete = after.iter().any(|c| c.name == "delete_me");
    assert!(has_keep, "keep_me cookie should still exist");
    assert!(!has_delete, "delete_me cookie should have been cleared");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// context.set_extra_http_headers()
// ============================================================================

#[tokio::test]
async fn test_context_set_extra_http_headers() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");

    // Set a custom header on the context
    let mut headers = std::collections::HashMap::new();
    headers.insert(
        "x-custom-header".to_string(),
        "custom-value-123".to_string(),
    );
    context
        .set_extra_http_headers(headers)
        .await
        .expect("Failed to set extra HTTP headers");

    // Navigate to the echo-headers endpoint
    page.goto(&format!("{}/echo-headers", server.url()), None)
        .await
        .expect("Failed to navigate");

    // Read the echoed headers from the page
    let headers_json = page
        .evaluate_value("document.getElementById('headers').textContent")
        .await
        .expect("Failed to evaluate headers");

    assert!(
        headers_json.contains("x-custom-header"),
        "Custom header name should be present in request. Got: {}",
        headers_json
    );
    assert!(
        headers_json.contains("custom-value-123"),
        "Custom header value should be present in request. Got: {}",
        headers_json
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_context_set_extra_http_headers_multiple() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");

    // Set multiple custom headers
    let mut headers = std::collections::HashMap::new();
    headers.insert("x-header-one".to_string(), "value-one".to_string());
    headers.insert("x-header-two".to_string(), "value-two".to_string());
    context
        .set_extra_http_headers(headers)
        .await
        .expect("Failed to set extra HTTP headers");

    page.goto(&format!("{}/echo-headers", server.url()), None)
        .await
        .expect("Failed to navigate");

    let headers_json = page
        .evaluate_value("document.getElementById('headers').textContent")
        .await
        .expect("Failed to evaluate headers");

    assert!(
        headers_json.contains("x-header-one"),
        "First header should be present. Got: {}",
        headers_json
    );
    assert!(
        headers_json.contains("x-header-two"),
        "Second header should be present. Got: {}",
        headers_json
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// context.grant_permissions() and context.clear_permissions()
// ============================================================================

#[tokio::test]
async fn test_context_grant_permissions() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Grant geolocation permission - should not error
    context
        .grant_permissions(&["geolocation"], None)
        .await
        .expect("Failed to grant geolocation permission");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_grant_permissions_with_origin() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Grant geolocation permission for a specific origin
    let options = GrantPermissionsOptions {
        origin: Some("https://example.com".to_string()),
    };
    context
        .grant_permissions(&["geolocation"], Some(options))
        .await
        .expect("Failed to grant permission with origin");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_grant_and_clear_permissions() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Grant notifications permission
    context
        .grant_permissions(&["notifications"], None)
        .await
        .expect("Failed to grant notifications");

    // Clear all permissions - should not error
    context
        .clear_permissions()
        .await
        .expect("Failed to clear permissions");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_grant_multiple_permissions() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Grant multiple permissions at once
    context
        .grant_permissions(&["geolocation", "notifications"], None)
        .await
        .expect("Failed to grant multiple permissions");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// context.set_geolocation()
// ============================================================================

#[tokio::test]
async fn test_context_set_geolocation() {
    crate::common::init_tracing();
    // Geolocation requires a secure context. localhost is treated as secure by Chromium.
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Must grant permission before location can be read
    context
        .grant_permissions(&["geolocation"], None)
        .await
        .expect("Failed to grant geolocation");

    let page = context.new_page().await.expect("Failed to create page");

    // Navigate to localhost so we are in a secure context (localhost is always trusted)
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Failed to navigate to test server");

    // Set a specific geolocation (Eiffel Tower)
    context
        .set_geolocation(Some(Geolocation {
            latitude: 48.8584,
            longitude: 2.2945,
            accuracy: Some(10.0),
        }))
        .await
        .expect("Failed to set geolocation");

    // Read position via JS
    let lat = page
        .evaluate_value(
            r#"new Promise(resolve => {
                navigator.geolocation.getCurrentPosition(
                    pos => resolve(pos.coords.latitude.toFixed(4)),
                    err => resolve('error:' + err.message)
                )
            })"#,
        )
        .await
        .expect("Failed to evaluate geolocation");

    assert_eq!(lat, "48.8584", "Latitude should match set value");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_context_set_geolocation_clear() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Set geolocation first
    context
        .set_geolocation(Some(Geolocation {
            latitude: 40.7128,
            longitude: -74.0060,
            accuracy: None,
        }))
        .await
        .expect("Failed to set geolocation");

    // Clear geolocation by passing None
    context
        .set_geolocation(None)
        .await
        .expect("Failed to clear geolocation");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// context.set_offline()
// ============================================================================

#[tokio::test]
async fn test_context_set_offline_blocks_navigation() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");

    // Set context offline
    context
        .set_offline(true)
        .await
        .expect("Failed to set offline");

    // Navigation to external URL should fail
    let result = page.goto("https://example.com", None).await;
    assert!(result.is_err(), "Navigation should fail when offline");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_context_set_offline_then_online() {
    crate::common::init_tracing();
    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");

    // Set offline
    context
        .set_offline(true)
        .await
        .expect("Failed to set offline");

    // Set back online
    context
        .set_offline(false)
        .await
        .expect("Failed to set back online");

    // Navigation should work now (use local test server to avoid external network)
    page.goto(&format!("{}/", server.url()), None)
        .await
        .expect("Navigation should succeed after going back online");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// page.bring_to_front()
// ============================================================================

#[tokio::test]
async fn test_page_bring_to_front() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");

    // Create two pages
    let page1 = context.new_page().await.expect("Failed to create page 1");
    let _page2 = context.new_page().await.expect("Failed to create page 2");

    // Bring page1 to front - should not error
    page1
        .bring_to_front()
        .await
        .expect("Failed to bring page to front");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// page.viewport_size()
// ============================================================================

#[tokio::test]
async fn test_page_viewport_size_with_viewport() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create context with specific viewport
    let options = BrowserContextOptions::builder()
        .viewport(Viewport {
            width: 1280,
            height: 720,
        })
        .build();
    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");

    // viewport_size() should return the configured viewport
    let viewport = page.viewport_size();
    assert!(
        viewport.is_some(),
        "viewport_size() should return Some when viewport is set"
    );
    let vp = viewport.unwrap();
    assert_eq!(vp.width, 1280, "Width should be 1280");
    assert_eq!(vp.height, 720, "Height should be 720");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_viewport_size_no_viewport() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create context with no viewport emulation
    let options = BrowserContextOptions::builder().no_viewport(true).build();
    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");

    // With no viewport, viewport_size() should return None
    let viewport = page.viewport_size();
    assert!(
        viewport.is_none(),
        "viewport_size() should return None when no_viewport is set"
    );

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_page_viewport_size_after_set() {
    crate::common::init_tracing();
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    let options = BrowserContextOptions::builder()
        .viewport(Viewport {
            width: 800,
            height: 600,
        })
        .build();
    let context = browser
        .new_context_with_options(options)
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");

    // Initial size
    let initial = page.viewport_size().expect("Should have viewport");
    assert_eq!(initial.width, 800);
    assert_eq!(initial.height, 600);

    // Change viewport
    page.set_viewport_size(Viewport {
        width: 1920,
        height: 1080,
    })
    .await
    .expect("Failed to set viewport size");

    // viewport_size() should reflect the updated size
    let updated = page
        .viewport_size()
        .expect("Should have viewport after set");
    assert_eq!(updated.width, 1920, "Width should be updated to 1920");
    assert_eq!(updated.height, 1080, "Height should be updated to 1080");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}
