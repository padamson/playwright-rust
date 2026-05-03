//! Debugger — programmatic control of the Playwright Inspector "PAUSED" overlay.
//!
//! Available on every [`BrowserContext`](crate::protocol::BrowserContext) via
//! [`debugger()`](crate::protocol::BrowserContext::debugger). Used by IDE
//! integrations and inspector-style tools to pause execution at the next
//! action call, then resume / step / run-to-location under programmatic
//! control. Distinct from the MCP / agent codegen path.
//!
//! # Example
//!
//! ```ignore
//! use playwright_rs::Playwright;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let pw = Playwright::launch().await?;
//!     let browser = pw.chromium().launch().await?;
//!     let context = browser.new_context().await?;
//!     let dbg = context.debugger().await?;
//!
//!     // Ask Playwright to pause before the next action runs.
//!     dbg.request_pause().await?;
//!
//!     // ... your IDE / tool decides when to step the user through ...
//!
//!     dbg.resume().await?;
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! See: <https://playwright.dev/docs/api/class-debugger>

use crate::error::Result;
use crate::server::channel::Channel;
use crate::server::channel_owner::{
    ChannelOwner, ChannelOwnerImpl, DisposeReason, ParentOrConnection,
};
use crate::server::connection::ConnectionLike;
use parking_lot::Mutex;
use serde_json::Value;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type PausedHandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;
type PausedHandler =
    Arc<dyn Fn(Option<PausedDetails>) -> PausedHandlerFuture + Send + Sync + 'static>;

/// Details of the currently paused execution, surfaced via the
/// `pausedStateChanged` event when the inspector overlay is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PausedDetails {
    /// Source location the engine paused at, when available.
    pub location: PausedLocation,
    /// Title shown on the overlay (typically the action name).
    pub title: String,
    /// Stack trace as a string, when the server provides one.
    pub stack: Option<String>,
}

/// Source location for [`PausedDetails`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PausedLocation {
    pub file: String,
    pub line: Option<i64>,
    pub column: Option<i64>,
}

/// Programmatic interface to Playwright Inspector's pause / resume /
/// step controls. See the module docs.
#[derive(Clone)]
pub struct Debugger {
    base: ChannelOwnerImpl,
    paused_details: Arc<Mutex<Option<PausedDetails>>>,
    paused_handlers: Arc<Mutex<Vec<PausedHandler>>>,
}

impl Debugger {
    pub fn new(
        parent: ParentOrConnection,
        type_name: String,
        guid: Arc<str>,
        initializer: Value,
    ) -> Result<Self> {
        Ok(Self {
            base: ChannelOwnerImpl::new(parent, type_name, guid, initializer),
            paused_details: Arc::new(Mutex::new(None)),
            paused_handlers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Asks Playwright to pause before the next action runs. The pause
    /// is surfaced via the `pausedStateChanged` event (register a
    /// handler with [`Debugger::on_paused_state_changed`]).
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn request_pause(&self) -> Result<()> {
        self.channel()
            .send_no_result("requestPause", serde_json::json!({}))
            .await
    }

    /// Resume execution from a paused state.
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn resume(&self) -> Result<()> {
        self.channel()
            .send_no_result("resume", serde_json::json!({}))
            .await
    }

    /// Step to the next action call, then pause again.
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn next(&self) -> Result<()> {
        self.channel()
            .send_no_result("next", serde_json::json!({}))
            .await
    }

    /// Run to a specific source location, then pause.
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn run_to(&self, location: PausedLocation) -> Result<()> {
        let mut loc = serde_json::json!({ "file": location.file });
        if let Some(line) = location.line {
            loc["line"] = serde_json::json!(line);
        }
        if let Some(column) = location.column {
            loc["column"] = serde_json::json!(column);
        }
        self.channel()
            .send_no_result("runTo", serde_json::json!({ "location": loc }))
            .await
    }

    /// Returns the current paused-state details, or `None` if execution
    /// is not currently paused. Updated as `pausedStateChanged` events
    /// arrive.
    pub fn paused_details(&self) -> Option<PausedDetails> {
        self.paused_details.lock().clone()
    }

    /// Convenience: `paused_details().is_some()`.
    pub fn is_paused(&self) -> bool {
        self.paused_details.lock().is_some()
    }

    /// Register a handler for the `pausedStateChanged` event. The
    /// handler receives `Some(details)` when execution becomes paused
    /// and `None` when it resumes.
    pub fn on_paused_state_changed<F, Fut>(&self, handler: F)
    where
        F: Fn(Option<PausedDetails>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let h: PausedHandler = Arc::new(move |d| -> PausedHandlerFuture { Box::pin(handler(d)) });
        self.paused_handlers.lock().push(h);
    }
}

fn parse_paused_details(params: &Value) -> Option<PausedDetails> {
    let pd = params.get("pausedDetails")?;
    if pd.is_null() {
        return None;
    }
    let location = pd.get("location")?;
    let file = location.get("file")?.as_str()?.to_string();
    let line = location.get("line").and_then(|v| v.as_i64());
    let column = location.get("column").and_then(|v| v.as_i64());
    let title = pd.get("title")?.as_str()?.to_string();
    let stack = pd.get("stack").and_then(|v| v.as_str()).map(String::from);
    Some(PausedDetails {
        location: PausedLocation { file, line, column },
        title,
        stack,
    })
}

impl ChannelOwner for Debugger {
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
    fn dispose(&self, reason: DisposeReason) {
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
        if method == "pausedStateChanged" {
            let details = parse_paused_details(&params);
            *self.paused_details.lock() = details.clone();
            let handlers = self.paused_handlers.lock().clone();
            for h in handlers {
                let d = details.clone();
                tokio::spawn(async move {
                    if let Err(e) = h(d).await {
                        tracing::warn!("Debugger paused-state handler error: {}", e);
                    }
                });
            }
        }
        self.base.on_event(method, params);
    }

    fn was_collected(&self) -> bool {
        self.base.was_collected()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl std::fmt::Debug for Debugger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Debugger")
            .field("guid", &self.guid())
            .field("paused", &self.is_paused())
            .finish()
    }
}
