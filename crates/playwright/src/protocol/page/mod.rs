// Page protocol object
//
// Represents a web page within a browser context.
// Pages are isolated tabs or windows within a context.

use crate::error::Result;
use crate::protocol::browser_context::Viewport;
use crate::protocol::{Dialog, Download, Request, ResponseObject, Route, WebSocket, Worker};
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex, RwLock};

use crate::protocol::event_registry::EventRegistry;

/// Page represents a web page within a browser context.
///
/// A Page is created when you call `BrowserContext::new_page()` or `Browser::new_page()`.
/// Each page is an isolated tab/window within its parent context.
///
/// A new page starts at "about:blank"; the navigation methods take it elsewhere.
///
/// # Example
///
/// The lifecycle every test follows: open a page, navigate, read through a
/// locator, close. Each method group below (navigation, evaluation, input,
/// events, network, bindings, capture, emulation) carries its own example.
///
/// ```no_run
/// # use playwright_rs::Playwright;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let playwright = Playwright::launch().await?;
/// let browser = playwright.chromium().launch().await?;
/// let page = browser.new_page().await?;
///
/// page.goto("https://example.com", None).await?;
/// let heading = page.locator("h1").text_content().await?;
/// println!("{}: {:?}", page.title().await?, heading);
///
/// page.close().await?;
/// browser.close().await?;
/// # Ok(())
/// # }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-page>
#[derive(Clone)]
pub struct Page {
    base: ChannelOwnerImpl,
    /// The page's main frame, resolved once at construction (the protocol
    /// guarantees the Frame object exists before the Page that references it)
    main_frame: crate::protocol::Frame,
    /// Route handlers for network interception
    route_handlers: Arc<Mutex<Vec<RouteHandlerEntry>>>,
    /// Download event handlers and one-shot `expect_download` waiters.
    download: Arc<EventRegistry<Download>>,
    /// Dialog event handlers (no `expect_*`; waiter queue stays empty).
    dialog: Arc<EventRegistry<Dialog>>,
    dialog_closed: Arc<EventRegistry<Dialog>>,
    /// Request event handlers and one-shot `expect_request` waiters.
    request: Arc<EventRegistry<Request>>,
    /// RequestFinished event handlers (the event has no `expect_*`, so its
    /// registry's waiter queue simply stays empty).
    request_finished: Arc<EventRegistry<Request>>,
    /// RequestFailed event handlers (no `expect_*`; waiter queue stays empty).
    request_failed: Arc<EventRegistry<Request>>,
    /// Response event handlers and one-shot `expect_response` waiters.
    response: Arc<EventRegistry<ResponseObject>>,
    /// WebSocket event handlers
    websocket_handlers: Arc<Mutex<Vec<WebSocketHandler>>>,
    /// WebSocketRoute handlers for route_web_socket()
    ws_route_handlers: Arc<Mutex<Vec<WsRouteHandlerEntry>>>,
    /// Current viewport size (None when no_viewport is set).
    /// Updated by set_viewport_size().
    viewport: Arc<RwLock<Option<Viewport>>>,
    /// Whether this page has been closed.
    /// Set to true when close() is called or a "close" event is received.
    is_closed: Arc<AtomicBool>,
    /// Default timeout for actions (milliseconds), stored as f64 bits.
    default_timeout_ms: Arc<AtomicU64>,
    /// Default timeout for navigation operations (milliseconds), stored as f64 bits.
    default_navigation_timeout_ms: Arc<AtomicU64>,
    /// Page-level binding callbacks registered via expose_function / expose_binding
    binding_callbacks: Arc<Mutex<HashMap<String, PageBindingCallback>>>,
    /// Screencast frame handlers
    screencast_frame_handlers: Arc<Mutex<Vec<ScreencastFrameHandler>>>,
    /// Active screencast Artifact GUID (set when `screencastStart` was
    /// called with a path; cleared on `screencastStop`).
    screencast_artifact_guid: Arc<Mutex<Option<String>>>,
    /// Path to save the screencast Artifact to on stop.
    screencast_save_path: Arc<Mutex<Option<std::path::PathBuf>>>,
    /// FileChooser event handlers and one-shot `expect_file_chooser` waiters.
    filechooser: Arc<EventRegistry<crate::protocol::FileChooser>>,
    /// Console event handlers and one-shot `expect_console_message` waiters.
    console: Arc<EventRegistry<crate::protocol::ConsoleMessage>>,
    /// `close` event: one-time transition; `dispatch_all` wakes every waiter.
    close: Arc<EventRegistry<()>>,
    /// `load` event: one-time transition; `dispatch_all` wakes every waiter.
    load: Arc<EventRegistry<()>>,
    /// `crash` event: one-time transition; `dispatch_all` wakes every waiter.
    crash: Arc<EventRegistry<()>>,
    /// `pageError` event: handlers and one-shot waiters.
    pageerror: Arc<EventRegistry<String>>,
    /// Popup event handlers and one-shot `expect_popup` waiters.
    popup: Arc<EventRegistry<Page>>,
    /// `frameAttached` event: handlers and one-shot waiters.
    frameattached: Arc<EventRegistry<crate::protocol::Frame>>,
    /// `frameDetached` event: handlers and one-shot waiters.
    framedetached: Arc<EventRegistry<crate::protocol::Frame>>,
    /// `frameNavigated` event: handlers and one-shot waiters.
    framenavigated: Arc<EventRegistry<crate::protocol::Frame>>,
    /// worker event handlers (fires when a web worker is created in the page)
    worker_handlers: Arc<Mutex<Vec<WorkerHandler>>>,
    /// One-shot senders waiting for the next "worker" event (expect_event("worker"))
    worker_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<crate::protocol::Worker>>>>,
    /// Accumulated console messages received so far (appended by trigger_console_event)
    console_messages_log: Arc<Mutex<Vec<crate::protocol::ConsoleMessage>>>,
    /// Accumulated uncaught JS error messages received so far (appended by trigger_pageerror_event)
    page_errors_log: Arc<Mutex<Vec<String>>>,
    /// Active web workers tracked via "worker" events (appended on creation)
    workers_list: Arc<Mutex<Vec<Worker>>>,
    /// Video object — Some when this page was created in a record_video context.
    /// The inner Video is created eagerly on Page construction; the underlying
    /// Artifact GUID is read from the Page initializer and resolved asynchronously.
    video: Option<crate::protocol::Video>,
    /// Registered locator handlers: maps uid -> (selector, handler fn, times_remaining)
    /// times_remaining is None when the handler should run indefinitely.
    locator_handlers: Arc<Mutex<Vec<LocatorHandlerEntry>>>,
}

/// Type alias for boxed route handler future
type RouteHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed websocket handler future
type WebSocketHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed WebSocketRoute handler future
type WebSocketRouteHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Storage for a single WebSocket route handler entry
#[derive(Clone)]
struct WsRouteHandlerEntry {
    pattern: String,
    handler:
        Arc<dyn Fn(crate::protocol::WebSocketRoute) -> WebSocketRouteHandlerFuture + Send + Sync>,
}

/// Storage for a single route handler
#[derive(Clone)]
struct RouteHandlerEntry {
    pattern: String,
    handler: Arc<dyn Fn(Route) -> RouteHandlerFuture + Send + Sync>,
}

/// WebSocket event handler
type WebSocketHandler = Arc<dyn Fn(WebSocket) -> WebSocketHandlerFuture + Send + Sync>;

/// Type alias for boxed screencast frame handler future
type ScreencastFrameHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Screencast frame handler
type ScreencastFrameHandler =
    Arc<dyn Fn(crate::protocol::ScreencastFrame) -> ScreencastFrameHandlerFuture + Send + Sync>;

/// Type alias for boxed worker handler future
type WorkerHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// worker event handler — receives the new Worker
type WorkerHandler = Arc<dyn Fn(crate::protocol::Worker) -> WorkerHandlerFuture + Send + Sync>;

/// Type alias for boxed page-level binding callback future
type PageBindingCallbackFuture = Pin<Box<dyn Future<Output = serde_json::Value> + Send>>;

/// Page-level binding callback: receives deserialized JS args, returns a JSON value
type PageBindingCallback =
    Arc<dyn Fn(Vec<serde_json::Value>) -> PageBindingCallbackFuture + Send + Sync>;

/// Type alias for boxed locator handler future
type LocatorHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Locator handler callback: receives the matching Locator
type LocatorHandlerFn = Arc<dyn Fn(crate::protocol::Locator) -> LocatorHandlerFuture + Send + Sync>;

/// Entry in the locator handler registry
struct LocatorHandlerEntry {
    uid: u32,
    selector: String,
    handler: LocatorHandlerFn,
    /// Remaining invocations; `None` means unlimited.
    times_remaining: Option<u32>,
}

impl LocatorHandlerEntry {
    /// Accounts for one trigger from the server and says what to do with it:
    /// whether to run the handler, and whether this was its last invocation
    /// so the entry is removed and the server told to stop watching.
    fn take_invocation(&mut self) -> Invocation {
        match self.times_remaining {
            // A counted handler whose last run failed keeps its entry at zero;
            // the next trigger removes it without running, as upstream does.
            Some(0) => Invocation {
                run: false,
                remove: true,
            },
            Some(ref mut n) => {
                *n -= 1;
                Invocation {
                    run: true,
                    remove: *n == 0,
                }
            }
            None => Invocation {
                run: true,
                remove: false,
            },
        }
    }
}

/// What a `locatorHandlerTriggered` event should do with its entry.
#[derive(Debug, PartialEq, Eq)]
struct Invocation {
    run: bool,
    remove: bool,
}
// Each concern is its own `impl Page` block in a child module. Rustdoc lists
// inherent impls in declaration order, so these follow the lifecycle block
// above in the order a reader meets the API. One comment per line on purpose:
// rustfmt sorts adjacent `mod` lines alphabetically.
// Construction, lifecycle, timeouts, frames and locator constructors.
mod lifecycle;
// Navigation.
mod navigation;
// Script evaluation and element queries.
mod evaluate;
// Keyboard, mouse and touchscreen.
mod input;
// Event subscriptions and waiters.
mod events;
// Routes and network interception.
mod network;
// Exposed functions and locator handlers.
mod bindings;
// Screenshots, PDF, screencast, coverage.
mod capture;
// Media, viewport, injected tags.
mod emulation;
// The navigation `Response` wrapper.
mod response;
// Server-to-registry event dispatch, last because it is internal.
mod dispatch;

pub use bindings::AddLocatorHandlerOptions;
pub use capture::{PdfMargin, PdfOptions, PdfOptionsBuilder};
pub use emulation::{
    AddScriptTagOptions, AddScriptTagOptionsBuilder, AddStyleTagOptions, AddStyleTagOptionsBuilder,
    ColorScheme, EmulateMediaOptions, EmulateMediaOptionsBuilder, ForcedColors, Media,
    ReducedMotion,
};
pub use navigation::{GotoOptions, WaitUntil};
pub use network::RouteFromHarOptions;
pub use response::Response;

impl std::fmt::Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Page")
            .field("guid", &self.guid())
            .field("url", &self.url())
            .finish()
    }
}

/// Shared helper: store timeout locally and notify the Playwright server.
/// Used by both Page and BrowserContext timeout setters.
pub(crate) async fn set_timeout_and_notify(
    channel: &crate::server::channel::Channel,
    method: &str,
    timeout: f64,
) {
    if let Err(e) = channel
        .send_no_result(method, serde_json::json!({ "timeout": timeout }))
        .await
    {
        tracing::warn!("{} send error: {}", method, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(times: Option<u32>) -> LocatorHandlerEntry {
        LocatorHandlerEntry {
            uid: 1,
            selector: "#x".into(),
            handler: Arc::new(|_| Box::pin(async { Ok(()) })),
            times_remaining: times,
        }
    }

    #[test]
    fn unlimited_handler_runs_every_time_and_is_never_removed() {
        let mut e = entry(None);
        for _ in 0..3 {
            assert_eq!(
                e.take_invocation(),
                Invocation {
                    run: true,
                    remove: false
                }
            );
        }
        assert_eq!(e.times_remaining, None);
    }

    #[test]
    fn counted_handler_runs_exactly_times_and_is_removed_on_the_last() {
        let mut e = entry(Some(2));
        assert_eq!(
            e.take_invocation(),
            Invocation {
                run: true,
                remove: false
            }
        );
        assert_eq!(e.times_remaining, Some(1));
        assert_eq!(
            e.take_invocation(),
            Invocation {
                run: true,
                remove: true
            }
        );
    }

    #[test]
    fn zero_times_never_runs_and_is_removed_without_underflow() {
        let mut e = entry(Some(0));
        assert_eq!(
            e.take_invocation(),
            Invocation {
                run: false,
                remove: true
            }
        );
        assert_eq!(e.times_remaining, Some(0));
    }
}
