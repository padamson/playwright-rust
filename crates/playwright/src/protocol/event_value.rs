// EventValue — typed return value for the generic expect_event() method.
//
// This enum allows the generic Page::expect_event() / BrowserContext::expect_event()
// API to return a typed value that callers can match on.
//
// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-wait-for-event>

/// Typed value returned by the generic `expect_event()` method on `Page` and `BrowserContext`.
///
/// This enum covers the full set of events supported by `expect_event()`.
/// Each variant wraps the event payload (or carries no data for unit events).
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::{EventValue, Playwright};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let browser = playwright.chromium().launch().await?;
///     let context = browser.new_context().await?;
///     let page = context.new_page().await?;
///
///     let _ = page.goto("about:blank", None).await;
///
///     // Set up the waiter BEFORE the action that triggers the event
///     let waiter = page.expect_event("console", None).await?;
///
///     // Trigger the event
///     page.evaluate::<(), ()>("() => { console.log('hello'); }", None).await?;
///
///     // Resolve and match the result
///     match waiter.wait().await? {
///         EventValue::ConsoleMessage(msg) => println!("Got console: {}", msg.text()),
///         other => panic!("Unexpected: {:?}", other),
///     }
///
///     // Context-level: wait for a new page
///     let waiter = context.expect_event("page", None).await?;
///     let _p = context.new_page().await?;
///     match waiter.wait().await? {
///         EventValue::Page(p) => println!("New page: {}", p.url()),
///         other => panic!("Unexpected: {:?}", other),
///     }
///
///     browser.close().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
#[derive(Clone)]
pub enum EventValue {
    /// A new page was created (popup or context "page" event).
    Page(crate::protocol::page::Page),
    /// A network request was issued.
    Request(crate::protocol::request::Request),
    /// A network response was received.
    Response(crate::protocol::response::ResponseObject),
    /// A file download started.
    Download(crate::protocol::download::Download),
    /// A console message was produced.
    ConsoleMessage(crate::protocol::console_message::ConsoleMessage),
    /// A file chooser dialog was opened.
    FileChooser(crate::protocol::file_chooser::FileChooser),
    /// A web socket connection was opened.
    WebSocket(crate::protocol::web_socket::WebSocket),
    /// A web worker was created.
    Worker(crate::protocol::worker::Worker),
    /// A web error (uncaught exception) was reported — context level.
    WebError(crate::protocol::web_error::WebError),
    /// The page or context was closed (no payload).
    Close,
    /// A frame was attached, detached, or navigated.
    Frame(crate::protocol::frame::Frame),
    /// The page "load" event fired (no payload).
    Load,
    /// The page "crash" event fired (no payload).
    Crash,
    /// An uncaught JS exception was reported — carries the error message.
    PageError(String),
}

impl std::fmt::Debug for EventValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventValue::Page(_) => write!(f, "EventValue::Page(..)"),
            EventValue::Request(_) => write!(f, "EventValue::Request(..)"),
            EventValue::Response(_) => write!(f, "EventValue::Response(..)"),
            EventValue::Download(_) => write!(f, "EventValue::Download(..)"),
            EventValue::ConsoleMessage(m) => {
                write!(f, "EventValue::ConsoleMessage({:?})", m.text())
            }
            EventValue::FileChooser(_) => write!(f, "EventValue::FileChooser(..)"),
            EventValue::WebSocket(_) => write!(f, "EventValue::WebSocket(..)"),
            EventValue::Worker(_) => write!(f, "EventValue::Worker(..)"),
            EventValue::WebError(_) => write!(f, "EventValue::WebError(..)"),
            EventValue::Close => write!(f, "EventValue::Close"),
            EventValue::Frame(_) => write!(f, "EventValue::Frame(..)"),
            EventValue::Load => write!(f, "EventValue::Load"),
            EventValue::Crash => write!(f, "EventValue::Crash"),
            EventValue::PageError(msg) => write!(f, "EventValue::PageError({:?})", msg),
        }
    }
}
