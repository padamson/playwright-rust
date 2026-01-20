// BrowserContext protocol object
//
// Represents an isolated browser context (session) within a browser instance.
// Multiple contexts can exist in a single browser, each with its own cookies,
// cache, and local storage.

use crate::error::Result;
use crate::protocol::{Browser, Page};
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
#[derive(Clone)]
pub struct BrowserContext {
    base: ChannelOwnerImpl,
    /// Browser instance that owns this context (None for persistent contexts)
    browser: Option<Browser>,
    /// All open pages in this context
    pages: Arc<Mutex<Vec<Page>>>,
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
        };

        // Enable dialog event subscription
        // Dialog events need to be explicitly subscribed to via updateSubscription command
        let channel = context.channel().clone();
        tokio::spawn(async move {
            let _ = channel
                .send_no_result(
                    "updateSubscription",
                    serde_json::json!({
                        "event": "dialog",
                        "enabled": true
                    }),
                )
                .await;
        });

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

        // Retrieve the Page object from the connection registry
        let page_arc = self.connection().get_object(&response.page.guid).await?;

        // Downcast to Page
        let page = page_arc.as_any().downcast_ref::<Page>().ok_or_else(|| {
            crate::error::Error::ProtocolError(format!(
                "Expected Page object, got {}",
                page_arc.type_name()
            ))
        })?;

        // Note: Don't track the page here - it will be tracked via the "page" event
        // that Playwright server sends automatically when a page is created.
        // Tracking it here would create duplicates.

        Ok(page.clone())
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
        // Send close RPC to server
        self.channel()
            .send_no_result("close", serde_json::json!({}))
            .await
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

                    tokio::spawn(async move {
                        // Get the Page object
                        let page_arc = match connection.get_object(&page_guid_owned).await {
                            Ok(obj) => obj,
                            Err(_) => return,
                        };

                        // Downcast to Page
                        let page = match page_arc.as_any().downcast_ref::<Page>() {
                            Some(p) => p.clone(),
                            None => return,
                        };

                        // Track the page
                        pages.lock().unwrap().push(page);
                    });
                }
            }
            "dialog" => {
                // Dialog events come to BrowserContext, need to forward to the associated Page
                // Event format: {dialog: {guid: "..."}}
                // The Dialog protocol object has the Page as its parent
                if let Some(dialog_guid) = params
                    .get("dialog")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let dialog_guid_owned = dialog_guid.to_string();

                    tokio::spawn(async move {
                        // Get the Dialog object
                        let dialog_arc = match connection.get_object(&dialog_guid_owned).await {
                            Ok(obj) => obj,
                            Err(_) => return,
                        };

                        // Downcast to Dialog
                        let dialog = match dialog_arc
                            .as_any()
                            .downcast_ref::<crate::protocol::Dialog>()
                        {
                            Some(d) => d.clone(),
                            None => return,
                        };

                        // Get the Page from the Dialog's parent
                        let page_arc = match dialog_arc.parent() {
                            Some(parent) => parent,
                            None => return,
                        };

                        // Downcast to Page
                        let page = match page_arc.as_any().downcast_ref::<Page>() {
                            Some(p) => p.clone(),
                            None => return,
                        };

                        // Forward to Page's dialog handlers
                        page.trigger_dialog_event(dialog).await;
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
    /// Origin URL (e.g., "https://example.com")
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
