// BrowserContext protocol object
//
// Represents an isolated browser context (session) within a browser instance.
// Multiple contexts can exist in a single browser, each with its own cookies,
// cache, and local storage.

use crate::api::launch_options::IgnoreDefaultArgs;
use crate::error::{Error, Result};
use crate::protocol::api_request_context::APIRequestContext;
use crate::protocol::cdp_session::CDPSession;
use crate::protocol::event_waiter::EventWaiter;
use crate::protocol::route::UnrouteBehavior;
use crate::protocol::tracing::Tracing;
use crate::protocol::{Browser, Page, ProxySettings, Request, ResponseObject, Route};
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::server::connection::ConnectionExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

/// BrowserContext represents an isolated browser session.
///
/// Contexts are isolated environments within a browser instance. Each context
/// has its own cookies, cache, and local storage, enabling independent sessions
/// without interference.
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
///
///     // Create isolated contexts
///     let context1 = browser.new_context().await?;
///     let context2 = browser.new_context().await?;
///
///     // Create pages in each context
///     let page1 = context1.new_page().await?;
///     let page2 = context2.new_page().await?;
///
///     // Access all pages in a context
///     let pages = context1.pages();
///     assert_eq!(pages.len(), 1);
///
///     // Access the browser from a context
///     let ctx_browser = context1.browser().unwrap();
///     assert_eq!(ctx_browser.name(), browser.name());
///
///     // App mode: access initial page created automatically
///     let chromium = playwright.chromium();
///     let app_context = chromium
///         .launch_persistent_context_with_options(
///             "/tmp/app-data",
///             playwright_rs::protocol::BrowserContextOptions::builder()
///                 .args(vec!["--app=https://example.com".to_string()])
///                 .headless(true)
///                 .build()
///         )
///         .await?;
///
///     // Get the initial page (don't create a new one!)
///     let app_pages = app_context.pages();
///     if !app_pages.is_empty() {
///         let initial_page = &app_pages[0];
///         // Use the initial page...
///     }
///
///     // Cleanup
///     context1.close().await?;
///     context2.close().await?;
///     app_context.close().await?;
///     browser.close().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-browsercontext>
/// Type alias for boxed route handler future
type RouteHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed page handler future
type PageHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed close handler future
type CloseHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed request handler future
type RequestHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed response handler future
type ResponseHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed dialog handler future
type DialogHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for boxed binding callback future
type BindingCallbackFuture = Pin<Box<dyn Future<Output = serde_json::Value> + Send>>;

/// Context-level page event handler
type PageHandler = Arc<dyn Fn(Page) -> PageHandlerFuture + Send + Sync>;

/// Context-level close event handler
type CloseHandler = Arc<dyn Fn() -> CloseHandlerFuture + Send + Sync>;

/// Context-level request event handler
type RequestHandler = Arc<dyn Fn(Request) -> RequestHandlerFuture + Send + Sync>;

/// Context-level response event handler
type ResponseHandler = Arc<dyn Fn(ResponseObject) -> ResponseHandlerFuture + Send + Sync>;

/// Context-level dialog event handler
type DialogHandler = Arc<dyn Fn(crate::protocol::Dialog) -> DialogHandlerFuture + Send + Sync>;

/// Type alias for boxed console handler future
type ConsoleHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Context-level console event handler
type ConsoleHandler =
    Arc<dyn Fn(crate::protocol::ConsoleMessage) -> ConsoleHandlerFuture + Send + Sync>;

/// Type alias for boxed weberror handler future
type WebErrorHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Context-level weberror event handler
type WebErrorHandler =
    Arc<dyn Fn(crate::protocol::WebError) -> WebErrorHandlerFuture + Send + Sync>;

/// Type alias for boxed service worker handler future
type ServiceWorkerHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Context-level service worker event handler
type ServiceWorkerHandler =
    Arc<dyn Fn(crate::protocol::Worker) -> ServiceWorkerHandlerFuture + Send + Sync>;

/// Binding callback: receives deserialized JS args, returns a JSON value
type BindingCallback = Arc<dyn Fn(Vec<serde_json::Value>) -> BindingCallbackFuture + Send + Sync>;

/// Type alias for boxed WebSocketRoute handler future
type WsRouteHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Storage for a single route handler
#[derive(Clone)]
struct RouteHandlerEntry {
    pattern: String,
    handler: Arc<dyn Fn(Route) -> RouteHandlerFuture + Send + Sync>,
}

/// Storage for a single WebSocket route handler entry
#[derive(Clone)]
struct ContextWsRouteHandlerEntry {
    pattern: String,
    handler: Arc<dyn Fn(crate::protocol::WebSocketRoute) -> WsRouteHandlerFuture + Send + Sync>,
}

#[derive(Clone)]
pub struct BrowserContext {
    base: ChannelOwnerImpl,
    /// Browser instance that owns this context (None for persistent contexts)
    browser: Option<Browser>,
    /// All open pages in this context
    pages: Arc<Mutex<Vec<Page>>>,
    /// Route handlers for context-level network interception
    route_handlers: Arc<Mutex<Vec<RouteHandlerEntry>>>,
    /// APIRequestContext GUID from initializer (resolved lazily)
    request_context_guid: Option<String>,
    /// Tracing GUID from initializer (resolved lazily)
    tracing_guid: Option<String>,
    /// Default action timeout for all pages in this context (milliseconds), stored as f64 bits.
    default_timeout_ms: Arc<std::sync::atomic::AtomicU64>,
    /// Default navigation timeout for all pages in this context (milliseconds), stored as f64 bits.
    default_navigation_timeout_ms: Arc<std::sync::atomic::AtomicU64>,
    /// Context-level page event handlers (fired when a new page is created)
    page_handlers: Arc<Mutex<Vec<PageHandler>>>,
    /// Context-level close event handlers (fired when the context is closed)
    close_handlers: Arc<Mutex<Vec<CloseHandler>>>,
    /// Context-level request event handlers
    request_handlers: Arc<Mutex<Vec<RequestHandler>>>,
    /// Context-level request finished event handlers
    request_finished_handlers: Arc<Mutex<Vec<RequestHandler>>>,
    /// Context-level request failed event handlers
    request_failed_handlers: Arc<Mutex<Vec<RequestHandler>>>,
    /// Context-level response event handlers
    response_handlers: Arc<Mutex<Vec<ResponseHandler>>>,
    /// One-shot senders waiting for the next "page" event (expect_page)
    page_waiters: Arc<Mutex<Vec<oneshot::Sender<Page>>>>,
    /// One-shot senders waiting for the next "close" event (expect_close)
    close_waiters: Arc<Mutex<Vec<oneshot::Sender<()>>>>,
    /// Context-level dialog event handlers (fired for dialogs on any page in the context)
    dialog_handlers: Arc<Mutex<Vec<DialogHandler>>>,
    /// Registered binding callbacks keyed by name (for expose_function / expose_binding)
    binding_callbacks: Arc<Mutex<HashMap<String, BindingCallback>>>,
    /// Context-level console event handlers
    console_handlers: Arc<Mutex<Vec<ConsoleHandler>>>,
    /// One-shot senders waiting for the next "console" event (expect_console_message)
    console_waiters: Arc<Mutex<Vec<oneshot::Sender<crate::protocol::ConsoleMessage>>>>,
    /// Context-level weberror event handlers (fired for uncaught JS exceptions from any page)
    weberror_handlers: Arc<Mutex<Vec<WebErrorHandler>>>,
    /// Context-level service worker event handlers (fired when a service worker is registered)
    serviceworker_handlers: Arc<Mutex<Vec<ServiceWorkerHandler>>>,
    /// One-shot senders waiting for the next "request" event (expect_event("request"))
    request_waiters: Arc<Mutex<Vec<oneshot::Sender<Request>>>>,
    /// One-shot senders waiting for the next "response" event (expect_event("response"))
    response_waiters: Arc<Mutex<Vec<oneshot::Sender<ResponseObject>>>>,
    /// One-shot senders waiting for the next "weberror" event (expect_event("weberror"))
    weberror_waiters: Arc<Mutex<Vec<oneshot::Sender<crate::protocol::WebError>>>>,
    /// One-shot senders waiting for the next "serviceworker" event (expect_event("serviceworker"))
    serviceworker_waiters: Arc<Mutex<Vec<oneshot::Sender<crate::protocol::Worker>>>>,
    /// Active service workers tracked via "serviceWorker" events
    service_workers_list: Arc<Mutex<Vec<crate::protocol::Worker>>>,
    /// WebSocketRoute handlers for route_web_socket()
    ws_route_handlers: Arc<Mutex<Vec<ContextWsRouteHandlerEntry>>>,
}

impl BrowserContext {
    /// Creates a new BrowserContext from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a BrowserContext object.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent Browser object
    /// * `type_name` - The protocol type name ("BrowserContext")
    /// * `guid` - The unique identifier for this context
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
        // Extract APIRequestContext GUID from initializer before moving it
        let request_context_guid = initializer
            .get("requestContext")
            .and_then(|v| v.get("guid"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract Tracing GUID from initializer before moving it
        let tracing_guid = initializer
            .get("tracing")
            .and_then(|v| v.get("guid"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent.clone()),
            type_name,
            guid,
            initializer,
        );

        // Store browser reference if parent is a Browser
        // Returns None only for special contexts (Android, Electron) where parent is not a Browser
        // For both regular contexts and persistent contexts, parent is a Browser instance
        let browser = parent.as_any().downcast_ref::<Browser>().cloned();

        let context = Self {
            base,
            browser,
            pages: Arc::new(Mutex::new(Vec::new())),
            route_handlers: Arc::new(Mutex::new(Vec::new())),
            request_context_guid,
            tracing_guid,
            default_timeout_ms: Arc::new(std::sync::atomic::AtomicU64::new(
                crate::DEFAULT_TIMEOUT_MS.to_bits(),
            )),
            default_navigation_timeout_ms: Arc::new(std::sync::atomic::AtomicU64::new(
                crate::DEFAULT_TIMEOUT_MS.to_bits(),
            )),
            page_handlers: Arc::new(Mutex::new(Vec::new())),
            close_handlers: Arc::new(Mutex::new(Vec::new())),
            request_handlers: Arc::new(Mutex::new(Vec::new())),
            request_finished_handlers: Arc::new(Mutex::new(Vec::new())),
            request_failed_handlers: Arc::new(Mutex::new(Vec::new())),
            response_handlers: Arc::new(Mutex::new(Vec::new())),
            page_waiters: Arc::new(Mutex::new(Vec::new())),
            close_waiters: Arc::new(Mutex::new(Vec::new())),
            dialog_handlers: Arc::new(Mutex::new(Vec::new())),
            binding_callbacks: Arc::new(Mutex::new(HashMap::new())),
            console_handlers: Arc::new(Mutex::new(Vec::new())),
            console_waiters: Arc::new(Mutex::new(Vec::new())),
            weberror_handlers: Arc::new(Mutex::new(Vec::new())),
            serviceworker_handlers: Arc::new(Mutex::new(Vec::new())),
            request_waiters: Arc::new(Mutex::new(Vec::new())),
            response_waiters: Arc::new(Mutex::new(Vec::new())),
            weberror_waiters: Arc::new(Mutex::new(Vec::new())),
            serviceworker_waiters: Arc::new(Mutex::new(Vec::new())),
            service_workers_list: Arc::new(Mutex::new(Vec::new())),
            ws_route_handlers: Arc::new(Mutex::new(Vec::new())),
        };

        // Enable dialog and console event subscriptions eagerly.
        // Console events must be subscribed to receive them without a registered handler,
        // enabling the console_messages() and page_errors() passive accumulators on Page.
        let channel = context.channel().clone();
        tokio::spawn(async move {
            _ = channel.update_subscription("dialog", true).await;
            _ = channel.update_subscription("console", true).await;
        });

        // Note: Selectors registration is done by the caller (e.g. Browser::new_context())
        // after this object is returned, so that add_context() can be awaited properly.

        Ok(context)
    }

    /// Returns the channel for sending protocol messages
    ///
    /// Used internally for sending RPC calls to the context.
    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    /// Adds a script which would be evaluated in one of the following scenarios:
    ///
    /// - Whenever a page is created in the browser context or is navigated.
    /// - Whenever a child frame is attached or navigated in any page in the browser context.
    ///
    /// The script is evaluated after the document was created but before any of its scripts
    /// were run. This is useful to amend the JavaScript environment, e.g. to seed Math.random.
    ///
    /// # Arguments
    ///
    /// * `script` - Script to be evaluated in all pages in the browser context.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-add-init-script>
    pub async fn add_init_script(&self, script: &str) -> Result<()> {
        self.channel()
            .send_no_result("addInitScript", serde_json::json!({ "source": script }))
            .await
    }

    /// Creates a new page in this browser context.
    ///
    /// Pages are isolated tabs/windows within a context. Each page starts
    /// at "about:blank" and can be navigated independently.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-new-page>
    pub async fn new_page(&self) -> Result<Page> {
        // Response contains the GUID of the created Page
        #[derive(Deserialize)]
        struct NewPageResponse {
            page: GuidRef,
        }

        #[derive(Deserialize)]
        struct GuidRef {
            #[serde(deserialize_with = "crate::server::connection::deserialize_arc_str")]
            guid: Arc<str>,
        }

        // Send newPage RPC to server
        let response: NewPageResponse = self
            .channel()
            .send("newPage", serde_json::json!({}))
            .await?;

        // Retrieve and downcast the Page object from the connection registry
        let page: Page = self
            .connection()
            .get_typed::<Page>(&response.page.guid)
            .await?;

        // Note: Don't track the page here - it will be tracked via the "page" event
        // that Playwright server sends automatically when a page is created.
        // Tracking it here would create duplicates.

        // Propagate context-level timeout defaults to the new page
        let ctx_timeout = self.default_timeout_ms();
        let ctx_nav_timeout = self.default_navigation_timeout_ms();
        if ctx_timeout.to_bits() != crate::DEFAULT_TIMEOUT_MS.to_bits() {
            page.set_default_timeout(ctx_timeout).await;
        }
        if ctx_nav_timeout.to_bits() != crate::DEFAULT_TIMEOUT_MS.to_bits() {
            page.set_default_navigation_timeout(ctx_nav_timeout).await;
        }

        Ok(page)
    }

    /// Returns all open pages in the context.
    ///
    /// This method provides a snapshot of all currently active pages that belong
    /// to this browser context instance. Pages created via `new_page()` and popup
    /// pages opened through user interactions are included.
    ///
    /// In persistent contexts launched with `--app=url`, this will include the
    /// initial page created automatically by Playwright.
    ///
    /// # Errors
    ///
    /// This method does not return errors. It provides a snapshot of pages at
    /// the time of invocation.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-pages>
    pub fn pages(&self) -> Vec<Page> {
        self.pages.lock().unwrap().clone()
    }

    /// Returns all active service workers registered in this browser context.
    ///
    /// Service workers are accumulated as they are registered (`serviceWorker` event).
    /// Each call returns a snapshot of the current list.
    ///
    /// Note: Testing service workers typically requires HTTPS. In plain HTTP or
    /// `about:blank` contexts this list is empty.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-service-workers>
    pub fn service_workers(&self) -> Vec<crate::protocol::Worker> {
        self.service_workers_list.lock().unwrap().clone()
    }

    /// Returns the browser instance that owns this context.
    ///
    /// Returns `None` only for contexts created outside of normal browser
    /// (e.g., Android or Electron contexts). For both regular contexts and
    /// persistent contexts, this returns the owning Browser instance.
    ///
    /// # Errors
    ///
    /// This method does not return errors.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-browser>
    pub fn browser(&self) -> Option<Browser> {
        self.browser.clone()
    }

    /// Returns the APIRequestContext associated with this context.
    ///
    /// The APIRequestContext is created automatically by the server for each
    /// BrowserContext. It enables performing HTTP requests and is used internally
    /// by `Route::fetch()`.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-request>
    pub async fn request(&self) -> Result<APIRequestContext> {
        let guid = self.request_context_guid.as_ref().ok_or_else(|| {
            crate::error::Error::ProtocolError(
                "No APIRequestContext available for this context".to_string(),
            )
        })?;

        self.connection().get_typed::<APIRequestContext>(guid).await
    }

    /// Creates a new Chrome DevTools Protocol session for the given page.
    ///
    /// CDPSession provides low-level access to the Chrome DevTools Protocol.
    /// This method is only available in Chromium-based browsers.
    ///
    /// # Arguments
    ///
    /// * `page` - The page to create a CDP session for
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The browser is not Chromium-based
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-new-cdp-session>
    pub async fn new_cdp_session(&self, page: &Page) -> Result<CDPSession> {
        #[derive(serde::Deserialize)]
        struct NewCDPSessionResponse {
            session: GuidRef,
        }

        #[derive(serde::Deserialize)]
        struct GuidRef {
            #[serde(deserialize_with = "crate::server::connection::deserialize_arc_str")]
            guid: Arc<str>,
        }

        let response: NewCDPSessionResponse = self
            .channel()
            .send(
                "newCDPSession",
                serde_json::json!({ "page": { "guid": page.guid() } }),
            )
            .await?;

        self.connection()
            .get_typed::<CDPSession>(&response.session.guid)
            .await
    }

    /// Returns the Tracing object for this browser context.
    ///
    /// The Tracing object is created automatically by the Playwright server for each
    /// BrowserContext. Use it to start and stop trace recording.
    ///
    /// # Errors
    ///
    /// Returns error if no Tracing object is available for this context (rare,
    /// should not happen in normal usage).
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-tracing>
    pub async fn tracing(&self) -> Result<Tracing> {
        let guid = self.tracing_guid.as_ref().ok_or_else(|| {
            crate::error::Error::ProtocolError(
                "No Tracing object available for this context".to_string(),
            )
        })?;

        self.connection().get_typed::<Tracing>(guid).await
    }

    /// Returns the Clock object for this browser context.
    ///
    /// The Clock object enables fake timer control — install fake timers,
    /// fast-forward time, pause/resume, and set fixed or system time.
    ///
    /// `page.clock()` delegates to this method via the page's parent context.
    ///
    /// See: <https://playwright.dev/docs/api/class-clock>
    pub fn clock(&self) -> crate::protocol::clock::Clock {
        crate::protocol::clock::Clock::new(self.channel().clone())
    }

    /// Closes the browser context and all its pages.
    ///
    /// This is a graceful operation that sends a close command to the context
    /// and waits for it to shut down properly.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has already been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-close>
    pub async fn close(&self) -> Result<()> {
        // Unregister from Selectors coordinator so closed channels are not sent future messages.
        let selectors = self.connection().selectors();
        selectors.remove_context(self.channel());

        // Send close RPC to server
        self.channel()
            .send_no_result("close", serde_json::json!({}))
            .await
    }

    /// Sets the default timeout for all operations in this browser context.
    ///
    /// This applies to all pages already open in this context as well as pages
    /// created subsequently. Pass `0` to disable timeouts.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-set-default-timeout>
    pub async fn set_default_timeout(&self, timeout: f64) {
        self.default_timeout_ms
            .store(timeout.to_bits(), std::sync::atomic::Ordering::Relaxed);
        let pages: Vec<Page> = self.pages.lock().unwrap().clone();
        for page in pages {
            page.set_default_timeout(timeout).await;
        }
        crate::protocol::page::set_timeout_and_notify(
            self.channel(),
            "setDefaultTimeoutNoReply",
            timeout,
        )
        .await;
    }

    /// Sets the default timeout for navigation operations in this browser context.
    ///
    /// This applies to all pages already open in this context as well as pages
    /// created subsequently. Pass `0` to disable timeouts.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-set-default-navigation-timeout>
    pub async fn set_default_navigation_timeout(&self, timeout: f64) {
        self.default_navigation_timeout_ms
            .store(timeout.to_bits(), std::sync::atomic::Ordering::Relaxed);
        let pages: Vec<Page> = self.pages.lock().unwrap().clone();
        for page in pages {
            page.set_default_navigation_timeout(timeout).await;
        }
        crate::protocol::page::set_timeout_and_notify(
            self.channel(),
            "setDefaultNavigationTimeoutNoReply",
            timeout,
        )
        .await;
    }

    /// Returns the context's current default action timeout in milliseconds.
    fn default_timeout_ms(&self) -> f64 {
        f64::from_bits(
            self.default_timeout_ms
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Returns the context's current default navigation timeout in milliseconds.
    fn default_navigation_timeout_ms(&self) -> f64 {
        f64::from_bits(
            self.default_navigation_timeout_ms
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Pauses the browser context.
    ///
    /// This pauses the execution of all pages in the context.
    pub async fn pause(&self) -> Result<()> {
        self.channel()
            .send_no_result("pause", serde_json::Value::Null)
            .await
    }

    /// Returns storage state for this browser context.
    ///
    /// Contains current cookies and local storage snapshots.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-storage-state>
    pub async fn storage_state(&self) -> Result<StorageState> {
        let response: StorageState = self
            .channel()
            .send("storageState", serde_json::json!({}))
            .await?;
        Ok(response)
    }

    /// Adds cookies into this browser context.
    ///
    /// All pages within this context will have these cookies installed. Cookies can be granularly specified
    /// with `name`, `value`, `url`, `domain`, `path`, `expires`, `httpOnly`, `secure`, `sameSite`.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-add-cookies>
    pub async fn add_cookies(&self, cookies: &[Cookie]) -> Result<()> {
        self.channel()
            .send_no_result(
                "addCookies",
                serde_json::json!({
                    "cookies": cookies
                }),
            )
            .await
    }

    /// Returns cookies for this browser context, optionally filtered by URLs.
    ///
    /// If `urls` is `None` or empty, all cookies are returned.
    ///
    /// # Arguments
    ///
    /// * `urls` - Optional list of URLs to filter cookies by
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-cookies>
    pub async fn cookies(&self, urls: Option<&[&str]>) -> Result<Vec<Cookie>> {
        let url_list: Vec<&str> = urls.unwrap_or(&[]).to_vec();
        #[derive(serde::Deserialize)]
        struct CookiesResponse {
            cookies: Vec<Cookie>,
        }
        let response: CookiesResponse = self
            .channel()
            .send("cookies", serde_json::json!({ "urls": url_list }))
            .await?;
        Ok(response.cookies)
    }

    /// Clears cookies from this browser context, with optional filters.
    ///
    /// When called with no options, all cookies are removed. Use `ClearCookiesOptions`
    /// to filter which cookies to clear by name, domain, or path.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional filters for which cookies to clear
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-clear-cookies>
    pub async fn clear_cookies(&self, options: Option<ClearCookiesOptions>) -> Result<()> {
        let params = match options {
            None => serde_json::json!({}),
            Some(opts) => serde_json::to_value(opts).unwrap_or(serde_json::json!({})),
        };
        self.channel().send_no_result("clearCookies", params).await
    }

    /// Sets extra HTTP headers that will be sent with every request from this context.
    ///
    /// These headers are merged with per-page extra headers set with `page.set_extra_http_headers()`.
    /// If the page has specific headers that conflict, page-level headers take precedence.
    ///
    /// # Arguments
    ///
    /// * `headers` - Map of header names to values. All header names are lowercased.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-set-extra-http-headers>
    pub async fn set_extra_http_headers(&self, headers: HashMap<String, String>) -> Result<()> {
        // Playwright protocol expects an array of {name, value} objects
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

    /// Grants browser permissions to the context.
    ///
    /// Permissions are granted for all pages in the context. The optional `origin`
    /// in `GrantPermissionsOptions` restricts the grant to a specific URL origin.
    ///
    /// Common permissions: `"geolocation"`, `"notifications"`, `"camera"`,
    /// `"microphone"`, `"clipboard-read"`, `"clipboard-write"`.
    ///
    /// # Arguments
    ///
    /// * `permissions` - List of permission strings to grant
    /// * `options` - Optional options, including `origin` to restrict the grant
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Permission name is not recognised
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-grant-permissions>
    pub async fn grant_permissions(
        &self,
        permissions: &[&str],
        options: Option<GrantPermissionsOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({ "permissions": permissions });
        if let Some(opts) = options
            && let Some(origin) = opts.origin
        {
            params["origin"] = serde_json::Value::String(origin);
        }
        self.channel()
            .send_no_result("grantPermissions", params)
            .await
    }

    /// Clears all permission overrides for this browser context.
    ///
    /// Reverts all permissions previously set with `grant_permissions()` back to
    /// the browser default state.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-clear-permissions>
    pub async fn clear_permissions(&self) -> Result<()> {
        self.channel()
            .send_no_result("clearPermissions", serde_json::json!({}))
            .await
    }

    /// Sets or clears the geolocation for all pages in this context.
    ///
    /// Pass `Some(Geolocation { ... })` to set a specific location, or `None` to
    /// clear the override and let the browser handle location requests naturally.
    ///
    /// Note: Geolocation access requires the `"geolocation"` permission to be granted
    /// via `grant_permissions()` for navigator.geolocation to succeed.
    ///
    /// # Arguments
    ///
    /// * `geolocation` - Location to set, or `None` to clear
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Latitude or longitude is out of range
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-set-geolocation>
    pub async fn set_geolocation(&self, geolocation: Option<Geolocation>) -> Result<()> {
        // Playwright protocol: omit the "geolocation" key entirely to clear;
        // passing null causes a validation error on the server side.
        let params = match geolocation {
            Some(geo) => serde_json::json!({ "geolocation": geo }),
            None => serde_json::json!({}),
        };
        self.channel()
            .send_no_result("setGeolocation", params)
            .await
    }

    /// Toggles the offline mode for this browser context.
    ///
    /// When `true`, all network requests from pages in this context will fail with
    /// a network error. Set to `false` to restore network connectivity.
    ///
    /// # Arguments
    ///
    /// * `offline` - `true` to go offline, `false` to go back online
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-set-offline>
    pub async fn set_offline(&self, offline: bool) -> Result<()> {
        self.channel()
            .send_no_result("setOffline", serde_json::json!({ "offline": offline }))
            .await
    }

    /// Registers a route handler for context-level network interception.
    ///
    /// Routes registered on a context apply to all pages within the context.
    /// Page-level routes take precedence over context-level routes.
    ///
    /// # Arguments
    ///
    /// * `pattern` - URL pattern to match (supports glob patterns like "**/*.png")
    /// * `handler` - Async closure that handles the route
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-route>
    pub async fn route<F, Fut>(&self, pattern: &str, handler: F) -> Result<()>
    where
        F: Fn(Route) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler =
            Arc::new(move |route: Route| -> RouteHandlerFuture { Box::pin(handler(route)) });

        self.route_handlers.lock().unwrap().push(RouteHandlerEntry {
            pattern: pattern.to_string(),
            handler,
        });

        self.enable_network_interception().await
    }

    /// Removes route handler(s) matching the given URL pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - URL pattern to remove handlers for
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-unroute>
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
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-unroute-all>
    pub async fn unroute_all(&self, _behavior: Option<UnrouteBehavior>) -> Result<()> {
        self.route_handlers.lock().unwrap().clear();
        self.enable_network_interception().await
    }

    /// Replays network requests from a HAR file recorded previously.
    ///
    /// Requests matching `options.url` (or all requests if omitted) will be
    /// served from the archive for every page in this context.  Unmatched
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
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-route-from-har>
    pub async fn route_from_har(
        &self,
        har_path: &str,
        options: Option<crate::protocol::RouteFromHarOptions>,
    ) -> Result<()> {
        let opts = options.unwrap_or_default();
        let not_found = opts.not_found.unwrap_or_else(|| "abort".to_string());
        let url_filter = opts.url.clone();

        let abs_path = std::path::Path::new(har_path).canonicalize().map_err(|e| {
            Error::InvalidPath(format!(
                "route_from_har: cannot resolve '{}': {}",
                har_path, e
            ))
        })?;
        let abs_str = abs_path.to_string_lossy().into_owned();

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

        let har_id = local_utils.har_open(&abs_str).await?;

        let pattern = url_filter.unwrap_or_else(|| "**/*".to_string());

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

                            let body_bytes = result.body.as_deref().map(|b64| {
                                use base64::Engine;
                                base64::engine::general_purpose::STANDARD
                                    .decode(b64)
                                    .unwrap_or_default()
                            });

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

    /// Adds a listener for the `page` event.
    ///
    /// The handler is called whenever a new page is created in this context,
    /// including popup pages opened through user interactions.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the new `Page`
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-page>
    pub async fn on_page<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Page) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |page: Page| -> PageHandlerFuture { Box::pin(handler(page)) });
        self.page_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Adds a listener for the `close` event.
    ///
    /// The handler is called when the browser context is closed.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function called with no arguments when the context closes
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-close>
    pub async fn on_close<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move || -> CloseHandlerFuture { Box::pin(handler()) });
        self.close_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Adds a listener for the `request` event.
    ///
    /// The handler fires whenever a request is issued from any page in the context.
    /// This is equivalent to subscribing to `on_request` on each individual page,
    /// but covers all current and future pages of the context.
    ///
    /// Context-level handlers fire before page-level handlers.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the `Request`
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-request>
    pub async fn on_request<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |request: Request| -> RequestHandlerFuture {
            Box::pin(handler(request))
        });
        let needs_subscription = self.request_handlers.lock().unwrap().is_empty();
        if needs_subscription {
            _ = self.channel().update_subscription("request", true).await;
        }
        self.request_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Adds a listener for the `requestFinished` event.
    ///
    /// The handler fires after the request has been successfully received by the server
    /// and a response has been fully downloaded for any page in the context.
    ///
    /// Context-level handlers fire before page-level handlers.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the completed `Request`
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-request-finished>
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

    /// Adds a listener for the `requestFailed` event.
    ///
    /// The handler fires when a request from any page in the context fails,
    /// for example due to a network error or if the server returned an error response.
    ///
    /// Context-level handlers fire before page-level handlers.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the failed `Request`
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-request-failed>
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

    /// Adds a listener for the `response` event.
    ///
    /// The handler fires whenever a response is received from any page in the context.
    ///
    /// Context-level handlers fire before page-level handlers.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the `ResponseObject`
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-response>
    pub async fn on_response<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(ResponseObject) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |response: ResponseObject| -> ResponseHandlerFuture {
            Box::pin(handler(response))
        });
        let needs_subscription = self.response_handlers.lock().unwrap().is_empty();
        if needs_subscription {
            _ = self.channel().update_subscription("response", true).await;
        }
        self.response_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Adds a listener for the `dialog` event on this browser context.
    ///
    /// The handler fires whenever a JavaScript dialog (alert, confirm, prompt,
    /// or beforeunload) is triggered from **any** page in the context. Context-level
    /// handlers fire before page-level handlers.
    ///
    /// The dialog must be explicitly accepted or dismissed; otherwise the page
    /// will freeze waiting for a response.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the [`Dialog`](crate::protocol::Dialog) and calls
    ///   `dialog.accept()` or `dialog.dismiss()`.
    ///
    /// # Errors
    ///
    /// Returns error if communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-dialog>
    pub async fn on_dialog<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::Dialog) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |dialog: crate::protocol::Dialog| -> DialogHandlerFuture {
                Box::pin(handler(dialog))
            },
        );
        self.dialog_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a context-level console event handler.
    ///
    /// The handler fires for any console message emitted by any page in this context.
    /// Context-level handlers fire before page-level handlers.
    ///
    /// The server only sends console events after the first handler is registered
    /// (subscription is managed automatically per context channel).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the [`ConsoleMessage`](crate::protocol::ConsoleMessage)
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-console>
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

        let needs_subscription = self.console_handlers.lock().unwrap().is_empty();
        if needs_subscription {
            _ = self.channel().update_subscription("console", true).await;
        }
        self.console_handlers.lock().unwrap().push(handler);

        Ok(())
    }

    /// Registers a context-level handler for uncaught JavaScript exceptions.
    ///
    /// The handler fires whenever a page in this context throws an unhandled
    /// JavaScript error (i.e. an exception that propagates to `window.onerror`
    /// or an unhandled promise rejection). The [`WebError`](crate::protocol::WebError)
    /// passed to the handler contains the error message and an optional back-reference
    /// to the originating page.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives a [`WebError`](crate::protocol::WebError).
    ///
    /// # Errors
    ///
    /// Returns error if communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-web-error>
    pub async fn on_weberror<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::WebError) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |web_error: crate::protocol::WebError| -> WebErrorHandlerFuture {
                Box::pin(handler(web_error))
            },
        );
        self.weberror_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `serviceWorker` event.
    ///
    /// The handler is called when a new service worker is registered in the browser context.
    ///
    /// Note: Service worker testing typically requires HTTPS and a registered service worker.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure called with the new [`Worker`](crate::protocol::Worker) object
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-service-worker>
    pub async fn on_serviceworker<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::Worker) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |worker: crate::protocol::Worker| -> ServiceWorkerHandlerFuture {
                Box::pin(handler(worker))
            },
        );
        self.serviceworker_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Exposes a Rust function to every page in this browser context as
    /// `window[name]` in JavaScript.
    ///
    /// When JavaScript code calls `window[name](arg1, arg2, …)` the Playwright
    /// server fires a `bindingCall` event that invokes `callback` with the
    /// deserialized arguments. The return value of `callback` is serialized back
    /// to JavaScript so the `await window[name](…)` expression resolves with it.
    ///
    /// The binding is injected into every existing page and every new page
    /// created in this context.
    ///
    /// # Arguments
    ///
    /// * `name`     – JavaScript identifier that will be available as `window[name]`.
    /// * `callback` – Async closure called with `Vec<serde_json::Value>` (the JS
    ///   arguments) and returning `serde_json::Value` (the result).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The context has been closed.
    /// - Communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-expose-function>
    pub async fn expose_function<F, Fut>(&self, name: &str, callback: F) -> Result<()>
    where
        F: Fn(Vec<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = serde_json::Value> + Send + 'static,
    {
        self.expose_binding_internal(name, false, callback).await
    }

    /// Exposes a Rust function to every page in this browser context as
    /// `window[name]` in JavaScript, with `needsHandle: true`.
    ///
    /// Identical to [`expose_function`](Self::expose_function) but the Playwright
    /// server passes the first argument as a `JSHandle` object rather than a plain
    /// value.  Use this when the JS caller passes complex objects that you want to
    /// inspect on the Rust side.
    ///
    /// # Arguments
    ///
    /// * `name`     – JavaScript identifier.
    /// * `callback` – Async closure with `Vec<serde_json::Value>` → `serde_json::Value`.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The context has been closed.
    /// - Communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-expose-binding>
    pub async fn expose_binding<F, Fut>(&self, name: &str, callback: F) -> Result<()>
    where
        F: Fn(Vec<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = serde_json::Value> + Send + 'static,
    {
        self.expose_binding_internal(name, true, callback).await
    }

    /// Internal implementation shared by expose_function and expose_binding.
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
        // Wrap callback with type erasure
        let callback: BindingCallback = Arc::new(move |args: Vec<serde_json::Value>| {
            Box::pin(callback(args)) as BindingCallbackFuture
        });

        // Store the callback before sending the RPC so that a race-condition
        // where a bindingCall arrives before we finish registering is avoided.
        self.binding_callbacks
            .lock()
            .unwrap()
            .insert(name.to_string(), callback);

        // Tell the Playwright server to inject window[name] into every page.
        // Always use needsHandle: false — see note above.
        self.channel()
            .send_no_result(
                "exposeBinding",
                serde_json::json!({ "name": name, "needsHandle": false }),
            )
            .await
    }

    /// Waits for a new page to be created in this browser context.
    ///
    /// Creates a one-shot waiter that resolves when the next `page` event fires.
    /// The waiter **must** be created before the action that triggers the new page
    /// (e.g. `new_page()` or a user action that opens a popup) to avoid a race
    /// condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Timeout`](crate::error::Error::Timeout) if no page is created within the timeout.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Set up the waiter BEFORE the triggering action
    /// let waiter = context.expect_page(None).await?;
    /// let _page = context.new_page().await?;
    /// let new_page = waiter.wait().await?;
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-wait-for-event>
    pub async fn expect_page(&self, timeout: Option<f64>) -> Result<EventWaiter<Page>> {
        let (tx, rx) = oneshot::channel();
        self.page_waiters.lock().unwrap().push(tx);
        Ok(EventWaiter::new(rx, timeout.or(Some(30_000.0))))
    }

    /// Waits for this browser context to be closed.
    ///
    /// Creates a one-shot waiter that resolves when the `close` event fires.
    /// The waiter **must** be created before the action that closes the context
    /// to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Timeout`](crate::error::Error::Timeout) if the context is not closed within the timeout.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Set up the waiter BEFORE closing
    /// let waiter = context.expect_close(None).await?;
    /// context.close().await?;
    /// waiter.wait().await?;
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-wait-for-event>
    pub async fn expect_close(&self, timeout: Option<f64>) -> Result<EventWaiter<()>> {
        let (tx, rx) = oneshot::channel();
        self.close_waiters.lock().unwrap().push(tx);
        Ok(EventWaiter::new(rx, timeout.or(Some(30_000.0))))
    }

    /// Waits for a console message from any page in this context.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-console>
    pub async fn expect_console_message(
        &self,
        timeout: Option<f64>,
    ) -> Result<EventWaiter<crate::protocol::ConsoleMessage>> {
        let needs_subscription = self.console_handlers.lock().unwrap().is_empty()
            && self.console_waiters.lock().unwrap().is_empty();
        if needs_subscription {
            _ = self.channel().update_subscription("console", true).await;
        }
        let (tx, rx) = oneshot::channel();
        self.console_waiters.lock().unwrap().push(tx);
        Ok(EventWaiter::new(rx, timeout.or(Some(30_000.0))))
    }

    /// Waits for the given event to fire and returns a typed `EventValue`.
    ///
    /// This is the generic version of the specific `expect_*` methods. It matches
    /// the playwright-python / playwright-js `context.expect_event(event_name)` API.
    ///
    /// The waiter **must** be created before the action that triggers the event.
    ///
    /// # Supported event names
    ///
    /// `"page"`, `"close"`, `"console"`, `"request"`, `"response"`,
    /// `"weberror"`, `"serviceworker"`
    ///
    /// # Arguments
    ///
    /// * `event` - Event name (case-sensitive, matches Playwright protocol names).
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`](crate::error::Error::InvalidArgument) for unknown event names.
    /// Returns [`Error::Timeout`](crate::error::Error::Timeout) if the event does not fire within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-wait-for-event>
    pub async fn expect_event(
        &self,
        event: &str,
        timeout: Option<f64>,
    ) -> crate::error::Result<EventWaiter<crate::protocol::EventValue>> {
        use crate::protocol::EventValue;
        use tokio::sync::oneshot;

        let timeout_ms = timeout.or(Some(30_000.0));

        match event {
            "page" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<Page>();
                self.page_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::Page(v));
                    }
                });

                Ok(EventWaiter::new(rx, timeout_ms))
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

                Ok(EventWaiter::new(rx, timeout_ms))
            }

            "console" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::ConsoleMessage>();

                let needs_subscription = self.console_handlers.lock().unwrap().is_empty()
                    && self.console_waiters.lock().unwrap().is_empty();
                if needs_subscription {
                    _ = self.channel().update_subscription("console", true).await;
                }
                self.console_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::ConsoleMessage(v));
                    }
                });

                Ok(EventWaiter::new(rx, timeout_ms))
            }

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

                Ok(EventWaiter::new(rx, timeout_ms))
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

                Ok(EventWaiter::new(rx, timeout_ms))
            }

            "weberror" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::WebError>();
                self.weberror_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::WebError(v));
                    }
                });

                Ok(EventWaiter::new(rx, timeout_ms))
            }

            "serviceworker" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::Worker>();
                self.serviceworker_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::Worker(v));
                    }
                });

                Ok(EventWaiter::new(rx, timeout_ms))
            }

            other => Err(crate::error::Error::InvalidArgument(format!(
                "Unknown event name '{}'. Supported: page, close, console, request, response, \
                 weberror, serviceworker",
                other
            ))),
        }
    }

    /// Intercepts WebSocket connections matching the given URL pattern for all pages in this context.
    ///
    /// When a WebSocket connection from any page in this context matches `url`,
    /// the `handler` is called with a [`WebSocketRoute`](crate::protocol::WebSocketRoute) object.
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
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-route-web-socket>
    pub async fn route_web_socket<F, Fut>(&self, url: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::WebSocketRoute) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |route: crate::protocol::WebSocketRoute| -> WsRouteHandlerFuture {
                Box::pin(handler(route))
            },
        );

        self.ws_route_handlers
            .lock()
            .unwrap()
            .push(ContextWsRouteHandlerEntry {
                pattern: url.to_string(),
                handler,
            });

        self.enable_ws_interception().await
    }

    /// Updates WebSocket interception patterns for this context.
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

    /// Updates network interception patterns for this context
    async fn enable_network_interception(&self) -> Result<()> {
        let patterns: Vec<serde_json::Value> = self
            .route_handlers
            .lock()
            .unwrap()
            .iter()
            .map(|entry| serde_json::json!({ "glob": entry.pattern }))
            .collect();

        self.channel()
            .send_no_result(
                "setNetworkInterceptionPatterns",
                serde_json::json!({ "patterns": patterns }),
            )
            .await
    }

    /// Deserializes binding call arguments from Playwright's protocol format.
    ///
    /// The `args` field in the BindingCall initializer is a JSON array where each
    /// element is in `serialize_argument` format: `{"value": <tagged>, "handles": []}`.
    /// This helper extracts the inner "value" from each entry and parses it.
    ///
    /// This is `pub` so that `Page::on_event("bindingCall")` can reuse it without
    /// duplicating the deserialization logic.
    pub fn deserialize_binding_args_pub(raw_args: &Value) -> Vec<Value> {
        Self::deserialize_binding_args(raw_args)
    }

    fn deserialize_binding_args(raw_args: &Value) -> Vec<Value> {
        let Some(arr) = raw_args.as_array() else {
            return vec![];
        };

        arr.iter()
            .map(|arg| {
                // Each arg is a direct Playwright type-tagged value, e.g. {"n": 3} or {"s": "hello"}
                // (NOT wrapped in {"value": ..., "handles": []} — that format is only for evaluate args)
                crate::protocol::evaluate_conversion::parse_value(arg, None)
            })
            .collect()
    }

    /// Handles a route event from the protocol
    async fn on_route_event(route_handlers: Arc<Mutex<Vec<RouteHandlerEntry>>>, route: Route) {
        let handlers = route_handlers.lock().unwrap().clone();
        let url = route.request().url().to_string();

        for entry in handlers.iter().rev() {
            if crate::protocol::route::matches_pattern(&entry.pattern, &url) {
                let handler = entry.handler.clone();
                if let Err(e) = handler(route.clone()).await {
                    tracing::warn!("Context route handler error: {}", e);
                    break;
                }
                if !route.was_handled() {
                    continue;
                }
                break;
            }
        }
    }

    fn dispatch_request_event(&self, method: &str, params: Value) {
        if let Some(request_guid) = params
            .get("request")
            .and_then(|v| v.get("guid"))
            .and_then(|v| v.as_str())
        {
            let connection = self.connection();
            let request_guid_owned = request_guid.to_owned();
            let page_guid_owned = params
                .get("page")
                .and_then(|v| v.get("guid"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_owned());
            // Extract failureText for requestFailed events
            let failure_text = params
                .get("failureText")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned());
            // Extract response GUID for requestFinished events (to read timing)
            let response_guid_owned = params
                .get("response")
                .and_then(|v| v.get("guid"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned());
            // Extract responseEndTiming from requestFinished event params
            let response_end_timing = params.get("responseEndTiming").and_then(|v| v.as_f64());
            let method = method.to_owned();
            // Clone context-level handler vecs for use in spawn
            let ctx_request_handlers = self.request_handlers.clone();
            let ctx_request_finished_handlers = self.request_finished_handlers.clone();
            let ctx_request_failed_handlers = self.request_failed_handlers.clone();
            let ctx_request_waiters = self.request_waiters.clone();
            tokio::spawn(async move {
                let request: Request =
                    match connection.get_typed::<Request>(&request_guid_owned).await {
                        Ok(r) => r,
                        Err(_) => return,
                    };

                // Set failure text on the request before dispatching to handlers
                if let Some(text) = failure_text {
                    request.set_failure_text(text);
                }

                // For requestFinished, extract timing from the Response object's initializer
                if method == "requestFinished"
                    && let Some(timing) =
                        extract_timing(&connection, response_guid_owned, response_end_timing).await
                {
                    request.set_timing(timing);
                }

                // Dispatch to context-level handlers first (matching playwright-python behavior)
                let ctx_handlers = match method.as_str() {
                    "request" => ctx_request_handlers.lock().unwrap().clone(),
                    "requestFinished" => ctx_request_finished_handlers.lock().unwrap().clone(),
                    "requestFailed" => ctx_request_failed_handlers.lock().unwrap().clone(),
                    _ => vec![],
                };
                for handler in ctx_handlers {
                    if let Err(e) = handler(request.clone()).await {
                        tracing::warn!("Context {} handler error: {}", method, e);
                    }
                }

                // Notify expect_event("request") waiters (only for "request" events)
                if method == "request"
                    && let Some(tx) = ctx_request_waiters.lock().unwrap().pop()
                {
                    let _ = tx.send(request.clone());
                }

                // Then dispatch to page-level handlers
                if let Some(page_guid) = page_guid_owned {
                    let page: Page = match connection.get_typed::<Page>(&page_guid).await {
                        Ok(p) => p,
                        Err(_) => return,
                    };
                    match method.as_str() {
                        "request" => page.trigger_request_event(request).await,
                        "requestFailed" => page.trigger_request_failed_event(request).await,
                        "requestFinished" => page.trigger_request_finished_event(request).await,
                        _ => unreachable!("Unreachable method {}", method),
                    }
                }
            });
        }
    }

    fn dispatch_response_event(&self, _method: &str, params: Value) {
        if let Some(response_guid) = params
            .get("response")
            .and_then(|v| v.get("guid"))
            .and_then(|v| v.as_str())
        {
            let connection = self.connection();
            let response_guid_owned = response_guid.to_owned();
            let page_guid_owned = params
                .get("page")
                .and_then(|v| v.get("guid"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_owned());
            let ctx_response_handlers = self.response_handlers.clone();
            let ctx_response_waiters = self.response_waiters.clone();
            tokio::spawn(async move {
                let response: ResponseObject = match connection
                    .get_typed::<ResponseObject>(&response_guid_owned)
                    .await
                {
                    Ok(r) => r,
                    Err(_) => return,
                };

                // Dispatch to context-level handlers first (matching playwright-python behavior)
                let ctx_handlers = ctx_response_handlers.lock().unwrap().clone();
                for handler in ctx_handlers {
                    if let Err(e) = handler(response.clone()).await {
                        tracing::warn!("Context response handler error: {}", e);
                    }
                }

                // Notify expect_event("response") waiters
                if let Some(tx) = ctx_response_waiters.lock().unwrap().pop() {
                    let _ = tx.send(response.clone());
                }

                // Then dispatch to page-level handlers
                if let Some(page_guid) = page_guid_owned {
                    let page: Page = match connection.get_typed::<Page>(&page_guid).await {
                        Ok(p) => p,
                        Err(_) => return,
                    };
                    page.trigger_response_event(response).await;
                }
            });
        }
    }
}

impl ChannelOwner for BrowserContext {
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
            "request" | "requestFailed" | "requestFinished" => {
                self.dispatch_request_event(method, params)
            }
            "response" => self.dispatch_response_event(method, params),
            "close" => {
                // BrowserContext close event — fire registered close handlers
                let close_handlers = self.close_handlers.clone();
                let close_waiters = self.close_waiters.clone();
                tokio::spawn(async move {
                    let handlers = close_handlers.lock().unwrap().clone();
                    for handler in handlers {
                        if let Err(e) = handler().await {
                            tracing::warn!("Context close handler error: {}", e);
                        }
                    }

                    // Notify all expect_close() waiters
                    let waiters: Vec<_> = close_waiters.lock().unwrap().drain(..).collect();
                    for tx in waiters {
                        let _ = tx.send(());
                    }
                });
            }
            "page" => {
                // Page events are triggered when pages are created, including:
                // - Initial page in persistent context with --app mode
                // - Popup pages opened through user interactions
                // Event format: {page: {guid: "..."}}
                if let Some(page_guid) = params
                    .get("page")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let page_guid_owned = page_guid.to_string();
                    let pages = self.pages.clone();
                    let page_handlers = self.page_handlers.clone();
                    let page_waiters = self.page_waiters.clone();

                    tokio::spawn(async move {
                        // Get and downcast the Page object
                        let page: Page = match connection.get_typed::<Page>(&page_guid_owned).await
                        {
                            Ok(p) => p,
                            Err(_) => return,
                        };

                        // Track the page
                        pages.lock().unwrap().push(page.clone());

                        // If this page has an opener, dispatch popup event to opener's handlers.
                        // The opener guid is in the page's initializer: {"opener": {"guid": "..."}}
                        if let Some(opener_guid) = page
                            .initializer()
                            .get("opener")
                            .and_then(|v| v.get("guid"))
                            .and_then(|v| v.as_str())
                            && let Ok(opener) = connection.get_typed::<Page>(opener_guid).await
                        {
                            opener.trigger_popup_event(page.clone()).await;
                        }

                        // Dispatch to context-level page handlers
                        let handlers = page_handlers.lock().unwrap().clone();
                        for handler in handlers {
                            if let Err(e) = handler(page.clone()).await {
                                tracing::warn!("Context page handler error: {}", e);
                            }
                        }

                        // Notify the first expect_page() waiter (FIFO order)
                        if let Some(tx) = page_waiters.lock().unwrap().pop() {
                            let _ = tx.send(page);
                        }
                    });
                }
            }
            "pageError" => {
                // pageError event: fired when an uncaught JS exception occurs on a page.
                // Event format:
                //   { "error": { "error": { "message": "...", "name": "...", "stack": "..." } },
                //     "page": { "guid": "page@..." } }
                //
                // Dispatch path:
                //  1. Construct WebError and fire context-level on_weberror handlers.
                //  2. Forward the raw message to the page's on_pageerror handlers.
                let message = params
                    .get("error")
                    .and_then(|e| e.get("error"))
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();

                let page_guid_owned = params
                    .get("page")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let connection = self.connection();
                let weberror_handlers = self.weberror_handlers.clone();
                let weberror_waiters = self.weberror_waiters.clone();

                tokio::spawn(async move {
                    // Resolve page (optional — may be None if page already closed)
                    let page = if let Some(ref guid) = page_guid_owned {
                        connection.get_typed::<Page>(guid).await.ok()
                    } else {
                        None
                    };

                    // 1. Dispatch to context-level weberror handlers
                    let web_error = crate::protocol::WebError::new(message.clone(), page.clone());
                    let handlers = weberror_handlers.lock().unwrap().clone();
                    for handler in handlers {
                        if let Err(e) = handler(web_error.clone()).await {
                            tracing::warn!("Context weberror handler error: {}", e);
                        }
                    }

                    // Notify expect_event("weberror") waiters
                    if let Some(tx) = weberror_waiters.lock().unwrap().pop() {
                        let _ = tx.send(web_error);
                    }

                    // 2. Forward to page-level pageerror handlers
                    if let Some(p) = page {
                        p.trigger_pageerror_event(message).await;
                    }
                });
            }
            "dialog" => {
                // Dialog events come to BrowserContext.
                // Dispatch to context-level handlers first, then forward to the Page.
                // Event format: {dialog: {guid: "..."}}
                // The Dialog protocol object has the Page as its parent
                if let Some(dialog_guid) = params
                    .get("dialog")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let dialog_guid_owned = dialog_guid.to_string();
                    let dialog_handlers = self.dialog_handlers.clone();

                    tokio::spawn(async move {
                        // Get and downcast the Dialog object
                        let dialog: crate::protocol::Dialog = match connection
                            .get_typed::<crate::protocol::Dialog>(&dialog_guid_owned)
                            .await
                        {
                            Ok(d) => d,
                            Err(_) => return,
                        };

                        // Dispatch to context-level dialog handlers first
                        let ctx_handlers = dialog_handlers.lock().unwrap().clone();
                        for handler in ctx_handlers {
                            if let Err(e) = handler(dialog.clone()).await {
                                tracing::warn!("Context dialog handler error: {}", e);
                            }
                        }

                        // Then forward to the Page's dialog handlers
                        let page: Page =
                            match crate::server::connection::downcast_parent::<Page>(&dialog) {
                                Some(p) => p,
                                None => return,
                            };

                        page.trigger_dialog_event(dialog).await;
                    });
                }
            }
            "bindingCall" => {
                // A JS caller invoked an exposed function. Dispatch to the registered
                // callback and send the result back via BindingCall::fulfill.
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

                        // Look up the registered callback
                        let callback = {
                            let callbacks = binding_callbacks.lock().unwrap();
                            callbacks.get(&name).cloned()
                        };

                        let Some(callback) = callback else {
                            tracing::warn!("No callback registered for binding '{}'", name);
                            let _ = binding_call
                                .reject(&format!("No Rust handler for binding '{name}'"))
                                .await;
                            return;
                        };

                        // Deserialize the args from Playwright protocol format
                        let raw_args = binding_call.args();
                        let args = Self::deserialize_binding_args(raw_args);

                        // Call the callback and serialize the result
                        let result_value = callback(args).await;
                        let serialized =
                            crate::protocol::evaluate_conversion::serialize_argument(&result_value);

                        if let Err(e) = binding_call.resolve(serialized).await {
                            tracing::warn!("Failed to resolve BindingCall '{}': {}", name, e);
                        }
                    });
                }
            }
            "route" => {
                // Handle context-level network routing event
                if let Some(route_guid) = params
                    .get("route")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let route_guid_owned = route_guid.to_string();
                    let route_handlers = self.route_handlers.clone();
                    let request_context_guid = self.request_context_guid.clone();

                    tokio::spawn(async move {
                        let route: Route =
                            match connection.get_typed::<Route>(&route_guid_owned).await {
                                Ok(r) => r,
                                Err(e) => {
                                    tracing::warn!("Failed to get route object: {}", e);
                                    return;
                                }
                            };

                        // Set APIRequestContext on the route for fetch() support
                        if let Some(ref guid) = request_context_guid
                            && let Ok(api_ctx) =
                                connection.get_typed::<APIRequestContext>(guid).await
                        {
                            route.set_api_request_context(api_ctx);
                        }

                        BrowserContext::on_route_event(route_handlers, route).await;
                    });
                }
            }
            "console" => {
                // Console events are sent to BrowserContext.
                // Construct ConsoleMessage from params, dispatch to context-level handlers,
                // then forward to the Page's on_console handlers.
                //
                // Event params format:
                // {
                //   type: "log"|"error"|"warning"|...,
                //   text: "rendered text",
                //   location: { url: "...", lineNumber: N, columnNumber: N },
                //   page: { guid: "page@..." },
                //   args: [ { guid: "JSHandle@..." }, ... ]  -- resolved to Arc<JSHandle>
                // }
                let type_ = params
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("log")
                    .to_string();
                let text = params
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let loc_url = params
                    .get("location")
                    .and_then(|v| v.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let loc_line = params
                    .get("location")
                    .and_then(|v| v.get("lineNumber"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                let loc_col = params
                    .get("location")
                    .and_then(|v| v.get("columnNumber"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                let page_guid_owned = params
                    .get("page")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                // Collect arg GUIDs before spawning.
                let arg_guids: Vec<String> = params
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| {
                                v.get("guid")
                                    .and_then(|g| g.as_str())
                                    .map(|s| s.to_string())
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let connection = self.connection();
                let ctx_console_handlers = self.console_handlers.clone();
                let ctx_console_waiters = self.console_waiters.clone();

                tokio::spawn(async move {
                    use crate::protocol::JSHandle;
                    use crate::protocol::console_message::{
                        ConsoleMessage, ConsoleMessageLocation,
                    };

                    // Optionally resolve the page back-reference
                    let page = if let Some(ref guid) = page_guid_owned {
                        connection.get_typed::<Page>(guid).await.ok()
                    } else {
                        None
                    };

                    // Resolve JSHandle args from the connection registry.
                    let args: Vec<std::sync::Arc<JSHandle>> = {
                        let mut resolved = Vec::with_capacity(arg_guids.len());
                        for guid in &arg_guids {
                            if let Ok(handle) = connection.get_typed::<JSHandle>(guid).await {
                                resolved.push(std::sync::Arc::new(handle));
                            }
                        }
                        resolved
                    };

                    let location = ConsoleMessageLocation {
                        url: loc_url,
                        line_number: loc_line,
                        column_number: loc_col,
                    };

                    let msg = ConsoleMessage::new(type_, text, location, page.clone(), args);

                    // Satisfy the first pending waiter (expect_console_message)
                    if let Some(tx) = ctx_console_waiters.lock().unwrap().pop() {
                        let _ = tx.send(msg.clone());
                    }

                    // Dispatch to context-level handlers
                    let ctx_handlers = ctx_console_handlers.lock().unwrap().clone();
                    for handler in ctx_handlers {
                        if let Err(e) = handler(msg.clone()).await {
                            tracing::warn!("Context console handler error: {}", e);
                        }
                    }

                    // Forward to page-level handlers
                    if let Some(p) = page {
                        p.trigger_console_event(msg).await;
                    }
                });
            }
            "serviceWorker" => {
                // A new service worker was registered in this context.
                // Event format: {worker: {guid: "Worker@..."}}
                if let Some(worker_guid) = params
                    .get("worker")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let worker_guid_owned = worker_guid.to_string();
                    let serviceworker_handlers = self.serviceworker_handlers.clone();
                    let serviceworker_waiters = self.serviceworker_waiters.clone();
                    let service_workers_list = self.service_workers_list.clone();

                    tokio::spawn(async move {
                        let worker: crate::protocol::Worker = match connection
                            .get_typed::<crate::protocol::Worker>(&worker_guid_owned)
                            .await
                        {
                            Ok(w) => w,
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to get Worker object for serviceWorker event: {}",
                                    e
                                );
                                return;
                            }
                        };

                        // Track for service_workers() accessor
                        service_workers_list.lock().unwrap().push(worker.clone());

                        let handlers = serviceworker_handlers.lock().unwrap().clone();
                        for handler in handlers {
                            let worker_clone = worker.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handler(worker_clone).await {
                                    tracing::error!("Error in serviceworker handler: {}", e);
                                }
                            });
                        }
                        // Notify expect_event("serviceworker") waiters
                        if let Some(tx) = serviceworker_waiters.lock().unwrap().pop() {
                            let _ = tx.send(worker);
                        }
                    });
                }
            }
            "webSocketRoute" => {
                // A WebSocket matched a route_web_socket pattern on the context.
                // Event format: {webSocketRoute: {guid: "WebSocketRoute@..."}}
                if let Some(wsr_guid) = params
                    .get("webSocketRoute")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let wsr_guid_owned = wsr_guid.to_string();
                    let ws_route_handlers = self.ws_route_handlers.clone();

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
                        let handlers = ws_route_handlers.lock().unwrap().clone();
                        for entry in handlers.iter().rev() {
                            if crate::protocol::route::matches_pattern(&entry.pattern, &url) {
                                let handler = entry.handler.clone();
                                let route_clone = route.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handler(route_clone).await {
                                        tracing::error!(
                                            "Error in context webSocketRoute handler: {}",
                                            e
                                        );
                                    }
                                });
                                break;
                            }
                        }
                    });
                }
            }
            _ => {
                // Other events will be handled in future phases
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

impl std::fmt::Debug for BrowserContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserContext")
            .field("guid", &self.guid())
            .finish()
    }
}

/// Viewport dimensions for browser context.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    /// Page width in pixels
    pub width: u32,
    /// Page height in pixels
    pub height: u32,
}

/// Geolocation coordinates.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geolocation {
    /// Latitude between -90 and 90
    pub latitude: f64,
    /// Longitude between -180 and 180
    pub longitude: f64,
    /// Optional accuracy in meters (default: 0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,
}

/// Cookie information for storage state.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    /// Cookie name
    pub name: String,
    /// Cookie value
    pub value: String,
    /// Cookie domain (use dot prefix for subdomain matching, e.g., ".example.com")
    pub domain: String,
    /// Cookie path
    pub path: String,
    /// Unix timestamp in seconds; -1 for session cookies
    pub expires: f64,
    /// HTTP-only flag
    pub http_only: bool,
    /// Secure flag
    pub secure: bool,
    /// SameSite attribute ("Strict", "Lax", "None")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<String>,
}

/// Local storage item for storage state.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStorageItem {
    /// Storage key
    pub name: String,
    /// Storage value
    pub value: String,
}

/// Origin with local storage items for storage state.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Origin {
    /// Origin URL (e.g., `https://example.com`)
    pub origin: String,
    /// Local storage items for this origin
    pub local_storage: Vec<LocalStorageItem>,
}

/// Storage state containing cookies and local storage.
///
/// Used to populate a browser context with saved authentication state,
/// enabling session persistence across context instances.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageState {
    /// List of cookies
    pub cookies: Vec<Cookie>,
    /// List of origins with local storage
    pub origins: Vec<Origin>,
}

/// Options for filtering which cookies to clear with `BrowserContext::clear_cookies()`.
///
/// All fields are optional; when provided they act as AND-combined filters.
///
/// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-clear-cookies>
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearCookiesOptions {
    /// Filter by cookie name (exact match).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Filter by cookie domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Filter by cookie path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Options for `BrowserContext::grant_permissions()`.
///
/// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-grant-permissions>
#[derive(Debug, Clone, Default)]
pub struct GrantPermissionsOptions {
    /// Optional origin to restrict the permission grant to.
    ///
    /// For example `"https://example.com"`.
    pub origin: Option<String>,
}

/// Options for recording HAR.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-record-har>
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordHar {
    /// Path on the filesystem to write the HAR file to.
    pub path: String,
    /// Optional setting to control whether to omit request content from the HAR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omit_content: Option<bool>,
    /// Optional setting to control resource content management.
    /// "omit" | "embed" | "attach"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// "full" | "minimal"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// A glob or regex pattern to filter requests that are stored in the HAR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_filter: Option<String>,
}

/// Options for recording video.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-record-video>
#[derive(Debug, Clone, Serialize, Default)]
pub struct RecordVideo {
    /// Path to the directory to put videos into.
    pub dir: String,
    /// Optional dimensions of the recorded videos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Viewport>,
}

/// Options for creating a new browser context.
///
/// Allows customizing viewport, user agent, locale, timezone, geolocation,
/// permissions, and other browser context settings.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserContextOptions {
    /// Sets consistent viewport for all pages in the context.
    /// Set to null via `no_viewport(true)` to disable viewport emulation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,

    /// Disables viewport emulation when set to true.
    /// Note: Playwright's public API calls this `noViewport`, but the protocol
    /// expects `noDefaultViewport`. playwright-python applies this transformation
    /// in `_prepare_browser_context_params`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "noDefaultViewport")]
    pub no_viewport: Option<bool>,

    /// Custom user agent string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// Locale for the context (e.g., "en-GB", "de-DE", "fr-FR")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,

    /// Timezone identifier (e.g., "America/New_York", "Europe/Berlin")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone_id: Option<String>,

    /// Geolocation coordinates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<Geolocation>,

    /// List of permissions to grant (e.g., "geolocation", "notifications")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,

    /// Network proxy settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxySettings>,

    /// Emulates 'prefers-colors-scheme' media feature ("light", "dark", "no-preference")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,

    /// Whether the viewport supports touch events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_touch: Option<bool>,

    /// Whether the meta viewport tag is respected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_mobile: Option<bool>,

    /// Whether JavaScript is enabled in the context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub javascript_enabled: Option<bool>,

    /// Emulates network being offline
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offline: Option<bool>,

    /// Whether to automatically download attachments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_downloads: Option<bool>,

    /// Whether to bypass Content-Security-Policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bypass_csp: Option<bool>,

    /// Whether to ignore HTTPS errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_https_errors: Option<bool>,

    /// Device scale factor (default: 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_scale_factor: Option<f64>,

    /// Extra HTTP headers to send with every request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_http_headers: Option<HashMap<String, String>>,

    /// Base URL for relative navigation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Storage state to populate the context (cookies, localStorage, sessionStorage).
    /// Can be an inline StorageState object or a file path string.
    /// Use builder methods `storage_state()` for inline or `storage_state_path()` for file path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_state: Option<StorageState>,

    /// Storage state file path (alternative to inline storage_state).
    /// This is handled by the builder and converted to storage_state during serialization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_state_path: Option<String>,

    // Launch options (for launch_persistent_context)
    /// Additional arguments to pass to browser instance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,

    /// Browser distribution channel (e.g., "chrome", "msedge")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,

    /// Enable Chromium sandboxing (default: false on Linux)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chromium_sandbox: Option<bool>,

    /// Auto-open DevTools (deprecated, default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devtools: Option<bool>,

    /// Directory to save downloads
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloads_path: Option<String>,

    /// Path to custom browser executable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,

    /// Firefox user preferences (Firefox only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firefox_user_prefs: Option<HashMap<String, serde_json::Value>>,

    /// Run in headless mode (default: true unless devtools=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headless: Option<bool>,

    /// Filter or disable default browser arguments.
    /// When `true`, Playwright does not pass its own default args.
    /// When an array, filters out the given default arguments.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsertype#browser-type-launch-persistent-context>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_default_args: Option<IgnoreDefaultArgs>,

    /// Slow down operations by N milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow_mo: Option<f64>,

    /// Timeout for browser launch in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,

    /// Directory to save traces
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traces_dir: Option<String>,

    /// Check if strict selectors mode is enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_selectors: Option<bool>,

    /// Emulates 'prefers-reduced-motion' media feature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduced_motion: Option<String>,

    /// Emulates 'forced-colors' media feature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forced_colors: Option<String>,

    /// Whether to allow sites to register Service workers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_workers: Option<String>,

    /// Options for recording HAR
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_har: Option<RecordHar>,

    /// Options for recording video
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_video: Option<RecordVideo>,
}

impl BrowserContextOptions {
    /// Creates a new builder for BrowserContextOptions
    pub fn builder() -> BrowserContextOptionsBuilder {
        BrowserContextOptionsBuilder::default()
    }
}

/// Builder for BrowserContextOptions
#[derive(Debug, Clone, Default)]
pub struct BrowserContextOptionsBuilder {
    viewport: Option<Viewport>,
    no_viewport: Option<bool>,
    user_agent: Option<String>,
    locale: Option<String>,
    timezone_id: Option<String>,
    geolocation: Option<Geolocation>,
    permissions: Option<Vec<String>>,
    proxy: Option<ProxySettings>,
    color_scheme: Option<String>,
    has_touch: Option<bool>,
    is_mobile: Option<bool>,
    javascript_enabled: Option<bool>,
    offline: Option<bool>,
    accept_downloads: Option<bool>,
    bypass_csp: Option<bool>,
    ignore_https_errors: Option<bool>,
    device_scale_factor: Option<f64>,
    extra_http_headers: Option<HashMap<String, String>>,
    base_url: Option<String>,
    storage_state: Option<StorageState>,
    storage_state_path: Option<String>,
    // Launch options
    args: Option<Vec<String>>,
    channel: Option<String>,
    chromium_sandbox: Option<bool>,
    devtools: Option<bool>,
    downloads_path: Option<String>,
    executable_path: Option<String>,
    firefox_user_prefs: Option<HashMap<String, serde_json::Value>>,
    headless: Option<bool>,
    ignore_default_args: Option<IgnoreDefaultArgs>,
    slow_mo: Option<f64>,
    timeout: Option<f64>,
    traces_dir: Option<String>,
    strict_selectors: Option<bool>,
    reduced_motion: Option<String>,
    forced_colors: Option<String>,
    service_workers: Option<String>,
    record_har: Option<RecordHar>,
    record_video: Option<RecordVideo>,
}

impl BrowserContextOptionsBuilder {
    /// Sets the viewport dimensions
    pub fn viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = Some(viewport);
        self.no_viewport = None; // Clear no_viewport if setting viewport
        self
    }

    /// Disables viewport emulation
    pub fn no_viewport(mut self, no_viewport: bool) -> Self {
        self.no_viewport = Some(no_viewport);
        if no_viewport {
            self.viewport = None; // Clear viewport if setting no_viewport
        }
        self
    }

    /// Sets the user agent string
    pub fn user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Sets the locale
    pub fn locale(mut self, locale: String) -> Self {
        self.locale = Some(locale);
        self
    }

    /// Sets the timezone identifier
    pub fn timezone_id(mut self, timezone_id: String) -> Self {
        self.timezone_id = Some(timezone_id);
        self
    }

    /// Sets the geolocation
    pub fn geolocation(mut self, geolocation: Geolocation) -> Self {
        self.geolocation = Some(geolocation);
        self
    }

    /// Sets the permissions to grant
    pub fn permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Sets the network proxy settings for this context.
    ///
    /// This allows routing all network traffic through a proxy server,
    /// useful for rotating proxies without creating new browsers.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use playwright_rs::protocol::{BrowserContextOptions, ProxySettings};
    ///
    /// let options = BrowserContextOptions::builder()
    ///     .proxy(ProxySettings {
    ///         server: "http://proxy.example.com:8080".to_string(),
    ///         bypass: Some(".example.com".to_string()),
    ///         username: Some("user".to_string()),
    ///         password: Some("pass".to_string()),
    ///     })
    ///     .build();
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
    pub fn proxy(mut self, proxy: ProxySettings) -> Self {
        self.proxy = Some(proxy);
        self
    }

    /// Sets the color scheme preference
    pub fn color_scheme(mut self, color_scheme: String) -> Self {
        self.color_scheme = Some(color_scheme);
        self
    }

    /// Sets whether the viewport supports touch events
    pub fn has_touch(mut self, has_touch: bool) -> Self {
        self.has_touch = Some(has_touch);
        self
    }

    /// Sets whether this is a mobile viewport
    pub fn is_mobile(mut self, is_mobile: bool) -> Self {
        self.is_mobile = Some(is_mobile);
        self
    }

    /// Sets whether JavaScript is enabled
    pub fn javascript_enabled(mut self, javascript_enabled: bool) -> Self {
        self.javascript_enabled = Some(javascript_enabled);
        self
    }

    /// Sets whether to emulate offline network
    pub fn offline(mut self, offline: bool) -> Self {
        self.offline = Some(offline);
        self
    }

    /// Sets whether to automatically download attachments
    pub fn accept_downloads(mut self, accept_downloads: bool) -> Self {
        self.accept_downloads = Some(accept_downloads);
        self
    }

    /// Sets whether to bypass Content-Security-Policy
    pub fn bypass_csp(mut self, bypass_csp: bool) -> Self {
        self.bypass_csp = Some(bypass_csp);
        self
    }

    /// Sets whether to ignore HTTPS errors
    pub fn ignore_https_errors(mut self, ignore_https_errors: bool) -> Self {
        self.ignore_https_errors = Some(ignore_https_errors);
        self
    }

    /// Sets the device scale factor
    pub fn device_scale_factor(mut self, device_scale_factor: f64) -> Self {
        self.device_scale_factor = Some(device_scale_factor);
        self
    }

    /// Sets extra HTTP headers
    pub fn extra_http_headers(mut self, extra_http_headers: HashMap<String, String>) -> Self {
        self.extra_http_headers = Some(extra_http_headers);
        self
    }

    /// Sets the base URL for relative navigation
    pub fn base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }

    /// Sets the storage state inline (cookies, localStorage).
    ///
    /// Populates the browser context with the provided storage state, including
    /// cookies and local storage. This is useful for initializing a context with
    /// a saved authentication state.
    ///
    /// Mutually exclusive with `storage_state_path()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use playwright_rs::protocol::{BrowserContextOptions, Cookie, StorageState, Origin, LocalStorageItem};
    ///
    /// let storage_state = StorageState {
    ///     cookies: vec![Cookie {
    ///         name: "session_id".to_string(),
    ///         value: "abc123".to_string(),
    ///         domain: ".example.com".to_string(),
    ///         path: "/".to_string(),
    ///         expires: -1.0,
    ///         http_only: true,
    ///         secure: true,
    ///         same_site: Some("Lax".to_string()),
    ///     }],
    ///     origins: vec![Origin {
    ///         origin: "https://example.com".to_string(),
    ///         local_storage: vec![LocalStorageItem {
    ///             name: "user_prefs".to_string(),
    ///             value: "{\"theme\":\"dark\"}".to_string(),
    ///         }],
    ///     }],
    /// };
    ///
    /// let options = BrowserContextOptions::builder()
    ///     .storage_state(storage_state)
    ///     .build();
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
    pub fn storage_state(mut self, storage_state: StorageState) -> Self {
        self.storage_state = Some(storage_state);
        self.storage_state_path = None; // Clear path if setting inline
        self
    }

    /// Sets the storage state from a file path.
    ///
    /// The file should contain a JSON representation of StorageState with cookies
    /// and origins. This is useful for loading authentication state saved from a
    /// previous session.
    ///
    /// Mutually exclusive with `storage_state()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use playwright_rs::protocol::BrowserContextOptions;
    ///
    /// let options = BrowserContextOptions::builder()
    ///     .storage_state_path("auth.json".to_string())
    ///     .build();
    /// ```
    ///
    /// The file should have this format:
    /// ```json
    /// {
    ///   "cookies": [{
    ///     "name": "session_id",
    ///     "value": "abc123",
    ///     "domain": ".example.com",
    ///     "path": "/",
    ///     "expires": -1,
    ///     "httpOnly": true,
    ///     "secure": true,
    ///     "sameSite": "Lax"
    ///   }],
    ///   "origins": [{
    ///     "origin": "https://example.com",
    ///     "localStorage": [{
    ///       "name": "user_prefs",
    ///       "value": "{\"theme\":\"dark\"}"
    ///     }]
    ///   }]
    /// }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
    pub fn storage_state_path(mut self, path: String) -> Self {
        self.storage_state_path = Some(path);
        self.storage_state = None; // Clear inline if setting path
        self
    }

    /// Sets additional arguments to pass to browser instance (for launch_persistent_context)
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }

    /// Sets browser distribution channel (for launch_persistent_context)
    pub fn channel(mut self, channel: String) -> Self {
        self.channel = Some(channel);
        self
    }

    /// Enables or disables Chromium sandboxing (for launch_persistent_context)
    pub fn chromium_sandbox(mut self, enabled: bool) -> Self {
        self.chromium_sandbox = Some(enabled);
        self
    }

    /// Auto-open DevTools (for launch_persistent_context)
    pub fn devtools(mut self, enabled: bool) -> Self {
        self.devtools = Some(enabled);
        self
    }

    /// Sets directory to save downloads (for launch_persistent_context)
    pub fn downloads_path(mut self, path: String) -> Self {
        self.downloads_path = Some(path);
        self
    }

    /// Sets path to custom browser executable (for launch_persistent_context)
    pub fn executable_path(mut self, path: String) -> Self {
        self.executable_path = Some(path);
        self
    }

    /// Sets Firefox user preferences (for launch_persistent_context, Firefox only)
    pub fn firefox_user_prefs(mut self, prefs: HashMap<String, serde_json::Value>) -> Self {
        self.firefox_user_prefs = Some(prefs);
        self
    }

    /// Run in headless mode (for launch_persistent_context)
    pub fn headless(mut self, enabled: bool) -> Self {
        self.headless = Some(enabled);
        self
    }

    /// Filter or disable default browser arguments (for launch_persistent_context).
    ///
    /// When `IgnoreDefaultArgs::Bool(true)`, Playwright does not pass its own
    /// default arguments and only uses the ones from `args`.
    /// When `IgnoreDefaultArgs::Array(vec)`, filters out the given default arguments.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsertype#browser-type-launch-persistent-context>
    pub fn ignore_default_args(mut self, args: IgnoreDefaultArgs) -> Self {
        self.ignore_default_args = Some(args);
        self
    }

    /// Slow down operations by N milliseconds (for launch_persistent_context)
    pub fn slow_mo(mut self, ms: f64) -> Self {
        self.slow_mo = Some(ms);
        self
    }

    /// Set timeout for browser launch in milliseconds (for launch_persistent_context)
    pub fn timeout(mut self, ms: f64) -> Self {
        self.timeout = Some(ms);
        self
    }

    /// Set directory to save traces (for launch_persistent_context)
    pub fn traces_dir(mut self, path: String) -> Self {
        self.traces_dir = Some(path);
        self
    }

    /// Check if strict selectors mode is enabled
    pub fn strict_selectors(mut self, enabled: bool) -> Self {
        self.strict_selectors = Some(enabled);
        self
    }

    /// Emulates 'prefers-reduced-motion' media feature
    pub fn reduced_motion(mut self, value: String) -> Self {
        self.reduced_motion = Some(value);
        self
    }

    /// Emulates 'forced-colors' media feature
    pub fn forced_colors(mut self, value: String) -> Self {
        self.forced_colors = Some(value);
        self
    }

    /// Whether to allow sites to register Service workers ("allow" | "block")
    pub fn service_workers(mut self, value: String) -> Self {
        self.service_workers = Some(value);
        self
    }

    /// Sets options for recording HAR
    pub fn record_har(mut self, record_har: RecordHar) -> Self {
        self.record_har = Some(record_har);
        self
    }

    /// Sets options for recording video
    pub fn record_video(mut self, record_video: RecordVideo) -> Self {
        self.record_video = Some(record_video);
        self
    }

    /// Builds the BrowserContextOptions
    pub fn build(self) -> BrowserContextOptions {
        BrowserContextOptions {
            viewport: self.viewport,
            no_viewport: self.no_viewport,
            user_agent: self.user_agent,
            locale: self.locale,
            timezone_id: self.timezone_id,
            geolocation: self.geolocation,
            permissions: self.permissions,
            proxy: self.proxy,
            color_scheme: self.color_scheme,
            has_touch: self.has_touch,
            is_mobile: self.is_mobile,
            javascript_enabled: self.javascript_enabled,
            offline: self.offline,
            accept_downloads: self.accept_downloads,
            bypass_csp: self.bypass_csp,
            ignore_https_errors: self.ignore_https_errors,
            device_scale_factor: self.device_scale_factor,
            extra_http_headers: self.extra_http_headers,
            base_url: self.base_url,
            storage_state: self.storage_state,
            storage_state_path: self.storage_state_path,
            // Launch options
            args: self.args,
            channel: self.channel,
            chromium_sandbox: self.chromium_sandbox,
            devtools: self.devtools,
            downloads_path: self.downloads_path,
            executable_path: self.executable_path,
            firefox_user_prefs: self.firefox_user_prefs,
            headless: self.headless,
            ignore_default_args: self.ignore_default_args,
            slow_mo: self.slow_mo,
            timeout: self.timeout,
            traces_dir: self.traces_dir,
            strict_selectors: self.strict_selectors,
            reduced_motion: self.reduced_motion,
            forced_colors: self.forced_colors,
            service_workers: self.service_workers,
            record_har: self.record_har,
            record_video: self.record_video,
        }
    }
}

/// Extracts timing data from a Response object's initializer, patching in
/// `responseEnd` from the event's `responseEndTiming` if available.
async fn extract_timing(
    connection: &std::sync::Arc<dyn crate::server::connection::ConnectionLike>,
    response_guid: Option<String>,
    response_end_timing: Option<f64>,
) -> Option<serde_json::Value> {
    let resp_guid = response_guid?;
    let resp_obj: crate::protocol::ResponseObject = connection
        .get_typed::<crate::protocol::ResponseObject>(&resp_guid)
        .await
        .ok()?;
    let mut timing = resp_obj.initializer().get("timing")?.clone();
    if let (Some(end), Some(obj)) = (response_end_timing, timing.as_object_mut())
        && let Some(n) = serde_json::Number::from_f64(end)
    {
        obj.insert("responseEnd".to_string(), serde_json::Value::Number(n));
    }
    Some(timing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::launch_options::IgnoreDefaultArgs;

    #[test]
    fn test_browser_context_options_ignore_default_args_bool_serialization() {
        let options = BrowserContextOptions::builder()
            .ignore_default_args(IgnoreDefaultArgs::Bool(true))
            .build();

        let value = serde_json::to_value(&options).unwrap();
        assert_eq!(value["ignoreDefaultArgs"], serde_json::json!(true));
    }

    #[test]
    fn test_browser_context_options_ignore_default_args_array_serialization() {
        let options = BrowserContextOptions::builder()
            .ignore_default_args(IgnoreDefaultArgs::Array(vec!["--foo".to_string()]))
            .build();

        let value = serde_json::to_value(&options).unwrap();
        assert_eq!(value["ignoreDefaultArgs"], serde_json::json!(["--foo"]));
    }

    #[test]
    fn test_browser_context_options_ignore_default_args_absent() {
        let options = BrowserContextOptions::builder().build();

        let value = serde_json::to_value(&options).unwrap();
        assert!(value.get("ignoreDefaultArgs").is_none());
    }
}
