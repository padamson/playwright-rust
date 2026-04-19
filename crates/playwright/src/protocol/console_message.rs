// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// ConsoleMessage - plain data struct constructed from console event params.
//
// ConsoleMessage is NOT a ChannelOwner. It is constructed directly from
// the event params when a "console" event is received.
//
// See: <https://playwright.dev/docs/api/class-consolemessage>

/// The source location of a console message.
///
/// Contains the URL, line number, and column number of the JavaScript
/// code that produced the console message.
///
/// See: <https://playwright.dev/docs/api/class-consolemessage#console-message-location>
#[derive(Clone, Debug)]
pub struct ConsoleMessageLocation {
    /// The URL of the resource that produced the console message.
    pub url: String,
    /// The line number in the resource (1-based).
    pub line_number: i32,
    /// The column number in the resource (0-based).
    pub column_number: i32,
}

/// Represents a console message emitted by a page.
///
/// ConsoleMessage objects are dispatched by the `"console"` event on both
/// [`Page`](crate::protocol::Page) (via `on_console`) and
/// [`BrowserContext`](crate::protocol::BrowserContext) (via `on_console`).
///
/// See: <https://playwright.dev/docs/api/class-consolemessage>
#[derive(Clone, Debug)]
pub struct ConsoleMessage {
    /// The console message type: "log", "error", "warning", "info", "debug", etc.
    type_: String,
    /// The rendered text of the console message.
    text: String,
    /// The source location of the console message.
    location: ConsoleMessageLocation,
    /// Back-reference to the page that produced this message.
    page: Option<crate::protocol::Page>,
    /// The JSHandle arguments passed to the console method.
    args: Vec<std::sync::Arc<crate::protocol::JSHandle>>,
    /// The timestamp when the console message was emitted (milliseconds since Unix epoch).
    timestamp: f64,
}

impl ConsoleMessage {
    /// Creates a new `ConsoleMessage` from event params.
    ///
    /// This is called by `BrowserContext::on_event("console")` when a console
    /// event is received from the Playwright server.
    pub(crate) fn new(
        type_: String,
        text: String,
        location: ConsoleMessageLocation,
        page: Option<crate::protocol::Page>,
        args: Vec<std::sync::Arc<crate::protocol::JSHandle>>,
        timestamp: f64,
    ) -> Self {
        Self {
            type_,
            text,
            location,
            page,
            args,
            timestamp,
        }
    }

    /// Returns the console message type.
    ///
    /// Possible values: `"log"`, `"debug"`, `"info"`, `"error"`, `"warning"`,
    /// `"dir"`, `"dirxml"`, `"table"`, `"trace"`, `"clear"`, `"startGroup"`,
    /// `"startGroupCollapsed"`, `"endGroup"`, `"assert"`, `"profile"`,
    /// `"profileEnd"`, `"count"`, `"timeEnd"`.
    ///
    /// See: <https://playwright.dev/docs/api/class-consolemessage#console-message-type>
    pub fn type_(&self) -> &str {
        &self.type_
    }

    /// Returns the text representation of the console message arguments.
    ///
    /// See: <https://playwright.dev/docs/api/class-consolemessage#console-message-text>
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the source location of the console message.
    ///
    /// See: <https://playwright.dev/docs/api/class-consolemessage#console-message-location>
    pub fn location(&self) -> &ConsoleMessageLocation {
        &self.location
    }

    /// Returns the page that produced the console message, if available.
    ///
    /// May be `None` if the page has already been closed or if the message
    /// originated in a context where the page cannot be resolved.
    ///
    /// See: <https://playwright.dev/docs/api/class-consolemessage#console-message-page>
    pub fn page(&self) -> Option<&crate::protocol::Page> {
        self.page.as_ref()
    }

    /// Returns the timestamp when this console message was emitted.
    ///
    /// The value is the number of milliseconds since the Unix epoch (January 1, 1970 UTC),
    /// as a floating-point number. This matches the value sent by the Playwright server
    /// in the `"console"` event payload.
    ///
    /// See: <https://playwright.dev/docs/api/class-consolemessage#console-message-timestamp>
    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }

    /// Returns the list of arguments passed to the console method.
    ///
    /// Each argument is a [`JSHandle`](crate::protocol::JSHandle) that can be
    /// inspected via `json_value()`, `get_property()`, etc.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use playwright_rs::protocol::Playwright;
    /// # use std::time::Duration;
    /// # use std::sync::{Arc, Mutex};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    /// let page = browser.new_page().await?;
    ///
    /// let captured = Arc::new(Mutex::new(None));
    /// let cap = captured.clone();
    /// page.on_console(move |msg| {
    ///     let cap = cap.clone();
    ///     async move {
    ///         *cap.lock().unwrap() = Some(msg.args().to_vec());
    ///         Ok(())
    ///     }
    /// }).await?;
    ///
    /// page.evaluate_expression("console.log('hello', 42)").await?;
    /// tokio::time::sleep(Duration::from_millis(200)).await;
    ///
    /// let args = captured.lock().unwrap().take().unwrap();
    /// assert_eq!(args.len(), 2);
    /// let first = args[0].json_value().await?;
    /// assert_eq!(first, serde_json::json!("hello"));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-consolemessage#console-message-args>
    pub fn args(&self) -> &[std::sync::Arc<crate::protocol::JSHandle>] {
        &self.args
    }
}
