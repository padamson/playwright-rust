use super::{
    Page, RouteHandlerEntry, RouteHandlerFuture, WebSocketRouteHandlerFuture, WsRouteHandlerEntry,
};
use crate::error::{Error, Result};
use crate::protocol::Route;
use crate::server::channel_owner::ChannelOwner;
use std::future::Future;
use std::sync::Arc;

/// Network interception: routes, HAR replay, WebSocket routes and extra headers.
///
/// ```no_run
/// # use playwright_rs::Playwright;
/// # use playwright_rs::protocol::FulfillOptions;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let page = Playwright::launch().await?.chromium().launch().await?.new_page().await?;
/// page.route("**/api/health", |route| async move {
///     let ok = FulfillOptions::builder().status(200).body(b"ok".to_vec()).build();
///     route.fulfill(ok).await
/// })
/// .await?;
/// page.route("**/*.png", |route| async move { route.abort(None).await }).await?;
///
/// page.goto("https://example.com", None).await?;
/// page.unroute_all(None).await?;
/// # Ok(())
/// # }
/// ```
impl Page {
    /// Registers a route handler for network interception.
    ///
    /// When a request matches the specified pattern, the handler will be called
    /// with a Route object that can abort, continue, or fulfill the request.
    ///
    /// # Arguments
    ///
    /// * `pattern` - URL pattern to match (supports glob patterns like "**/*.png")
    /// * `handler` - Async closure that handles the route
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-route>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), url = %pattern))]
    pub async fn route<F, Fut>(&self, pattern: &str, handler: F) -> Result<()>
    where
        F: Fn(Route) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        // 1. Wrap handler in Arc with type erasure
        let handler =
            Arc::new(move |route: Route| -> RouteHandlerFuture { Box::pin(handler(route)) });

        // 2. Store in handlers list
        self.route_handlers.lock().unwrap().push(RouteHandlerEntry {
            pattern: pattern.to_string(),
            handler,
        });

        // 3. Enable network interception via protocol
        self.enable_network_interception().await?;

        Ok(())
    }

    /// Fulfills matching requests from an in-process tower `Service`, such as
    /// an axum `Router` or a tower-http `ServeDir`, with no socket.
    ///
    /// Each request whose URL matches `pattern` is rebuilt as an
    /// `http::Request`, handed to a clone of `service`, and fulfilled with the
    /// response. The [`route_service`](crate::protocol::route_service) module
    /// documents what the service sees, the limits of route interception
    /// compared with a real listener, and how to wait on a wasm frontend.
    ///
    /// # Arguments
    ///
    /// * `pattern` - URL pattern to match (supports glob patterns like `"https://app.example/**"`)
    /// * `service` - Any [`RouteService`](crate::protocol::route_service::RouteService):
    ///   an axum `Router`, a tower-http `ServeDir`, a `tower::service_fn`; cloned per request
    ///
    /// # Errors
    ///
    /// Returns an error if network interception cannot be enabled. A service
    /// that fails at request time aborts that request and logs the error; it
    /// does not surface here.
    ///
    /// See: <https://playwright.dev/docs/mock>
    #[cfg(feature = "route-service")]
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), url = %pattern))]
    pub async fn route_service<S: crate::protocol::route_service::RouteService>(
        &self,
        pattern: &str,
        service: S,
    ) -> Result<()> {
        // Captured here, a hop away, so the service's view of the browser
        // (its engine, its cookie jar) does not depend on walking each
        // request's object chain.
        let context = self.context().ok();
        self.route(pattern, move |route| {
            crate::protocol::route_service::fulfill_from_service(
                route,
                service.clone(),
                context.clone(),
            )
        })
        .await
    }

    /// Updates network interception patterns for this page
    async fn enable_network_interception(&self) -> Result<()> {
        // Collect all patterns from registered handlers
        // Each pattern must be an object with "glob" field
        let patterns: Vec<serde_json::Value> = self
            .route_handlers
            .lock()
            .unwrap()
            .iter()
            .map(|entry| serde_json::json!({ "glob": entry.pattern }))
            .collect();

        // Send protocol command to update network interception patterns
        // Follows playwright-python's approach
        self.channel()
            .send_no_result(
                "setNetworkInterceptionPatterns",
                serde_json::json!({
                    "patterns": patterns
                }),
            )
            .await
    }

    /// Removes route handler(s) matching the given URL pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - URL pattern to remove handlers for
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-unroute>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), url = %pattern))]
    pub async fn unroute(&self, pattern: &str) -> Result<()> {
        self.route_handlers
            .lock()
            .unwrap()
            .retain(|entry| entry.pattern != pattern);
        self.enable_network_interception().await
    }

    /// Removes all registered route handlers.
    ///
    /// # Arguments
    ///
    /// * `behavior` - Optional behavior for in-flight handlers
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-unroute-all>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn unroute_all(
        &self,
        _behavior: Option<crate::protocol::route::UnrouteBehavior>,
    ) -> Result<()> {
        self.route_handlers.lock().unwrap().clear();
        self.enable_network_interception().await
    }

    /// Replays network requests from a HAR file recorded previously.
    ///
    /// Requests matching `options.url` (or all requests if omitted) will be
    /// served from the archive instead of hitting the network.  Unmatched
    /// requests are either aborted or passed through depending on
    /// `options.not_found` (`"abort"` is the default).
    ///
    /// # Arguments
    ///
    /// * `har_path` - Path to the `.har` file on disk
    /// * `options` - Optional settings (url filter, not_found policy, update mode)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - `har_path` does not exist or cannot be read by the Playwright server
    /// - The Playwright server fails to open the archive
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-route-from-har>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn route_from_har(
        &self,
        har_path: &str,
        options: impl Into<Option<RouteFromHarOptions>>,
    ) -> Result<()> {
        let options = options.into();
        let opts = options.unwrap_or_default();
        let not_found = opts.not_found.unwrap_or_else(|| "abort".to_string());
        let url_filter = opts.url.clone();

        // Resolve to an absolute path so the Playwright server can open it
        // regardless of its working directory.
        let abs_path = std::path::Path::new(har_path).canonicalize().map_err(|e| {
            Error::InvalidPath(format!(
                "route_from_har: cannot resolve '{}': {}",
                har_path, e
            ))
        })?;
        let abs_str = abs_path.to_string_lossy().into_owned();

        // Locate LocalUtils in the connection object registry by type name.
        // The Playwright server registers it with a guid like "localUtils@1"
        // so we scan all objects for the one with type_name "LocalUtils".
        let connection = self.connection();
        let local_utils = {
            let all = connection.all_objects_sync();
            all.into_iter()
                .find(|o| o.type_name() == "LocalUtils")
                .and_then(|o| {
                    o.as_any()
                        .downcast_ref::<crate::protocol::LocalUtils>()
                        .cloned()
                })
                .ok_or_else(|| {
                    Error::ProtocolError(
                        "route_from_har: LocalUtils not found in connection registry".to_string(),
                    )
                })?
        };

        // Open the HAR archive on the server side.
        let har_id = local_utils.har_open(&abs_str).await?;

        // Determine the URL pattern to intercept.
        let pattern = url_filter.clone().unwrap_or_else(|| "**/*".to_string());

        // Register a route handler that performs HAR lookup for each request.
        let har_id_clone = har_id.clone();
        let local_utils_clone = local_utils.clone();
        let not_found_clone = not_found.clone();

        self.route(&pattern, move |route| {
            let har_id = har_id_clone.clone();
            let local_utils = local_utils_clone.clone();
            let not_found = not_found_clone.clone();
            async move {
                let request = route.request();
                let req_url = request.url().to_string();
                let req_method = request.method().to_string();

                // Build headers array as [{name, value}]
                let headers = crate::protocol::route_params::header_array(request.header_pairs());

                let lookup = local_utils
                    .har_lookup(
                        &har_id,
                        &req_url,
                        &req_method,
                        headers,
                        None,
                        request.is_navigation_request(),
                    )
                    .await;

                match lookup {
                    Err(e) => {
                        tracing::warn!("har_lookup error for {}: {}", req_url, e);
                        route.continue_(None).await
                    }
                    Ok(result) => match result.action.as_str() {
                        "redirect" => {
                            let redirect_url = result.redirect_url.unwrap_or_default();
                            let opts = crate::protocol::ContinueOptions::builder()
                                .url(redirect_url)
                                .build();
                            route.continue_(Some(opts)).await
                        }
                        "fulfill" => {
                            route
                                .fulfill(Some(crate::protocol::route_params::har_fulfill_options(
                                    result.status,
                                    result.body.as_deref(),
                                    result.headers.as_deref(),
                                )))
                                .await
                        }
                        _ => {
                            // "fallback" or "error" or unknown
                            if not_found == "fallback" {
                                route.fallback(None).await
                            } else {
                                route.abort(None).await
                            }
                        }
                    },
                }
            }
        })
        .await
    }

    /// Intercepts WebSocket connections matching the given URL pattern.
    ///
    /// When a WebSocket connection from the page matches `url`, the `handler`
    /// is called with a [`WebSocketRoute`](crate::protocol::WebSocketRoute) object.
    /// The handler must call [`connect_to_server`](crate::protocol::WebSocketRoute::connect_to_server)
    /// to forward the connection to the real server, or
    /// [`close`](crate::protocol::WebSocketRoute::close) to terminate it.
    ///
    /// # Arguments
    ///
    /// * `url` — URL glob pattern (e.g. `"ws://**"` or `"wss://example.com/ws"`).
    /// * `handler` — Async closure receiving a `WebSocketRoute`.
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call to enable interception fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-route-web-socket>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), url = %url))]
    pub async fn route_web_socket<F, Fut>(&self, url: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::WebSocketRoute) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |route: crate::protocol::WebSocketRoute| -> WebSocketRouteHandlerFuture {
                Box::pin(handler(route))
            },
        );

        self.ws_route_handlers
            .lock()
            .unwrap()
            .push(WsRouteHandlerEntry {
                pattern: url.to_string(),
                handler,
            });

        self.enable_ws_interception().await
    }

    /// Updates WebSocket interception patterns for this page.
    async fn enable_ws_interception(&self) -> Result<()> {
        let patterns: Vec<serde_json::Value> = self
            .ws_route_handlers
            .lock()
            .unwrap()
            .iter()
            .map(|entry| serde_json::json!({ "glob": entry.pattern }))
            .collect();

        self.channel()
            .send_no_result(
                "setWebSocketInterceptionPatterns",
                serde_json::json!({ "patterns": patterns }),
            )
            .await
    }

    /// Handles a route event from the protocol
    ///
    /// Called by on_event when a "route" event is received.
    /// Supports handler chaining via `route.fallback()` — if a handler calls
    /// `fallback()` instead of `continue_()`, `abort()`, or `fulfill()`, the
    /// next matching handler in the chain is tried.
    pub(super) async fn on_route_event(&self, route: Route) {
        let handlers = self.route_handlers.lock().unwrap().clone();
        let url = route.request().url().to_string();

        // Find matching handler (last registered wins, with fallback chaining)
        for entry in handlers.iter().rev() {
            if crate::protocol::route::matches_pattern(&entry.pattern, &url) {
                let handler = entry.handler.clone();
                if let Err(e) = handler(route.clone()).await {
                    tracing::warn!("Route handler error: {}", e);
                    // A handler that failed before reaching a route command
                    // leaves the request pending; abort it so the browser
                    // sees a failed request instead of waiting out its timeout.
                    if !route.was_handled()
                        && let Err(abort_error) = route.abort(Some("failed")).await
                    {
                        tracing::warn!("aborting the unhandled route failed too: {}", abort_error);
                    }
                    break;
                }
                // If handler called fallback(), try the next matching handler
                if !route.was_handled() {
                    continue;
                }
                break;
            }
        }
    }

    /// Sets extra HTTP headers that will be sent with every request from this page.
    ///
    /// These headers are sent in addition to headers set on the browser context via
    /// `BrowserContext::set_extra_http_headers()`. Page-level headers take precedence
    /// over context-level headers when names conflict.
    ///
    /// # Arguments
    ///
    /// * `headers` - Map of header names to values.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-set-extra-http-headers>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn set_extra_http_headers(
        &self,
        headers: std::collections::HashMap<String, String>,
    ) -> Result<()> {
        // Playwright protocol expects an array of {name, value} objects
        // This RPC is sent on the Page channel (not the Frame channel)
        let headers_array = crate::protocol::route_params::header_array(headers);
        self.channel()
            .send_no_result(
                "setExtraHTTPHeaders",
                serde_json::json!({ "headers": headers_array }),
            )
            .await
    }
}

/// Options for `page.route_from_har()` and `context.route_from_har()`.
///
/// See: <https://playwright.dev/docs/api/class-page#page-route-from-har>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RouteFromHarOptions {
    /// URL glob pattern — only requests matching this pattern are served from
    /// the HAR file.  All requests are intercepted when omitted.
    pub url: Option<String>,

    /// Policy for requests not found in the HAR file.
    ///
    /// - `"abort"` (default) — terminate the request with a network error.
    /// - `"fallback"` — pass the request through to the next handler (or network).
    pub not_found: Option<String>,

    /// When `true`, record new network activity into the HAR file instead of
    /// replaying existing entries.  Defaults to `false`.
    pub update: Option<bool>,

    /// Content storage strategy used when `update` is `true`.
    ///
    /// - `"embed"` (default) — inline base64-encoded content in the HAR.
    /// - `"attach"` — store content as separate files alongside the HAR.
    pub update_content: Option<String>,

    /// Recording detail level used when `update` is `true`.
    ///
    /// - `"minimal"` (default) — omit timing, cookies, and security info.
    /// - `"full"` — record everything.
    pub update_mode: Option<String>,
}

impl RouteFromHarOptions {
    /// Only serve requests matching this URL glob from the HAR.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
    /// Behavior for requests not found in the HAR ("abort" or "fallback").
    pub fn not_found(mut self, not_found: impl Into<String>) -> Self {
        self.not_found = Some(not_found.into());
        self
    }
    /// Record new entries into the HAR instead of serving from it.
    pub fn update(mut self, update: bool) -> Self {
        self.update = Some(update);
        self
    }

    /// What to record for response bodies when updating: `"embed"`,
    /// `"attach"` or `"omit"`.
    pub fn update_content(mut self, update_content: impl Into<String>) -> Self {
        self.update_content = Some(update_content.into());
        self
    }

    /// How much of each request to record when updating: `"full"` or
    /// `"minimal"`.
    pub fn update_mode(mut self, update_mode: impl Into<String>) -> Self {
        self.update_mode = Some(update_mode.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_from_har_setters_write_their_own_fields() {
        let opts = RouteFromHarOptions::default()
            .url("**/api/**")
            .not_found("abort")
            .update(true)
            .update_content("attach")
            .update_mode("full");
        assert_eq!(opts.url.as_deref(), Some("**/api/**"));
        assert_eq!(opts.not_found.as_deref(), Some("abort"));
        assert_eq!(opts.update, Some(true));
        assert_eq!(opts.update_content.as_deref(), Some("attach"));
        assert_eq!(opts.update_mode.as_deref(), Some("full"));
    }
}
