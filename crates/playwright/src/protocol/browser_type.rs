// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// BrowserType - Represents a browser type (Chromium, Firefox, WebKit)
//
// Reference:
// - Python: playwright-python/playwright/_impl/_browser_type.py
// - Protocol: protocol.yml (BrowserType interface)

use crate::api::{ConnectOptions, ConnectOverCdpOptions, LaunchOptions};
use crate::error::Result;
use crate::protocol::{Browser, BrowserContext, BrowserContextOptions};
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::server::connection::{ConnectionExt, ConnectionLike};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// BrowserType represents a browser engine (Chromium, Firefox, or WebKit).
///
/// Each Playwright instance provides three BrowserType objects accessible via:
/// - `playwright.chromium()`
/// - `playwright.firefox()`
/// - `playwright.webkit()`
///
/// BrowserType provides three main modes:
/// 1. **Launch**: Creates a new browser instance
/// 2. **Launch Persistent Context**: Creates browser + context with persistent storage
/// 3. **Connect**: Connects to an existing remote browser instance
///
/// # Example
///
/// ```ignore
/// # use playwright_rs::protocol::Playwright;
/// # use playwright_rs::api::LaunchOptions;
/// # use playwright_rs::protocol::BrowserContextOptions;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let playwright = Playwright::launch().await?;
/// let chromium = playwright.chromium();
///
/// // Verify browser type info
/// assert_eq!(chromium.name(), "chromium");
/// assert!(!chromium.executable_path().is_empty());
///
/// // === Standard Launch ===
/// // Launch with default options
/// let browser1 = chromium.launch().await?;
/// assert_eq!(browser1.name(), "chromium");
/// assert!(!browser1.version().is_empty());
/// browser1.close().await?;
///
/// // === Remote Connection ===
/// // Connect to a remote browser (e.g., started with `npx playwright launch-server`)
/// // let browser3 = chromium.connect("ws://localhost:3000", None).await?;
/// // browser3.close().await?;
///
/// // === CDP Connection (Chromium only) ===
/// // Connect to a Chrome instance with remote debugging enabled
/// // let browser4 = chromium.connect_over_cdp("http://localhost:9222", None).await?;
/// // browser4.close().await?;
///
/// // === Persistent Context Launch ===
/// // Launch with persistent storage (cookies, local storage, etc.)
/// let context = chromium
///     .launch_persistent_context("/tmp/user-data")
///     .await?;
/// let page = context.new_page().await?;
/// page.goto("https://example.com", None).await?;
/// context.close().await?; // Closes browser too
///
/// // === App Mode (Standalone Window) ===
/// // Launch as a standalone application window
/// let app_options = BrowserContextOptions::builder()
///     .args(vec!["--app=https://example.com".to_string()])
///     .headless(true) // Set to true for CI, but app mode is typically headed
///     .build();
///
/// let app_context = chromium
///     .launch_persistent_context_with_options("/tmp/app-data", app_options)
///     .await?;
/// // Browser opens directly to URL without address bar
/// app_context.close().await?;
/// # Ok(())
/// # }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-browsertype>
#[derive(Clone)]
pub struct BrowserType {
    /// Base ChannelOwner implementation
    base: ChannelOwnerImpl,
    /// Browser name ("chromium", "firefox", or "webkit")
    name: String,
    /// Path to browser executable
    executable_path: String,
}

impl BrowserType {
    /// Creates a new BrowserType object from protocol initialization.
    ///
    /// Called by the object factory when server sends __create__ message.
    ///
    /// # Arguments
    /// * `parent` - Parent (Connection for root objects, or another ChannelOwner)
    /// * `type_name` - Protocol type name ("BrowserType")
    /// * `guid` - Unique GUID from server (e.g., "browserType@chromium")
    /// * `initializer` - Initial state with name and executablePath
    pub fn new(
        parent: ParentOrConnection,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        let base = ChannelOwnerImpl::new(parent, type_name, guid, initializer.clone());

        // Extract fields from initializer
        let name = initializer["name"]
            .as_str()
            .ok_or_else(|| {
                crate::error::Error::ProtocolError(
                    "BrowserType initializer missing 'name'".to_string(),
                )
            })?
            .to_string();

        let executable_path = initializer["executablePath"]
            .as_str()
            .unwrap_or_default() // executablePath might be optional/empty for remote connection objects
            .to_string();

        Ok(Self {
            base,
            name,
            executable_path,
        })
    }

    /// Returns the browser name ("chromium", "firefox", or "webkit").
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the path to the browser executable.
    pub fn executable_path(&self) -> &str {
        &self.executable_path
    }

    /// Launches a browser instance with default options.
    ///
    /// This is equivalent to calling `launch_with_options(LaunchOptions::default())`.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser executable not found
    /// - Launch timeout (default 30s)
    /// - Browser process fails to start
    ///
    /// See: <https://playwright.dev/docs/api/class-browsertype#browser-type-launch>
    pub async fn launch(&self) -> Result<Browser> {
        self.launch_with_options(LaunchOptions::default()).await
    }

    /// Launches a browser instance with custom options.
    ///
    /// # Arguments
    ///
    /// * `options` - Launch options (headless, args, etc.)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser executable not found
    /// - Launch timeout
    /// - Invalid options
    /// - Browser process fails to start
    ///
    /// See: <https://playwright.dev/docs/api/class-browsertype#browser-type-launch>
    pub async fn launch_with_options(&self, options: LaunchOptions) -> Result<Browser> {
        // Add Windows CI-specific browser args to prevent hanging
        let options = {
            #[cfg(windows)]
            {
                let mut options = options;
                // Check if we're in a CI environment (GitHub Actions, Jenkins, etc.)
                let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();

                if is_ci {
                    tracing::debug!(
                        "[playwright-rust] Detected Windows CI environment, adding stability flags"
                    );

                    // Get existing args or create empty vec
                    let mut args = options.args.unwrap_or_default();

                    // Add Windows CI stability flags if not already present
                    let ci_flags = vec![
                        "--no-sandbox",            // Disable sandboxing (often problematic in CI)
                        "--disable-dev-shm-usage", // Overcome limited /dev/shm resources
                        "--disable-gpu",           // Disable GPU hardware acceleration
                        "--disable-web-security",  // Avoid CORS issues in CI
                        "--disable-features=IsolateOrigins,site-per-process", // Reduce process overhead
                    ];

                    for flag in ci_flags {
                        if !args.iter().any(|a| a == flag) {
                            args.push(flag.to_string());
                        }
                    }

                    // Update options with enhanced args
                    options.args = Some(args);

                    // Increase timeout for Windows CI (slower startup)
                    if options.timeout.is_none() {
                        options.timeout = Some(60000.0); // 60 seconds for Windows CI
                    }
                }
                options
            }

            #[cfg(not(windows))]
            {
                options
            }
        };

        // Normalize options for protocol transmission
        let params = options.normalize();

        // Send launch RPC to server
        let response: LaunchResponse = self.base.channel().send("launch", params).await?;

        // Get browser object from registry and downcast
        let browser: Browser = self
            .connection()
            .get_typed::<Browser>(&response.browser.guid)
            .await?;

        Ok(browser)
    }

    /// Launches a browser with persistent storage using default options.
    ///
    /// Returns a persistent browser context. Closing this context will automatically
    /// close the browser.
    ///
    /// This method is useful for:
    /// - Preserving authentication state across sessions
    /// - Testing with real user profiles
    /// - Creating standalone applications with app mode
    /// - Simulating real user behavior with cookies and storage
    ///
    /// # Arguments
    ///
    /// * `user_data_dir` - Path to a user data directory (stores cookies, local storage)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser executable not found
    /// - Launch timeout (default 30s)
    /// - Browser process fails to start
    /// - User data directory cannot be created
    ///
    /// See: <https://playwright.dev/docs/api/class-browsertype#browser-type-launch-persistent-context>
    pub async fn launch_persistent_context(
        &self,
        user_data_dir: impl Into<String>,
    ) -> Result<BrowserContext> {
        self.launch_persistent_context_with_options(user_data_dir, BrowserContextOptions::default())
            .await
    }

    /// Launches a browser with persistent storage and custom options.
    ///
    /// Returns a persistent browser context with the specified configuration.
    /// Closing this context will automatically close the browser.
    ///
    /// This method accepts both launch options (headless, args, etc.) and context
    /// options (viewport, locale, etc.) in a single BrowserContextOptions struct.
    ///
    /// # Arguments
    ///
    /// * `user_data_dir` - Path to a user data directory (stores cookies, local storage)
    /// * `options` - Combined launch and context options
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Browser executable not found
    /// - Launch timeout
    /// - Invalid options
    /// - Browser process fails to start
    /// - User data directory cannot be created
    ///
    /// See: <https://playwright.dev/docs/api/class-browsertype#browser-type-launch-persistent-context>
    pub async fn launch_persistent_context_with_options(
        &self,
        user_data_dir: impl Into<String>,
        mut options: BrowserContextOptions,
    ) -> Result<BrowserContext> {
        // Add Windows CI-specific browser args to prevent hanging
        #[cfg(windows)]
        {
            let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();

            if is_ci {
                tracing::debug!(
                    "[playwright-rust] Detected Windows CI environment, adding stability flags"
                );

                // Get existing args or create empty vec
                let mut args = options.args.unwrap_or_default();

                // Add Windows CI stability flags if not already present
                let ci_flags = vec![
                    "--no-sandbox",            // Disable sandboxing (often problematic in CI)
                    "--disable-dev-shm-usage", // Overcome limited /dev/shm resources
                    "--disable-gpu",           // Disable GPU hardware acceleration
                    "--disable-web-security",  // Avoid CORS issues in CI
                    "--disable-features=IsolateOrigins,site-per-process", // Reduce process overhead
                ];

                for flag in ci_flags {
                    if !args.iter().any(|a| a == flag) {
                        args.push(flag.to_string());
                    }
                }

                // Update options with enhanced args
                options.args = Some(args);

                // Increase timeout for Windows CI (slower startup)
                if options.timeout.is_none() {
                    options.timeout = Some(60000.0); // 60 seconds for Windows CI
                }
            }
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

        // Convert options to JSON with userDataDir
        let mut params = serde_json::to_value(&options).map_err(|e| {
            crate::error::Error::ProtocolError(format!(
                "Failed to serialize context options: {}",
                e
            ))
        })?;

        // Add userDataDir to params
        params["userDataDir"] = serde_json::json!(user_data_dir.into());

        // Set default timeout if not specified (required in Playwright 1.56.1+)
        if params.get("timeout").is_none() {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }

        // Convert bool ignoreDefaultArgs to ignoreAllDefaultArgs
        // (matches playwright-python's parameter normalization)
        if let Some(ignore) = params.get("ignoreDefaultArgs")
            && let Some(b) = ignore.as_bool()
        {
            if b {
                params["ignoreAllDefaultArgs"] = serde_json::json!(true);
            }
            params.as_object_mut().unwrap().remove("ignoreDefaultArgs");
        }

        // Send launchPersistentContext RPC to server
        let response: LaunchPersistentContextResponse = self
            .base
            .channel()
            .send("launchPersistentContext", params)
            .await?;

        // Get context object from registry and downcast
        let context: BrowserContext = self
            .connection()
            .get_typed::<BrowserContext>(&response.context.guid)
            .await?;

        Ok(context)
    }
    /// Connects to an existing browser instance.
    ///
    /// # Arguments
    /// * `ws_endpoint` - A WebSocket endpoint to connect to.
    /// * `options` - Connection options.
    ///
    /// # Errors
    /// Returns error if connection fails or handshake fails.
    pub async fn connect(
        &self,
        ws_endpoint: &str,
        options: Option<ConnectOptions>,
    ) -> Result<Browser> {
        use crate::server::connection::Connection;
        use crate::server::transport::WebSocketTransport;

        let options = options.unwrap_or_default();

        // Get timeout (default 30 seconds, 0 = no timeout)
        let timeout_ms = options.timeout.unwrap_or(30000.0);

        // 1. Connect to WebSocket
        tracing::debug!("Connecting to remote browser at {}", ws_endpoint);

        let connect_future = WebSocketTransport::connect(ws_endpoint, options.headers);
        let (transport, message_rx) = if timeout_ms > 0.0 {
            let timeout = std::time::Duration::from_millis(timeout_ms as u64);
            tokio::time::timeout(timeout, connect_future)
                .await
                .map_err(|_| {
                    crate::error::Error::Timeout(format!(
                        "Connection to {} timed out after {} ms",
                        ws_endpoint, timeout_ms
                    ))
                })??
        } else {
            connect_future.await?
        };
        let (sender, receiver) = transport.into_parts();

        // 2. Create Connection
        let connection = Arc::new(Connection::new(sender, receiver, message_rx));

        // 3. Start message loop
        let conn_for_loop = Arc::clone(&connection);
        tokio::spawn(async move {
            conn_for_loop.run().await;
        });

        // 4. Initialize Playwright
        // This exchanges the "initialize" message and returns the root Playwright object
        let playwright_obj = connection.initialize_playwright().await?;

        // 5. Get pre-launched browser from initializer
        // The server sends a "preLaunchedBrowser" field in the Playwright object's initializer
        let initializer = playwright_obj.initializer();

        let browser_guid = initializer["preLaunchedBrowser"]["guid"]
            .as_str()
            .ok_or_else(|| {
                 crate::error::Error::ProtocolError(
                     "Remote server did not return a pre-launched browser. Ensure server was launched in server mode.".to_string()
                 )
            })?;

        // 6. Get the existing Browser object and downcast
        let browser: Browser = connection.get_typed::<Browser>(browser_guid).await?;

        Ok(browser)
    }

    /// Connects to a browser over the Chrome DevTools Protocol.
    ///
    /// This method is only supported for Chromium. It connects to an existing Chrome
    /// instance that exposes a CDP endpoint (e.g., `--remote-debugging-port`), or to
    /// CDP-compatible services like browserless.
    ///
    /// Unlike `connect()`, which uses Playwright's proprietary WebSocket protocol,
    /// this method connects directly via CDP. The Playwright server manages the CDP
    /// connection internally.
    ///
    /// # Arguments
    /// * `endpoint_url` - A CDP endpoint URL (e.g., `http://localhost:9222` or
    ///   `ws://localhost:9222/devtools/browser/...`)
    /// * `options` - Optional connection options.
    ///
    /// # Errors
    /// Returns error if:
    /// - Called on a non-Chromium browser type
    /// - Connection to the CDP endpoint fails
    /// - Connection timeout
    ///
    /// See: <https://playwright.dev/docs/api/class-browsertype#browser-type-connect-over-cdp>
    pub async fn connect_over_cdp(
        &self,
        endpoint_url: &str,
        options: Option<ConnectOverCdpOptions>,
    ) -> Result<Browser> {
        // connect_over_cdp is Chromium-only
        if self.name() != "chromium" {
            return Err(crate::error::Error::ProtocolError(
                "Connecting over CDP is only supported in Chromium.".to_string(),
            ));
        }

        let options = options.unwrap_or_default();

        // Convert headers from HashMap to array of {name, value} objects
        let headers_array = options.headers.map(|h| {
            h.into_iter()
                .map(|(name, value)| HeaderEntry { name, value })
                .collect::<Vec<_>>()
        });

        let params = ConnectOverCdpParams {
            endpoint_url: endpoint_url.to_string(),
            headers: headers_array,
            slow_mo: options.slow_mo,
            timeout: options.timeout.unwrap_or(crate::DEFAULT_TIMEOUT_MS),
        };

        // Send connectOverCDP RPC to the local Playwright server
        let response: ConnectOverCdpResponse =
            self.base.channel().send("connectOverCDP", params).await?;

        // Get browser object from registry and downcast
        let browser: Browser = self
            .connection()
            .get_typed::<Browser>(&response.browser.guid)
            .await?;

        Ok(browser)
    }
}

/// Response from BrowserType.launch() protocol call
#[derive(Debug, Deserialize, Serialize)]
struct LaunchResponse {
    browser: BrowserRef,
}

/// Response from BrowserType.launchPersistentContext() protocol call
#[derive(Debug, Deserialize, Serialize)]
struct LaunchPersistentContextResponse {
    context: ContextRef,
}

/// Reference to a Browser object in the protocol
#[derive(Debug, Deserialize, Serialize)]
struct BrowserRef {
    #[serde(
        serialize_with = "crate::server::connection::serialize_arc_str",
        deserialize_with = "crate::server::connection::deserialize_arc_str"
    )]
    guid: Arc<str>,
}

/// Reference to a BrowserContext object in the protocol
#[derive(Debug, Deserialize, Serialize)]
struct ContextRef {
    #[serde(
        serialize_with = "crate::server::connection::serialize_arc_str",
        deserialize_with = "crate::server::connection::deserialize_arc_str"
    )]
    guid: Arc<str>,
}

/// Parameters for BrowserType.connectOverCDP() protocol call
#[derive(Debug, Serialize)]
struct ConnectOverCdpParams {
    #[serde(rename = "endpointURL")]
    endpoint_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<Vec<HeaderEntry>>,
    #[serde(rename = "slowMo", skip_serializing_if = "Option::is_none")]
    slow_mo: Option<f64>,
    timeout: f64,
}

/// A single HTTP header as {name, value} for the connectOverCDP protocol
#[derive(Debug, Serialize)]
struct HeaderEntry {
    name: String,
    value: String,
}

/// Response from BrowserType.connectOverCDP() protocol call
#[derive(Debug, Deserialize)]
struct ConnectOverCdpResponse {
    browser: BrowserRef,
    #[serde(rename = "defaultContext")]
    #[allow(dead_code)]
    default_context: Option<ContextRef>,
}

impl ChannelOwner for BrowserType {
    fn guid(&self) -> &str {
        self.base.guid()
    }

    fn type_name(&self) -> &str {
        self.base.type_name()
    }

    fn parent(&self) -> Option<Arc<dyn ChannelOwner>> {
        self.base.parent()
    }

    fn connection(&self) -> Arc<dyn ConnectionLike> {
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
        self.base.on_event(method, params)
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for BrowserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserType")
            .field("guid", &self.guid())
            .field("name", &self.name)
            .field("executable_path", &self.executable_path)
            .finish()
    }
}

// Note: BrowserType testing is done via integration tests since it requires:
// - A real Connection with object registry
// - Protocol messages from the server
// See: crates/playwright-core/tests/connection_integration.rs
