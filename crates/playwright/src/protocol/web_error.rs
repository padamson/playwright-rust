// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// WebError - plain data struct constructed from the pageError event params.
//
// WebError is NOT a ChannelOwner. It is constructed inline in
// BrowserContext::on_event("pageError") when the context-level weberror event
// is dispatched.
//
// See: <https://playwright.dev/docs/api/class-weberror>

/// Represents an uncaught JavaScript exception thrown on any page in a browser context.
///
/// `WebError` is the context-level companion to the page-level `on_pageerror` event.
/// It wraps the error message alongside an optional back-reference to the [`Page`](crate::protocol::Page)
/// that threw the error.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::Playwright;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let browser = playwright.chromium().launch().await?;
///     let context = browser.new_context().await?;
///
///     context
///         .on_weberror(|web_error| async move {
///             println!(
///                 "Uncaught error on page {:?}: {}",
///                 web_error.page().map(|p| p.url()),
///                 web_error.error()
///             );
///             Ok(())
///         })
///         .await?;
///
///     let page = context.new_page().await?;
///     page.goto("about:blank", None).await?;
///     // Trigger an uncaught error asynchronously
///     let _ = page
///         .evaluate_expression("setTimeout(() => { throw new Error('boom') }, 0)")
///         .await;
///
///     browser.close().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-weberror>
#[derive(Clone, Debug)]
pub struct WebError {
    /// The page that threw the error, if still available.
    page: Option<crate::protocol::Page>,
    /// The error message extracted from the uncaught exception.
    error: String,
}

impl WebError {
    /// Creates a new `WebError` from event params.
    ///
    /// Called by `BrowserContext::on_event("pageError")` when the context-level
    /// weberror dispatch path fires.
    pub(crate) fn new(error: String, page: Option<crate::protocol::Page>) -> Self {
        Self { page, error }
    }

    /// Returns the page that produced this error, if available.
    ///
    /// May be `None` if the page has already been closed or the page reference
    /// could not be resolved from the connection registry.
    ///
    /// See: <https://playwright.dev/docs/api/class-weberror#web-error-page>
    pub fn page(&self) -> Option<&crate::protocol::Page> {
        self.page.as_ref()
    }

    /// Returns the error message of the uncaught JavaScript exception.
    ///
    /// See: <https://playwright.dev/docs/api/class-weberror#web-error-error>
    pub fn error(&self) -> &str {
        &self.error
    }
}
