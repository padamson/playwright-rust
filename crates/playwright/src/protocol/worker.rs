// Worker — Web Worker and Service Worker

use crate::error::Result;
use crate::protocol::evaluate_conversion::{parse_result, serialize_argument, serialize_null};
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::server::connection::ConnectionExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

/// Worker represents a Web Worker or Service Worker.
///
/// Workers are created by the page using the `Worker` constructor or by browsers
/// for registered service workers. They run JS in an isolated global scope.
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
///     let page = browser.new_page().await?;
///
///     page.on_worker(|worker| {
///         println!("Worker created: {}", worker.url());
///         Box::pin(async move { Ok(()) })
///     }).await?;
///
///     browser.close().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-worker>
#[derive(Clone)]
pub struct Worker {
    base: ChannelOwnerImpl,
    /// The URL of this worker (from initializer)
    url: String,
}

impl Worker {
    /// Creates a new Worker from protocol initialization.
    ///
    /// Called by the object factory when the server sends a `__create__` message
    /// for a Worker object.
    pub fn new(
        parent: Arc<dyn ChannelOwner>,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        let url = initializer
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent),
            type_name,
            guid,
            initializer,
        );

        Ok(Self { base, url })
    }

    /// Returns the URL of this worker.
    ///
    /// See: <https://playwright.dev/docs/api/class-worker#worker-url>
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the channel for sending protocol messages.
    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    /// Evaluates a JavaScript expression in the worker context.
    ///
    /// The expression is evaluated in the worker's global scope. Returns the
    /// JSON-serializable result deserialized into type `R`.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript expression or function body
    /// * `arg` - Optional argument to pass to the expression
    ///
    /// # Errors
    ///
    /// Returns an error if the JavaScript expression throws.
    ///
    /// See: <https://playwright.dev/docs/api/class-worker#worker-evaluate>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn evaluate<R, T>(&self, expression: &str, arg: Option<T>) -> Result<R>
    where
        R: DeserializeOwned,
        T: Serialize,
    {
        let serialized_arg = match arg {
            Some(a) => serialize_argument(&a),
            None => serialize_null(),
        };

        let params = serde_json::json!({
            "expression": expression,
            "arg": serialized_arg
        });

        #[derive(Deserialize)]
        struct EvaluateResult {
            value: Value,
        }

        let result: EvaluateResult = self.channel().send("evaluateExpression", params).await?;
        let parsed = parse_result(&result.value);

        serde_json::from_value(parsed).map_err(|e| {
            crate::error::Error::ProtocolError(format!("Failed to deserialize result: {}", e))
        })
    }

    /// Evaluates a JavaScript expression in the worker context, returning a JSHandle.
    ///
    /// Unlike [`evaluate`](Worker::evaluate) which deserializes the result,
    /// this returns a live handle to the in-worker JavaScript object.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript expression or function body
    ///
    /// See: <https://playwright.dev/docs/api/class-worker#worker-evaluate-handle>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn evaluate_handle(
        &self,
        expression: &str,
    ) -> Result<Arc<crate::protocol::JSHandle>> {
        let trimmed = expression.trim();
        let is_function = trimmed.starts_with('(')
            || trimmed.starts_with("function")
            || trimmed.starts_with("async ");

        let params = serde_json::json!({
            "expression": expression,
            "isFunction": is_function,
            "arg": {"value": {"v": "undefined"}, "handles": []}
        });

        #[derive(Deserialize)]
        struct HandleRef {
            guid: String,
        }
        #[derive(Deserialize)]
        struct EvaluateHandleResponse {
            handle: HandleRef,
        }

        let response: EvaluateHandleResponse = self
            .channel()
            .send("evaluateExpressionHandle", params)
            .await?;

        let guid = &response.handle.guid;
        let connection = self.base.connection();
        let mut attempts = 0;
        let max_attempts = 20;

        let handle = loop {
            match connection
                .get_typed::<crate::protocol::JSHandle>(guid)
                .await
            {
                Ok(h) => break h,
                Err(_) if attempts < max_attempts => {
                    attempts += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(e) => return Err(e),
            }
        };

        Ok(Arc::new(handle))
    }
}

impl ChannelOwner for Worker {
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

    fn on_event(&self, _method: &str, _params: Value) {
        // Worker emits a "close" event when terminated; no internal state needs updating.
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for Worker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Worker")
            .field("guid", &self.guid())
            .field("url", &self.url)
            .finish()
    }
}
