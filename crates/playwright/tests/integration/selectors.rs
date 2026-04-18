// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Reference:
// - Python: playwright-python/playwright/_impl/_selectors.py
// - Docs: https://playwright.dev/docs/api/class-selectors

use super::common;

/// Verify that `playwright.selectors()` returns a usable Selectors object.
#[tokio::test]
async fn test_selectors_accessible_from_playwright() {
    let (playwright, browser, page) = common::setup().await;

    let selectors = playwright.selectors();
    // Selectors should be accessible; verify we can call a method on it
    // (set_test_id_attribute with the default value is a no-op round-trip)
    selectors
        .set_test_id_attribute("data-testid")
        .await
        .expect("Selectors should be accessible and accept set_test_id_attribute");

    page.close().await.unwrap();
    browser.close().await.unwrap();
    playwright.shutdown().await.unwrap();
}

/// Verify that `set_test_id_attribute` changes the attribute used by `get_by_test_id`.
///
/// Steps:
/// 1. Change test ID attribute to `data-custom-id`
/// 2. Navigate to a page with elements having `data-custom-id`
/// 3. Verify `get_by_test_id("my-element")` finds the element
/// 4. Reset to default `data-testid`
#[tokio::test]
async fn test_set_test_id_attribute() {
    let (playwright, browser, page) = common::setup().await;

    let selectors = playwright.selectors();

    // Change test ID attribute to custom value
    selectors
        .set_test_id_attribute("data-custom-id")
        .await
        .expect("set_test_id_attribute should succeed");

    // Navigate to a page with that attribute
    page.goto(
        "data:text/html,<html><body>\
            <button data-custom-id=\"my-button\">Click me</button>\
        </body></html>",
        None,
    )
    .await
    .expect("goto should succeed");

    // get_by_test_id should now use data-custom-id
    let button = page.get_by_test_id("my-button").await;
    let text = button
        .text_content()
        .await
        .expect("text_content should succeed");
    assert_eq!(text, Some("Click me".to_string()));

    // Restore default
    selectors
        .set_test_id_attribute("data-testid")
        .await
        .expect("restoring default test ID attribute should succeed");

    page.close().await.unwrap();
    browser.close().await.unwrap();
    playwright.shutdown().await.unwrap();
}

/// Verify that registering a custom selector engine allows using it in locators.
///
/// Registers a simple tag-based selector engine called "tag" that finds elements
/// by their tag name, then uses it in a locator expression.
#[tokio::test]
async fn test_register_custom_selector() {
    let (playwright, browser, page) = common::setup().await;

    let selectors = playwright.selectors();

    // Register a custom selector engine that finds elements by tag name.
    // The engine must expose `query` and `queryAll` methods.
    let script = r#"
        {
            query(root, selector) {
                return root.querySelector(selector);
            },
            queryAll(root, selector) {
                return Array.from(root.querySelectorAll(selector));
            }
        }
    "#;

    selectors
        .register("tag", script, None)
        .await
        .expect("register should succeed");

    // Navigate to a page with elements
    page.goto(
        "data:text/html,<html><body><article>Hello article</article></body></html>",
        None,
    )
    .await
    .expect("goto should succeed");

    // Use the custom selector engine
    let locator = page.locator("tag=article").await;
    let text = locator
        .text_content()
        .await
        .expect("text_content should succeed");
    assert_eq!(text, Some("Hello article".to_string()));

    page.close().await.unwrap();
    browser.close().await.unwrap();
    playwright.shutdown().await.unwrap();
}

/// Verify that registering a selector with content_script=true works.
#[tokio::test]
async fn test_register_custom_selector_with_content_script() {
    let (playwright, browser, page) = common::setup().await;

    let selectors = playwright.selectors();

    let script = r#"
        {
            query(root, selector) {
                return root.querySelector(selector);
            },
            queryAll(root, selector) {
                return Array.from(root.querySelectorAll(selector));
            }
        }
    "#;

    // Register with content_script=true
    selectors
        .register("cstag", script, Some(true))
        .await
        .expect("register with content_script should succeed");

    page.goto(
        "data:text/html,<html><body><section>Content</section></body></html>",
        None,
    )
    .await
    .expect("goto should succeed");

    let locator = page.locator("cstag=section").await;
    let text = locator
        .text_content()
        .await
        .expect("text_content should succeed");
    assert_eq!(text, Some("Content".to_string()));

    page.close().await.unwrap();
    browser.close().await.unwrap();
    playwright.shutdown().await.unwrap();
}

/// Verify that re-registering a selector engine with the same name returns an error.
#[tokio::test]
async fn test_register_duplicate_selector_name_fails() {
    let (playwright, browser, _page) = common::setup().await;

    let selectors = playwright.selectors();

    let script = r#"
        {
            query(root, selector) { return root.querySelector(selector); },
            queryAll(root, selector) { return Array.from(root.querySelectorAll(selector)); }
        }
    "#;

    selectors
        .register("dup", script, None)
        .await
        .expect("first register should succeed");

    // Second registration with same name should fail
    let result = selectors.register("dup", script, None).await;
    assert!(
        result.is_err(),
        "re-registering a selector engine should return an error"
    );

    browser.close().await.unwrap();
    playwright.shutdown().await.unwrap();
}
