use super::Page;
use crate::protocol::{Dialog, Download, Request, ResponseObject, Route, WebSocket, Worker};
use crate::server::channel::Channel;
use crate::server::channel_owner::ChannelOwner;
use crate::server::connection::{ConnectionExt, downcast_parent};
use base64::Engine;
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tracing::Instrument;

/// Protocol event dispatch from the server into the page's registries.
impl Page {
    /// Handles a download event from the protocol
    async fn on_download_event(&self, download: Download) {
        self.download.dispatch(download).await;
    }

    /// Handles a dialog event from the protocol
    async fn on_dialog_event(&self, dialog: Dialog) {
        self.dialog.dispatch(dialog).await;
    }

    async fn on_request_event(&self, request: Request) {
        self.request.dispatch(request).await;
    }

    async fn on_request_failed_event(&self, request: Request) {
        self.request_failed.dispatch(request).await;
    }

    async fn on_request_finished_event(&self, request: Request) {
        self.request_finished.dispatch(request).await;
    }

    async fn on_response_event(&self, response: ResponseObject) {
        self.response.dispatch(response).await;
    }

    /// Triggers dialog event (called by BrowserContext when dialog events arrive)
    ///
    /// Dialog events are sent to BrowserContext and forwarded to the associated Page.
    /// This method is public so BrowserContext can forward dialog events.
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn trigger_dialog_event(&self, dialog: Dialog) {
        self.on_dialog_event(dialog).await;
    }

    /// Triggers the `dialogclosed` event (called by BrowserContext when the
    /// dialog it forwarded has been answered).
    pub(crate) async fn trigger_dialog_closed_event(&self, dialog: Dialog) {
        self.dialog_closed.dispatch(dialog).await;
    }

    /// Triggers request event (called by BrowserContext when request events arrive)
    pub(crate) async fn trigger_request_event(&self, request: Request) {
        self.on_request_event(request).await;
    }

    pub(crate) async fn trigger_request_finished_event(&self, request: Request) {
        self.on_request_finished_event(request).await;
    }

    pub(crate) async fn trigger_request_failed_event(&self, request: Request) {
        self.on_request_failed_event(request).await;
    }

    /// Triggers response event (called by BrowserContext when response events arrive)
    pub(crate) async fn trigger_response_event(&self, response: ResponseObject) {
        self.on_response_event(response).await;
    }

    /// Triggers console event (called by BrowserContext when console events arrive).
    ///
    /// The BrowserContext receives all `"console"` events, constructs the
    /// [`ConsoleMessage`](crate::protocol::ConsoleMessage), dispatches to
    /// context-level handlers, then calls this method to forward to page-level handlers.
    pub(crate) async fn trigger_console_event(&self, msg: crate::protocol::ConsoleMessage) {
        self.on_console_event(msg).await;
    }

    async fn on_console_event(&self, msg: crate::protocol::ConsoleMessage) {
        // Accumulate message for console_messages() accessor
        self.console_messages_log.lock().unwrap().push(msg.clone());
        self.console.dispatch(msg).await;
    }

    /// Dispatches a FileChooser event to registered handlers and one-shot waiters.
    async fn on_filechooser_event(&self, chooser: crate::protocol::FileChooser) {
        self.filechooser.dispatch(chooser).await;
    }

    /// Triggers load event (called by Frame when loadstate "load" is added)
    pub(crate) async fn trigger_load_event(&self) {
        self.on_load_event().await;
    }

    /// Triggers pageError event (called by BrowserContext when pageError arrives)
    pub(crate) async fn trigger_pageerror_event(&self, message: String) {
        self.on_pageerror_event(message).await;
    }

    /// Triggers popup event (called by BrowserContext when a page is opened with an opener)
    pub(crate) async fn trigger_popup_event(&self, popup: Page) {
        self.on_popup_event(popup).await;
    }

    /// Triggers frameNavigated event (called by Frame when "navigated" is received)
    pub(crate) async fn trigger_framenavigated_event(&self, frame: crate::protocol::Frame) {
        self.on_framenavigated_event(frame).await;
    }

    async fn on_close_event(&self) {
        self.close.dispatch_all(()).await;
    }

    async fn on_load_event(&self) {
        self.load.dispatch_all(()).await;
    }

    async fn on_crash_event(&self) {
        self.crash.dispatch_all(()).await;
    }

    async fn on_pageerror_event(&self, message: String) {
        // Accumulate error for page_errors() accessor
        self.page_errors_log.lock().unwrap().push(message.clone());
        self.pageerror.dispatch(message).await;
    }

    async fn on_popup_event(&self, popup: Page) {
        self.popup.dispatch(popup).await;
    }

    async fn on_frameattached_event(&self, frame: crate::protocol::Frame) {
        self.frameattached.dispatch(frame).await;
    }

    async fn on_framedetached_event(&self, frame: crate::protocol::Frame) {
        self.framedetached.dispatch(frame).await;
    }

    async fn on_framenavigated_event(&self, frame: crate::protocol::Frame) {
        self.framenavigated.dispatch(frame).await;
    }
}

impl ChannelOwner for Page {
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
            "navigated" => {
                // The main frame tracks navigation; nothing to update here.
            }
            "route" => {
                // Handle network routing event
                if let Some(route_guid) = params
                    .get("route")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    // Get the Route object from connection's registry
                    let connection = self.connection();
                    let route_guid_owned = route_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(
                        async move {
                            // Get and downcast Route object
                            let route: Route =
                                match connection.get_typed::<Route>(&route_guid_owned).await {
                                    Ok(r) => r,
                                    Err(e) => {
                                        tracing::warn!("Failed to get route object: {}", e);
                                        return;
                                    }
                                };

                            // Set APIRequestContext on the route for fetch() support.
                            // Page's parent is BrowserContext, which has the request context.
                            if let Some(ctx) =
                                downcast_parent::<crate::protocol::BrowserContext>(&self_clone)
                                && let Ok(api_ctx) = ctx.request().await
                            {
                                route.set_api_request_context(api_ctx);
                            }

                            // Call the route handler and wait for completion
                            self_clone.on_route_event(route).await;
                        }
                        .in_current_span(),
                    );
                }
            }
            "download" => {
                // Handle download event
                // Event params: {url, suggestedFilename, artifact: {guid: "..."}}
                let url = params
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let suggested_filename = params
                    .get("suggestedFilename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(artifact_guid) = params
                    .get("artifact")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let artifact_guid_owned = artifact_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(
                        async move {
                            // Wait for Artifact object to be created
                            let artifact_arc =
                                match connection.get_object(&artifact_guid_owned).await {
                                    Ok(obj) => obj,
                                    Err(e) => {
                                        tracing::warn!("Failed to get artifact object: {}", e);
                                        return;
                                    }
                                };

                            // Create Download wrapper from Artifact + event params
                            let download = Download::from_artifact(
                                artifact_arc,
                                url,
                                suggested_filename,
                                self_clone.clone(),
                            );

                            // Call the download handlers
                            self_clone.on_download_event(download).await;
                        }
                        .in_current_span(),
                    );
                }
            }
            "dialog" => {
                // Dialog events are handled by BrowserContext and forwarded to Page
                // This case should not be reached, but keeping for completeness
            }
            "webSocket" => {
                if let Some(ws_guid) = params
                    .get("webSocket")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let ws_guid_owned = ws_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(
                        async move {
                            // Get and downcast WebSocket object
                            let ws: WebSocket =
                                match connection.get_typed::<WebSocket>(&ws_guid_owned).await {
                                    Ok(ws) => ws,
                                    Err(e) => {
                                        tracing::warn!("Failed to get WebSocket object: {}", e);
                                        return;
                                    }
                                };

                            // Call handlers
                            let handlers = self_clone.websocket_handlers.lock().unwrap().clone();
                            for handler in handlers {
                                let ws_clone = ws.clone();
                                tokio::spawn(
                                    async move {
                                        if let Err(e) = handler(ws_clone).await {
                                            tracing::error!("Error in websocket handler: {}", e);
                                        }
                                    }
                                    .in_current_span(),
                                );
                            }
                        }
                        .in_current_span(),
                    );
                }
            }
            "webSocketRoute" => {
                // A WebSocket matched a route_web_socket pattern.
                // Event format: {webSocketRoute: {guid: "WebSocketRoute@..."}}
                if let Some(wsr_guid) = params
                    .get("webSocketRoute")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let wsr_guid_owned = wsr_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(
                        async move {
                            let route: crate::protocol::WebSocketRoute = match connection
                                .get_typed::<crate::protocol::WebSocketRoute>(&wsr_guid_owned)
                                .await
                            {
                                Ok(r) => r,
                                Err(e) => {
                                    tracing::warn!("Failed to get WebSocketRoute object: {}", e);
                                    return;
                                }
                            };

                            let url = route.url().to_string();
                            let handlers = self_clone.ws_route_handlers.lock().unwrap().clone();
                            for entry in handlers.iter().rev() {
                                if crate::protocol::route::matches_pattern(&entry.pattern, &url) {
                                    let handler = entry.handler.clone();
                                    let route_clone = route.clone();
                                    tokio::spawn(
                                        async move {
                                            if let Err(e) = handler(route_clone).await {
                                                tracing::error!(
                                                    "Error in webSocketRoute handler: {}",
                                                    e
                                                );
                                            }
                                        }
                                        .in_current_span(),
                                    );
                                    break;
                                }
                            }
                        }
                        .in_current_span(),
                    );
                }
            }
            "worker" => {
                // A new Web Worker was created in the page.
                // Event format: {worker: {guid: "Worker@..."}}
                if let Some(worker_guid) = params
                    .get("worker")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let worker_guid_owned = worker_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(
                        async move {
                            let worker: Worker =
                                match connection.get_typed::<Worker>(&worker_guid_owned).await {
                                    Ok(w) => w,
                                    Err(e) => {
                                        tracing::warn!("Failed to get Worker object: {}", e);
                                        return;
                                    }
                                };

                            // Track the worker for workers() accessor
                            self_clone.workers_list.lock().unwrap().push(worker.clone());

                            let handlers = self_clone.worker_handlers.lock().unwrap().clone();
                            for handler in handlers {
                                let worker_clone = worker.clone();
                                tokio::spawn(
                                    async move {
                                        if let Err(e) = handler(worker_clone).await {
                                            tracing::error!("Error in worker handler: {}", e);
                                        }
                                    }
                                    .in_current_span(),
                                );
                            }
                            // Notify expect_event("worker") waiters
                            if let Some(tx) = self_clone.worker_waiters.lock().unwrap().pop() {
                                let _ = tx.send(worker);
                            }
                        }
                        .in_current_span(),
                    );
                }
            }
            "bindingCall" => {
                // A JS caller on this page invoked a page-level exposed function.
                // Event format: {binding: {guid: "..."}}
                if let Some(binding_guid) = params
                    .get("binding")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let binding_guid_owned = binding_guid.to_string();
                    let binding_callbacks = self.binding_callbacks.clone();

                    tokio::spawn(async move {
                        let binding_call: crate::protocol::BindingCall = match connection
                            .get_typed::<crate::protocol::BindingCall>(&binding_guid_owned)
                            .await
                        {
                            Ok(bc) => bc,
                            Err(e) => {
                                tracing::warn!("Failed to get BindingCall object: {}", e);
                                return;
                            }
                        };

                        let name = binding_call.name().to_string();

                        // Look up page-level callback
                        let callback = {
                            let callbacks = binding_callbacks.lock().unwrap();
                            callbacks.get(&name).cloned()
                        };

                        let Some(callback) = callback else {
                            // No page-level handler — the context-level handler on
                            // BrowserContext::on_event("bindingCall") will handle it.
                            return;
                        };

                        // Deserialize args from Playwright protocol format
                        let raw_args = binding_call.args();
                        let args = crate::protocol::browser_context::BrowserContext::deserialize_binding_args_pub(raw_args);

                        // Call callback and serialize result
                        let result_value = callback(args).await;
                        let serialized =
                            crate::protocol::evaluate_conversion::serialize_argument(&result_value);

                        if let Err(e) = binding_call.resolve(serialized).await {
                            tracing::warn!("Failed to resolve BindingCall '{}': {}", name, e);
                        }
                    }.in_current_span());
                }
            }
            "fileChooser" => {
                // FileChooser event: sent when an <input type="file"> is interacted with.
                // Event params: {element: {guid: "..."}, isMultiple: bool}
                let is_multiple = params
                    .get("isMultiple")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if let Some(element_guid) = params
                    .get("element")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let element_guid_owned = element_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(
                        async move {
                            let element: crate::protocol::ElementHandle = match connection
                                .get_typed::<crate::protocol::ElementHandle>(&element_guid_owned)
                                .await
                            {
                                Ok(e) => e,
                                Err(err) => {
                                    tracing::warn!(
                                        "Failed to get ElementHandle for fileChooser: {}",
                                        err
                                    );
                                    return;
                                }
                            };

                            let chooser = crate::protocol::FileChooser::new(
                                self_clone.clone(),
                                std::sync::Arc::new(element),
                                is_multiple,
                            );

                            self_clone.on_filechooser_event(chooser).await;
                        }
                        .in_current_span(),
                    );
                }
            }
            "close" => {
                // Server-initiated close (e.g. context was closed)
                self.is_closed.store(true, Ordering::Relaxed);
                // Dispatch close handlers
                let self_clone = self.clone();
                tokio::spawn(
                    async move {
                        self_clone.on_close_event().await;
                    }
                    .in_current_span(),
                );
            }
            "load" => {
                let self_clone = self.clone();
                tokio::spawn(
                    async move {
                        self_clone.on_load_event().await;
                    }
                    .in_current_span(),
                );
            }
            "crash" => {
                let self_clone = self.clone();
                tokio::spawn(
                    async move {
                        self_clone.on_crash_event().await;
                    }
                    .in_current_span(),
                );
            }
            "pageError" => {
                // params: {"error": {"message": "...", "stack": "..."}}
                let message = params
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
                let self_clone = self.clone();
                tokio::spawn(
                    async move {
                        self_clone.on_pageerror_event(message).await;
                    }
                    .in_current_span(),
                );
            }
            "screencastFrame" => {
                // params: {"frameId": <int>, "data": "<base64 jpeg>", ...}
                //
                // Playwright 1.62 made frame delivery flow-controlled: the
                // driver sends a bounded number of frames and then waits for
                // `screencastFrameAck` before sending more. Without the ack a
                // live screencast delivers a handful of frames and then goes
                // silent forever, which looks like the page stopped animating
                // rather than like a protocol error. Ack as soon as the frame
                // is taken, not after the handlers finish, so a slow handler
                // throttles nothing.
                if let Some(frame_id) = params.get("frameId").and_then(|v| v.as_i64()) {
                    let self_clone = self.clone();
                    tokio::spawn(
                        async move {
                            if let Err(e) = self_clone
                                .channel()
                                .send::<_, serde_json::Value>(
                                    "screencastFrameAck",
                                    serde_json::json!({ "frameId": frame_id }),
                                )
                                .await
                            {
                                tracing::warn!("Failed to ack screencast frame: {}", e);
                            }
                        }
                        .in_current_span(),
                    );
                }

                if let Some(b64) = params.get("data").and_then(|v| v.as_str()) {
                    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) {
                        // Wrap once in `Bytes`; each handler-clone below is a refcount bump.
                        let frame = crate::protocol::ScreencastFrame {
                            data: bytes::Bytes::from(bytes),
                            timestamp: params.get("timestamp").and_then(|v| v.as_f64()),
                        };
                        let handlers = self.screencast_frame_handlers.lock().unwrap().clone();
                        for h in handlers {
                            let f = frame.clone();
                            tokio::spawn(
                                async move {
                                    if let Err(e) = h(f).await {
                                        tracing::warn!("Screencast frame handler error: {}", e);
                                    }
                                }
                                .in_current_span(),
                            );
                        }
                    } else {
                        tracing::warn!("Failed to decode screencast frame data");
                    }
                }
            }
            // "popup" is forwarded from BrowserContext::on_event when a "page" event
            // is received for a page that has an opener. No direct "popup" event on Page.
            "frameAttached" => {
                // params: {"frame": {"guid": "..."}}
                if let Some(frame_guid) = params
                    .get("frame")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let frame_guid_owned = frame_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(
                        async move {
                            let frame: crate::protocol::Frame = match connection
                                .get_typed::<crate::protocol::Frame>(&frame_guid_owned)
                                .await
                            {
                                Ok(f) => f,
                                Err(e) => {
                                    tracing::warn!("Failed to get Frame for frameAttached: {}", e);
                                    return;
                                }
                            };
                            self_clone.on_frameattached_event(frame).await;
                        }
                        .in_current_span(),
                    );
                }
            }
            "frameDetached" => {
                // params: {"frame": {"guid": "..."}}
                if let Some(frame_guid) = params
                    .get("frame")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let frame_guid_owned = frame_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(
                        async move {
                            let frame: crate::protocol::Frame = match connection
                                .get_typed::<crate::protocol::Frame>(&frame_guid_owned)
                                .await
                            {
                                Ok(f) => f,
                                Err(e) => {
                                    tracing::warn!("Failed to get Frame for frameDetached: {}", e);
                                    return;
                                }
                            };
                            self_clone.on_framedetached_event(frame).await;
                        }
                        .in_current_span(),
                    );
                }
            }
            "frameNavigated" => {
                // params: {"frame": {"guid": "..."}}
                // Note: frameNavigated may also contain url, name, etc. at top level
                // The frame guid is in the "frame" field (same as attached/detached)
                if let Some(frame_guid) = params
                    .get("frame")
                    .and_then(|v| v.get("guid"))
                    .and_then(|v| v.as_str())
                {
                    let connection = self.connection();
                    let frame_guid_owned = frame_guid.to_string();
                    let self_clone = self.clone();

                    tokio::spawn(
                        async move {
                            let frame: crate::protocol::Frame = match connection
                                .get_typed::<crate::protocol::Frame>(&frame_guid_owned)
                                .await
                            {
                                Ok(f) => f,
                                Err(e) => {
                                    tracing::warn!("Failed to get Frame for frameNavigated: {}", e);
                                    return;
                                }
                            };
                            self_clone.on_framenavigated_event(frame).await;
                        }
                        .in_current_span(),
                    );
                }
            }
            "locatorHandlerTriggered" => {
                // Server fires this when a registered locator matches an element.
                // params: {"uid": N}
                if let Some(uid) = params.get("uid").and_then(|v| v.as_u64()).map(|v| v as u32) {
                    let locator_handlers = self.locator_handlers.clone();
                    let self_clone = self.clone();

                    tokio::spawn(
                        async move {
                            // Look up handler and decrement times_remaining
                            let (handler, selector, should_remove) = {
                                let mut handlers = locator_handlers.lock().unwrap();
                                let entry = handlers.iter_mut().find(|e| e.uid == uid);
                                match entry {
                                    None => return,
                                    Some(e) => {
                                        let handler = e.handler.clone();
                                        let selector = e.selector.clone();
                                        let remove = match e.times_remaining {
                                            Some(1) => true,
                                            Some(ref mut n) => {
                                                *n -= 1;
                                                false
                                            }
                                            None => false,
                                        };
                                        (handler, selector, remove)
                                    }
                                }
                            };

                            // Build a Locator for the handler to receive
                            let locator = self_clone.locator(&selector);

                            // Run the handler
                            if let Err(e) = handler(locator).await {
                                tracing::warn!("locator handler error (uid={}): {}", uid, e);
                            }

                            // Send resolveLocatorHandler — remove=true if times exhausted
                            let _ = self_clone
                                .channel()
                                .send_no_result(
                                    "resolveLocatorHandler",
                                    serde_json::json!({ "uid": uid, "remove": should_remove }),
                                )
                                .await;

                            // Remove from local registry if one-shot
                            if should_remove {
                                self_clone
                                    .locator_handlers
                                    .lock()
                                    .unwrap()
                                    .retain(|e| e.uid != uid);
                            }
                        }
                        .in_current_span(),
                    );
                }
            }
            _ => {
                // Other events not yet handled
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
