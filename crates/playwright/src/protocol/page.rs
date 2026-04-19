// Page protocol object
//
// Represents a web page within a browser context.
// Pages are isolated tabs or windows within a context.

use crate::error::{Error, Result};
use crate::protocol::browser_context::Viewport;
use crate::protocol::{Dialog, Download, Request, ResponseObject, Route, WebSocket, Worker};
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::server::connection::{ConnectionExt, downcast_parent};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

/// Page represents a web page within a browser context.
///
/// A Page is created when you call `BrowserContext::new_page()` or `Browser::new_page()`.
/// Each page is an isolated tab/window within its parent context.
///
/// Initially, pages are navigated to "about:blank". Use navigation methods
/// Use navigation methods to navigate to URLs.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::{
///     Playwright, ScreenshotOptions, ScreenshotType, AddStyleTagOptions, AddScriptTagOptions,
///     EmulateMediaOptions, Media, ColorScheme, Viewport,
/// };
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let browser = playwright.chromium().launch().await?;
///     let page = browser.new_page().await?;
///
///     // Demonstrate url() - initially at about:blank
///     assert_eq!(page.url(), "about:blank");
///
///     // Demonstrate goto() - navigate to a page
///     let html = r#"<!DOCTYPE html>
///         <html>
///             <head><title>Test Page</title></head>
///             <body>
///                 <h1 id="heading">Hello World</h1>
///                 <p>First paragraph</p>
///                 <p>Second paragraph</p>
///                 <button onclick="alert('Alert!')">Alert</button>
///                 <a href="data:text/plain,file" download="test.txt">Download</a>
///             </body>
///         </html>
///     "#;
///     // Data URLs may not return a response (this is normal)
///     let _response = page.goto(&format!("data:text/html,{}", html), None).await?;
///
///     // Demonstrate title()
///     let title = page.title().await?;
///     assert_eq!(title, "Test Page");
///
///     // Demonstrate content() - returns full HTML including DOCTYPE
///     let content = page.content().await?;
///     assert!(content.contains("<!DOCTYPE html>") || content.to_lowercase().contains("<!doctype html>"));
///     assert!(content.contains("<title>Test Page</title>"));
///     assert!(content.contains("Hello World"));
///
///     // Demonstrate locator()
///     let heading = page.locator("#heading").await;
///     let text = heading.text_content().await?;
///     assert_eq!(text, Some("Hello World".to_string()));
///
///     // Demonstrate query_selector()
///     let element = page.query_selector("h1").await?;
///     assert!(element.is_some(), "Should find the h1 element");
///
///     // Demonstrate query_selector_all()
///     let paragraphs = page.query_selector_all("p").await?;
///     assert_eq!(paragraphs.len(), 2);
///
///     // Demonstrate evaluate()
///     page.evaluate::<(), ()>("console.log('Hello from Playwright!')", None).await?;
///
///     // Demonstrate evaluate_value()
///     let result = page.evaluate_value("1 + 1").await?;
///     assert_eq!(result, "2");
///
///     // Demonstrate screenshot()
///     let bytes = page.screenshot(None).await?;
///     assert!(!bytes.is_empty());
///
///     // Demonstrate screenshot_to_file()
///     let temp_dir = std::env::temp_dir();
///     let path = temp_dir.join("playwright_doctest_screenshot.png");
///     let bytes = page.screenshot_to_file(&path, Some(
///         ScreenshotOptions::builder()
///             .screenshot_type(ScreenshotType::Png)
///             .build()
///     )).await?;
///     assert!(!bytes.is_empty());
///
///     // Demonstrate reload()
///     // Data URLs may not return a response on reload (this is normal)
///     let _response = page.reload(None).await?;
///
///     // Demonstrate route() - network interception
///     page.route("**/*.png", |route| async move {
///         route.abort(None).await
///     }).await?;
///
///     // Demonstrate on_download() - download handler
///     page.on_download(|download| async move {
///         println!("Download started: {}", download.url());
///         Ok(())
///     }).await?;
///
///     // Demonstrate on_dialog() - dialog handler
///     page.on_dialog(|dialog| async move {
///         println!("Dialog: {} - {}", dialog.type_(), dialog.message());
///         dialog.accept(None).await
///     }).await?;
///
///     // Demonstrate add_style_tag() - inject CSS
///     page.add_style_tag(
///         AddStyleTagOptions::builder()
///             .content("body { background-color: blue; }")
///             .build()
///     ).await?;
///
///     // Demonstrate set_extra_http_headers() - set page-level headers
///     let mut headers = std::collections::HashMap::new();
///     headers.insert("x-custom-header".to_string(), "value".to_string());
///     page.set_extra_http_headers(headers).await?;
///
///     // Demonstrate emulate_media() - emulate print media type
///     page.emulate_media(Some(
///         EmulateMediaOptions::builder()
///             .media(Media::Print)
///             .color_scheme(ColorScheme::Dark)
///             .build()
///     )).await?;
///
///     // Demonstrate add_script_tag() - inject a script
///     page.add_script_tag(Some(
///         AddScriptTagOptions::builder()
///             .content("window.injectedByScriptTag = true;")
///             .build()
///     )).await?;
///
///     // Demonstrate pdf() - generate PDF (Chromium only)
///     let pdf_bytes = page.pdf(None).await?;
///     assert!(!pdf_bytes.is_empty());
///
///     // Demonstrate set_viewport_size() - responsive testing
///     let mobile_viewport = Viewport {
///         width: 375,
///         height: 667,
///     };
///     page.set_viewport_size(mobile_viewport).await?;
///
///     // Demonstrate close()
///     page.close().await?;
///
///     browser.close().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-page>
#[derive(Clone)]
pub struct Page {
    base: ChannelOwnerImpl,
    /// Current URL of the page
    /// Wrapped in RwLock to allow updates from events
    url: Arc<RwLock<String>>,
    /// GUID of the main frame
    main_frame_guid: Arc<str>,
    /// Cached reference to the main frame for synchronous URL access
    /// This is populated after the first call to main_frame()
    cached_main_frame: Arc<Mutex<Option<crate::protocol::Frame>>>,
    /// Route handlers for network interception
    route_handlers: Arc<Mutex<Vec<RouteHandlerEntry>>>,
    /// Download event handlers
    download_handlers: Arc<Mutex<Vec<DownloadHandler>>>,
    /// Dialog event handlers
    dialog_handlers: Arc<Mutex<Vec<DialogHandler>>>,
    /// Request event handlers
    request_handlers: Arc<Mutex<Vec<RequestHandler>>>,
    /// Request finished event handlers
    request_finished_handlers: Arc<Mutex<Vec<RequestHandler>>>,
    /// Request failed event handlers
    request_failed_handlers: Arc<Mutex<Vec<RequestHandler>>>,
    /// Response event handlers
    response_handlers: Arc<Mutex<Vec<ResponseHandler>>>,
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
    /// Console event handlers
    console_handlers: Arc<Mutex<Vec<ConsoleHandler>>>,
    /// FileChooser event handlers
    filechooser_handlers: Arc<Mutex<Vec<FileChooserHandler>>>,
    /// One-shot senders waiting for the next "fileChooser" event (expect_file_chooser)
    filechooser_waiters:
        Arc<Mutex<Vec<tokio::sync::oneshot::Sender<crate::protocol::FileChooser>>>>,
    /// One-shot senders waiting for the next "popup" event (expect_popup)
    popup_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<Page>>>>,
    /// One-shot senders waiting for the next "download" event (expect_download)
    download_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<Download>>>>,
    /// One-shot senders waiting for the next "response" event (expect_response)
    response_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<ResponseObject>>>>,
    /// One-shot senders waiting for the next "request" event (expect_request)
    request_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<Request>>>>,
    /// One-shot senders waiting for the next "console" event (expect_console_message)
    console_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<crate::protocol::ConsoleMessage>>>>,
    /// close event handlers (fires when page is closed)
    close_handlers: Arc<Mutex<Vec<CloseHandler>>>,
    /// load event handlers (fires when page fully loads)
    load_handlers: Arc<Mutex<Vec<LoadHandler>>>,
    /// crash event handlers (fires when page crashes)
    crash_handlers: Arc<Mutex<Vec<CrashHandler>>>,
    /// pageError event handlers (fires on uncaught JS exceptions)
    pageerror_handlers: Arc<Mutex<Vec<PageErrorHandler>>>,
    /// popup event handlers (fires when a popup window opens)
    popup_handlers: Arc<Mutex<Vec<PopupHandler>>>,
    /// frameAttached event handlers
    frameattached_handlers: Arc<Mutex<Vec<FrameAttachedHandler>>>,
    /// frameDetached event handlers
    framedetached_handlers: Arc<Mutex<Vec<FrameDetachedHandler>>>,
    /// frameNavigated event handlers
    framenavigated_handlers: Arc<Mutex<Vec<FrameNavigatedHandler>>>,
    /// worker event handlers (fires when a web worker is created in the page)
    worker_handlers: Arc<Mutex<Vec<WorkerHandler>>>,
    /// One-shot senders waiting for the next "close" event (expect_event("close"))
    close_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<()>>>>,
    /// One-shot senders waiting for the next "load" event (expect_event("load"))
    load_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<()>>>>,
    /// One-shot senders waiting for the next "crash" event (expect_event("crash"))
    crash_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<()>>>>,
    /// One-shot senders waiting for the next "pageerror" event (expect_event("pageerror"))
    pageerror_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<String>>>>,
    /// One-shot senders waiting for the next frame event (frameattached/detached/navigated)
    frameattached_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<crate::protocol::Frame>>>>,
    framedetached_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<crate::protocol::Frame>>>>,
    framenavigated_waiters: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<crate::protocol::Frame>>>>,
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
    /// Artifact is wired up when the server fires the "video" event.
    video: Option<crate::protocol::Video>,
    /// Registered locator handlers: maps uid -> (selector, handler fn, times_remaining)
    /// times_remaining is None when the handler should run indefinitely.
    locator_handlers: Arc<Mutex<Vec<LocatorHandlerEntry>>>,
}

/// Type alias for boxed route handler future
type RouteHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed download handler future
type DownloadHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed dialog handler future
type DialogHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed request handler future
type RequestHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed response handler future
type ResponseHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

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

/// Download event handler
type DownloadHandler = Arc<dyn Fn(Download) -> DownloadHandlerFuture + Send + Sync>;

/// Dialog event handler
type DialogHandler = Arc<dyn Fn(Dialog) -> DialogHandlerFuture + Send + Sync>;

/// Request event handler
type RequestHandler = Arc<dyn Fn(Request) -> RequestHandlerFuture + Send + Sync>;

/// Response event handler
type ResponseHandler = Arc<dyn Fn(ResponseObject) -> ResponseHandlerFuture + Send + Sync>;

/// WebSocket event handler
type WebSocketHandler = Arc<dyn Fn(WebSocket) -> WebSocketHandlerFuture + Send + Sync>;

/// Type alias for boxed console handler future
type ConsoleHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Console event handler
type ConsoleHandler =
    Arc<dyn Fn(crate::protocol::ConsoleMessage) -> ConsoleHandlerFuture + Send + Sync>;

/// Type alias for boxed filechooser handler future
type FileChooserHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// FileChooser event handler
type FileChooserHandler =
    Arc<dyn Fn(crate::protocol::FileChooser) -> FileChooserHandlerFuture + Send + Sync>;

/// Type alias for boxed close handler future
type CloseHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// close event handler (no arguments)
type CloseHandler = Arc<dyn Fn() -> CloseHandlerFuture + Send + Sync>;

/// Type alias for boxed load handler future
type LoadHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// load event handler (no arguments)
type LoadHandler = Arc<dyn Fn() -> LoadHandlerFuture + Send + Sync>;

/// Type alias for boxed crash handler future
type CrashHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// crash event handler (no arguments)
type CrashHandler = Arc<dyn Fn() -> CrashHandlerFuture + Send + Sync>;

/// Type alias for boxed pageError handler future
type PageErrorHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// pageError event handler — receives the error message as a String
type PageErrorHandler = Arc<dyn Fn(String) -> PageErrorHandlerFuture + Send + Sync>;

/// Type alias for boxed popup handler future
type PopupHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// popup event handler — receives the new popup Page
type PopupHandler = Arc<dyn Fn(Page) -> PopupHandlerFuture + Send + Sync>;

/// Type alias for boxed frameAttached/Detached/Navigated handler future
type FrameEventHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// frameAttached event handler
type FrameAttachedHandler =
    Arc<dyn Fn(crate::protocol::Frame) -> FrameEventHandlerFuture + Send + Sync>;

/// frameDetached event handler
type FrameDetachedHandler =
    Arc<dyn Fn(crate::protocol::Frame) -> FrameEventHandlerFuture + Send + Sync>;

/// frameNavigated event handler
type FrameNavigatedHandler =
    Arc<dyn Fn(crate::protocol::Frame) -> FrameEventHandlerFuture + Send + Sync>;

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

impl Page {
    /// Creates a new Page from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Page object.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent BrowserContext object
    /// * `type_name` - The protocol type name ("Page")
    /// * `guid` - The unique identifier for this page
    /// * `initializer` - The initialization data from the server
    ///
    /// # Errors
    ///
    /// Returns error if initializer is malformed
    pub fn new(
        parent: Arc<dyn ChannelOwner>,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        // Extract mainFrame GUID from initializer
        let main_frame_guid: Arc<str> =
            Arc::from(initializer["mainFrame"]["guid"].as_str().ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "Page initializer missing 'mainFrame.guid' field".to_string(),
                )
            })?);

        // Check the parent BrowserContext's initializer for record_video before
        // moving `parent` into ChannelOwnerImpl. The Page initializer itself does
        // not carry video metadata — the Artifact arrives later via a "video" event.
        let has_video = parent
            .initializer()
            .get("options")
            .and_then(|opts| opts.get("recordVideo"))
            .is_some();

        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent),
            type_name,
            guid,
            initializer,
        );

        // Initialize URL to about:blank
        let url = Arc::new(RwLock::new("about:blank".to_string()));

        // Initialize empty route handlers
        let route_handlers = Arc::new(Mutex::new(Vec::new()));

        // Initialize empty event handlers
        let download_handlers = Arc::new(Mutex::new(Vec::new()));
        let dialog_handlers = Arc::new(Mutex::new(Vec::new()));
        let websocket_handlers = Arc::new(Mutex::new(Vec::new()));
        let ws_route_handlers = Arc::new(Mutex::new(Vec::new()));

        // Initialize cached main frame as empty (will be populated on first access)
        let cached_main_frame = Arc::new(Mutex::new(None));

        // Extract viewport from initializer (may be null for no_viewport contexts)
        let initial_viewport: Option<Viewport> =
            base.initializer().get("viewportSize").and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    serde_json::from_value(v.clone()).ok()
                }
            });
        let viewport = Arc::new(RwLock::new(initial_viewport));

        let video = if has_video {
            Some(crate::protocol::Video::new())
        } else {
            None
        };

        Ok(Self {
            base,
            url,
            main_frame_guid,
            cached_main_frame,
            route_handlers,
            download_handlers,
            dialog_handlers,
            request_handlers: Default::default(),
            request_finished_handlers: Default::default(),
            request_failed_handlers: Default::default(),
            response_handlers: Default::default(),
            websocket_handlers,
            ws_route_handlers,
            viewport,
            is_closed: Arc::new(AtomicBool::new(false)),
            default_timeout_ms: Arc::new(AtomicU64::new(crate::DEFAULT_TIMEOUT_MS.to_bits())),
            default_navigation_timeout_ms: Arc::new(AtomicU64::new(
                crate::DEFAULT_TIMEOUT_MS.to_bits(),
            )),
            binding_callbacks: Arc::new(Mutex::new(HashMap::new())),
            console_handlers: Arc::new(Mutex::new(Vec::new())),
            filechooser_handlers: Arc::new(Mutex::new(Vec::new())),
            filechooser_waiters: Arc::new(Mutex::new(Vec::new())),
            popup_waiters: Arc::new(Mutex::new(Vec::new())),
            download_waiters: Arc::new(Mutex::new(Vec::new())),
            response_waiters: Arc::new(Mutex::new(Vec::new())),
            request_waiters: Arc::new(Mutex::new(Vec::new())),
            console_waiters: Arc::new(Mutex::new(Vec::new())),
            close_handlers: Arc::new(Mutex::new(Vec::new())),
            load_handlers: Arc::new(Mutex::new(Vec::new())),
            crash_handlers: Arc::new(Mutex::new(Vec::new())),
            pageerror_handlers: Arc::new(Mutex::new(Vec::new())),
            popup_handlers: Arc::new(Mutex::new(Vec::new())),
            frameattached_handlers: Arc::new(Mutex::new(Vec::new())),
            framedetached_handlers: Arc::new(Mutex::new(Vec::new())),
            framenavigated_handlers: Arc::new(Mutex::new(Vec::new())),
            worker_handlers: Arc::new(Mutex::new(Vec::new())),
            close_waiters: Arc::new(Mutex::new(Vec::new())),
            load_waiters: Arc::new(Mutex::new(Vec::new())),
            crash_waiters: Arc::new(Mutex::new(Vec::new())),
            pageerror_waiters: Arc::new(Mutex::new(Vec::new())),
            frameattached_waiters: Arc::new(Mutex::new(Vec::new())),
            framedetached_waiters: Arc::new(Mutex::new(Vec::new())),
            framenavigated_waiters: Arc::new(Mutex::new(Vec::new())),
            worker_waiters: Arc::new(Mutex::new(Vec::new())),
            console_messages_log: Arc::new(Mutex::new(Vec::new())),
            page_errors_log: Arc::new(Mutex::new(Vec::new())),
            workers_list: Arc::new(Mutex::new(Vec::new())),
            video,
            locator_handlers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Returns the channel for sending protocol messages
    ///
    /// Used internally for sending RPC calls to the page.
    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    /// Returns the main frame of the page.
    ///
    /// The main frame is where navigation and DOM operations actually happen.
    ///
    /// This method also wires up the back-reference from the frame to the page so that
    /// `frame.page()`, `frame.locator()`, and `frame.get_by_*()` work correctly.
    pub async fn main_frame(&self) -> Result<crate::protocol::Frame> {
        // Get and downcast the Frame object from the connection's object registry
        let frame: crate::protocol::Frame = self
            .connection()
            .get_typed::<crate::protocol::Frame>(&self.main_frame_guid)
            .await?;

        // Wire up the back-reference so frame.page() / frame.locator() work.
        // This is safe to call multiple times (subsequent calls are no-ops once set).
        frame.set_page(self.clone());

        // Cache the frame for synchronous access in url()
        if let Ok(mut cached) = self.cached_main_frame.lock() {
            *cached = Some(frame.clone());
        }

        Ok(frame)
    }

    /// Returns the current URL of the page.
    ///
    /// This returns the last committed URL, including hash fragments from anchor navigation.
    /// Initially, pages are at "about:blank".
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-url>
    pub fn url(&self) -> String {
        // Try to get URL from the cached main frame (source of truth for navigation including hashes)
        if let Ok(cached) = self.cached_main_frame.lock()
            && let Some(frame) = cached.as_ref()
        {
            return frame.url();
        }

        // Fallback to cached URL if frame not yet loaded
        self.url.read().unwrap().clone()
    }

    /// Closes the page.
    ///
    /// This is a graceful operation that sends a close command to the page
    /// and waits for it to shut down properly.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has already been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-close>
    pub async fn close(&self) -> Result<()> {
        // Send close RPC to server
        let result = self
            .channel()
            .send_no_result("close", serde_json::json!({}))
            .await;
        // Mark as closed regardless of error (best-effort)
        self.is_closed.store(true, Ordering::Relaxed);
        result
    }

    /// Returns whether the page has been closed.
    ///
    /// Returns `true` after `close()` has been called on this page, or after the
    /// page receives a close event from the server (e.g. when the browser context
    /// is closed).
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-is-closed>
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Relaxed)
    }

    /// Returns all console messages received so far on this page.
    ///
    /// Messages are accumulated in order as they arrive via the `console` event.
    /// Each call returns a snapshot; new messages arriving concurrently may or may not
    /// be included depending on timing.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-console-messages>
    pub fn console_messages(&self) -> Vec<crate::protocol::ConsoleMessage> {
        self.console_messages_log.lock().unwrap().clone()
    }

    /// Returns all uncaught JavaScript error messages received so far on this page.
    ///
    /// Errors are accumulated in order as they arrive via the `pageError` event.
    /// Each string is the `.message` field of the thrown `Error`.
    pub fn page_errors(&self) -> Vec<String> {
        self.page_errors_log.lock().unwrap().clone()
    }

    /// Returns the page that opened this popup, or `None` if this page was not opened
    /// by another page.
    ///
    /// The opener is available from the page's initializer — it is the page that called
    /// `window.open()` or triggered a link with `target="_blank"`. Returns `None` for
    /// top-level pages that were not opened as popups.
    ///
    /// # Errors
    ///
    /// Returns error if the opener page GUID is present in the initializer but the
    /// object is not found in the connection registry.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-opener>
    pub async fn opener(&self) -> Result<Option<Page>> {
        // The opener guid is stored in the page initializer as {"opener": {"guid": "..."}}.
        // It is set when the page is created as a popup; absent for non-popup pages.
        let opener_guid = self
            .base
            .initializer()
            .get("opener")
            .and_then(|v| v.get("guid"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        match opener_guid {
            None => Ok(None),
            Some(guid) => {
                let page = self.connection().get_typed::<Page>(&guid).await?;
                Ok(Some(page))
            }
        }
    }

    /// Returns all active web workers belonging to this page.
    ///
    /// Workers are tracked as they are created (`worker` event) and this method
    /// returns a snapshot of the current list.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-workers>
    pub fn workers(&self) -> Vec<Worker> {
        self.workers_list.lock().unwrap().clone()
    }

    /// Sets the default timeout for all operations on this page.
    ///
    /// The timeout applies to actions such as `click`, `fill`, `locator.wait_for`, etc.
    /// Pass `0` to disable timeouts.
    ///
    /// This stores the value locally so that subsequent action calls use it when
    /// no explicit timeout is provided, and also notifies the Playwright server
    /// so it can apply the same default on its side.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-set-default-timeout>
    pub async fn set_default_timeout(&self, timeout: f64) {
        self.default_timeout_ms
            .store(timeout.to_bits(), Ordering::Relaxed);
        set_timeout_and_notify(self.channel(), "setDefaultTimeoutNoReply", timeout).await;
    }

    /// Sets the default timeout for navigation operations on this page.
    ///
    /// The timeout applies to navigation actions such as `goto`, `reload`,
    /// `go_back`, and `go_forward`. Pass `0` to disable timeouts.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-set-default-navigation-timeout>
    pub async fn set_default_navigation_timeout(&self, timeout: f64) {
        self.default_navigation_timeout_ms
            .store(timeout.to_bits(), Ordering::Relaxed);
        set_timeout_and_notify(
            self.channel(),
            "setDefaultNavigationTimeoutNoReply",
            timeout,
        )
        .await;
    }

    /// Returns the current default action timeout in milliseconds.
    pub fn default_timeout_ms(&self) -> f64 {
        f64::from_bits(self.default_timeout_ms.load(Ordering::Relaxed))
    }

    /// Returns the current default navigation timeout in milliseconds.
    pub fn default_navigation_timeout_ms(&self) -> f64 {
        f64::from_bits(self.default_navigation_timeout_ms.load(Ordering::Relaxed))
    }

    /// Returns GotoOptions with the navigation timeout filled in if not already set.
    ///
    /// Used internally to ensure the page's configured default navigation timeout
    /// is used when the caller does not provide an explicit timeout.
    fn with_navigation_timeout(&self, options: Option<GotoOptions>) -> GotoOptions {
        let nav_timeout = self.default_navigation_timeout_ms();
        match options {
            Some(opts) if opts.timeout.is_some() => opts,
            Some(mut opts) => {
                opts.timeout = Some(std::time::Duration::from_millis(nav_timeout as u64));
                opts
            }
            None => GotoOptions {
                timeout: Some(std::time::Duration::from_millis(nav_timeout as u64)),
                wait_until: None,
            },
        }
    }

    /// Returns all frames in the page, including the main frame.
    ///
    /// Currently returns only the main (top-level) frame. Iframe enumeration
    /// is not yet implemented and will be added in a future release.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-frames>
    pub async fn frames(&self) -> Result<Vec<crate::protocol::Frame>> {
        // Start with the main frame
        let main = self.main_frame().await?;
        Ok(vec![main])
    }

    /// Navigates to the specified URL.
    ///
    /// Returns `None` when navigating to URLs that don't produce responses (e.g., data URLs,
    /// about:blank). This matches Playwright's behavior across all language bindings.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to navigate to
    /// * `options` - Optional navigation options (timeout, wait_until)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - URL is invalid
    /// - Navigation timeout (default 30s)
    /// - Network error
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-goto>
    pub async fn goto(&self, url: &str, options: Option<GotoOptions>) -> Result<Option<Response>> {
        // Inject the page-level navigation timeout when no explicit timeout is given
        let options = self.with_navigation_timeout(options);

        // Delegate to main frame
        let frame = self.main_frame().await.map_err(|e| match e {
            Error::TargetClosed { context, .. } => Error::TargetClosed {
                target_type: "Page".to_string(),
                context,
            },
            other => other,
        })?;

        let response = frame.goto(url, Some(options)).await.map_err(|e| match e {
            Error::TargetClosed { context, .. } => Error::TargetClosed {
                target_type: "Page".to_string(),
                context,
            },
            other => other,
        })?;

        // Update the page's URL if we got a response
        if let Some(ref resp) = response
            && let Ok(mut page_url) = self.url.write()
        {
            *page_url = resp.url().to_string();
        }

        Ok(response)
    }

    /// Returns the browser context that the page belongs to.
    pub fn context(&self) -> Result<crate::protocol::BrowserContext> {
        downcast_parent::<crate::protocol::BrowserContext>(self)
            .ok_or_else(|| Error::ProtocolError("Page parent is not a BrowserContext".to_string()))
    }

    /// Returns the Clock object for this page's browser context.
    ///
    /// This is a convenience accessor that delegates to the parent context's clock.
    /// All clock RPCs are sent on the BrowserContext channel regardless of whether
    /// the Clock is obtained via `page.clock()` or `context.clock()`.
    ///
    /// # Errors
    ///
    /// Returns error if the page's parent is not a BrowserContext.
    ///
    /// See: <https://playwright.dev/docs/api/class-clock>
    pub fn clock(&self) -> Result<crate::protocol::clock::Clock> {
        Ok(self.context()?.clock())
    }

    /// Returns the `Video` object associated with this page, if video recording is enabled.
    ///
    /// Returns `Some(Video)` when the browser context was created with the `record_video`
    /// option; returns `None` otherwise.
    ///
    /// The `Video` shell is created eagerly. The underlying recording artifact is wired
    /// up when the Playwright server fires the internal `"video"` event (which typically
    /// happens when the page is first navigated). Calling [`crate::protocol::Video::save_as`] or
    /// [`crate::protocol::Video::path`] before the artifact arrives returns an error; close the page
    /// first to guarantee the artifact is ready.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-video>
    pub fn video(&self) -> Option<crate::protocol::Video> {
        self.video.clone()
    }

    /// Pauses script execution.
    ///
    /// Playwright will stop executing the script and wait for the user to either press
    /// "Resume" in the page overlay or in the debugger.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-pause>
    pub async fn pause(&self) -> Result<()> {
        self.context()?.pause().await
    }

    /// Returns the page's title.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-title>
    pub async fn title(&self) -> Result<String> {
        // Delegate to main frame
        let frame = self.main_frame().await?;
        frame.title().await
    }

    /// Returns the full HTML content of the page, including the DOCTYPE.
    ///
    /// This method retrieves the complete HTML markup of the page,
    /// including the doctype declaration and all DOM elements.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-content>
    pub async fn content(&self) -> Result<String> {
        // Delegate to main frame
        let frame = self.main_frame().await?;
        frame.content().await
    }

    /// Sets the content of the page.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-set-content>
    pub async fn set_content(&self, html: &str, options: Option<GotoOptions>) -> Result<()> {
        let frame = self.main_frame().await?;
        frame.set_content(html, options).await
    }

    /// Waits for the required load state to be reached.
    ///
    /// This resolves when the page reaches a required load state, `load` by default.
    /// The navigation must have been committed when this method is called. If the current
    /// document has already reached the required state, resolves immediately.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-load-state>
    pub async fn wait_for_load_state(&self, state: Option<WaitUntil>) -> Result<()> {
        let frame = self.main_frame().await?;
        frame.wait_for_load_state(state).await
    }

    /// Waits for the main frame to navigate to a URL matching the given string or glob pattern.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-url>
    pub async fn wait_for_url(&self, url: &str, options: Option<GotoOptions>) -> Result<()> {
        let frame = self.main_frame().await?;
        frame.wait_for_url(url, options).await
    }

    /// Creates a locator for finding elements on the page.
    ///
    /// Locators are the central piece of Playwright's auto-waiting and retry-ability.
    /// They don't execute queries until an action is performed.
    ///
    /// # Arguments
    ///
    /// * `selector` - CSS selector or other locating strategy
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-locator>
    pub async fn locator(&self, selector: &str) -> crate::protocol::Locator {
        // Get the main frame
        let frame = self.main_frame().await.expect("Main frame should exist");

        crate::protocol::Locator::new(Arc::new(frame), selector.to_string(), self.clone())
    }

    /// Creates a [`FrameLocator`](crate::protocol::FrameLocator) for an iframe on this page.
    ///
    /// The `selector` identifies the iframe element (e.g., `"iframe[name='content']"`).
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-frame-locator>
    pub async fn frame_locator(&self, selector: &str) -> crate::protocol::FrameLocator {
        let frame = self.main_frame().await.expect("Main frame should exist");
        crate::protocol::FrameLocator::new(Arc::new(frame), selector.to_string(), self.clone())
    }

    /// Returns a locator that matches elements containing the given text.
    ///
    /// By default, matching is case-insensitive and searches for a substring.
    /// Set `exact` to `true` for case-sensitive exact matching.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-text>
    pub async fn get_by_text(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_text_selector(text, exact))
            .await
    }

    /// Returns a locator that matches elements by their associated label text.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-label>
    pub async fn get_by_label(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_label_selector(
            text, exact,
        ))
        .await
    }

    /// Returns a locator that matches elements by their placeholder text.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-placeholder>
    pub async fn get_by_placeholder(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_placeholder_selector(
            text, exact,
        ))
        .await
    }

    /// Returns a locator that matches elements by their alt text.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-alt-text>
    pub async fn get_by_alt_text(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_alt_text_selector(
            text, exact,
        ))
        .await
    }

    /// Returns a locator that matches elements by their title attribute.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-title>
    pub async fn get_by_title(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_title_selector(
            text, exact,
        ))
        .await
    }

    /// Returns a locator that matches elements by their test ID attribute.
    ///
    /// By default, uses the `data-testid` attribute. Call
    /// [`playwright.selectors().set_test_id_attribute()`](crate::protocol::Selectors::set_test_id_attribute)
    /// to change the attribute name.
    ///
    /// Always uses exact matching (case-sensitive).
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-test-id>
    pub async fn get_by_test_id(&self, test_id: &str) -> crate::protocol::Locator {
        let attr = self.connection().selectors().test_id_attribute();
        self.locator(&crate::protocol::locator::get_by_test_id_selector_with_attr(test_id, &attr))
            .await
    }

    /// Returns a locator that matches elements by their ARIA role.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-role>
    pub async fn get_by_role(
        &self,
        role: crate::protocol::locator::AriaRole,
        options: Option<crate::protocol::locator::GetByRoleOptions>,
    ) -> crate::protocol::Locator {
        self.locator(&crate::protocol::locator::get_by_role_selector(
            role, options,
        ))
        .await
    }

    /// Returns the keyboard instance for low-level keyboard control.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-keyboard>
    pub fn keyboard(&self) -> crate::protocol::Keyboard {
        crate::protocol::Keyboard::new(self.clone())
    }

    /// Returns the mouse instance for low-level mouse control.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-mouse>
    pub fn mouse(&self) -> crate::protocol::Mouse {
        crate::protocol::Mouse::new(self.clone())
    }

    // Internal keyboard methods (called by Keyboard struct)

    pub(crate) async fn keyboard_down(&self, key: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "keyboardDown",
                serde_json::json!({
                    "key": key
                }),
            )
            .await
    }

    pub(crate) async fn keyboard_up(&self, key: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "keyboardUp",
                serde_json::json!({
                    "key": key
                }),
            )
            .await
    }

    pub(crate) async fn keyboard_press(
        &self,
        key: &str,
        options: Option<crate::protocol::KeyboardOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "key": key
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("keyboardPress", params).await
    }

    pub(crate) async fn keyboard_type(
        &self,
        text: &str,
        options: Option<crate::protocol::KeyboardOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "text": text
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("keyboardType", params).await
    }

    pub(crate) async fn keyboard_insert_text(&self, text: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "keyboardInsertText",
                serde_json::json!({
                    "text": text
                }),
            )
            .await
    }

    // Internal mouse methods (called by Mouse struct)

    pub(crate) async fn mouse_move(
        &self,
        x: i32,
        y: i32,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "x": x,
            "y": y
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("mouseMove", params).await
    }

    pub(crate) async fn mouse_click(
        &self,
        x: i32,
        y: i32,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "x": x,
            "y": y
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("mouseClick", params).await
    }

    pub(crate) async fn mouse_dblclick(
        &self,
        x: i32,
        y: i32,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "x": x,
            "y": y,
            "clickCount": 2
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("mouseClick", params).await
    }

    pub(crate) async fn mouse_down(
        &self,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("mouseDown", params).await
    }

    pub(crate) async fn mouse_up(
        &self,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("mouseUp", params).await
    }

    pub(crate) async fn mouse_wheel(&self, delta_x: i32, delta_y: i32) -> Result<()> {
        self.channel()
            .send_no_result(
                "mouseWheel",
                serde_json::json!({
                    "deltaX": delta_x,
                    "deltaY": delta_y
                }),
            )
            .await
    }

    // Internal touchscreen method (called by Touchscreen struct)

    pub(crate) async fn touchscreen_tap(&self, x: f64, y: f64) -> Result<()> {
        self.channel()
            .send_no_result(
                "touchscreenTap",
                serde_json::json!({
                    "x": x,
                    "y": y
                }),
            )
            .await
    }

    /// Returns the touchscreen instance for low-level touch input simulation.
    ///
    /// Requires a touch-enabled browser context (`has_touch: true` in
    /// [`BrowserContextOptions`](crate::protocol::browser_context::BrowserContext)).
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-touchscreen>
    pub fn touchscreen(&self) -> crate::protocol::Touchscreen {
        crate::protocol::Touchscreen::new(self.clone())
    }

    /// Performs a drag from source selector to target selector.
    ///
    /// This is the page-level equivalent of `Locator::drag_to()`. It resolves
    /// both selectors in the main frame and performs the drag.
    ///
    /// # Arguments
    ///
    /// * `source` - A CSS selector for the element to drag from
    /// * `target` - A CSS selector for the element to drop onto
    /// * `options` - Optional drag options (positions, force, timeout, trial)
    ///
    /// # Errors
    ///
    /// Returns error if either selector does not resolve to an element, the
    /// drag action times out, or the page has been closed.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-drag-and-drop>
    pub async fn drag_and_drop(
        &self,
        source: &str,
        target: &str,
        options: Option<crate::protocol::DragToOptions>,
    ) -> Result<()> {
        let frame = self.main_frame().await?;
        frame.locator_drag_to(source, target, options).await
    }

    /// Reloads the current page.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional reload options (timeout, wait_until)
    ///
    /// Returns `None` when reloading pages that don't produce responses (e.g., data URLs,
    /// about:blank). This matches Playwright's behavior across all language bindings.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-reload>
    pub async fn reload(&self, options: Option<GotoOptions>) -> Result<Option<Response>> {
        self.navigate_history("reload", options).await
    }

    /// Navigates to the previous page in history.
    ///
    /// Returns the main resource response. In case of multiple server redirects, the navigation
    /// will resolve with the response of the last redirect. If can not go back, returns `None`.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-go-back>
    pub async fn go_back(&self, options: Option<GotoOptions>) -> Result<Option<Response>> {
        self.navigate_history("goBack", options).await
    }

    /// Navigates to the next page in history.
    ///
    /// Returns the main resource response. In case of multiple server redirects, the navigation
    /// will resolve with the response of the last redirect. If can not go forward, returns `None`.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-go-forward>
    pub async fn go_forward(&self, options: Option<GotoOptions>) -> Result<Option<Response>> {
        self.navigate_history("goForward", options).await
    }

    /// Shared implementation for reload, go_back and go_forward.
    async fn navigate_history(
        &self,
        method: &str,
        options: Option<GotoOptions>,
    ) -> Result<Option<Response>> {
        // Inject the page-level navigation timeout when no explicit timeout is given
        let opts = self.with_navigation_timeout(options);
        let mut params = serde_json::json!({});

        // opts.timeout is always Some(...) because with_navigation_timeout guarantees it
        if let Some(timeout) = opts.timeout {
            params["timeout"] = serde_json::json!(timeout.as_millis() as u64);
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }
        if let Some(wait_until) = opts.wait_until {
            params["waitUntil"] = serde_json::json!(wait_until.as_str());
        }

        #[derive(Deserialize)]
        struct NavigationResponse {
            response: Option<ResponseReference>,
        }

        #[derive(Deserialize)]
        struct ResponseReference {
            #[serde(deserialize_with = "crate::server::connection::deserialize_arc_str")]
            guid: Arc<str>,
        }

        let result: NavigationResponse = self.channel().send(method, params).await?;

        if let Some(response_ref) = result.response {
            let response_arc = {
                let mut attempts = 0;
                let max_attempts = 20;
                loop {
                    match self.connection().get_object(&response_ref.guid).await {
                        Ok(obj) => break obj,
                        Err(_) if attempts < max_attempts => {
                            attempts += 1;
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        }
                        Err(e) => return Err(e),
                    }
                }
            };

            let initializer = response_arc.initializer();

            let status = initializer["status"].as_u64().ok_or_else(|| {
                crate::error::Error::ProtocolError("Response missing status".to_string())
            })? as u16;

            let headers = initializer["headers"]
                .as_array()
                .ok_or_else(|| {
                    crate::error::Error::ProtocolError("Response missing headers".to_string())
                })?
                .iter()
                .filter_map(|h| {
                    let name = h["name"].as_str()?;
                    let value = h["value"].as_str()?;
                    Some((name.to_string(), value.to_string()))
                })
                .collect();

            let response = Response::new(
                initializer["url"]
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::Error::ProtocolError("Response missing url".to_string())
                    })?
                    .to_string(),
                status,
                initializer["statusText"].as_str().unwrap_or("").to_string(),
                headers,
                Some(response_arc),
            );

            if let Ok(mut page_url) = self.url.write() {
                *page_url = response.url().to_string();
            }

            Ok(Some(response))
        } else {
            Ok(None)
        }
    }

    /// Returns the first element matching the selector, or None if not found.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-query-selector>
    pub async fn query_selector(
        &self,
        selector: &str,
    ) -> Result<Option<Arc<crate::protocol::ElementHandle>>> {
        let frame = self.main_frame().await?;
        frame.query_selector(selector).await
    }

    /// Returns all elements matching the selector.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-query-selector-all>
    pub async fn query_selector_all(
        &self,
        selector: &str,
    ) -> Result<Vec<Arc<crate::protocol::ElementHandle>>> {
        let frame = self.main_frame().await?;
        frame.query_selector_all(selector).await
    }

    /// Takes a screenshot of the page and returns the image bytes.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-screenshot>
    pub async fn screenshot(
        &self,
        options: Option<crate::protocol::ScreenshotOptions>,
    ) -> Result<Vec<u8>> {
        let params = if let Some(opts) = options {
            opts.to_json()
        } else {
            // Default to PNG with required timeout
            serde_json::json!({
                "type": "png",
                "timeout": crate::DEFAULT_TIMEOUT_MS
            })
        };

        #[derive(Deserialize)]
        struct ScreenshotResponse {
            binary: String,
        }

        let response: ScreenshotResponse = self.channel().send("screenshot", params).await?;

        // Decode base64 to bytes
        let bytes = base64::prelude::BASE64_STANDARD
            .decode(&response.binary)
            .map_err(|e| {
                crate::error::Error::ProtocolError(format!("Failed to decode screenshot: {}", e))
            })?;

        Ok(bytes)
    }

    /// Takes a screenshot and saves it to a file, also returning the bytes.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-screenshot>
    pub async fn screenshot_to_file(
        &self,
        path: &std::path::Path,
        options: Option<crate::protocol::ScreenshotOptions>,
    ) -> Result<Vec<u8>> {
        // Get the screenshot bytes
        let bytes = self.screenshot(options).await?;

        // Write to file
        tokio::fs::write(path, &bytes).await.map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to write screenshot file: {}", e))
        })?;

        Ok(bytes)
    }

    /// Evaluates JavaScript in the page context (without return value).
    ///
    /// Executes the provided JavaScript expression or function within the page's
    /// context without returning a value.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-evaluate>
    pub async fn evaluate_expression(&self, expression: &str) -> Result<()> {
        // Delegate to the main frame
        let frame = self.main_frame().await?;
        frame.frame_evaluate_expression(expression).await
    }

    /// Evaluates JavaScript in the page context with optional arguments.
    ///
    /// Executes the provided JavaScript expression or function within the page's
    /// context and returns the result. The return value must be JSON-serializable.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript code to evaluate
    /// * `arg` - Optional argument to pass to the expression (must implement Serialize)
    ///
    /// # Returns
    ///
    /// The result as a `serde_json::Value`
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-evaluate>
    pub async fn evaluate<T: serde::Serialize, U: serde::de::DeserializeOwned>(
        &self,
        expression: &str,
        arg: Option<&T>,
    ) -> Result<U> {
        // Delegate to the main frame
        let frame = self.main_frame().await?;
        let result = frame.evaluate(expression, arg).await?;
        serde_json::from_value(result).map_err(Error::from)
    }

    /// Evaluates a JavaScript expression and returns the result as a String.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript code to evaluate
    ///
    /// # Returns
    ///
    /// The result converted to a String
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-evaluate>
    pub async fn evaluate_value(&self, expression: &str) -> Result<String> {
        let frame = self.main_frame().await?;
        frame.frame_evaluate_expression_value(expression).await
    }

    /// Registers a route handler for network interception.
    ///
    /// When a request matches the specified pattern, the handler will be called
    /// with a Route object that can abort, continue, or fulfill the request.
    ///
    /// # Arguments
    ///
    /// * `pattern` - URL pattern to match (supports glob patterns like "**/*.png")
    /// * `handler` - Async closure that handles the route
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-route>
    pub async fn route<F, Fut>(&self, pattern: &str, handler: F) -> Result<()>
    where
        F: Fn(Route) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        // 1. Wrap handler in Arc with type erasure
        let handler =
            Arc::new(move |route: Route| -> RouteHandlerFuture { Box::pin(handler(route)) });

        // 2. Store in handlers list
        self.route_handlers.lock().unwrap().push(RouteHandlerEntry {
            pattern: pattern.to_string(),
            handler,
        });

        // 3. Enable network interception via protocol
        self.enable_network_interception().await?;

        Ok(())
    }

    /// Updates network interception patterns for this page
    async fn enable_network_interception(&self) -> Result<()> {
        // Collect all patterns from registered handlers
        // Each pattern must be an object with "glob" field
        let patterns: Vec<serde_json::Value> = self
            .route_handlers
            .lock()
            .unwrap()
            .iter()
            .map(|entry| serde_json::json!({ "glob": entry.pattern }))
            .collect();

        // Send protocol command to update network interception patterns
        // Follows playwright-python's approach
        self.channel()
            .send_no_result(
                "setNetworkInterceptionPatterns",
                serde_json::json!({
                    "patterns": patterns
                }),
            )
            .await
    }

    /// Removes route handler(s) matching the given URL pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - URL pattern to remove handlers for
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-unroute>
    pub async fn unroute(&self, pattern: &str) -> Result<()> {
        self.route_handlers
            .lock()
            .unwrap()
            .retain(|entry| entry.pattern != pattern);
        self.enable_network_interception().await
    }

    /// Removes all registered route handlers.
    ///
    /// # Arguments
    ///
    /// * `behavior` - Optional behavior for in-flight handlers
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-unroute-all>
    pub async fn unroute_all(
        &self,
        _behavior: Option<crate::protocol::route::UnrouteBehavior>,
    ) -> Result<()> {
        self.route_handlers.lock().unwrap().clear();
        self.enable_network_interception().await
    }

    /// Replays network requests from a HAR file recorded previously.
    ///
    /// Requests matching `options.url` (or all requests if omitted) will be
    /// served from the archive instead of hitting the network.  Unmatched
    /// requests are either aborted or passed through depending on
    /// `options.not_found` (`"abort"` is the default).
    ///
    /// # Arguments
    ///
    /// * `har_path` - Path to the `.har` file on disk
    /// * `options` - Optional settings (url filter, not_found policy, update mode)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - `har_path` does not exist or cannot be read by the Playwright server
    /// - The Playwright server fails to open the archive
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-route-from-har>
    pub async fn route_from_har(
        &self,
        har_path: &str,
        options: Option<RouteFromHarOptions>,
    ) -> Result<()> {
        let opts = options.unwrap_or_default();
        let not_found = opts.not_found.unwrap_or_else(|| "abort".to_string());
        let url_filter = opts.url.clone();

        // Resolve to an absolute path so the Playwright server can open it
        // regardless of its working directory.
        let abs_path = std::path::Path::new(har_path).canonicalize().map_err(|e| {
            Error::InvalidPath(format!(
                "route_from_har: cannot resolve '{}': {}",
                har_path, e
            ))
        })?;
        let abs_str = abs_path.to_string_lossy().into_owned();

        // Locate LocalUtils in the connection object registry by type name.
        // The Playwright server registers it with a guid like "localUtils@1"
        // so we scan all objects for the one with type_name "LocalUtils".
        let connection = self.connection();
        let local_utils = {
            let all = connection.all_objects_sync();
            all.into_iter()
                .find(|o| o.type_name() == "LocalUtils")
                .and_then(|o| {
                    o.as_any()
                        .downcast_ref::<crate::protocol::LocalUtils>()
                        .cloned()
                })
                .ok_or_else(|| {
                    Error::ProtocolError(
                        "route_from_har: LocalUtils not found in connection registry".to_string(),
                    )
                })?
        };

        // Open the HAR archive on the server side.
        let har_id = local_utils.har_open(&abs_str).await?;

        // Determine the URL pattern to intercept.
        let pattern = url_filter.clone().unwrap_or_else(|| "**/*".to_string());

        // Register a route handler that performs HAR lookup for each request.
        let har_id_clone = har_id.clone();
        let local_utils_clone = local_utils.clone();
        let not_found_clone = not_found.clone();

        self.route(&pattern, move |route| {
            let har_id = har_id_clone.clone();
            let local_utils = local_utils_clone.clone();
            let not_found = not_found_clone.clone();
            async move {
                let request = route.request();
                let req_url = request.url().to_string();
                let req_method = request.method().to_string();

                // Build headers array as [{name, value}]
                let headers: Vec<serde_json::Value> = request
                    .headers()
                    .iter()
                    .map(|(k, v)| serde_json::json!({"name": k, "value": v}))
                    .collect();

                let lookup = local_utils
                    .har_lookup(
                        &har_id,
                        &req_url,
                        &req_method,
                        headers,
                        None,
                        request.is_navigation_request(),
                    )
                    .await;

                match lookup {
                    Err(e) => {
                        tracing::warn!("har_lookup error for {}: {}", req_url, e);
                        route.continue_(None).await
                    }
                    Ok(result) => match result.action.as_str() {
                        "redirect" => {
                            let redirect_url = result.redirect_url.unwrap_or_default();
                            let opts = crate::protocol::ContinueOptions::builder()
                                .url(redirect_url)
                                .build();
                            route.continue_(Some(opts)).await
                        }
                        "fulfill" => {
                            let status = result.status.unwrap_or(200);

                            // Decode base64 body if present
                            let body_bytes = result.body.as_deref().map(|b64| {
                                base64::engine::general_purpose::STANDARD
                                    .decode(b64)
                                    .unwrap_or_default()
                            });

                            // Build headers map
                            let mut headers_map = std::collections::HashMap::new();
                            if let Some(raw_headers) = result.headers {
                                for h in raw_headers {
                                    if let (Some(name), Some(value)) = (
                                        h.get("name").and_then(|v| v.as_str()),
                                        h.get("value").and_then(|v| v.as_str()),
                                    ) {
                                        headers_map.insert(name.to_string(), value.to_string());
                                    }
                                }
                            }

                            let mut builder =
                                crate::protocol::FulfillOptions::builder().status(status);

                            if !headers_map.is_empty() {
                                builder = builder.headers(headers_map);
                            }

                            if let Some(body) = body_bytes {
                                builder = builder.body(body);
                            }

                            route.fulfill(Some(builder.build())).await
                        }
                        _ => {
                            // "fallback" or "error" or unknown
                            if not_found == "fallback" {
                                route.fallback(None).await
                            } else {
                                route.abort(None).await
                            }
                        }
                    },
                }
            }
        })
        .await
    }

    /// Intercepts WebSocket connections matching the given URL pattern.
    ///
    /// When a WebSocket connection from the page matches `url`, the `handler`
    /// is called with a [`WebSocketRoute`](crate::protocol::WebSocketRoute) object.
    /// The handler must call [`connect_to_server`](crate::protocol::WebSocketRoute::connect_to_server)
    /// to forward the connection to the real server, or
    /// [`close`](crate::protocol::WebSocketRoute::close) to terminate it.
    ///
    /// # Arguments
    ///
    /// * `url` — URL glob pattern (e.g. `"ws://**"` or `"wss://example.com/ws"`).
    /// * `handler` — Async closure receiving a `WebSocketRoute`.
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call to enable interception fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-route-web-socket>
    pub async fn route_web_socket<F, Fut>(&self, url: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::WebSocketRoute) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |route: crate::protocol::WebSocketRoute| -> WebSocketRouteHandlerFuture {
                Box::pin(handler(route))
            },
        );

        self.ws_route_handlers
            .lock()
            .unwrap()
            .push(WsRouteHandlerEntry {
                pattern: url.to_string(),
                handler,
            });

        self.enable_ws_interception().await
    }

    /// Updates WebSocket interception patterns for this page.
    async fn enable_ws_interception(&self) -> Result<()> {
        let patterns: Vec<serde_json::Value> = self
            .ws_route_handlers
            .lock()
            .unwrap()
            .iter()
            .map(|entry| serde_json::json!({ "glob": entry.pattern }))
            .collect();

        self.channel()
            .send_no_result(
                "setWebSocketInterceptionPatterns",
                serde_json::json!({ "patterns": patterns }),
            )
            .await
    }

    /// Handles a route event from the protocol
    ///
    /// Called by on_event when a "route" event is received.
    /// Supports handler chaining via `route.fallback()` — if a handler calls
    /// `fallback()` instead of `continue_()`, `abort()`, or `fulfill()`, the
    /// next matching handler in the chain is tried.
    async fn on_route_event(&self, route: Route) {
        let handlers = self.route_handlers.lock().unwrap().clone();
        let url = route.request().url().to_string();

        // Find matching handler (last registered wins, with fallback chaining)
        for entry in handlers.iter().rev() {
            if crate::protocol::route::matches_pattern(&entry.pattern, &url) {
                let handler = entry.handler.clone();
                if let Err(e) = handler(route.clone()).await {
                    tracing::warn!("Route handler error: {}", e);
                    break;
                }
                // If handler called fallback(), try the next matching handler
                if !route.was_handled() {
                    continue;
                }
                break;
            }
        }
    }

    /// Registers a download event handler.
    ///
    /// The handler will be called when a download is triggered by the page.
    /// Downloads occur when the page initiates a file download (e.g., clicking a link
    /// with the download attribute, or a server response with Content-Disposition: attachment).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the Download object
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-download>
    pub async fn on_download<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Download) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        // Wrap handler with type erasure
        let handler = Arc::new(move |download: Download| -> DownloadHandlerFuture {
            Box::pin(handler(download))
        });

        // Store handler
        self.download_handlers.lock().unwrap().push(handler);

        Ok(())
    }

    /// Registers a dialog event handler.
    ///
    /// The handler will be called when a JavaScript dialog is triggered (alert, confirm, prompt, or beforeunload).
    /// The dialog must be explicitly accepted or dismissed, otherwise the page will freeze.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the Dialog object
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-dialog>
    pub async fn on_dialog<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Dialog) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        // Wrap handler with type erasure
        let handler =
            Arc::new(move |dialog: Dialog| -> DialogHandlerFuture { Box::pin(handler(dialog)) });

        // Store handler
        self.dialog_handlers.lock().unwrap().push(handler);

        // Dialog events are auto-emitted (no subscription needed)

        Ok(())
    }

    /// Registers a console event handler.
    ///
    /// The handler is called whenever the page emits a JavaScript console message
    /// (e.g. `console.log`, `console.error`, `console.warn`, etc.).
    ///
    /// The server only sends console events after the first handler is registered
    /// (subscription is managed automatically).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the [`ConsoleMessage`](crate::protocol::ConsoleMessage)
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-console>
    pub async fn on_console<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::ConsoleMessage) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |msg: crate::protocol::ConsoleMessage| -> ConsoleHandlerFuture {
                Box::pin(handler(msg))
            },
        );

        let needs_subscription = {
            let handlers = self.console_handlers.lock().unwrap();
            let waiters = self.console_waiters.lock().unwrap();
            handlers.is_empty() && waiters.is_empty()
        };
        if needs_subscription {
            _ = self.channel().update_subscription("console", true).await;
        }
        self.console_handlers.lock().unwrap().push(handler);

        Ok(())
    }

    /// Registers a handler for file chooser events.
    ///
    /// The handler is called whenever the page opens a file chooser dialog
    /// (e.g. when the user clicks an `<input type="file">` element).
    ///
    /// Use [`FileChooser::set_files`](crate::protocol::FileChooser::set_files) inside
    /// the handler to satisfy the file chooser without OS-level interaction.
    ///
    /// The server only sends `"fileChooser"` events after the first handler is
    /// registered (subscription is managed automatically via `updateSubscription`).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives a [`FileChooser`](crate::protocol::FileChooser)
    ///
    /// # Example
    ///
    /// ```ignore
    /// page.on_filechooser(|chooser| async move {
    ///     chooser.set_files(&[std::path::PathBuf::from("/tmp/file.txt")]).await
    /// }).await?;
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-file-chooser>
    pub async fn on_filechooser<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::FileChooser) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |chooser: crate::protocol::FileChooser| -> FileChooserHandlerFuture {
                Box::pin(handler(chooser))
            },
        );

        let needs_subscription = {
            let handlers = self.filechooser_handlers.lock().unwrap();
            let waiters = self.filechooser_waiters.lock().unwrap();
            handlers.is_empty() && waiters.is_empty()
        };
        if needs_subscription {
            _ = self
                .channel()
                .update_subscription("fileChooser", true)
                .await;
        }
        self.filechooser_handlers.lock().unwrap().push(handler);

        Ok(())
    }

    /// Creates a one-shot waiter that resolves when the next file chooser opens.
    ///
    /// The waiter **must** be created before the action that triggers the file
    /// chooser to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if the file chooser
    /// does not open within the timeout.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Set up waiter BEFORE triggering the file chooser
    /// let waiter = page.expect_file_chooser(None).await?;
    /// page.locator("input[type=file]").await.click(None).await?;
    /// let chooser = waiter.wait().await?;
    /// chooser.set_files(&[PathBuf::from("/tmp/file.txt")]).await?;
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    pub async fn expect_file_chooser(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<crate::protocol::FileChooser>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let needs_subscription = {
            let handlers = self.filechooser_handlers.lock().unwrap();
            let waiters = self.filechooser_waiters.lock().unwrap();
            handlers.is_empty() && waiters.is_empty()
        };
        if needs_subscription {
            _ = self
                .channel()
                .update_subscription("fileChooser", true)
                .await;
        }
        self.filechooser_waiters.lock().unwrap().push(tx);

        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Creates a one-shot waiter that resolves when the next popup window opens.
    ///
    /// The waiter **must** be created before the action that opens the popup to
    /// avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if no popup
    /// opens within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    pub async fn expect_popup(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<Page>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.popup_waiters.lock().unwrap().push(tx);
        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Creates a one-shot waiter that resolves when the next download starts.
    ///
    /// The waiter **must** be created before the action that triggers the download
    /// to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if no download
    /// starts within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    pub async fn expect_download(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<Download>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.download_waiters.lock().unwrap().push(tx);
        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Creates a one-shot waiter that resolves when the next network response is received.
    ///
    /// The waiter **must** be created before the action that triggers the response
    /// to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if no response
    /// arrives within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    pub async fn expect_response(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<ResponseObject>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let needs_subscription = {
            let handlers = self.response_handlers.lock().unwrap();
            let waiters = self.response_waiters.lock().unwrap();
            handlers.is_empty() && waiters.is_empty()
        };
        if needs_subscription {
            _ = self.channel().update_subscription("response", true).await;
        }
        self.response_waiters.lock().unwrap().push(tx);

        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Creates a one-shot waiter that resolves when the next network request is issued.
    ///
    /// The waiter **must** be created before the action that issues the request
    /// to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if no request
    /// is issued within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    pub async fn expect_request(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<Request>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let needs_subscription = {
            let handlers = self.request_handlers.lock().unwrap();
            let waiters = self.request_waiters.lock().unwrap();
            handlers.is_empty() && waiters.is_empty()
        };
        if needs_subscription {
            _ = self.channel().update_subscription("request", true).await;
        }
        self.request_waiters.lock().unwrap().push(tx);

        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Creates a one-shot waiter that resolves when the next console message is produced.
    ///
    /// The waiter **must** be created before the action that produces the console
    /// message to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if no console
    /// message is produced within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    pub async fn expect_console_message(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<crate::protocol::ConsoleMessage>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let needs_subscription = {
            let handlers = self.console_handlers.lock().unwrap();
            let waiters = self.console_waiters.lock().unwrap();
            handlers.is_empty() && waiters.is_empty()
        };
        if needs_subscription {
            _ = self.channel().update_subscription("console", true).await;
        }
        self.console_waiters.lock().unwrap().push(tx);

        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Waits for the given event to fire and returns a typed `EventValue`.
    ///
    /// This is the generic version of the specific `expect_*` methods. It matches
    /// the playwright-python / playwright-js `page.expect_event(event_name)` API.
    ///
    /// The waiter **must** be created before the action that triggers the event.
    ///
    /// # Supported event names
    ///
    /// `"request"`, `"response"`, `"popup"`, `"download"`, `"console"`,
    /// `"filechooser"`, `"close"`, `"load"`, `"crash"`, `"pageerror"`,
    /// `"frameattached"`, `"framedetached"`, `"framenavigated"`, `"worker"`
    ///
    /// # Arguments
    ///
    /// * `event` - Event name (case-sensitive, matches Playwright protocol names).
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::InvalidArgument`] for unknown event names.
    /// Returns [`crate::error::Error::Timeout`] if the event does not fire within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    pub async fn expect_event(
        &self,
        event: &str,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<crate::protocol::EventValue>> {
        use crate::protocol::EventValue;
        use tokio::sync::oneshot;

        let timeout_ms = timeout.or(Some(30_000.0));

        match event {
            "request" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<Request>();

                let needs_subscription = {
                    let handlers = self.request_handlers.lock().unwrap();
                    let waiters = self.request_waiters.lock().unwrap();
                    handlers.is_empty() && waiters.is_empty()
                };
                if needs_subscription {
                    _ = self.channel().update_subscription("request", true).await;
                }
                self.request_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::Request(v));
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "response" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<ResponseObject>();

                let needs_subscription = {
                    let handlers = self.response_handlers.lock().unwrap();
                    let waiters = self.response_waiters.lock().unwrap();
                    handlers.is_empty() && waiters.is_empty()
                };
                if needs_subscription {
                    _ = self.channel().update_subscription("response", true).await;
                }
                self.response_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::Response(v));
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "popup" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<Page>();
                self.popup_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::Page(v));
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "download" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::Download>();
                self.download_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::Download(v));
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "console" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::ConsoleMessage>();

                let needs_subscription = {
                    let handlers = self.console_handlers.lock().unwrap();
                    let waiters = self.console_waiters.lock().unwrap();
                    handlers.is_empty() && waiters.is_empty()
                };
                if needs_subscription {
                    _ = self.channel().update_subscription("console", true).await;
                }
                self.console_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::ConsoleMessage(v));
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "filechooser" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::FileChooser>();

                let needs_subscription = {
                    let handlers = self.filechooser_handlers.lock().unwrap();
                    let waiters = self.filechooser_waiters.lock().unwrap();
                    handlers.is_empty() && waiters.is_empty()
                };
                if needs_subscription {
                    _ = self
                        .channel()
                        .update_subscription("fileChooser", true)
                        .await;
                }
                self.filechooser_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::FileChooser(v));
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "close" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<()>();
                self.close_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if inner_rx.await.is_ok() {
                        let _ = tx.send(EventValue::Close);
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "load" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<()>();
                self.load_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if inner_rx.await.is_ok() {
                        let _ = tx.send(EventValue::Load);
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "crash" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<()>();
                self.crash_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if inner_rx.await.is_ok() {
                        let _ = tx.send(EventValue::Crash);
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "pageerror" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<String>();
                self.pageerror_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(msg) = inner_rx.await {
                        let _ = tx.send(EventValue::PageError(msg));
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "frameattached" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::Frame>();
                self.frameattached_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::Frame(v));
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "framedetached" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::Frame>();
                self.framedetached_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::Frame(v));
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "framenavigated" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::Frame>();
                self.framenavigated_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::Frame(v));
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "worker" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::Worker>();
                self.worker_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::Worker(v));
                    }
                });

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            other => Err(Error::InvalidArgument(format!(
                "Unknown event name '{}'. Supported: request, response, popup, download, \
                 console, filechooser, close, load, crash, pageerror, \
                 frameattached, framedetached, framenavigated, worker",
                other
            ))),
        }
    }

    /// See: <https://playwright.dev/docs/api/class-page#page-event-request>
    pub async fn on_request<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |request: Request| -> RequestHandlerFuture {
            Box::pin(handler(request))
        });

        let needs_subscription = {
            let handlers = self.request_handlers.lock().unwrap();
            let waiters = self.request_waiters.lock().unwrap();
            handlers.is_empty() && waiters.is_empty()
        };
        if needs_subscription {
            _ = self.channel().update_subscription("request", true).await;
        }
        self.request_handlers.lock().unwrap().push(handler);

        Ok(())
    }

    /// See: <https://playwright.dev/docs/api/class-page#page-event-request-finished>
    pub async fn on_request_finished<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |request: Request| -> RequestHandlerFuture {
            Box::pin(handler(request))
        });

        let needs_subscription = self.request_finished_handlers.lock().unwrap().is_empty();
        if needs_subscription {
            _ = self
                .channel()
                .update_subscription("requestFinished", true)
                .await;
        }
        self.request_finished_handlers.lock().unwrap().push(handler);

        Ok(())
    }

    /// See: <https://playwright.dev/docs/api/class-page#page-event-request-failed>
    pub async fn on_request_failed<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |request: Request| -> RequestHandlerFuture {
            Box::pin(handler(request))
        });

        let needs_subscription = self.request_failed_handlers.lock().unwrap().is_empty();
        if needs_subscription {
            _ = self
                .channel()
                .update_subscription("requestFailed", true)
                .await;
        }
        self.request_failed_handlers.lock().unwrap().push(handler);

        Ok(())
    }

    /// See: <https://playwright.dev/docs/api/class-page#page-event-response>
    pub async fn on_response<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(ResponseObject) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |response: ResponseObject| -> ResponseHandlerFuture {
            Box::pin(handler(response))
        });

        let needs_subscription = {
            let handlers = self.response_handlers.lock().unwrap();
            let waiters = self.response_waiters.lock().unwrap();
            handlers.is_empty() && waiters.is_empty()
        };
        if needs_subscription {
            _ = self.channel().update_subscription("response", true).await;
        }
        self.response_handlers.lock().unwrap().push(handler);

        Ok(())
    }

    /// Adds a listener for the `websocket` event.
    ///
    /// The handler will be called when a WebSocket request is dispatched.
    ///
    /// # Arguments
    ///
    /// * `handler` - The function to call when the event occurs
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-on-websocket>
    pub async fn on_websocket<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(WebSocket) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler =
            Arc::new(move |ws: WebSocket| -> WebSocketHandlerFuture { Box::pin(handler(ws)) });
        self.websocket_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `worker` event.
    ///
    /// The handler is called when a new Web Worker is created in the page.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure called with the new [`Worker`] object
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-worker>
    pub async fn on_worker<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Worker) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |w: Worker| -> WorkerHandlerFuture { Box::pin(handler(w)) });
        self.worker_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `close` event.
    ///
    /// The handler is called when the page is closed, either by calling `page.close()`,
    /// by the browser context being closed, or when the browser process exits.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure called with no arguments when the page closes
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-close>
    pub async fn on_close<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move || -> CloseHandlerFuture { Box::pin(handler()) });
        self.close_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `load` event.
    ///
    /// The handler is called when the page's `load` event fires, i.e. after
    /// all resources including stylesheets and images have finished loading.
    ///
    /// The server only sends `"load"` events after the first handler is registered
    /// (subscription is managed automatically).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure called with no arguments when the page loads
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-load>
    pub async fn on_load<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move || -> LoadHandlerFuture { Box::pin(handler()) });
        // "load" events come via Frame's "loadstate" event, no subscription needed.
        self.load_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `crash` event.
    ///
    /// The handler is called when the page crashes (e.g. runs out of memory).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure called with no arguments when the page crashes
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-crash>
    pub async fn on_crash<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move || -> CrashHandlerFuture { Box::pin(handler()) });
        self.crash_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `pageError` event.
    ///
    /// The handler is called when an uncaught JavaScript exception is thrown in the page.
    /// The handler receives the error message as a `String`.
    ///
    /// The server only sends `"pageError"` events after the first handler is registered
    /// (subscription is managed automatically).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the error message string
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-page-error>
    pub async fn on_pageerror<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler =
            Arc::new(move |msg: String| -> PageErrorHandlerFuture { Box::pin(handler(msg)) });
        // "pageError" events come via BrowserContext, no subscription needed.
        self.pageerror_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `popup` event.
    ///
    /// The handler is called when the page opens a popup window (e.g. via `window.open()`).
    /// The handler receives the new popup [`Page`] object.
    ///
    /// The server only sends `"popup"` events after the first handler is registered
    /// (subscription is managed automatically).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the popup Page
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-popup>
    pub async fn on_popup<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Page) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |page: Page| -> PopupHandlerFuture { Box::pin(handler(page)) });
        // "popup" events arrive via BrowserContext's "page" event when a page has an opener.
        self.popup_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `frameAttached` event.
    ///
    /// The handler is called when a new frame (iframe) is attached to the page.
    /// The handler receives the attached [`Frame`](crate::protocol::Frame) object.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the attached Frame
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-frameattached>
    pub async fn on_frameattached<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::Frame) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |frame: crate::protocol::Frame| -> FrameEventHandlerFuture {
                Box::pin(handler(frame))
            },
        );
        self.frameattached_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `frameDetached` event.
    ///
    /// The handler is called when a frame (iframe) is detached from the page.
    /// The handler receives the detached [`Frame`](crate::protocol::Frame) object.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the detached Frame
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-framedetached>
    pub async fn on_framedetached<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::Frame) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |frame: crate::protocol::Frame| -> FrameEventHandlerFuture {
                Box::pin(handler(frame))
            },
        );
        self.framedetached_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `frameNavigated` event.
    ///
    /// The handler is called when a frame navigates to a new URL.
    /// The handler receives the navigated [`Frame`](crate::protocol::Frame) object.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the navigated Frame
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-framenavigated>
    pub async fn on_framenavigated<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::Frame) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |frame: crate::protocol::Frame| -> FrameEventHandlerFuture {
                Box::pin(handler(frame))
            },
        );
        self.framenavigated_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Exposes a Rust function to this page as `window[name]` in JavaScript.
    ///
    /// When JavaScript code calls `window[name](arg1, arg2, …)` the Playwright
    /// server fires a `bindingCall` event on the **page** channel that invokes
    /// `callback` with the deserialized arguments. The return value is sent back
    /// to JS so the `await window[name](…)` expression resolves with it.
    ///
    /// The binding is page-scoped and not visible to other pages in the same context.
    ///
    /// # Arguments
    ///
    /// * `name`     – JavaScript identifier that will be available as `window[name]`.
    /// * `callback` – Async closure called with `Vec<serde_json::Value>` (JS arguments)
    ///   returning `serde_json::Value` (the result).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The page has been closed.
    /// - Communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-expose-function>
    pub async fn expose_function<F, Fut>(&self, name: &str, callback: F) -> Result<()>
    where
        F: Fn(Vec<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = serde_json::Value> + Send + 'static,
    {
        self.expose_binding_internal(name, false, callback).await
    }

    /// Exposes a Rust function to this page as `window[name]` in JavaScript,
    /// with `needsHandle: true`.
    ///
    /// Identical to [`expose_function`](Self::expose_function) but the Playwright
    /// server passes the first argument as a `JSHandle` object rather than a plain
    /// value.
    ///
    /// # Arguments
    ///
    /// * `name`     – JavaScript identifier.
    /// * `callback` – Async closure with `Vec<serde_json::Value>` → `serde_json::Value`.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The page has been closed.
    /// - Communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-expose-binding>
    pub async fn expose_binding<F, Fut>(&self, name: &str, callback: F) -> Result<()>
    where
        F: Fn(Vec<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = serde_json::Value> + Send + 'static,
    {
        self.expose_binding_internal(name, true, callback).await
    }

    /// Internal implementation shared by page-level expose_function and expose_binding.
    ///
    /// Both `expose_function` and `expose_binding` use `needsHandle: false` because
    /// the current implementation does not support JSHandle objects. Using
    /// `needsHandle: true` would cause the Playwright server to wrap the first
    /// argument as a `JSHandle`, which requires a JSHandle protocol object that
    /// is not yet implemented.
    async fn expose_binding_internal<F, Fut>(
        &self,
        name: &str,
        _needs_handle: bool,
        callback: F,
    ) -> Result<()>
    where
        F: Fn(Vec<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = serde_json::Value> + Send + 'static,
    {
        let callback: PageBindingCallback = Arc::new(move |args: Vec<serde_json::Value>| {
            Box::pin(callback(args)) as PageBindingCallbackFuture
        });

        // Store callback before sending RPC (avoids race with early bindingCall events)
        self.binding_callbacks
            .lock()
            .unwrap()
            .insert(name.to_string(), callback);

        // Tell the Playwright server to inject window[name] into this page.
        // Always use needsHandle: false — see note above.
        self.channel()
            .send_no_result(
                "exposeBinding",
                serde_json::json!({ "name": name, "needsHandle": false }),
            )
            .await
    }

    /// Handles a download event from the protocol
    async fn on_download_event(&self, download: Download) {
        let handlers = self.download_handlers.lock().unwrap().clone();

        for handler in handlers {
            if let Err(e) = handler(download.clone()).await {
                tracing::warn!("Download handler error: {}", e);
            }
        }
        // Notify the first expect_download() waiter (FIFO order)
        if let Some(tx) = self.download_waiters.lock().unwrap().pop() {
            let _ = tx.send(download);
        }
    }

    /// Handles a dialog event from the protocol
    async fn on_dialog_event(&self, dialog: Dialog) {
        let handlers = self.dialog_handlers.lock().unwrap().clone();

        for handler in handlers {
            if let Err(e) = handler(dialog.clone()).await {
                tracing::warn!("Dialog handler error: {}", e);
            }
        }
    }

    async fn on_request_event(&self, request: Request) {
        let handlers = self.request_handlers.lock().unwrap().clone();

        for handler in handlers {
            if let Err(e) = handler(request.clone()).await {
                tracing::warn!("Request handler error: {}", e);
            }
        }
        // Notify the first expect_request() waiter (FIFO order)
        if let Some(tx) = self.request_waiters.lock().unwrap().pop() {
            let _ = tx.send(request);
        }
    }

    async fn on_request_failed_event(&self, request: Request) {
        let handlers = self.request_failed_handlers.lock().unwrap().clone();

        for handler in handlers {
            if let Err(e) = handler(request.clone()).await {
                tracing::warn!("RequestFailed handler error: {}", e);
            }
        }
    }

    async fn on_request_finished_event(&self, request: Request) {
        let handlers = self.request_finished_handlers.lock().unwrap().clone();

        for handler in handlers {
            if let Err(e) = handler(request.clone()).await {
                tracing::warn!("RequestFinished handler error: {}", e);
            }
        }
    }

    async fn on_response_event(&self, response: ResponseObject) {
        let handlers = self.response_handlers.lock().unwrap().clone();

        for handler in handlers {
            if let Err(e) = handler(response.clone()).await {
                tracing::warn!("Response handler error: {}", e);
            }
        }
        // Notify the first expect_response() waiter (FIFO order)
        if let Some(tx) = self.response_waiters.lock().unwrap().pop() {
            let _ = tx.send(response);
        }
    }

    /// Registers a handler function that runs whenever a locator matches an element on the page.
    ///
    /// This is useful for handling overlays (cookie banners, modals, permission dialogs)
    /// that appear unexpectedly and need to be dismissed before test actions can proceed.
    ///
    /// When a matching element appears, Playwright sends a `locatorHandlerTriggered` event.
    /// The handler is called with the matching `Locator`. After the handler completes,
    /// Playwright is notified via `resolveLocatorHandler` so it can resume pending actions.
    ///
    /// # Arguments
    ///
    /// * `locator` - A locator identifying the overlay element to watch for
    /// * `handler` - Async function called with the matching Locator when the element appears
    /// * `options` - Optional settings (no_wait_after, times)
    ///
    /// # Errors
    ///
    /// Returns error if communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-add-locator-handler>
    pub async fn add_locator_handler<F, Fut>(
        &self,
        locator: &crate::protocol::Locator,
        handler: F,
        options: Option<AddLocatorHandlerOptions>,
    ) -> Result<()>
    where
        F: Fn(crate::protocol::Locator) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let selector = locator.selector().to_string();
        let no_wait_after = options
            .as_ref()
            .and_then(|o| o.no_wait_after)
            .unwrap_or(false);
        let times = options.as_ref().and_then(|o| o.times);

        // Send registerLocatorHandler RPC — returns {"uid": N}
        let params = serde_json::json!({
            "selector": selector,
            "noWaitAfter": no_wait_after,
        });
        let result: Value = self
            .channel()
            .send("registerLocatorHandler", params)
            .await?;

        let uid = result
            .get("uid")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .ok_or_else(|| {
                Error::ProtocolError("registerLocatorHandler response missing 'uid'".to_string())
            })?;

        let handler_fn: LocatorHandlerFn = Arc::new(
            move |loc: crate::protocol::Locator| -> LocatorHandlerFuture { Box::pin(handler(loc)) },
        );

        self.locator_handlers
            .lock()
            .unwrap()
            .push(LocatorHandlerEntry {
                uid,
                selector,
                handler: handler_fn,
                times_remaining: times,
            });

        Ok(())
    }

    /// Removes a previously registered locator handler.
    ///
    /// Sends `unregisterLocatorHandler` to the Playwright server using the uid
    /// that was assigned when the handler was first registered.
    ///
    /// # Arguments
    ///
    /// * `locator` - The same locator that was passed to `add_locator_handler`
    ///
    /// # Errors
    ///
    /// Returns error if no handler for this locator is registered, or if
    /// communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-remove-locator-handler>
    pub async fn remove_locator_handler(&self, locator: &crate::protocol::Locator) -> Result<()> {
        let selector = locator.selector();

        // Find the uid for this selector
        let uid = {
            let handlers = self.locator_handlers.lock().unwrap();
            handlers
                .iter()
                .find(|e| e.selector == selector)
                .map(|e| e.uid)
        };

        let uid = uid.ok_or_else(|| {
            Error::ProtocolError(format!(
                "No locator handler registered for selector '{}'",
                selector
            ))
        })?;

        // Send unregisterLocatorHandler RPC
        self.channel()
            .send_no_result(
                "unregisterLocatorHandler",
                serde_json::json!({ "uid": uid }),
            )
            .await?;

        // Remove from local registry
        self.locator_handlers
            .lock()
            .unwrap()
            .retain(|e| e.uid != uid);

        Ok(())
    }

    /// Triggers dialog event (called by BrowserContext when dialog events arrive)
    ///
    /// Dialog events are sent to BrowserContext and forwarded to the associated Page.
    /// This method is public so BrowserContext can forward dialog events.
    pub async fn trigger_dialog_event(&self, dialog: Dialog) {
        self.on_dialog_event(dialog).await;
    }

    /// Triggers request event (called by BrowserContext when request events arrive)
    pub(crate) async fn trigger_request_event(&self, request: Request) {
        self.on_request_event(request).await;
    }

    pub(crate) async fn trigger_request_finished_event(&self, request: Request) {
        self.on_request_finished_event(request).await;
    }

    pub(crate) async fn trigger_request_failed_event(&self, request: Request) {
        self.on_request_failed_event(request).await;
    }

    /// Triggers response event (called by BrowserContext when response events arrive)
    pub(crate) async fn trigger_response_event(&self, response: ResponseObject) {
        self.on_response_event(response).await;
    }

    /// Triggers console event (called by BrowserContext when console events arrive).
    ///
    /// The BrowserContext receives all `"console"` events, constructs the
    /// [`ConsoleMessage`](crate::protocol::ConsoleMessage), dispatches to
    /// context-level handlers, then calls this method to forward to page-level handlers.
    pub(crate) async fn trigger_console_event(&self, msg: crate::protocol::ConsoleMessage) {
        self.on_console_event(msg).await;
    }

    async fn on_console_event(&self, msg: crate::protocol::ConsoleMessage) {
        // Accumulate message for console_messages() accessor
        self.console_messages_log.lock().unwrap().push(msg.clone());
        // Notify the first expect_console_message() waiter (FIFO order)
        if let Some(tx) = self.console_waiters.lock().unwrap().pop() {
            let _ = tx.send(msg.clone());
        }
        let handlers = self.console_handlers.lock().unwrap().clone();
        for handler in handlers {
            if let Err(e) = handler(msg.clone()).await {
                tracing::warn!("Console handler error: {}", e);
            }
        }
    }

    /// Dispatches a FileChooser event to registered handlers and one-shot waiters.
    async fn on_filechooser_event(&self, chooser: crate::protocol::FileChooser) {
        // Dispatch to persistent handlers
        let handlers = self.filechooser_handlers.lock().unwrap().clone();
        for handler in handlers {
            if let Err(e) = handler(chooser.clone()).await {
                tracing::warn!("FileChooser handler error: {}", e);
            }
        }

        // Notify the first expect_file_chooser() waiter (FIFO order)
        if let Some(tx) = self.filechooser_waiters.lock().unwrap().pop() {
            let _ = tx.send(chooser);
        }
    }

    /// Triggers load event (called by Frame when loadstate "load" is added)
    pub(crate) async fn trigger_load_event(&self) {
        self.on_load_event().await;
    }

    /// Triggers pageError event (called by BrowserContext when pageError arrives)
    pub(crate) async fn trigger_pageerror_event(&self, message: String) {
        self.on_pageerror_event(message).await;
    }

    /// Triggers popup event (called by BrowserContext when a page is opened with an opener)
    pub(crate) async fn trigger_popup_event(&self, popup: Page) {
        self.on_popup_event(popup).await;
    }

    /// Triggers frameNavigated event (called by Frame when "navigated" is received)
    pub(crate) async fn trigger_framenavigated_event(&self, frame: crate::protocol::Frame) {
        self.on_framenavigated_event(frame).await;
    }

    async fn on_close_event(&self) {
        let handlers = self.close_handlers.lock().unwrap().clone();
        for handler in handlers {
            if let Err(e) = handler().await {
                tracing::warn!("Close handler error: {}", e);
            }
        }
        // Notify expect_event("close") waiters
        let waiters: Vec<_> = self.close_waiters.lock().unwrap().drain(..).collect();
        for tx in waiters {
            let _ = tx.send(());
        }
    }

    async fn on_load_event(&self) {
        let handlers = self.load_handlers.lock().unwrap().clone();
        for handler in handlers {
            if let Err(e) = handler().await {
                tracing::warn!("Load handler error: {}", e);
            }
        }
        // Notify expect_event("load") waiters
        let waiters: Vec<_> = self.load_waiters.lock().unwrap().drain(..).collect();
        for tx in waiters {
            let _ = tx.send(());
        }
    }

    async fn on_crash_event(&self) {
        let handlers = self.crash_handlers.lock().unwrap().clone();
        for handler in handlers {
            if let Err(e) = handler().await {
                tracing::warn!("Crash handler error: {}", e);
            }
        }
        // Notify expect_event("crash") waiters
        let waiters: Vec<_> = self.crash_waiters.lock().unwrap().drain(..).collect();
        for tx in waiters {
            let _ = tx.send(());
        }
    }

    async fn on_pageerror_event(&self, message: String) {
        // Accumulate error for page_errors() accessor
        self.page_errors_log.lock().unwrap().push(message.clone());
        let handlers = self.pageerror_handlers.lock().unwrap().clone();
        for handler in handlers {
            if let Err(e) = handler(message.clone()).await {
                tracing::warn!("PageError handler error: {}", e);
            }
        }
        // Notify expect_event("pageerror") waiters
        if let Some(tx) = self.pageerror_waiters.lock().unwrap().pop() {
            let _ = tx.send(message);
        }
    }

    async fn on_popup_event(&self, popup: Page) {
        let handlers = self.popup_handlers.lock().unwrap().clone();
        for handler in handlers {
            if let Err(e) = handler(popup.clone()).await {
                tracing::warn!("Popup handler error: {}", e);
            }
        }
        // Notify the first expect_popup() waiter (FIFO order)
        if let Some(tx) = self.popup_waiters.lock().unwrap().pop() {
            let _ = tx.send(popup);
        }
    }

    async fn on_frameattached_event(&self, frame: crate::protocol::Frame) {
        let handlers = self.frameattached_handlers.lock().unwrap().clone();
        for handler in handlers {
            if let Err(e) = handler(frame.clone()).await {
                tracing::warn!("FrameAttached handler error: {}", e);
            }
        }
        if let Some(tx) = self.frameattached_waiters.lock().unwrap().pop() {
            let _ = tx.send(frame);
        }
    }

    async fn on_framedetached_event(&self, frame: crate::protocol::Frame) {
        let handlers = self.framedetached_handlers.lock().unwrap().clone();
        for handler in handlers {
            if let Err(e) = handler(frame.clone()).await {
                tracing::warn!("FrameDetached handler error: {}", e);
            }
        }
        if let Some(tx) = self.framedetached_waiters.lock().unwrap().pop() {
            let _ = tx.send(frame);
        }
    }

    async fn on_framenavigated_event(&self, frame: crate::protocol::Frame) {
        let handlers = self.framenavigated_handlers.lock().unwrap().clone();
        for handler in handlers {
            if let Err(e) = handler(frame.clone()).await {
                tracing::warn!("FrameNavigated handler error: {}", e);
            }
        }
        if let Some(tx) = self.framenavigated_waiters.lock().unwrap().pop() {
            let _ = tx.send(frame);
        }
    }

    /// Adds a `<style>` tag into the page with the desired content.
    ///
    /// # Arguments
    ///
    /// * `options` - Style tag options (content, url, or path)
    ///
    /// # Returns
    ///
    /// Returns an ElementHandle pointing to the injected `<style>` tag
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::{Playwright, AddStyleTagOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let context = browser.new_context().await?;
    /// # let page = context.new_page().await?;
    /// use playwright_rs::protocol::AddStyleTagOptions;
    ///
    /// // With inline CSS
    /// page.add_style_tag(
    ///     AddStyleTagOptions::builder()
    ///         .content("body { background-color: red; }")
    ///         .build()
    /// ).await?;
    ///
    /// // With external URL
    /// page.add_style_tag(
    ///     AddStyleTagOptions::builder()
    ///         .url("https://example.com/style.css")
    ///         .build()
    /// ).await?;
    ///
    /// // From file
    /// page.add_style_tag(
    ///     AddStyleTagOptions::builder()
    ///         .path("./styles/custom.css")
    ///         .build()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-add-style-tag>
    pub async fn add_style_tag(
        &self,
        options: AddStyleTagOptions,
    ) -> Result<Arc<crate::protocol::ElementHandle>> {
        let frame = self.main_frame().await?;
        frame.add_style_tag(options).await
    }

    /// Adds a script which would be evaluated in one of the following scenarios:
    /// - Whenever the page is navigated
    /// - Whenever a child frame is attached or navigated
    ///
    /// The script is evaluated after the document was created but before any of its scripts were run.
    ///
    /// # Arguments
    ///
    /// * `script` - JavaScript code to be injected into the page
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let context = browser.new_context().await?;
    /// # let page = context.new_page().await?;
    /// page.add_init_script("window.injected = 123;").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-add-init-script>
    pub async fn add_init_script(&self, script: &str) -> Result<()> {
        self.channel()
            .send_no_result("addInitScript", serde_json::json!({ "source": script }))
            .await
    }

    /// Sets the viewport size for the page.
    ///
    /// This method allows dynamic resizing of the viewport after page creation,
    /// useful for testing responsive layouts at different screen sizes.
    ///
    /// # Arguments
    ///
    /// * `viewport` - The viewport dimensions (width and height in pixels)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::{Playwright, Viewport};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// // Set viewport to mobile size
    /// let mobile = Viewport {
    ///     width: 375,
    ///     height: 667,
    /// };
    /// page.set_viewport_size(mobile).await?;
    ///
    /// // Later, test desktop layout
    /// let desktop = Viewport {
    ///     width: 1920,
    ///     height: 1080,
    /// };
    /// page.set_viewport_size(desktop).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-set-viewport-size>
    pub async fn set_viewport_size(&self, viewport: crate::protocol::Viewport) -> Result<()> {
        // Store the new viewport locally so viewport_size() can reflect the change
        if let Ok(mut guard) = self.viewport.write() {
            *guard = Some(viewport.clone());
        }
        self.channel()
            .send_no_result(
                "setViewportSize",
                serde_json::json!({ "viewportSize": viewport }),
            )
            .await
    }

    /// Brings this page to the front (activates the tab).
    ///
    /// Activates the page in the browser, making it the focused tab. This is
    /// useful in multi-page tests to ensure actions target the correct page.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-bring-to-front>
    pub async fn bring_to_front(&self) -> Result<()> {
        self.channel()
            .send_no_result("bringToFront", serde_json::json!({}))
            .await
    }

    /// Forces garbage collection in the browser (Chromium only).
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-request-gc>
    pub async fn request_gc(&self) -> Result<()> {
        self.channel()
            .send_no_result("requestGC", serde_json::json!({}))
            .await
    }

    /// Sets extra HTTP headers that will be sent with every request from this page.
    ///
    /// These headers are sent in addition to headers set on the browser context via
    /// `BrowserContext::set_extra_http_headers()`. Page-level headers take precedence
    /// over context-level headers when names conflict.
    ///
    /// # Arguments
    ///
    /// * `headers` - Map of header names to values.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-set-extra-http-headers>
    pub async fn set_extra_http_headers(
        &self,
        headers: std::collections::HashMap<String, String>,
    ) -> Result<()> {
        // Playwright protocol expects an array of {name, value} objects
        // This RPC is sent on the Page channel (not the Frame channel)
        let headers_array: Vec<serde_json::Value> = headers
            .into_iter()
            .map(|(name, value)| serde_json::json!({ "name": name, "value": value }))
            .collect();
        self.channel()
            .send_no_result(
                "setExtraHTTPHeaders",
                serde_json::json!({ "headers": headers_array }),
            )
            .await
    }

    /// Emulates media features for the page.
    ///
    /// This method allows emulating CSS media features such as `media`, `color-scheme`,
    /// `reduced-motion`, and `forced-colors`. Pass `None` to call with no changes.
    ///
    /// To reset a specific feature to the browser default, use the `NoOverride` variant.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional emulation options. If `None`, this is a no-op.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::{Playwright, EmulateMediaOptions, Media, ColorScheme};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// // Emulate print media
    /// page.emulate_media(Some(
    ///     EmulateMediaOptions::builder()
    ///         .media(Media::Print)
    ///         .build()
    /// )).await?;
    ///
    /// // Emulate dark color scheme
    /// page.emulate_media(Some(
    ///     EmulateMediaOptions::builder()
    ///         .color_scheme(ColorScheme::Dark)
    ///         .build()
    /// )).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
    pub async fn emulate_media(&self, options: Option<EmulateMediaOptions>) -> Result<()> {
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            if let Some(media) = opts.media {
                params["media"] = serde_json::to_value(media).map_err(|e| {
                    crate::error::Error::ProtocolError(format!("Failed to serialize media: {}", e))
                })?;
            }
            if let Some(color_scheme) = opts.color_scheme {
                params["colorScheme"] = serde_json::to_value(color_scheme).map_err(|e| {
                    crate::error::Error::ProtocolError(format!(
                        "Failed to serialize colorScheme: {}",
                        e
                    ))
                })?;
            }
            if let Some(reduced_motion) = opts.reduced_motion {
                params["reducedMotion"] = serde_json::to_value(reduced_motion).map_err(|e| {
                    crate::error::Error::ProtocolError(format!(
                        "Failed to serialize reducedMotion: {}",
                        e
                    ))
                })?;
            }
            if let Some(forced_colors) = opts.forced_colors {
                params["forcedColors"] = serde_json::to_value(forced_colors).map_err(|e| {
                    crate::error::Error::ProtocolError(format!(
                        "Failed to serialize forcedColors: {}",
                        e
                    ))
                })?;
            }
        }

        self.channel().send_no_result("emulateMedia", params).await
    }

    /// Generates a PDF of the page and returns it as bytes.
    ///
    /// Note: Generating a PDF is only supported in Chromium headless. PDF generation is
    /// not supported in Firefox or WebKit.
    ///
    /// The PDF bytes are returned. If `options.path` is set, the PDF will also be
    /// saved to that file.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional PDF generation options
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// let pdf_bytes = page.pdf(None).await?;
    /// assert!(!pdf_bytes.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The browser is not Chromium (PDF only supported in Chromium)
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-pdf>
    pub async fn pdf(&self, options: Option<PdfOptions>) -> Result<Vec<u8>> {
        let mut params = serde_json::json!({});
        let mut save_path: Option<std::path::PathBuf> = None;

        if let Some(opts) = options {
            // Capture the file path before consuming opts
            save_path = opts.path;

            if let Some(scale) = opts.scale {
                params["scale"] = serde_json::json!(scale);
            }
            if let Some(v) = opts.display_header_footer {
                params["displayHeaderFooter"] = serde_json::json!(v);
            }
            if let Some(v) = opts.header_template {
                params["headerTemplate"] = serde_json::json!(v);
            }
            if let Some(v) = opts.footer_template {
                params["footerTemplate"] = serde_json::json!(v);
            }
            if let Some(v) = opts.print_background {
                params["printBackground"] = serde_json::json!(v);
            }
            if let Some(v) = opts.landscape {
                params["landscape"] = serde_json::json!(v);
            }
            if let Some(v) = opts.page_ranges {
                params["pageRanges"] = serde_json::json!(v);
            }
            if let Some(v) = opts.format {
                params["format"] = serde_json::json!(v);
            }
            if let Some(v) = opts.width {
                params["width"] = serde_json::json!(v);
            }
            if let Some(v) = opts.height {
                params["height"] = serde_json::json!(v);
            }
            if let Some(v) = opts.prefer_css_page_size {
                params["preferCSSPageSize"] = serde_json::json!(v);
            }
            if let Some(margin) = opts.margin {
                params["margin"] = serde_json::to_value(margin).map_err(|e| {
                    crate::error::Error::ProtocolError(format!("Failed to serialize margin: {}", e))
                })?;
            }
        }

        #[derive(Deserialize)]
        struct PdfResponse {
            pdf: String,
        }

        let response: PdfResponse = self.channel().send("pdf", params).await?;

        // Decode base64 to bytes
        let pdf_bytes = base64::engine::general_purpose::STANDARD
            .decode(&response.pdf)
            .map_err(|e| {
                crate::error::Error::ProtocolError(format!("Failed to decode PDF base64: {}", e))
            })?;

        // If a path was specified, save the PDF to disk as well
        if let Some(path) = save_path {
            tokio::fs::write(&path, &pdf_bytes).await.map_err(|e| {
                crate::error::Error::InvalidArgument(format!(
                    "Failed to write PDF to '{}': {}",
                    path.display(),
                    e
                ))
            })?;
        }

        Ok(pdf_bytes)
    }

    /// Adds a `<script>` tag into the page with the desired URL or content.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional script tag options (content, url, or path).
    ///   If `None`, returns an error because no source is specified.
    ///
    /// At least one of `content`, `url`, or `path` must be provided.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::{Playwright, AddScriptTagOptions};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// # let context = browser.new_context().await?;
    /// # let page = context.new_page().await?;
    /// // With inline JavaScript
    /// page.add_script_tag(Some(
    ///     AddScriptTagOptions::builder()
    ///         .content("window.myVar = 42;")
    ///         .build()
    /// )).await?;
    ///
    /// // With external URL
    /// page.add_script_tag(Some(
    ///     AddScriptTagOptions::builder()
    ///         .url("https://example.com/script.js")
    ///         .build()
    /// )).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - `options` is `None` or no content/url/path is specified
    /// - Page has been closed
    /// - Script loading fails (e.g., invalid URL)
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-add-script-tag>
    pub async fn add_script_tag(
        &self,
        options: Option<AddScriptTagOptions>,
    ) -> Result<Arc<crate::protocol::ElementHandle>> {
        let opts = options.ok_or_else(|| {
            Error::InvalidArgument(
                "At least one of content, url, or path must be specified".to_string(),
            )
        })?;
        let frame = self.main_frame().await?;
        frame.add_script_tag(opts).await
    }

    /// Returns the current viewport size of the page, or `None` if no viewport is set.
    ///
    /// Returns `None` when the context was created with `no_viewport: true`. Otherwise
    /// returns the dimensions configured at context creation time or updated via
    /// `set_viewport_size()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use playwright_rs::protocol::{Playwright, BrowserContextOptions, Viewport};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// let context = browser.new_context_with_options(
    ///     BrowserContextOptions::builder().viewport(Viewport { width: 1280, height: 720 }).build()
    /// ).await?;
    /// let page = context.new_page().await?;
    /// let size = page.viewport_size().expect("Viewport should be set");
    /// assert_eq!(size.width, 1280);
    /// assert_eq!(size.height, 720);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-viewport-size>
    pub fn viewport_size(&self) -> Option<Viewport> {
        self.viewport.read().ok()?.clone()
    }

    /// Returns the `Accessibility` object for this page.
    ///
    /// Use `accessibility().snapshot()` to capture the current state of the
    /// page's accessibility tree.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-accessibility>
    pub fn accessibility(&self) -> crate::protocol::Accessibility {
        crate::protocol::Accessibility::new(self.clone())
    }

    /// Returns the `Coverage` object for this page (Chromium only).
    ///
    /// Use `coverage().start_js_coverage()` / `stop_js_coverage()` and
    /// `start_css_coverage()` / `stop_css_coverage()` to collect code coverage data.
    ///
    /// Coverage is only available in Chromium. Calling coverage methods on
    /// Firefox or WebKit will return an error from the Playwright server.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-coverage>
    pub fn coverage(&self) -> crate::protocol::Coverage {
        crate::protocol::Coverage::new(self.clone())
    }

    // Internal accessibility method (called by Accessibility struct)
    //
    // The legacy `accessibilitySnapshot` RPC was removed in modern Playwright.
    // We implement snapshot() using `FrameAriaSnapshot` on the main frame, which
    // returns the ARIA accessibility tree as a YAML string (the current equivalent).
    // The YAML string is returned as a JSON string Value for API compatibility.

    pub(crate) async fn accessibility_snapshot(
        &self,
        _options: Option<crate::protocol::accessibility::AccessibilitySnapshotOptions>,
    ) -> Result<serde_json::Value> {
        let frame = self.main_frame().await?;
        let timeout = self.default_timeout_ms();
        let snapshot = frame.aria_snapshot_raw("body", timeout).await?;
        Ok(serde_json::Value::String(snapshot))
    }

    // Internal coverage methods (called by Coverage struct)

    pub(crate) async fn coverage_start_js(
        &self,
        options: Option<crate::protocol::coverage::StartJSCoverageOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            if let Some(v) = opts.reset_on_navigation {
                params["resetOnNavigation"] = serde_json::json!(v);
            }
            if let Some(v) = opts.report_anonymous_scripts {
                params["reportAnonymousScripts"] = serde_json::json!(v);
            }
        }

        self.channel()
            .send_no_result("startJSCoverage", params)
            .await
    }

    pub(crate) async fn coverage_stop_js(
        &self,
    ) -> Result<Vec<crate::protocol::coverage::JSCoverageEntry>> {
        #[derive(serde::Deserialize)]
        struct StopJSCoverageResponse {
            entries: Vec<crate::protocol::coverage::JSCoverageEntry>,
        }

        let response: StopJSCoverageResponse = self
            .channel()
            .send("stopJSCoverage", serde_json::json!({}))
            .await?;

        Ok(response.entries)
    }

    pub(crate) async fn coverage_start_css(
        &self,
        options: Option<crate::protocol::coverage::StartCSSCoverageOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({});

        if let Some(opts) = options
            && let Some(v) = opts.reset_on_navigation
        {
            params["resetOnNavigation"] = serde_json::json!(v);
        }

        self.channel()
            .send_no_result("startCSSCoverage", params)
            .await
    }

    pub(crate) async fn coverage_stop_css(
        &self,
    ) -> Result<Vec<crate::protocol::coverage::CSSCoverageEntry>> {
        #[derive(serde::Deserialize)]
        struct StopCSSCoverageResponse {
            entries: Vec<crate::protocol::coverage::CSSCoverageEntry>,
        }

        let response: StopCSSCoverageResponse = self
            .channel()
            .send("stopCSSCoverage", serde_json::json!({}))
            .await?;

        Ok(response.entries)
    }
}

impl ChannelOwner for Page {
    fn guid(&self) -> &str {
        self.base.guid()
    }

    fn type_name(&self) -> &str {
        self.base.type_name()
    }

    fn parent(&self) -> Option<Arc<dyn ChannelOwner>> {
        self.base.parent()
    }

    fn connection(&self) -> Arc<dyn crate::server::connection::ConnectionLike> {
        self.base.connection()
    }

    fn initializer(&self) -> &Value {
        self.base.initializer()
    }

    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    fn dispose(&self, reason: crate::server::channel_owner::DisposeReason) {
        self.base.dispose(reason)
    }

    fn adopt(&self, child: Arc<dyn ChannelOwner>) {
        self.base.adopt(child)
    }

    fn add_child(&self, guid: Arc<str>, child: Arc<dyn ChannelOwner>) {
        self.base.add_child(guid, child)
    }

    fn remove_child(&self, guid: &str) {
        self.base.remove_child(guid)
    }

    fn on_event(&self, method: &str, params: Value) {
        match method {
            "navigated" => {
                // Update URL when page navigates
                if let Some(url_value) = params.get("url")
                    && let Some(url_str) = url_value.as_str()
                    && let Ok(mut url) = self.url.write()
                {
                    *url = url_str.to_string();
                }
            }
            "route" => {
                // Handle network routing event
                if let Some(route_guid) = params
                    .get("route")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    // Get the Route object from connection's registry
                    let connection = self.connection();
                    let route_guid_owned = route_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        // Get and downcast Route object
                        let route: Route =
                            match connection.get_typed::<Route>(&route_guid_owned).await {
                                Ok(r) => r,
                                Err(e) => {
                                    tracing::warn!("Failed to get route object: {}", e);
                                    return;
                                }
                            };

                        // Set APIRequestContext on the route for fetch() support.
                        // Page's parent is BrowserContext, which has the request context.
                        if let Some(ctx) =
                            downcast_parent::<crate::protocol::BrowserContext>(&self_clone)
                            && let Ok(api_ctx) = ctx.request().await
                        {
                            route.set_api_request_context(api_ctx);
                        }

                        // Call the route handler and wait for completion
                        self_clone.on_route_event(route).await;
                    });
                }
            }
            "video" => {
                // Handle video event: delivered once recording starts.
                // params: {artifact: {guid: "Artifact@..."}}
                if let Some(artifact_guid) = params
                    .get("artifact")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let artifact_guid_owned = artifact_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        let artifact_arc = match connection.get_object(&artifact_guid_owned).await {
                            Ok(obj) => obj,
                            Err(e) => {
                                tracing::warn!("Failed to get Artifact for video event: {}", e);
                                return;
                            }
                        };

                        if let Some(video) = &self_clone.video {
                            video.set_artifact(artifact_arc);
                        }
                    });
                }
            }
            "download" => {
                // Handle download event
                // Event params: {url, suggestedFilename, artifact: {guid: "..."}}
                let url = params
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let suggested_filename = params
                    .get("suggestedFilename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(artifact_guid) = params
                    .get("artifact")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let artifact_guid_owned = artifact_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        // Wait for Artifact object to be created
                        let artifact_arc = match connection.get_object(&artifact_guid_owned).await {
                            Ok(obj) => obj,
                            Err(e) => {
                                tracing::warn!("Failed to get artifact object: {}", e);
                                return;
                            }
                        };

                        // Create Download wrapper from Artifact + event params
                        let download = Download::from_artifact(
                            artifact_arc,
                            url,
                            suggested_filename,
                            self_clone.clone(),
                        );

                        // Call the download handlers
                        self_clone.on_download_event(download).await;
                    });
                }
            }
            "dialog" => {
                // Dialog events are handled by BrowserContext and forwarded to Page
                // This case should not be reached, but keeping for completeness
            }
            "webSocket" => {
                if let Some(ws_guid) = params
                    .get("webSocket")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let ws_guid_owned = ws_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        // Get and downcast WebSocket object
                        let ws: WebSocket =
                            match connection.get_typed::<WebSocket>(&ws_guid_owned).await {
                                Ok(ws) => ws,
                                Err(e) => {
                                    tracing::warn!("Failed to get WebSocket object: {}", e);
                                    return;
                                }
                            };

                        // Call handlers
                        let handlers = self_clone.websocket_handlers.lock().unwrap().clone();
                        for handler in handlers {
                            let ws_clone = ws.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handler(ws_clone).await {
                                    tracing::error!("Error in websocket handler: {}", e);
                                }
                            });
                        }
                    });
                }
            }
            "webSocketRoute" => {
                // A WebSocket matched a route_web_socket pattern.
                // Event format: {webSocketRoute: {guid: "WebSocketRoute@..."}}
                if let Some(wsr_guid) = params
                    .get("webSocketRoute")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let wsr_guid_owned = wsr_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        let route: crate::protocol::WebSocketRoute = match connection
                            .get_typed::<crate::protocol::WebSocketRoute>(&wsr_guid_owned)
                            .await
                        {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::warn!("Failed to get WebSocketRoute object: {}", e);
                                return;
                            }
                        };

                        let url = route.url().to_string();
                        let handlers = self_clone.ws_route_handlers.lock().unwrap().clone();
                        for entry in handlers.iter().rev() {
                            if crate::protocol::route::matches_pattern(&entry.pattern, &url) {
                                let handler = entry.handler.clone();
                                let route_clone = route.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handler(route_clone).await {
                                        tracing::error!("Error in webSocketRoute handler: {}", e);
                                    }
                                });
                                break;
                            }
                        }
                    });
                }
            }
            "worker" => {
                // A new Web Worker was created in the page.
                // Event format: {worker: {guid: "Worker@..."}}
                if let Some(worker_guid) = params
                    .get("worker")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let worker_guid_owned = worker_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        let worker: Worker =
                            match connection.get_typed::<Worker>(&worker_guid_owned).await {
                                Ok(w) => w,
                                Err(e) => {
                                    tracing::warn!("Failed to get Worker object: {}", e);
                                    return;
                                }
                            };

                        // Track the worker for workers() accessor
                        self_clone.workers_list.lock().unwrap().push(worker.clone());

                        let handlers = self_clone.worker_handlers.lock().unwrap().clone();
                        for handler in handlers {
                            let worker_clone = worker.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handler(worker_clone).await {
                                    tracing::error!("Error in worker handler: {}", e);
                                }
                            });
                        }
                        // Notify expect_event("worker") waiters
                        if let Some(tx) = self_clone.worker_waiters.lock().unwrap().pop() {
                            let _ = tx.send(worker);
                        }
                    });
                }
            }
            "bindingCall" => {
                // A JS caller on this page invoked a page-level exposed function.
                // Event format: {binding: {guid: "..."}}
                if let Some(binding_guid) = params
                    .get("binding")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let binding_guid_owned = binding_guid.to_string();
                    let binding_callbacks = self.binding_callbacks.clone();

                    tokio::spawn(async move {
                        let binding_call: crate::protocol::BindingCall = match connection
                            .get_typed::<crate::protocol::BindingCall>(&binding_guid_owned)
                            .await
                        {
                            Ok(bc) => bc,
                            Err(e) => {
                                tracing::warn!("Failed to get BindingCall object: {}", e);
                                return;
                            }
                        };

                        let name = binding_call.name().to_string();

                        // Look up page-level callback
                        let callback = {
                            let callbacks = binding_callbacks.lock().unwrap();
                            callbacks.get(&name).cloned()
                        };

                        let Some(callback) = callback else {
                            // No page-level handler — the context-level handler on
                            // BrowserContext::on_event("bindingCall") will handle it.
                            return;
                        };

                        // Deserialize args from Playwright protocol format
                        let raw_args = binding_call.args();
                        let args = crate::protocol::browser_context::BrowserContext::deserialize_binding_args_pub(raw_args);

                        // Call callback and serialize result
                        let result_value = callback(args).await;
                        let serialized =
                            crate::protocol::evaluate_conversion::serialize_argument(&result_value);

                        if let Err(e) = binding_call.resolve(serialized).await {
                            tracing::warn!("Failed to resolve BindingCall '{}': {}", name, e);
                        }
                    });
                }
            }
            "fileChooser" => {
                // FileChooser event: sent when an <input type="file"> is interacted with.
                // Event params: {element: {guid: "..."}, isMultiple: bool}
                let is_multiple = params
                    .get("isMultiple")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if let Some(element_guid) = params
                    .get("element")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let element_guid_owned = element_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        let element: crate::protocol::ElementHandle = match connection
                            .get_typed::<crate::protocol::ElementHandle>(&element_guid_owned)
                            .await
                        {
                            Ok(e) => e,
                            Err(err) => {
                                tracing::warn!(
                                    "Failed to get ElementHandle for fileChooser: {}",
                                    err
                                );
                                return;
                            }
                        };

                        let chooser = crate::protocol::FileChooser::new(
                            self_clone.clone(),
                            std::sync::Arc::new(element),
                            is_multiple,
                        );

                        self_clone.on_filechooser_event(chooser).await;
                    });
                }
            }
            "close" => {
                // Server-initiated close (e.g. context was closed)
                self.is_closed.store(true, Ordering::Relaxed);
                // Dispatch close handlers
                let self_clone = self.clone();
                tokio::spawn(async move {
                    self_clone.on_close_event().await;
                });
            }
            "load" => {
                let self_clone = self.clone();
                tokio::spawn(async move {
                    self_clone.on_load_event().await;
                });
            }
            "crash" => {
                let self_clone = self.clone();
                tokio::spawn(async move {
                    self_clone.on_crash_event().await;
                });
            }
            "pageError" => {
                // params: {"error": {"message": "...", "stack": "..."}}
                let message = params
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
                let self_clone = self.clone();
                tokio::spawn(async move {
                    self_clone.on_pageerror_event(message).await;
                });
            }
            // "popup" is forwarded from BrowserContext::on_event when a "page" event
            // is received for a page that has an opener. No direct "popup" event on Page.
            "frameAttached" => {
                // params: {"frame": {"guid": "..."}}
                if let Some(frame_guid) = params
                    .get("frame")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let frame_guid_owned = frame_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        let frame: crate::protocol::Frame = match connection
                            .get_typed::<crate::protocol::Frame>(&frame_guid_owned)
                            .await
                        {
                            Ok(f) => f,
                            Err(e) => {
                                tracing::warn!("Failed to get Frame for frameAttached: {}", e);
                                return;
                            }
                        };
                        self_clone.on_frameattached_event(frame).await;
                    });
                }
            }
            "frameDetached" => {
                // params: {"frame": {"guid": "..."}}
                if let Some(frame_guid) = params
                    .get("frame")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let frame_guid_owned = frame_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        let frame: crate::protocol::Frame = match connection
                            .get_typed::<crate::protocol::Frame>(&frame_guid_owned)
                            .await
                        {
                            Ok(f) => f,
                            Err(e) => {
                                tracing::warn!("Failed to get Frame for frameDetached: {}", e);
                                return;
                            }
                        };
                        self_clone.on_framedetached_event(frame).await;
                    });
                }
            }
            "frameNavigated" => {
                // params: {"frame": {"guid": "..."}}
                // Note: frameNavigated may also contain url, name, etc. at top level
                // The frame guid is in the "frame" field (same as attached/detached)
                if let Some(frame_guid) = params
                    .get("frame")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let frame_guid_owned = frame_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        let frame: crate::protocol::Frame = match connection
                            .get_typed::<crate::protocol::Frame>(&frame_guid_owned)
                            .await
                        {
                            Ok(f) => f,
                            Err(e) => {
                                tracing::warn!("Failed to get Frame for frameNavigated: {}", e);
                                return;
                            }
                        };
                        self_clone.on_framenavigated_event(frame).await;
                    });
                }
            }
            "locatorHandlerTriggered" => {
                // Server fires this when a registered locator matches an element.
                // params: {"uid": N}
                if let Some(uid) = params.get("uid").and_then(|v| v.as_u64()).map(|v| v as u32) {
                    let locator_handlers = self.locator_handlers.clone();
                    let self_clone = self.clone();

                    tokio::spawn(async move {
                        // Look up handler and decrement times_remaining
                        let (handler, selector, should_remove) = {
                            let mut handlers = locator_handlers.lock().unwrap();
                            let entry = handlers.iter_mut().find(|e| e.uid == uid);
                            match entry {
                                None => return,
                                Some(e) => {
                                    let handler = e.handler.clone();
                                    let selector = e.selector.clone();
                                    let remove = match e.times_remaining {
                                        Some(1) => true,
                                        Some(ref mut n) => {
                                            *n -= 1;
                                            false
                                        }
                                        None => false,
                                    };
                                    (handler, selector, remove)
                                }
                            }
                        };

                        // Build a Locator for the handler to receive
                        let locator = self_clone.locator(&selector).await;

                        // Run the handler
                        if let Err(e) = handler(locator).await {
                            tracing::warn!("locator handler error (uid={}): {}", uid, e);
                        }

                        // Send resolveLocatorHandler — remove=true if times exhausted
                        let _ = self_clone
                            .channel()
                            .send_no_result(
                                "resolveLocatorHandler",
                                serde_json::json!({ "uid": uid, "remove": should_remove }),
                            )
                            .await;

                        // Remove from local registry if one-shot
                        if should_remove {
                            self_clone
                                .locator_handlers
                                .lock()
                                .unwrap()
                                .retain(|e| e.uid != uid);
                        }
                    });
                }
            }
            _ => {
                // Other events not yet handled
            }
        }
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Page")
            .field("guid", &self.guid())
            .field("url", &self.url())
            .finish()
    }
}

/// Options for page.goto() and page.reload()
#[derive(Debug, Clone)]
pub struct GotoOptions {
    /// Maximum operation time in milliseconds
    pub timeout: Option<std::time::Duration>,
    /// When to consider operation succeeded
    pub wait_until: Option<WaitUntil>,
}

impl GotoOptions {
    /// Creates new GotoOptions with default values
    pub fn new() -> Self {
        Self {
            timeout: None,
            wait_until: None,
        }
    }

    /// Sets the timeout
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the wait_until option
    pub fn wait_until(mut self, wait_until: WaitUntil) -> Self {
        self.wait_until = Some(wait_until);
        self
    }
}

impl Default for GotoOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// When to consider navigation succeeded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitUntil {
    /// Consider operation to be finished when the `load` event is fired
    Load,
    /// Consider operation to be finished when the `DOMContentLoaded` event is fired
    DomContentLoaded,
    /// Consider operation to be finished when there are no network connections for at least 500ms
    NetworkIdle,
    /// Consider operation to be finished when the commit event is fired
    Commit,
}

impl WaitUntil {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            WaitUntil::Load => "load",
            WaitUntil::DomContentLoaded => "domcontentloaded",
            WaitUntil::NetworkIdle => "networkidle",
            WaitUntil::Commit => "commit",
        }
    }
}

/// Options for adding a style tag to the page
///
/// See: <https://playwright.dev/docs/api/class-page#page-add-style-tag>
#[derive(Debug, Clone, Default)]
pub struct AddStyleTagOptions {
    /// Raw CSS content to inject
    pub content: Option<String>,
    /// URL of the `<link>` tag to add
    pub url: Option<String>,
    /// Path to a CSS file to inject
    pub path: Option<String>,
}

impl AddStyleTagOptions {
    /// Creates a new builder for AddStyleTagOptions
    pub fn builder() -> AddStyleTagOptionsBuilder {
        AddStyleTagOptionsBuilder::default()
    }

    /// Validates that at least one option is specified
    pub(crate) fn validate(&self) -> Result<()> {
        if self.content.is_none() && self.url.is_none() && self.path.is_none() {
            return Err(Error::InvalidArgument(
                "At least one of content, url, or path must be specified".to_string(),
            ));
        }
        Ok(())
    }
}

/// Builder for AddStyleTagOptions
#[derive(Debug, Clone, Default)]
pub struct AddStyleTagOptionsBuilder {
    content: Option<String>,
    url: Option<String>,
    path: Option<String>,
}

impl AddStyleTagOptionsBuilder {
    /// Sets the CSS content to inject
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Sets the URL of the stylesheet
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the path to a CSS file
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Builds the AddStyleTagOptions
    pub fn build(self) -> AddStyleTagOptions {
        AddStyleTagOptions {
            content: self.content,
            url: self.url,
            path: self.path,
        }
    }
}

// ============================================================================
// AddScriptTagOptions
// ============================================================================

/// Options for adding a `<script>` tag to the page.
///
/// At least one of `content`, `url`, or `path` must be specified.
///
/// See: <https://playwright.dev/docs/api/class-page#page-add-script-tag>
#[derive(Debug, Clone, Default)]
pub struct AddScriptTagOptions {
    /// Raw JavaScript content to inject
    pub content: Option<String>,
    /// URL of the `<script>` tag to add
    pub url: Option<String>,
    /// Path to a JavaScript file to inject (file contents will be read and sent as content)
    pub path: Option<String>,
    /// Script type attribute (e.g., `"module"`)
    pub type_: Option<String>,
}

impl AddScriptTagOptions {
    /// Creates a new builder for AddScriptTagOptions
    pub fn builder() -> AddScriptTagOptionsBuilder {
        AddScriptTagOptionsBuilder::default()
    }

    /// Validates that at least one option is specified
    pub(crate) fn validate(&self) -> Result<()> {
        if self.content.is_none() && self.url.is_none() && self.path.is_none() {
            return Err(Error::InvalidArgument(
                "At least one of content, url, or path must be specified".to_string(),
            ));
        }
        Ok(())
    }
}

/// Builder for AddScriptTagOptions
#[derive(Debug, Clone, Default)]
pub struct AddScriptTagOptionsBuilder {
    content: Option<String>,
    url: Option<String>,
    path: Option<String>,
    type_: Option<String>,
}

impl AddScriptTagOptionsBuilder {
    /// Sets the JavaScript content to inject
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Sets the URL of the script to load
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the path to a JavaScript file to inject
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Sets the script type attribute (e.g., `"module"`)
    pub fn type_(mut self, type_: impl Into<String>) -> Self {
        self.type_ = Some(type_.into());
        self
    }

    /// Builds the AddScriptTagOptions
    pub fn build(self) -> AddScriptTagOptions {
        AddScriptTagOptions {
            content: self.content,
            url: self.url,
            path: self.path,
            type_: self.type_,
        }
    }
}

// ============================================================================
// EmulateMediaOptions and related enums
// ============================================================================

/// Media type for `page.emulate_media()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Media {
    /// Emulate screen media type
    Screen,
    /// Emulate print media type
    Print,
    /// Reset media emulation to browser default (sends `"no-override"` to protocol)
    #[serde(rename = "no-override")]
    NoOverride,
}

/// Preferred color scheme for `page.emulate_media()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ColorScheme {
    /// Emulate light color scheme
    #[serde(rename = "light")]
    Light,
    /// Emulate dark color scheme
    #[serde(rename = "dark")]
    Dark,
    /// Emulate no preference for color scheme
    #[serde(rename = "no-preference")]
    NoPreference,
    /// Reset color scheme to browser default
    #[serde(rename = "no-override")]
    NoOverride,
}

/// Reduced motion preference for `page.emulate_media()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReducedMotion {
    /// Emulate reduced motion preference
    #[serde(rename = "reduce")]
    Reduce,
    /// Emulate no preference for reduced motion
    #[serde(rename = "no-preference")]
    NoPreference,
    /// Reset reduced motion to browser default
    #[serde(rename = "no-override")]
    NoOverride,
}

/// Forced colors preference for `page.emulate_media()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ForcedColors {
    /// Emulate active forced colors
    #[serde(rename = "active")]
    Active,
    /// Emulate no forced colors
    #[serde(rename = "none")]
    None_,
    /// Reset forced colors to browser default
    #[serde(rename = "no-override")]
    NoOverride,
}

/// Options for `page.emulate_media()`.
///
/// All fields are optional. Fields that are `None` are omitted from the protocol
/// message (meaning they are not changed). To reset a field to browser default,
/// use the `NoOverride` variant.
///
/// See: <https://playwright.dev/docs/api/class-page#page-emulate-media>
#[derive(Debug, Clone, Default)]
pub struct EmulateMediaOptions {
    /// Media type to emulate (screen, print, or no-override)
    pub media: Option<Media>,
    /// Color scheme preference to emulate
    pub color_scheme: Option<ColorScheme>,
    /// Reduced motion preference to emulate
    pub reduced_motion: Option<ReducedMotion>,
    /// Forced colors preference to emulate
    pub forced_colors: Option<ForcedColors>,
}

impl EmulateMediaOptions {
    /// Creates a new builder for EmulateMediaOptions
    pub fn builder() -> EmulateMediaOptionsBuilder {
        EmulateMediaOptionsBuilder::default()
    }
}

/// Builder for EmulateMediaOptions
#[derive(Debug, Clone, Default)]
pub struct EmulateMediaOptionsBuilder {
    media: Option<Media>,
    color_scheme: Option<ColorScheme>,
    reduced_motion: Option<ReducedMotion>,
    forced_colors: Option<ForcedColors>,
}

impl EmulateMediaOptionsBuilder {
    /// Sets the media type to emulate
    pub fn media(mut self, media: Media) -> Self {
        self.media = Some(media);
        self
    }

    /// Sets the color scheme preference
    pub fn color_scheme(mut self, color_scheme: ColorScheme) -> Self {
        self.color_scheme = Some(color_scheme);
        self
    }

    /// Sets the reduced motion preference
    pub fn reduced_motion(mut self, reduced_motion: ReducedMotion) -> Self {
        self.reduced_motion = Some(reduced_motion);
        self
    }

    /// Sets the forced colors preference
    pub fn forced_colors(mut self, forced_colors: ForcedColors) -> Self {
        self.forced_colors = Some(forced_colors);
        self
    }

    /// Builds the EmulateMediaOptions
    pub fn build(self) -> EmulateMediaOptions {
        EmulateMediaOptions {
            media: self.media,
            color_scheme: self.color_scheme,
            reduced_motion: self.reduced_motion,
            forced_colors: self.forced_colors,
        }
    }
}

// ============================================================================
// PdfOptions
// ============================================================================

/// Margin options for PDF generation.
///
/// See: <https://playwright.dev/docs/api/class-page#page-pdf>
#[derive(Debug, Clone, Default, Serialize)]
pub struct PdfMargin {
    /// Top margin (e.g. `"1in"`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<String>,
    /// Right margin
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<String>,
    /// Bottom margin
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom: Option<String>,
    /// Left margin
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<String>,
}

/// Options for generating a PDF from a page.
///
/// Note: PDF generation is only supported by Chromium. Calling `page.pdf()` on
/// Firefox or WebKit will result in an error.
///
/// See: <https://playwright.dev/docs/api/class-page#page-pdf>
#[derive(Debug, Clone, Default)]
pub struct PdfOptions {
    /// If specified, the PDF will also be saved to this file path.
    pub path: Option<std::path::PathBuf>,
    /// Scale of the webpage rendering, between 0.1 and 2 (default 1).
    pub scale: Option<f64>,
    /// Whether to display header and footer (default false).
    pub display_header_footer: Option<bool>,
    /// HTML template for the print header. Should be valid HTML.
    pub header_template: Option<String>,
    /// HTML template for the print footer.
    pub footer_template: Option<String>,
    /// Whether to print background graphics (default false).
    pub print_background: Option<bool>,
    /// Paper orientation — `true` for landscape (default false).
    pub landscape: Option<bool>,
    /// Paper ranges to print, e.g. `"1-5, 8"`. Defaults to empty string (all pages).
    pub page_ranges: Option<String>,
    /// Paper format, e.g. `"Letter"` or `"A4"`. Overrides `width`/`height`.
    pub format: Option<String>,
    /// Paper width in CSS units, e.g. `"8.5in"`. Overrides `format`.
    pub width: Option<String>,
    /// Paper height in CSS units, e.g. `"11in"`. Overrides `format`.
    pub height: Option<String>,
    /// Whether or not to prefer page size as defined by CSS.
    pub prefer_css_page_size: Option<bool>,
    /// Paper margins, defaulting to none.
    pub margin: Option<PdfMargin>,
}

impl PdfOptions {
    /// Creates a new builder for PdfOptions
    pub fn builder() -> PdfOptionsBuilder {
        PdfOptionsBuilder::default()
    }
}

/// Builder for PdfOptions
#[derive(Debug, Clone, Default)]
pub struct PdfOptionsBuilder {
    path: Option<std::path::PathBuf>,
    scale: Option<f64>,
    display_header_footer: Option<bool>,
    header_template: Option<String>,
    footer_template: Option<String>,
    print_background: Option<bool>,
    landscape: Option<bool>,
    page_ranges: Option<String>,
    format: Option<String>,
    width: Option<String>,
    height: Option<String>,
    prefer_css_page_size: Option<bool>,
    margin: Option<PdfMargin>,
}

impl PdfOptionsBuilder {
    /// Sets the file path for saving the PDF
    pub fn path(mut self, path: std::path::PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Sets the scale of the webpage rendering
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = Some(scale);
        self
    }

    /// Sets whether to display header and footer
    pub fn display_header_footer(mut self, display: bool) -> Self {
        self.display_header_footer = Some(display);
        self
    }

    /// Sets the HTML template for the print header
    pub fn header_template(mut self, template: impl Into<String>) -> Self {
        self.header_template = Some(template.into());
        self
    }

    /// Sets the HTML template for the print footer
    pub fn footer_template(mut self, template: impl Into<String>) -> Self {
        self.footer_template = Some(template.into());
        self
    }

    /// Sets whether to print background graphics
    pub fn print_background(mut self, print: bool) -> Self {
        self.print_background = Some(print);
        self
    }

    /// Sets whether to use landscape orientation
    pub fn landscape(mut self, landscape: bool) -> Self {
        self.landscape = Some(landscape);
        self
    }

    /// Sets the page ranges to print
    pub fn page_ranges(mut self, ranges: impl Into<String>) -> Self {
        self.page_ranges = Some(ranges.into());
        self
    }

    /// Sets the paper format (e.g., `"Letter"`, `"A4"`)
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Sets the paper width
    pub fn width(mut self, width: impl Into<String>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the paper height
    pub fn height(mut self, height: impl Into<String>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Sets whether to prefer page size as defined by CSS
    pub fn prefer_css_page_size(mut self, prefer: bool) -> Self {
        self.prefer_css_page_size = Some(prefer);
        self
    }

    /// Sets the paper margins
    pub fn margin(mut self, margin: PdfMargin) -> Self {
        self.margin = Some(margin);
        self
    }

    /// Builds the PdfOptions
    pub fn build(self) -> PdfOptions {
        PdfOptions {
            path: self.path,
            scale: self.scale,
            display_header_footer: self.display_header_footer,
            header_template: self.header_template,
            footer_template: self.footer_template,
            print_background: self.print_background,
            landscape: self.landscape,
            page_ranges: self.page_ranges,
            format: self.format,
            width: self.width,
            height: self.height,
            prefer_css_page_size: self.prefer_css_page_size,
            margin: self.margin,
        }
    }
}

/// Response from navigation operations.
///
/// Returned from `page.goto()`, `page.reload()`, `page.go_back()`, and similar
/// navigation methods. Provides access to the HTTP response status, headers, and body.
///
/// See: <https://playwright.dev/docs/api/class-response>
#[derive(Clone)]
pub struct Response {
    url: String,
    status: u16,
    status_text: String,
    ok: bool,
    headers: std::collections::HashMap<String, String>,
    /// Reference to the backing channel owner for RPC calls (body, rawHeaders, etc.)
    /// Stored as the generic trait object so it can be downcast to ResponseObject when needed.
    response_channel_owner: Option<std::sync::Arc<dyn crate::server::channel_owner::ChannelOwner>>,
}

impl Response {
    /// Creates a new Response from protocol data.
    ///
    /// This is used internally when constructing a Response from the protocol
    /// initializer (e.g., after `goto` or `reload`).
    pub(crate) fn new(
        url: String,
        status: u16,
        status_text: String,
        headers: std::collections::HashMap<String, String>,
        response_channel_owner: Option<
            std::sync::Arc<dyn crate::server::channel_owner::ChannelOwner>,
        >,
    ) -> Self {
        Self {
            url,
            status,
            status_text,
            ok: (200..300).contains(&status),
            headers,
            response_channel_owner,
        }
    }
}

impl Response {
    /// Returns the URL of the response.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-url>
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the HTTP status code.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-status>
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Returns the HTTP status text.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-status-text>
    pub fn status_text(&self) -> &str {
        &self.status_text
    }

    /// Returns whether the response was successful (status 200-299).
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-ok>
    pub fn ok(&self) -> bool {
        self.ok
    }

    /// Returns the response headers as a HashMap.
    ///
    /// Note: these are the headers from the protocol initializer. For the full
    /// raw headers (including duplicates), use `headers_array()` or `all_headers()`.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-headers>
    pub fn headers(&self) -> &std::collections::HashMap<String, String> {
        &self.headers
    }

    /// Returns the [`Request`] that triggered this response.
    ///
    /// Navigates the protocol object hierarchy: ResponseObject → parent (Request).
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-request>
    pub fn request(&self) -> Option<crate::protocol::Request> {
        let owner = self.response_channel_owner.as_ref()?;
        downcast_parent::<crate::protocol::Request>(&**owner)
    }

    /// Returns the [`Frame`](crate::protocol::Frame) that initiated the request for this response.
    ///
    /// Navigates the protocol object hierarchy: ResponseObject → Request → Frame.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-frame>
    pub fn frame(&self) -> Option<crate::protocol::Frame> {
        let request = self.request()?;
        request.frame()
    }

    /// Returns the backing `ResponseObject`, or an error if unavailable.
    pub(crate) fn response_object(&self) -> crate::error::Result<crate::protocol::ResponseObject> {
        let arc = self.response_channel_owner.as_ref().ok_or_else(|| {
            crate::error::Error::ProtocolError(
                "Response has no backing protocol object".to_string(),
            )
        })?;
        arc.as_any()
            .downcast_ref::<crate::protocol::ResponseObject>()
            .cloned()
            .ok_or_else(|| crate::error::Error::TypeMismatch {
                guid: arc.guid().to_string(),
                expected: "ResponseObject".to_string(),
                actual: arc.type_name().to_string(),
            })
    }

    /// Returns TLS/SSL security details for HTTPS connections, or `None` for HTTP.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-security-details>
    pub async fn security_details(
        &self,
    ) -> crate::error::Result<Option<crate::protocol::response::SecurityDetails>> {
        self.response_object()?.security_details().await
    }

    /// Returns the server's IP address and port, or `None`.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-server-addr>
    pub async fn server_addr(
        &self,
    ) -> crate::error::Result<Option<crate::protocol::response::RemoteAddr>> {
        self.response_object()?.server_addr().await
    }

    /// Waits for this response to finish loading.
    ///
    /// For responses obtained from navigation methods (`goto`, `reload`), the response
    /// is already finished when returned. For responses from `on_response` handlers,
    /// the body may still be loading.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-finished>
    pub async fn finished(&self) -> crate::error::Result<()> {
        // The Playwright protocol dispatches `requestFinished` as a separate event
        // rather than exposing a `finished` RPC method on Response.
        // For responses from goto/reload, the response is already complete.
        // TODO: For on_response handlers, implement proper waiting via requestFinished event.
        Ok(())
    }

    /// Returns the response body as raw bytes.
    ///
    /// Makes an RPC call to the Playwright server to fetch the response body.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No backing protocol object is available (edge case)
    /// - The RPC call to the server fails
    /// - The base64 response cannot be decoded
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-body>
    pub async fn body(&self) -> crate::error::Result<Vec<u8>> {
        self.response_object()?.body().await
    }

    /// Returns the response body as a UTF-8 string.
    ///
    /// Calls `body()` then converts bytes to a UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `body()` fails
    /// - The body is not valid UTF-8
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-text>
    pub async fn text(&self) -> crate::error::Result<String> {
        let bytes = self.body().await?;
        String::from_utf8(bytes).map_err(|e| {
            crate::error::Error::ProtocolError(format!("Response body is not valid UTF-8: {}", e))
        })
    }

    /// Parses the response body as JSON and deserializes it into type `T`.
    ///
    /// Calls `text()` then uses `serde_json` to deserialize the body.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `text()` fails
    /// - The body is not valid JSON or doesn't match the expected type
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-json>
    pub async fn json<T: serde::de::DeserializeOwned>(&self) -> crate::error::Result<T> {
        let text = self.text().await?;
        serde_json::from_str(&text).map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to parse response JSON: {}", e))
        })
    }

    /// Returns all response headers as name-value pairs, preserving duplicates.
    ///
    /// Makes an RPC call for `"rawHeaders"` which returns the complete header list.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No backing protocol object is available (edge case)
    /// - The RPC call to the server fails
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-headers-array>
    pub async fn headers_array(
        &self,
    ) -> crate::error::Result<Vec<crate::protocol::response::HeaderEntry>> {
        self.response_object()?.raw_headers().await
    }

    /// Returns all response headers merged into a HashMap with lowercase keys.
    ///
    /// When multiple headers have the same name, their values are joined with `, `.
    /// This matches the behavior of `response.allHeaders()` in other Playwright bindings.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No backing protocol object is available (edge case)
    /// - The RPC call to the server fails
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-all-headers>
    pub async fn all_headers(
        &self,
    ) -> crate::error::Result<std::collections::HashMap<String, String>> {
        let entries = self.headers_array().await?;
        let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for entry in entries {
            let key = entry.name.to_lowercase();
            map.entry(key)
                .and_modify(|v| {
                    v.push_str(", ");
                    v.push_str(&entry.value);
                })
                .or_insert(entry.value);
        }
        Ok(map)
    }

    /// Returns the value for a single response header, or `None` if not present.
    ///
    /// The lookup is case-insensitive.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No backing protocol object is available (edge case)
    /// - The RPC call to the server fails
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-header-value>
    /// Returns the value for a single response header, or `None` if not present.
    ///
    /// The lookup is case-insensitive. When multiple headers share the same name,
    /// their values are joined with `, ` (matching Playwright's behavior).
    ///
    /// Uses the raw headers from the server for accurate results.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `headers_array()` RPC call fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-response#response-header-value>
    pub async fn header_value(&self, name: &str) -> crate::error::Result<Option<String>> {
        let entries = self.headers_array().await?;
        let name_lower = name.to_lowercase();
        let mut values: Vec<String> = entries
            .into_iter()
            .filter(|h| h.name.to_lowercase() == name_lower)
            .map(|h| h.value)
            .collect();

        if values.is_empty() {
            Ok(None)
        } else if values.len() == 1 {
            Ok(Some(values.remove(0)))
        } else {
            Ok(Some(values.join(", ")))
        }
    }
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Response")
            .field("url", &self.url)
            .field("status", &self.status)
            .field("status_text", &self.status_text)
            .field("ok", &self.ok)
            .finish_non_exhaustive()
    }
}

/// Options for `page.route_from_har()` and `context.route_from_har()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-route-from-har>
#[derive(Debug, Clone, Default)]
pub struct RouteFromHarOptions {
    /// URL glob pattern — only requests matching this pattern are served from
    /// the HAR file.  All requests are intercepted when omitted.
    pub url: Option<String>,

    /// Policy for requests not found in the HAR file.
    ///
    /// - `"abort"` (default) — terminate the request with a network error.
    /// - `"fallback"` — pass the request through to the next handler (or network).
    pub not_found: Option<String>,

    /// When `true`, record new network activity into the HAR file instead of
    /// replaying existing entries.  Defaults to `false`.
    pub update: Option<bool>,

    /// Content storage strategy used when `update` is `true`.
    ///
    /// - `"embed"` (default) — inline base64-encoded content in the HAR.
    /// - `"attach"` — store content as separate files alongside the HAR.
    pub update_content: Option<String>,

    /// Recording detail level used when `update` is `true`.
    ///
    /// - `"minimal"` (default) — omit timing, cookies, and security info.
    /// - `"full"` — record everything.
    pub update_mode: Option<String>,
}

/// Options for `page.add_locator_handler()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-add-locator-handler>
#[derive(Debug, Clone, Default)]
pub struct AddLocatorHandlerOptions {
    /// Whether to keep the page frozen after the handler has been called.
    ///
    /// When `false` (default), Playwright resumes normal page operation after
    /// the handler completes. When `true`, the page stays paused.
    pub no_wait_after: Option<bool>,

    /// Maximum number of times to invoke this handler.
    ///
    /// Once exhausted, the handler is automatically unregistered.
    /// `None` (default) means the handler runs indefinitely.
    pub times: Option<u32>,
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
