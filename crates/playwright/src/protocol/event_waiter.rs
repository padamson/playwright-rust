// EventWaiter — generic one-shot event waiter with timeout support.
//
// Used by BrowserContext::expect_page(), expect_close(), etc. to implement
// the Playwright pattern of waiting for an event to fire within a timeout.
//
// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-wait-for-event>

use std::time::Duration;
use tokio::sync::oneshot;

use crate::error::{Error, Result};

/// A one-shot waiter for a Playwright event.
///
/// `EventWaiter<T>` is created by methods like `BrowserContext::expect_page()` and
/// `BrowserContext::expect_close()`. It resolves to a value of type `T` when the
/// corresponding event fires, or returns a timeout error if the event does not occur
/// within the configured timeout.
///
/// # Usage
///
/// ```ignore
/// use playwright_rs::protocol::Playwright;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let browser = playwright.chromium().launch().await?;
///     let context = browser.new_context().await?;
///
///     // Set up the waiter BEFORE the action that triggers the event
///     let waiter = context.expect_page(None).await?;
///
///     // Trigger the event
///     let _page = context.new_page().await?;
///
///     // Resolve the waiter
///     let page = waiter.wait().await?;
///     println!("New page URL: {}", page.url());
///
///     browser.close().await?;
///     Ok(())
/// }
/// ```
pub struct EventWaiter<T> {
    receiver: oneshot::Receiver<T>,
    timeout_ms: Option<f64>,
}

impl<T: Send + 'static> EventWaiter<T> {
    /// Creates a new `EventWaiter` from a oneshot receiver and optional timeout.
    ///
    /// # Arguments
    ///
    /// * `receiver` - The oneshot receiver that will receive the event value.
    /// * `timeout_ms` - Timeout in milliseconds. If `None`, defaults to 30 000 ms.
    pub(crate) fn new(receiver: oneshot::Receiver<T>, timeout_ms: Option<f64>) -> Self {
        Self {
            receiver,
            timeout_ms,
        }
    }

    /// Waits for the event to fire and returns the associated value.
    ///
    /// Returns a timeout error if the event does not fire within the configured
    /// timeout (default: 30 000 ms).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Timeout`] if the timeout elapses before the event fires.
    /// Returns [`Error::ProtocolError`] if the event source (the `BrowserContext`)
    /// is dropped before the event fires.
    pub async fn wait(self) -> Result<T> {
        let timeout_ms = self.timeout_ms.unwrap_or(30_000.0);
        let timeout_duration = Duration::from_millis(timeout_ms as u64);

        match tokio::time::timeout(timeout_duration, self.receiver).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(_)) => Err(Error::ProtocolError(
                "Event source closed before event fired".to_string(),
            )),
            Err(_) => Err(Error::Timeout(format!(
                "Timed out waiting for event after {timeout_ms}ms"
            ))),
        }
    }
}

impl<T> std::fmt::Debug for EventWaiter<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventWaiter")
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}
