// Browser protocol object
//
// Represents a browser instance created by BrowserType.launch()

use crate::error::Result;
use crate::protocol::{BrowserContext, BrowserType, Page};
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::server::connection::ConnectionExt;
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Type alias for the future returned by a disconnected handler.
type DisconnectedHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for a registered disconnected event handler.
type DisconnectedHandler = Arc<dyn Fn() -> DisconnectedHandlerFuture + Send + Sync>;

/// Options for `Browser::start_tracing()`.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-start-tracing>
#[derive(Debug, Default, Clone)]
pub struct StartTracingOptions {
    /// If specified, tracing captures screenshots for this page.
    /// Pass `Some(page)` to associate the trace with a specific page.
    pub page: Option<Page>,
    /// Whether to capture screenshots during tracing. Default false.
    pub screenshots: Option<bool>,
    /// Trace categories to enable. If omitted, uses a default set.
    pub categories: Option<Vec<String>>,
}

/// Browser represents a browser instance.
///
/// A Browser is created when you call `BrowserType::launch()`. It provides methods
/// to create browser contexts and pages.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::Playwright;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let chromium = playwright.chromium();
///
///     let browser = chromium.launch().await?;
///     println!("Browser: {} version {}", browser.name(), browser.version());
///     assert!(browser.is_connected());
///
///     let bt = browser.browser_type();
///     assert_eq!(bt.name(), "chromium");
///
///     let context = browser.new_context().await?;
///     let _page = context.new_page().await?;
///     assert_eq!(browser.contexts().len(), 1);
///
///     browser.on_disconnected(|| async { Ok(()) }).await?;
///
///     browser.start_tracing(None).await?;
///     let _trace_bytes = browser.stop_tracing().await?;
///
///     browser.close().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-browser>
#[derive(Clone)]
pub struct Browser {
    base: ChannelOwnerImpl,
    version: String,
    name: String,
    is_connected: Arc<AtomicBool>,
    /// Registered handlers for the "disconnected" event.
    disconnected_handlers: Arc<Mutex<Vec<DisconnectedHandler>>>,
}

impl Browser {
    /// Creates a new Browser from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Browser object.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent BrowserType object
    /// * `type_name` - The protocol type name ("Browser")
    /// * `guid` - The unique identifier for this browser instance
    /// * `initializer` - The initialization data from the server
    ///
    /// # Errors
    ///
    /// Returns error if initializer is missing required fields (version, name)
    pub fn new(
        parent: Arc<dyn ChannelOwner>,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent),
            type_name,
            guid,
            initializer.clone(),
        );

        let version = initializer["version"]
            .as_str()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "Browser initializer missing 'version' field".to_string(),
                )
            })?
            .to_string();

        let name = initializer["name"]
            .as_str()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "Browser initializer missing 'name' field".to_string(),
                )
            })?
            .to_string();

        Ok(Self {
            base,
            version,
            name,
            is_connected: Arc::new(AtomicBool::new(true)),
            disconnected_handlers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Returns the browser version string.
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-version>
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the browser name (e.g., "chromium", "firefox", "webkit").
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-name>
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns true if the browser is connected.
    ///
    /// The browser is connected when it is launched and becomes disconnected when:
    /// - `browser.close()` is called
    /// - The browser process crashes
    /// - The browser is closed by the user
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-is-connected>
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    /// Returns the channel for sending protocol messages
    ///
    /// Used internally for sending RPC calls to the browser.
    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    /// Creates a new browser context.
    ///
    /// A browser context is an isolated session within the browser instance,
    /// similar to an incognito profile. Each context has its own cookies,
    /// cache, and local storage.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
    pub async fn new_context(&self) -> Result<BrowserContext> {
        #[derive(Deserialize)]
        struct NewContextResponse {
            context: GuidRef,
        }

        #[derive(Deserialize)]
        struct GuidRef {
            #[serde(deserialize_with = "crate::server::connection::deserialize_arc_str")]
            guid: Arc<str>,
        }

        let response: NewContextResponse = self
            .channel()
            .send("newContext", serde_json::json!({}))
            .await?;

        let context: BrowserContext = self
            .connection()
            .get_typed::<BrowserContext>(&response.context.guid)
            .await?;

        let selectors = self.connection().selectors();
        if let Err(e) = selectors.add_context(context.channel().clone()).await {
            tracing::warn!("Failed to register BrowserContext with Selectors: {}", e);
        }

        Ok(context)
    }

    /// Creates a new browser context with custom options.
    ///
    /// A browser context is an isolated session within the browser instance,
    /// similar to an incognito profile. Each context has its own cookies,
    /// cache, and local storage.
    ///
    /// This method allows customizing viewport, user agent, locale, timezone,
    /// and other settings.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser has been closed
    /// - Communication with browser process fails
    /// - Invalid options provided
    /// - Storage state file cannot be read or parsed
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
    pub async fn new_context_with_options(
        &self,
        mut options: crate::protocol::BrowserContextOptions,
    ) -> Result<BrowserContext> {
        // Response contains the GUID of the created BrowserContext
        #[derive(Deserialize)]
        struct NewContextResponse {
            context: GuidRef,
        }

        #[derive(Deserialize)]
        struct GuidRef {
            #[serde(deserialize_with = "crate::server::connection::deserialize_arc_str")]
            guid: Arc<str>,
        }

        // Handle storage_state_path: read file and convert to inline storage_state
        if let Some(path) = &options.storage_state_path {
            let file_content = tokio::fs::read_to_string(path).await.map_err(|e| {
                crate::error::Error::ProtocolError(format!(
                    "Failed to read storage state file '{}': {}",
                    path, e
                ))
            })?;

            let storage_state: crate::protocol::StorageState = serde_json::from_str(&file_content)
                .map_err(|e| {
                    crate::error::Error::ProtocolError(format!(
                        "Failed to parse storage state file '{}': {}",
                        path, e
                    ))
                })?;

            options.storage_state = Some(storage_state);
            options.storage_state_path = None; // Clear path since we've converted to inline
        }

        // Convert options to JSON
        let options_json = serde_json::to_value(options).map_err(|e| {
            crate::error::Error::ProtocolError(format!(
                "Failed to serialize context options: {}",
                e
            ))
        })?;

        // Send newContext RPC to server with options
        let response: NewContextResponse = self.channel().send("newContext", options_json).await?;

        // Retrieve and downcast the BrowserContext object from the connection registry
        let context: BrowserContext = self
            .connection()
            .get_typed::<BrowserContext>(&response.context.guid)
            .await?;

        // Register new context with the Selectors coordinator.
        let selectors = self.connection().selectors();
        if let Err(e) = selectors.add_context(context.channel().clone()).await {
            tracing::warn!("Failed to register BrowserContext with Selectors: {}", e);
        }

        Ok(context)
    }

    /// Creates a new page in a new browser context.
    ///
    /// This is a convenience method that creates a default context and then
    /// creates a page in it. This is equivalent to calling `browser.new_context().await?.new_page().await?`.
    ///
    /// The created context is not directly accessible, but will be cleaned up
    /// when the page is closed.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-page>
    pub async fn new_page(&self) -> Result<Page> {
        // Create a default context and then create a page in it
        let context = self.new_context().await?;
        context.new_page().await
    }

    /// Returns all open browser contexts.
    ///
    /// A new browser starts with no contexts. Contexts are created via
    /// `new_context()` and cleaned up when they are closed.
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-contexts>
    pub fn contexts(&self) -> Vec<BrowserContext> {
        let my_guid = self.guid();
        self.connection()
            .all_objects_sync()
            .into_iter()
            .filter_map(|obj| {
                let ctx = obj.as_any().downcast_ref::<BrowserContext>()?.clone();
                let parent_guid = ctx.parent().map(|p| p.guid().to_string());
                if parent_guid.as_deref() == Some(my_guid) {
                    Some(ctx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the `BrowserType` that was used to launch this browser.
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-browser-type>
    pub fn browser_type(&self) -> BrowserType {
        self.base
            .parent()
            .expect("Browser always has a BrowserType parent")
            .as_any()
            .downcast_ref::<BrowserType>()
            .expect("Browser parent is always a BrowserType")
            .clone()
    }

    /// Registers a handler that fires when the browser is disconnected.
    ///
    /// The browser can become disconnected when it is closed, crashes, or
    /// the process is killed. The handler is called with no arguments.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure called when the browser disconnects.
    ///
    /// # Errors
    ///
    /// Returns an error only if the mutex is poisoned (practically never).
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-event-disconnected>
    pub async fn on_disconnected<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move || -> DisconnectedHandlerFuture { Box::pin(handler()) });
        self.disconnected_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Starts CDP tracing on this browser (Chromium only).
    ///
    /// Only one trace may be active at a time per browser instance.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional tracing configuration (screenshots, categories, page).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Tracing is already active
    /// - Called on a non-Chromium browser
    /// - Communication with the browser fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-start-tracing>
    pub async fn start_tracing(&self, options: Option<StartTracingOptions>) -> Result<()> {
        #[derive(serde::Serialize)]
        struct StartTracingParams {
            #[serde(skip_serializing_if = "Option::is_none")]
            page: Option<serde_json::Value>,
            #[serde(skip_serializing_if = "Option::is_none")]
            screenshots: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            categories: Option<Vec<String>>,
        }

        let opts = options.unwrap_or_default();

        let page_ref = opts
            .page
            .as_ref()
            .map(|p| serde_json::json!({ "guid": p.guid() }));

        let params = StartTracingParams {
            page: page_ref,
            screenshots: opts.screenshots,
            categories: opts.categories,
        };

        self.channel()
            .send_no_result(
                "startTracing",
                serde_json::to_value(params).map_err(|e| {
                    crate::error::Error::ProtocolError(format!(
                        "serialize startTracing params: {e}"
                    ))
                })?,
            )
            .await
    }

    /// Stops CDP tracing and returns the raw trace data.
    ///
    /// The returned bytes can be written to a `.json` file and loaded in
    /// `chrome://tracing` or [Perfetto](https://ui.perfetto.dev).
    ///
    /// # Errors
    ///
    /// Returns error if no tracing was started or communication fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-stop-tracing>
    pub async fn stop_tracing(&self) -> Result<Vec<u8>> {
        #[derive(Deserialize)]
        struct StopTracingResponse {
            artifact: ArtifactRef,
        }

        #[derive(Deserialize)]
        struct ArtifactRef {
            #[serde(deserialize_with = "crate::server::connection::deserialize_arc_str")]
            guid: Arc<str>,
        }

        let response: StopTracingResponse = self
            .channel()
            .send("stopTracing", serde_json::json!({}))
            .await?;

        // save_as() rather than streaming because Stream protocol is not yet implemented
        let artifact: crate::protocol::artifact::Artifact = self
            .connection()
            .get_typed::<crate::protocol::artifact::Artifact>(&response.artifact.guid)
            .await?;

        let tmp_path = std::env::temp_dir().join(format!(
            "playwright-trace-{}.json",
            response.artifact.guid.replace('@', "-")
        ));
        let tmp_str = tmp_path
            .to_str()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "Temporary path contains non-UTF-8 characters".to_string(),
                )
            })?
            .to_string();

        artifact.save_as(&tmp_str).await?;

        let bytes = tokio::fs::read(&tmp_path).await.map_err(|e| {
            crate::error::Error::ProtocolError(format!(
                "Failed to read tracing artifact from '{}': {}",
                tmp_str, e
            ))
        })?;

        let _ = tokio::fs::remove_file(&tmp_path).await;

        Ok(bytes)
    }

    /// Closes the browser and all of its pages (if any were opened).
    ///
    /// This is a graceful operation that sends a close command to the browser
    /// and waits for it to shut down properly.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser has already been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-close>
    pub async fn close(&self) -> Result<()> {
        // Send close RPC to server
        // The protocol expects an empty object as params
        let result = self
            .channel()
            .send_no_result("close", serde_json::json!({}))
            .await;

        // Add delay on Windows CI to ensure browser process fully terminates
        // This prevents subsequent browser launches from hanging
        #[cfg(windows)]
        {
            let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
            if is_ci {
                tracing::debug!("[playwright-rust] Adding Windows CI browser cleanup delay");
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        result
    }
}

impl ChannelOwner for Browser {
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
        // Use compare_exchange so handlers fire exactly once across both the
        // "disconnected" event path and the __dispose__ path.
        if self
            .is_connected
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let handlers = self.disconnected_handlers.lock().unwrap().clone();
            tokio::spawn(async move {
                for handler in handlers {
                    if let Err(e) = handler().await {
                        tracing::warn!("Browser disconnected handler error (from dispose): {}", e);
                    }
                }
            });
        }
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
        if method == "disconnected" {
            // Use compare_exchange to fire handlers exactly once (guards against
            // both the "disconnected" event and the __dispose__ path firing them).
            if self
                .is_connected
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                let handlers = self.disconnected_handlers.lock().unwrap().clone();
                tokio::spawn(async move {
                    for handler in handlers {
                        if let Err(e) = handler().await {
                            tracing::warn!("Browser disconnected handler error: {}", e);
                        }
                    }
                });
            }
        }
        self.base.on_event(method, params)
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for Browser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Browser")
            .field("guid", &self.guid())
            .field("name", &self.name)
            .field("version", &self.version)
            .finish()
    }
}

// Note: Browser testing is done via integration tests since it requires:
// - A real Connection with object registry
// - Protocol messages from the server
// - BrowserType.launch() to create Browser objects
// See: crates/playwright-core/tests/browser_launch_integration.rs (Phase 2 Slice 3)
