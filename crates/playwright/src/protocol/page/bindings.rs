use super::{
    LocatorHandlerEntry, LocatorHandlerFn, LocatorHandlerFuture, Page, PageBindingCallback,
    PageBindingCallbackFuture,
};
use crate::error::{Error, Result};
use crate::server::channel_owner::ChannelOwner;
use serde_json::Value;
use std::future::Future;
use std::sync::Arc;

/// Exposed functions and bindings, init scripts and locator handlers.
impl Page {
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
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), name = %name))]
    pub async fn expose_function<F, Fut>(&self, name: &str, callback: F) -> Result<()>
    where
        F: Fn(Vec<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = serde_json::Value> + Send + 'static,
    {
        self.expose_binding_internal(name, false, callback).await
    }

    /// Exposes a Rust function to this page as `window[name]` in JavaScript.
    ///
    /// Currently identical to [`expose_function`](Self::expose_function):
    /// arguments arrive as plain serialized values. Upstream Playwright's
    /// `exposeBinding` can additionally hand the callback a source
    /// (page/frame) descriptor, which this crate does not surface yet.
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
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), name = %name))]
    pub async fn expose_binding<F, Fut>(&self, name: &str, callback: F) -> Result<()>
    where
        F: Fn(Vec<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = serde_json::Value> + Send + 'static,
    {
        self.expose_binding_internal(name, false, callback).await
    }

    /// Internal implementation shared by page-level expose_function and expose_binding.
    pub(super) async fn expose_binding_internal<F, Fut>(
        &self,
        name: &str,
        no_global: bool,
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

        // Tell the Playwright server to register the binding. `noGlobal`
        // suppresses the `window[name]` injection; it is how
        // `evaluate_with_callback` passes a function the page can only reach
        // through the bindings controller, never off `window`.
        let mut params = serde_json::json!({ "name": name });
        if no_global {
            params["noGlobal"] = serde_json::json!(true);
        }
        self.channel().send_no_result("exposeBinding", params).await
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
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn add_locator_handler<F, Fut>(
        &self,
        locator: &crate::protocol::Locator,
        handler: F,
        options: impl Into<Option<AddLocatorHandlerOptions>>,
    ) -> Result<()>
    where
        F: Fn(crate::protocol::Locator) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let options = options.into();
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
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
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
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn add_init_script(&self, script: &str) -> Result<()> {
        self.channel()
            .send_no_result("addInitScript", serde_json::json!({ "source": script }))
            .await
    }
}

/// Options for `page.add_locator_handler()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-add-locator-handler>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
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
