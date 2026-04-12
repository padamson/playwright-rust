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
/// # Known Limitations
///
/// The `args` field (JSHandle references) is not yet supported because
/// `JSHandle` is not implemented. The raw args from the event are ignored.
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
    ) -> Self {
        Self {
            type_,
            text,
            location,
            page,
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
}
