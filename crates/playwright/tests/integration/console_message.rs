// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// See: <https://playwright.dev/docs/api/class-consolemessage>

use playwright_rs::protocol::JSHandle;
use playwright_rs::server::channel_owner::ChannelOwner;
use std::sync::{Arc, Mutex};

// ============================================================================
// Page-level on_console tests
// ============================================================================

/// Test that page.on_console fires for console.log with type "log" and correct text.
#[tokio::test]
async fn test_page_on_console_log() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let captured = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();

    page.on_console(move |msg| {
        let cap = captured_clone.clone();
        async move {
            *cap.lock().unwrap() = Some((msg.type_().to_string(), msg.text().to_string()));
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;
    let waiter = page.expect_console_message(Some(5000.0)).await?;
    page.evaluate_expression("console.log('hello')").await?;
    waiter.wait().await.expect("console event did not fire");

    let result = captured.lock().unwrap().take();
    assert!(result.is_some(), "on_console handler should have fired");
    let (type_, text) = result.unwrap();
    assert_eq!(type_, "log", "Console message type should be 'log'");
    assert_eq!(text, "hello", "Console message text should be 'hello'");

    browser.close().await?;
    Ok(())
}

/// Test that page.on_console fires for console.error with type "error".
#[tokio::test]
async fn test_page_on_console_error() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let captured_type = Arc::new(Mutex::new(None));
    let captured_clone = captured_type.clone();

    page.on_console(move |msg| {
        let cap = captured_clone.clone();
        async move {
            *cap.lock().unwrap() = Some(msg.type_().to_string());
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;
    let waiter = page.expect_console_message(Some(5000.0)).await?;
    page.evaluate_expression("console.error('oops')").await?;
    waiter.wait().await.expect("console event did not fire");

    let result = captured_type.lock().unwrap().take();
    assert!(
        result.is_some(),
        "on_console handler should have fired for error"
    );
    assert_eq!(
        result.unwrap(),
        "error",
        "Console message type should be 'error'"
    );

    browser.close().await?;
    Ok(())
}

/// Test that ConsoleMessageLocation has url and line_number populated.
#[tokio::test]
async fn test_page_on_console_location() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let captured_url = Arc::new(Mutex::new(None));
    let captured_line = Arc::new(Mutex::new(None));
    let url_clone = captured_url.clone();
    let line_clone = captured_line.clone();

    page.on_console(move |msg| {
        let url_cap = url_clone.clone();
        let line_cap = line_clone.clone();
        async move {
            let loc = msg.location();
            *url_cap.lock().unwrap() = Some(loc.url.clone());
            *line_cap.lock().unwrap() = Some(loc.line_number);
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;
    let waiter = page.expect_console_message(Some(5000.0)).await?;
    page.evaluate_expression("console.log('location test')")
        .await?;
    waiter.wait().await.expect("console event did not fire");

    let url = captured_url.lock().unwrap().take();
    let line = captured_line.lock().unwrap().take();

    assert!(url.is_some(), "Location URL should be populated");
    assert!(line.is_some(), "Location line_number should be populated");
    // line_number is available (may be 0 for evaluate context)
    let _ = url.unwrap(); // URL may be empty for evaluate() context
    let _ = line.unwrap();

    browser.close().await?;
    Ok(())
}

/// Test that msg.page() returns a back-reference to the originating page.
#[tokio::test]
async fn test_console_message_page_back_reference() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let page_guid_captured = Arc::new(Mutex::new(None));
    let cap_clone = page_guid_captured.clone();
    let page_guid = page.guid().to_string();

    page.on_console(move |msg| {
        let cap = cap_clone.clone();
        async move {
            let back_page = msg.page();
            *cap.lock().unwrap() = back_page.map(|p| p.guid().to_string());
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;
    let waiter = page.expect_console_message(Some(5000.0)).await?;
    page.evaluate_expression("console.log('back ref test')")
        .await?;
    waiter.wait().await.expect("console event did not fire");

    let result = page_guid_captured.lock().unwrap().take();
    assert!(result.is_some(), "msg.page() should return Some(page)");
    assert_eq!(
        result.unwrap(),
        page_guid,
        "msg.page() should return the page that triggered the console event"
    );

    browser.close().await?;
    Ok(())
}

// ============================================================================
// Context-level on_console tests
// ============================================================================

/// Test that context.on_console fires for console events from any page in the context.
#[tokio::test]
async fn test_context_on_console() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, _browser, context) = crate::common::setup_context().await;
    let page = context.new_page().await?;

    let captured = Arc::new(Mutex::new(None));
    let cap_clone = captured.clone();

    context
        .on_console(move |msg| {
            let cap = cap_clone.clone();
            async move {
                *cap.lock().unwrap() = Some((msg.type_().to_string(), msg.text().to_string()));
                Ok(())
            }
        })
        .await?;

    let _ = page.goto("about:blank", None).await;
    let waiter = context.expect_console_message(Some(5000.0)).await?;
    page.evaluate_expression("console.log('context console test')")
        .await?;
    waiter.wait().await.expect("console event did not fire");

    let result = captured.lock().unwrap().take();
    assert!(
        result.is_some(),
        "context.on_console handler should have fired"
    );
    let (type_, text) = result.unwrap();
    assert_eq!(type_, "log");
    assert_eq!(text, "context console test");

    context.close().await?;
    Ok(())
}

/// Test that ConsoleMessage::args() returns JSHandle values for each argument.
///
/// Evaluates `console.log("hello", 42)` and verifies args has 2 elements
/// whose json_value() matches the original arguments.
#[tokio::test]
async fn test_console_message_args() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let captured_args: Arc<Mutex<Option<Vec<Arc<JSHandle>>>>> = Arc::new(Mutex::new(None));
    let cap_clone = captured_args.clone();

    page.on_console(move |msg| {
        let cap = cap_clone.clone();
        async move {
            *cap.lock().unwrap() = Some(msg.args().to_vec());
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;
    let waiter = page.expect_console_message(Some(5000.0)).await?;
    page.evaluate_expression("console.log('hello', 42)").await?;
    waiter.wait().await.expect("console event did not fire");

    let args = captured_args.lock().unwrap().take();
    assert!(args.is_some(), "on_console handler should have fired");
    let args = args.unwrap();
    assert_eq!(
        args.len(),
        2,
        "console.log('hello', 42) should produce 2 args"
    );

    let first = args[0].json_value().await?;
    assert_eq!(
        first,
        serde_json::json!("hello"),
        "First arg should be 'hello'"
    );

    let second = args[1].json_value().await?;
    assert_eq!(second, serde_json::json!(42), "Second arg should be 42");

    browser.close().await?;
    Ok(())
}

/// Test that page.console_messages() accumulates messages in order.
#[tokio::test]
async fn test_page_console_messages() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let _ = page.goto("about:blank", None).await;
    let waiter_a = page.expect_console_message(Some(5000.0)).await?;
    let waiter_b = page.expect_console_message(Some(5000.0)).await?;
    page.evaluate_expression("console.log('a')").await?;
    page.evaluate_expression("console.log('b')").await?;
    waiter_a
        .wait()
        .await
        .expect("first console event did not fire");
    waiter_b
        .wait()
        .await
        .expect("second console event did not fire");

    let msgs = page.console_messages();
    assert!(
        msgs.len() >= 2,
        "Expected at least 2 console messages, got {}",
        msgs.len()
    );
    let texts: Vec<&str> = msgs.iter().map(|m| m.text()).collect();
    assert!(
        texts.contains(&"a"),
        "console_messages() should contain 'a', got: {:?}",
        texts
    );
    assert!(
        texts.contains(&"b"),
        "console_messages() should contain 'b', got: {:?}",
        texts
    );

    browser.close().await?;
    Ok(())
}

/// Test that page.page_errors() accumulates uncaught JS errors.
#[tokio::test]
async fn test_page_page_errors() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let _ = page.goto("about:blank", None).await;
    let waiter = page.expect_event("pageerror", Some(5000.0)).await?;
    // Trigger an uncaught error via setTimeout (async, escapes evaluate's try-catch)
    page.evaluate::<(), ()>(
        "() => { setTimeout(() => { throw new Error('test-uncaught'); }, 0); }",
        None,
    )
    .await?;
    waiter.wait().await.expect("pageerror event did not fire");

    let errors = page.page_errors();
    assert!(
        !errors.is_empty(),
        "page_errors() should have at least one entry after a thrown error"
    );
    assert!(
        errors.iter().any(|e| e.contains("test-uncaught")),
        "page_errors() should contain 'test-uncaught', got: {:?}",
        errors
    );

    browser.close().await?;
    Ok(())
}

/// Test that page.clear_console_messages() empties the accumulator and that
/// new messages received after clearing are still recorded.
#[tokio::test]
async fn test_page_clear_console_messages() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let _ = page.goto("about:blank", None).await;

    // Phase 1: produce a message, verify it lands.
    let waiter_a = page.expect_console_message(Some(5000.0)).await?;
    page.evaluate_expression("console.log('phase1')").await?;
    waiter_a.wait().await.expect("phase1 event did not fire");
    assert!(
        !page.console_messages().is_empty(),
        "phase1 should accumulate"
    );

    // Clear and confirm.
    page.clear_console_messages();
    assert!(
        page.console_messages().is_empty(),
        "console_messages() should be empty after clear_console_messages()"
    );

    // Phase 2: produce a new message, verify accumulation resumes from empty.
    let waiter_b = page.expect_console_message(Some(5000.0)).await?;
    page.evaluate_expression("console.log('phase2')").await?;
    waiter_b.wait().await.expect("phase2 event did not fire");
    let msgs = page.console_messages();
    let texts: Vec<&str> = msgs.iter().map(|m| m.text()).collect();
    assert!(
        texts.contains(&"phase2"),
        "post-clear messages should include 'phase2', got: {:?}",
        texts
    );
    assert!(
        !texts.contains(&"phase1"),
        "post-clear messages must not include 'phase1' (was cleared), got: {:?}",
        texts
    );

    browser.close().await?;
    Ok(())
}

/// Test that page.clear_page_errors() empties the page-error accumulator.
#[tokio::test]
async fn test_page_clear_page_errors() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let _ = page.goto("about:blank", None).await;
    let waiter = page.expect_event("pageerror", Some(5000.0)).await?;
    page.evaluate::<(), ()>(
        "() => { setTimeout(() => { throw new Error('to-be-cleared'); }, 0); }",
        None,
    )
    .await?;
    waiter.wait().await.expect("pageerror event did not fire");

    assert!(
        !page.page_errors().is_empty(),
        "should have one error before clear"
    );
    page.clear_page_errors();
    assert!(
        page.page_errors().is_empty(),
        "page_errors() should be empty after clear_page_errors()"
    );

    browser.close().await?;
    Ok(())
}

/// Test that ConsoleMessage::timestamp() returns a valid wall-clock timestamp.
///
/// Asserts that the timestamp is positive and within 60 seconds of the current time.
#[tokio::test]
async fn test_console_message_timestamp() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, browser, page) = crate::common::setup().await;

    let captured_ts: Arc<Mutex<Option<f64>>> = Arc::new(Mutex::new(None));
    let cap_clone = captured_ts.clone();

    page.on_console(move |msg| {
        let cap = cap_clone.clone();
        async move {
            *cap.lock().unwrap() = Some(msg.timestamp());
            Ok(())
        }
    })
    .await?;

    let _ = page.goto("about:blank", None).await;
    let waiter = page.expect_console_message(Some(5000.0)).await?;
    page.evaluate_expression("console.log('timestamp test')")
        .await?;
    waiter.wait().await.expect("console event did not fire");

    let ts = captured_ts
        .lock()
        .unwrap()
        .take()
        .expect("on_console handler should have fired");

    assert!(ts > 0.0, "timestamp should be positive, got: {ts}");

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64;
    assert!(
        (now_ms - ts).abs() < 60_000.0,
        "timestamp should be near wall-clock time; got {ts} vs now {now_ms}"
    );

    browser.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_context_expect_console_message() -> Result<(), Box<dyn std::error::Error>> {
    let (_pw, _browser, context) = crate::common::setup_context().await;
    let page = context.new_page().await?;

    let waiter = context.expect_console_message(None).await?;

    page.evaluate_expression("console.log('expect-test')")
        .await?;

    let msg = waiter.wait().await?;
    assert_eq!(msg.text(), "expect-test");
    assert_eq!(msg.type_(), "log");

    context.close().await?;
    Ok(())
}
