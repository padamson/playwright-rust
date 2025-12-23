// BrowserContext protocol object
//
// Represents an isolated browser context (session) within a browser instance.
// Multiple contexts can exist in a single browser, each with its own cookies,
// cache, and local storage.

use crate::error::Result;
use crate::protocol::Page;
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

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
///     // Cleanup
///     context1.close().await?;
///     context2.close().await?;
///     browser.close().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-browsercontext>
#[derive(Clone)]
pub struct BrowserContext {
    base: ChannelOwnerImpl,
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
            ParentOrConnection::Parent(parent),
            type_name,
            guid,
            initializer,
        );

        let context = Self { base };

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

        Ok(page.clone())
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
        }
    }
}
