use super::Page;
use crate::error::{Error, Result};
use crate::server::channel_owner::ChannelOwner;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Script evaluation and element queries.
impl Page {
    /// Waits until `expression` returns a truthy value in the main frame,
    /// then resolves to its result as a [`JSHandle`](crate::protocol::JSHandle).
    ///
    /// Polls on `requestAnimationFrame` by default; set
    /// [`WaitForFunctionOptions::polling_interval`](crate::protocol::WaitForFunctionOptions)
    /// to poll on a timer instead, which is what you want for state the page
    /// changes off-frame.
    ///
    /// # Errors
    ///
    /// Returns an error if the expression does not become truthy within the
    /// timeout (default 30s), or if the page closes first. The timeout is
    /// enforced by the driver, so it surfaces as a protocol error carrying
    /// the driver's "Timeout ...ms exceeded" message.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-function>
    pub async fn wait_for_function(
        &self,
        expression: &str,
        options: impl Into<Option<crate::protocol::WaitForFunctionOptions>>,
    ) -> Result<std::sync::Arc<crate::protocol::JSHandle>> {
        // Resolve the page's configured default here: the frame only knows
        // it through a back-reference that a bare Frame may not have.
        let mut options = options.into().unwrap_or_default();
        if options.timeout.is_none() {
            options.timeout = Some(self.default_timeout_ms());
        }
        self.main_frame()
            .await?
            .wait_for_function(expression, options)
            .await
    }

    /// Returns the first element matching the selector, or None if not found.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-query-selector>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn query_selector(
        &self,
        selector: &str,
    ) -> Result<Option<Arc<crate::protocol::ElementHandle>>> {
        let frame = self.main_frame().await?;
        frame.query_selector(selector).await
    }

    /// Returns all elements matching the selector.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-query-selector-all>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn query_selector_all(
        &self,
        selector: &str,
    ) -> Result<Vec<Arc<crate::protocol::ElementHandle>>> {
        let frame = self.main_frame().await?;
        frame.query_selector_all(selector).await
    }

    /// Evaluates JavaScript in the page context (without return value).
    ///
    /// Executes the provided JavaScript expression or function within the page's
    /// context without returning a value.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-evaluate>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn evaluate_expression(&self, expression: &str) -> Result<()> {
        // Delegate to the main frame
        let frame = self.main_frame().await?;
        frame.frame_evaluate_expression(expression).await
    }

    /// Evaluates JavaScript in the page context with optional arguments,
    /// deserializing the result into any `DeserializeOwned` type.
    ///
    /// This is the right method whenever a test needs structured data out of
    /// the page: define a struct for the shape the JS returns and let serde do
    /// the parsing. Reaching for [`evaluate_value`](Self::evaluate_value) and
    /// string-parsing its output is never necessary.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript code to evaluate
    /// * `arg` - Optional argument to pass to the expression (must implement
    ///   Serialize). With no argument, name the type: `None::<&()>`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::Playwright;
    /// # #[derive(serde::Deserialize)]
    /// # struct Metrics { width: f64, height: f64, title: String }
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pw = Playwright::launch().await?;
    /// # let page = pw.chromium().launch().await?.new_page().await?;
    /// let metrics: Metrics = page
    ///     .evaluate(
    ///         "() => ({ width: innerWidth, height: innerHeight, title: document.title })",
    ///         None::<&()>,
    ///     )
    ///     .await?;
    /// assert!(!metrics.title.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// A runnable walkthrough (structs in and out, element geometry) lives in
    /// `examples/evaluate_typed.rs`.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-evaluate>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn evaluate<T: serde::Serialize, U: serde::de::DeserializeOwned>(
        &self,
        expression: &str,
        arg: Option<&T>,
    ) -> Result<U> {
        // Delegate to the main frame
        let frame = self.main_frame().await?;
        let result = frame.evaluate(expression, arg).await?;
        serde_json::from_value(result).map_err(Error::from)
    }

    /// Evaluates a JavaScript expression and returns the result coerced to a
    /// String.
    ///
    /// Convenient for one-off scalar probes (`document.title`, a count, a
    /// flag). For anything structured, prefer [`evaluate`](Self::evaluate),
    /// which deserializes straight into your own type; returning delimited
    /// strings from JS and splitting them in Rust is a smell that `evaluate`
    /// removes.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript code to evaluate
    ///
    /// # Returns
    ///
    /// The result converted to a String
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-evaluate>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn evaluate_value(&self, expression: &str) -> Result<String> {
        let frame = self.main_frame().await?;
        frame.frame_evaluate_expression_value(expression).await
    }

    /// Evaluates `expression` with a Rust closure bound as its argument.
    ///
    /// The closure arrives in JavaScript as an async function: calling it
    /// routes the arguments back to Rust, awaits the closure, and resolves
    /// with its return value. It is not installed on `window`; the expression
    /// receives it as its argument and decides what to do with it.
    ///
    /// This is the Rust shape of upstream's function-valued evaluate
    /// arguments. JavaScript callers pass a closure directly; Rust has no
    /// function value that can travel inside serialized data, so the
    /// callback is a dedicated parameter instead — the capability is the
    /// same, the composition point is the method signature.
    ///
    /// Each call registers a binding that lives until the page closes, which
    /// is what lets the expression stash the function and call it later
    /// (e.g. from an event listener). The cost is that the binding is never
    /// reclaimed earlier: calling this in a tight loop against a long-lived
    /// page accretes one binding per call. Register once and stash when you
    /// need repetition.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::protocol::Playwright;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pw = Playwright::launch().await?;
    /// # let browser = pw.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// let sum: i64 = page
    ///     .evaluate_with_callback("async cb => await cb(20, 22)", |args| async move {
    ///         let a = args[0].as_i64().unwrap_or(0);
    ///         let b = args[1].as_i64().unwrap_or(0);
    ///         serde_json::json!(a + b)
    ///     })
    ///     .await?;
    /// assert_eq!(sum, 42);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the expression throws, if the page or its context
    /// has closed, or if the result does not deserialize into `U`.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-evaluate>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn evaluate_with_callback<U, F, Fut>(
        &self,
        expression: &str,
        callback: F,
    ) -> Result<U>
    where
        U: serde::de::DeserializeOwned,
        F: Fn(Vec<serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = serde_json::Value> + Send + 'static,
    {
        // The `__pw_fn_` prefix matches upstream's kFunctionBindingPrefix
        // and is load-bearing: the server-to-page serializer only carries a
        // function value whose binding name starts with it, so a rename off
        // the prefix would make the argument deserialize as `undefined` in
        // the page.
        static CALLBACK_SEQ: AtomicU64 = AtomicU64::new(0);
        let name = format!(
            "__pw_fn_rs_{}",
            CALLBACK_SEQ.fetch_add(1, Ordering::Relaxed)
        );

        self.expose_binding_internal(&name, true, callback).await?;

        let frame = self.main_frame().await?;
        let result = frame.evaluate_with_fn_arg(expression, &name).await?;
        serde_json::from_value(result).map_err(Error::from)
    }
}
