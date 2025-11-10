// BrowserContext protocol object
//
// Represents an isolated browser context (session) within a browser instance.
// Multiple contexts can exist in a single browser, each with its own cookies,
// cache, and local storage.

use crate::channel::Channel;
use crate::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::error::Result;
use crate::protocol::Page;
use serde::Deserialize;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// BrowserContext represents an isolated browser session.
///
/// Contexts are isolated environments within a browser instance. Each context
/// has its own cookies, cache, and local storage, enabling independent sessions
/// without interference.
///
/// # Example
///
/// ```no_run
/// # use playwright_core::protocol::Playwright;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let playwright = Playwright::launch().await?;
/// let browser = playwright.chromium().launch().await?;
///
/// // Create an isolated context
/// let context = browser.new_context().await?;
///
/// // Context has its own session
/// // ... create pages in context ...
///
/// // Cleanup
/// context.close().await?;
/// browser.close().await?;
/// # Ok(())
/// # }
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

    /// Creates a new page in this browser context.
    ///
    /// Pages are isolated tabs/windows within a context. Each page starts
    /// at "about:blank" and can be navigated independently.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// let context = browser.new_context().await?;
    ///
    /// // Create pages in the context
    /// let page1 = context.new_page().await?;
    /// let page2 = context.new_page().await?;
    ///
    /// // Each page is isolated
    /// assert_eq!(page1.url(), "about:blank");
    /// assert_eq!(page2.url(), "about:blank");
    ///
    /// // Cleanup
    /// page1.close().await?;
    /// page2.close().await?;
    /// context.close().await?;
    /// # Ok(())
    /// # }
    /// ```
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
            #[serde(deserialize_with = "crate::connection::deserialize_arc_str")]
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
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_core::protocol::Playwright;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let playwright = Playwright::launch().await?;
    /// # let browser = playwright.chromium().launch().await?;
    /// let context = browser.new_context().await?;
    ///
    /// // Do work with context...
    ///
    /// // Close context when done
    /// context.close().await?;
    /// # Ok(())
    /// # }
    /// ```
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

    fn connection(&self) -> Arc<dyn crate::connection::ConnectionLike> {
        self.base.connection()
    }

    fn initializer(&self) -> &Value {
        self.base.initializer()
    }

    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    fn dispose(&self, reason: crate::channel_owner::DisposeReason) {
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
