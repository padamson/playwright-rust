use super::{Page, Response};
use crate::error::{Error, Result};
use crate::server::channel_owner::ChannelOwner;
use serde::Deserialize;
use std::sync::Arc;

/// Navigation: `goto`, history, load state and URL fragments.
impl Page {
    /// Navigates to the specified URL.
    ///
    /// Returns `None` when navigating to URLs that don't produce responses (e.g., data URLs,
    /// about:blank). This matches Playwright's behavior across all language bindings.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to navigate to
    /// * `options` - Optional navigation options (timeout, wait_until)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - URL is invalid
    /// - Navigation timeout (default 30s)
    /// - Network error
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-goto>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid(), url = %url, status = tracing::field::Empty))]
    pub async fn goto(
        &self,
        url: &str,
        options: impl Into<Option<GotoOptions>>,
    ) -> Result<Option<Response>> {
        let options = options.into();
        // Inject the page-level navigation timeout when no explicit timeout is given
        let options = self.with_navigation_timeout(options);

        // Delegate to main frame
        let frame = self.main_frame().await.map_err(|e| match e {
            Error::TargetClosed { context, .. } => Error::TargetClosed {
                target_type: "Page".to_string(),
                context,
            },
            other => other,
        })?;

        let response = frame.goto(url, Some(options)).await.map_err(|e| match e {
            Error::TargetClosed { context, .. } => Error::TargetClosed {
                target_type: "Page".to_string(),
                context,
            },
            other => other,
        })?;

        if let Some(ref resp) = response {
            tracing::Span::current().record("status", resp.status());
        }
        Ok(response)
    }

    /// Waits for the required load state to be reached.
    ///
    /// This resolves when the page reaches a required load state, `load` by default.
    /// The navigation must have been committed when this method is called. If the current
    /// document has already reached the required state, resolves immediately.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-load-state>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn wait_for_load_state(&self, state: Option<WaitUntil>) -> Result<()> {
        let frame = self.main_frame().await?;
        frame.wait_for_load_state(state).await
    }

    /// Waits for the main frame to navigate to a URL matching the given string or glob pattern.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-url>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), url = %url))]
    pub async fn wait_for_url(
        &self,
        url: &str,
        options: impl Into<Option<GotoOptions>>,
    ) -> Result<()> {
        let options = options.into();
        let frame = self.main_frame().await?;
        frame.wait_for_url(url, options).await
    }

    /// Replace the URL fragment without firing a navigation.
    ///
    /// Wraps `history.replaceState(null, '', <pathname+search+#hash>)`.
    /// A leading `#` on `hash` is optional — both `"foo"` and `"#foo"`
    /// produce the same result.
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn set_url_fragment(&self, hash: &str) -> Result<()> {
        let normalized = if hash.starts_with('#') {
            hash.to_string()
        } else {
            format!("#{hash}")
        };
        // JSON-encode so quotes / backslashes / control chars in `hash`
        // don't break the surrounding JS string literal.
        let json = serde_json::to_string(&normalized).map_err(|e| {
            crate::error::Error::ProtocolError(format!("serialize url fragment: {e}"))
        })?;
        let js =
            format!("history.replaceState(null, '', location.pathname + location.search + {json})");
        self.evaluate_expression(&js).await
    }

    /// Clear the URL fragment without firing a navigation.
    ///
    /// Wraps `history.replaceState(null, '', <pathname+search>)`,
    /// stripping any trailing `#...`. Pairs with
    /// [`set_url_fragment`](Self::set_url_fragment).
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn clear_url_fragment(&self) -> Result<()> {
        self.evaluate_expression(
            "history.replaceState(null, '', location.pathname + location.search)",
        )
        .await
    }

    /// Reloads the current page.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional reload options (timeout, wait_until)
    ///
    /// Returns `None` when reloading pages that don't produce responses (e.g., data URLs,
    /// about:blank). This matches Playwright's behavior across all language bindings.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-reload>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn reload(
        &self,
        options: impl Into<Option<GotoOptions>>,
    ) -> Result<Option<Response>> {
        let options = options.into();
        self.navigate_history("reload", options).await
    }

    /// Navigates to the previous page in history.
    ///
    /// Returns the main resource response. In case of multiple server redirects, the navigation
    /// will resolve with the response of the last redirect. If can not go back, returns `None`.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-go-back>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn go_back(
        &self,
        options: impl Into<Option<GotoOptions>>,
    ) -> Result<Option<Response>> {
        let options = options.into();
        self.navigate_history("goBack", options).await
    }

    /// Navigates to the next page in history.
    ///
    /// Returns the main resource response. In case of multiple server redirects, the navigation
    /// will resolve with the response of the last redirect. If can not go forward, returns `None`.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-go-forward>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn go_forward(
        &self,
        options: impl Into<Option<GotoOptions>>,
    ) -> Result<Option<Response>> {
        let options = options.into();
        self.navigate_history("goForward", options).await
    }

    /// Shared implementation for reload, go_back and go_forward.
    async fn navigate_history(
        &self,
        method: &str,
        options: Option<GotoOptions>,
    ) -> Result<Option<Response>> {
        // Inject the page-level navigation timeout when no explicit timeout is given
        let opts = self.with_navigation_timeout(options);
        let mut params = serde_json::json!({});

        // opts.timeout is always Some(...) because with_navigation_timeout guarantees it
        if let Some(timeout) = opts.timeout {
            params["timeout"] = serde_json::json!(timeout.as_millis() as u64);
        } else {
            params["timeout"] = serde_json::json!(crate::DEFAULT_TIMEOUT_MS);
        }
        if let Some(wait_until) = opts.wait_until {
            params["waitUntil"] = serde_json::json!(wait_until.as_str());
        }

        #[derive(Deserialize)]
        struct NavigationResponse {
            response: Option<ResponseReference>,
        }

        #[derive(Deserialize)]
        struct ResponseReference {
            #[serde(deserialize_with = "crate::server::connection::deserialize_arc_str")]
            guid: Arc<str>,
        }

        let result: NavigationResponse = self.channel().send(method, params).await?;

        if let Some(response_ref) = result.response {
            // The Response's __create__ may arrive just after the response.
            let response_arc = self
                .connection()
                .wait_for_object(&response_ref.guid)
                .await?;

            let initializer = response_arc.initializer();

            let status = initializer["status"].as_u64().ok_or_else(|| {
                crate::error::Error::ProtocolError("Response missing status".to_string())
            })? as u16;

            let headers = initializer["headers"]
                .as_array()
                .ok_or_else(|| {
                    crate::error::Error::ProtocolError("Response missing headers".to_string())
                })?
                .iter()
                .filter_map(|h| {
                    let name = h["name"].as_str()?;
                    let value = h["value"].as_str()?;
                    Some((name.to_string(), value.to_string()))
                })
                .collect();

            let response = Response::new(
                initializer["url"]
                    .as_str()
                    .ok_or_else(|| {
                        crate::error::Error::ProtocolError("Response missing url".to_string())
                    })?
                    .to_string(),
                status,
                initializer["statusText"].as_str().unwrap_or("").to_string(),
                headers,
                Some(response_arc),
            );

            Ok(Some(response))
        } else {
            Ok(None)
        }
    }

    /// Returns GotoOptions with the navigation timeout filled in if not already set.
    ///
    /// Used internally to ensure the page's configured default navigation timeout
    /// is used when the caller does not provide an explicit timeout.
    fn with_navigation_timeout(&self, options: Option<GotoOptions>) -> GotoOptions {
        let nav_timeout = self.default_navigation_timeout_ms();
        match options {
            Some(opts) if opts.timeout.is_some() => opts,
            Some(mut opts) => {
                opts.timeout = Some(std::time::Duration::from_millis(nav_timeout as u64));
                opts
            }
            None => GotoOptions {
                timeout: Some(std::time::Duration::from_millis(nav_timeout as u64)),
                wait_until: None,
            },
        }
    }
}

/// Options for page.goto() and page.reload()
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GotoOptions {
    /// Maximum operation time in milliseconds
    pub timeout: Option<std::time::Duration>,
    /// When to consider operation succeeded
    pub wait_until: Option<WaitUntil>,
}

impl GotoOptions {
    /// Creates new GotoOptions with default values
    pub fn new() -> Self {
        Self {
            timeout: None,
            wait_until: None,
        }
    }

    /// Sets the timeout
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the wait_until option
    pub fn wait_until(mut self, wait_until: WaitUntil) -> Self {
        self.wait_until = Some(wait_until);
        self
    }
}

impl Default for GotoOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// When to consider navigation succeeded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WaitUntil {
    /// Consider operation to be finished when the `load` event is fired
    Load,
    /// Consider operation to be finished when the `DOMContentLoaded` event is fired
    DomContentLoaded,
    /// Consider operation to be finished when there are no network connections for at least 500ms
    NetworkIdle,
    /// Consider operation to be finished when the commit event is fired
    Commit,
}

impl WaitUntil {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            WaitUntil::Load => "load",
            WaitUntil::DomContentLoaded => "domcontentloaded",
            WaitUntil::NetworkIdle => "networkidle",
            WaitUntil::Commit => "commit",
        }
    }
}
