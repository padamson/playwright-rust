use super::{Page, WebSocketHandlerFuture, WorkerHandlerFuture};
use crate::error::{Error, Result};
use crate::protocol::event_registry::{EventRegistry, Handler};
use crate::protocol::{Dialog, Download, Request, ResponseObject, WebSocket, Worker};
use crate::server::channel_owner::ChannelOwner;
use std::future::Future;
use std::sync::Arc;
use tracing::Instrument;

/// Event subscriptions (`on_*`) and one-shot waiters (`expect_*`).
impl Page {
    /// Subscribe to `reg`'s event if nothing is listening yet.
    ///
    /// The server only pushes an event once we ask for it, so the first
    /// handler or `expect_*` on a given event has to turn the subscription on.
    /// The event name comes from the registry, so it is written once at
    /// construction instead of restated at every registration site.
    async fn subscribe_if_idle<T>(&self, reg: &EventRegistry<T>) {
        if reg.is_idle() {
            _ = self.channel().update_subscription(reg.name(), true).await;
        }
    }

    /// Returns all console messages received so far on this page.
    ///
    /// Messages are accumulated in order as they arrive via the `console` event.
    /// Each call returns a snapshot; new messages arriving concurrently may or may not
    /// be included depending on timing.
    ///
    /// To get a filtered subset, chain a standard iterator filter:
    ///
    /// ```no_run
    /// # use playwright_rs::Playwright;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pw = Playwright::launch().await?;
    /// # let browser = pw.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// let errors: Vec<_> = page
    ///     .console_messages()
    ///     .into_iter()
    ///     .filter(|m| m.type_() == "error")
    ///     .collect();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Use [`clear_console_messages`](Self::clear_console_messages) to drop
    /// the accumulator (e.g. between test phases).
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-console-messages>
    pub fn console_messages(&self) -> Vec<crate::protocol::ConsoleMessage> {
        self.console_messages_log.lock().unwrap().clone()
    }

    /// Drops every console message accumulated so far. New messages arriving
    /// after this call still get recorded; the accumulator just starts empty
    /// again. Useful between test phases when you want to assert against
    /// only messages from a specific phase.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-clear-console-messages>
    pub fn clear_console_messages(&self) {
        self.console_messages_log.lock().unwrap().clear();
    }

    /// Returns all uncaught JavaScript error messages received so far on this page.
    ///
    /// Errors are accumulated in order as they arrive via the `pageError` event.
    /// Each string is the `.message` field of the thrown `Error`.
    ///
    /// To get a filtered subset, chain a standard iterator filter:
    ///
    /// ```no_run
    /// # use playwright_rs::Playwright;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pw = Playwright::launch().await?;
    /// # let browser = pw.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// let typeerrors: Vec<_> = page
    ///     .page_errors()
    ///     .into_iter()
    ///     .filter(|e| e.starts_with("TypeError"))
    ///     .collect();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Use [`clear_page_errors`](Self::clear_page_errors) to drop the
    /// accumulator (e.g. between test phases).
    pub fn page_errors(&self) -> Vec<String> {
        self.page_errors_log.lock().unwrap().clone()
    }

    /// Drops every page error accumulated so far. New errors arriving after
    /// this call still get recorded.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-clear-page-errors>
    pub fn clear_page_errors(&self) {
        self.page_errors_log.lock().unwrap().clear();
    }

    /// Registers a download event handler.
    ///
    /// The handler will be called when a download is triggered by the page.
    /// Downloads occur when the page initiates a file download (e.g., clicking a link
    /// with the download attribute, or a server response with Content-Disposition: attachment).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the Download object
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-download>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_download<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Download) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<Download> = Arc::new(move |download| Box::pin(handler(download)));
        // "download" events are auto-emitted; no subscription needed.
        self.download.add_handler(handler);

        Ok(())
    }

    /// Registers a dialog event handler.
    ///
    /// The handler will be called when a JavaScript dialog is triggered (alert, confirm, prompt, or beforeunload).
    /// The dialog must be explicitly accepted or dismissed, otherwise the page will freeze.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the Dialog object
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-dialog>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_dialog<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Dialog) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<Dialog> = Arc::new(move |dialog| Box::pin(handler(dialog)));
        // Dialog events are auto-emitted (no subscription needed).
        self.dialog.add_handler(handler);

        Ok(())
    }

    /// Registers a handler for the `dialogclosed` event, which fires once a
    /// dialog has been accepted, dismissed, or closed by the user.
    ///
    /// Waiting for this rather than for `dialog` is how a test knows the page
    /// is interactive again.
    ///
    /// # Errors
    ///
    /// Returns an error if the handler cannot be registered.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-dialog-closed>
    pub async fn on_dialog_closed<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Dialog) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<Dialog> = Arc::new(move |dialog| Box::pin(handler(dialog)));
        // The event is delivered to the owning context, which subscribes on
        // its own first handler. A page-only listener has to ask for the
        // subscription itself, or nothing arrives.
        self.context()?.ensure_dialog_closed_subscription().await;
        self.dialog_closed.add_handler(handler);
        Ok(())
    }

    /// Registers a console event handler.
    ///
    /// The handler is called whenever the page emits a JavaScript console message
    /// (e.g. `console.log`, `console.error`, `console.warn`, etc.).
    ///
    /// The server only sends console events after the first handler is registered
    /// (subscription is managed automatically).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the [`ConsoleMessage`](crate::protocol::ConsoleMessage)
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-console>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_console<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::ConsoleMessage) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<crate::protocol::ConsoleMessage> =
            Arc::new(move |msg| Box::pin(handler(msg)));

        self.subscribe_if_idle(&self.console).await;
        self.console.add_handler(handler);

        Ok(())
    }

    /// Registers a handler for file chooser events.
    ///
    /// The handler is called whenever the page opens a file chooser dialog
    /// (e.g. when the user clicks an `<input type="file">` element).
    ///
    /// Use [`FileChooser::set_files`](crate::protocol::FileChooser::set_files) inside
    /// the handler to satisfy the file chooser without OS-level interaction.
    ///
    /// The server only sends `"fileChooser"` events after the first handler is
    /// registered (subscription is managed automatically via `updateSubscription`).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives a [`FileChooser`](crate::protocol::FileChooser)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::Playwright;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pw = Playwright::launch().await?;
    /// # let browser = pw.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// page.on_filechooser(|chooser| async move {
    ///     chooser.set_files(&[std::path::PathBuf::from("/tmp/file.txt")]).await
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-file-chooser>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_filechooser<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::FileChooser) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<crate::protocol::FileChooser> =
            Arc::new(move |chooser| Box::pin(handler(chooser)));

        self.subscribe_if_idle(&self.filechooser).await;
        self.filechooser.add_handler(handler);

        Ok(())
    }

    /// Creates a one-shot waiter that resolves when the next file chooser opens.
    ///
    /// The waiter **must** be created before the action that triggers the file
    /// chooser to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if the file chooser
    /// does not open within the timeout.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::Playwright;
    /// # use std::path::PathBuf;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pw = Playwright::launch().await?;
    /// # let browser = pw.chromium().launch().await?;
    /// # let page = browser.new_page().await?;
    /// // Set up waiter BEFORE triggering the file chooser
    /// let waiter = page.expect_file_chooser(None).await?;
    /// page.locator("input[type=file]").click(None).await?;
    /// let chooser = waiter.wait().await?;
    /// chooser.set_files(&[PathBuf::from("/tmp/file.txt")]).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn expect_file_chooser(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<crate::protocol::FileChooser>> {
        self.subscribe_if_idle(&self.filechooser).await;
        let rx = self.filechooser.wait();

        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Creates a one-shot waiter that resolves when the next popup window opens.
    ///
    /// The waiter **must** be created before the action that opens the popup to
    /// avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if no popup
    /// opens within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn expect_popup(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<Page>> {
        let rx = self.popup.wait();
        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Creates a one-shot waiter that resolves when the next download starts.
    ///
    /// The waiter **must** be created before the action that triggers the download
    /// to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if no download
    /// starts within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn expect_download(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<Download>> {
        let rx = self.download.wait();
        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Creates a one-shot waiter that resolves when the next network response is received.
    ///
    /// The waiter **must** be created before the action that triggers the response
    /// to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if no response
    /// arrives within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn expect_response(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<ResponseObject>> {
        self.subscribe_if_idle(&self.response).await;
        let rx = self.response.wait();

        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Creates a one-shot waiter that resolves when the next network request is issued.
    ///
    /// The waiter **must** be created before the action that issues the request
    /// to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if no request
    /// is issued within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn expect_request(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<Request>> {
        self.subscribe_if_idle(&self.request).await;
        let rx = self.request.wait();

        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Creates a one-shot waiter that resolves when the next console message is produced.
    ///
    /// The waiter **must** be created before the action that produces the console
    /// message to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if no console
    /// message is produced within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn expect_console_message(
        &self,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<crate::protocol::ConsoleMessage>> {
        self.subscribe_if_idle(&self.console).await;
        let rx = self.console.wait();

        Ok(crate::protocol::EventWaiter::new(
            rx,
            timeout.or(Some(30_000.0)),
        ))
    }

    /// Waits for the given event to fire and returns a typed `EventValue`.
    ///
    /// This is the generic version of the specific `expect_*` methods. It matches
    /// the playwright-python / playwright-js `page.expect_event(event_name)` API.
    ///
    /// The waiter **must** be created before the action that triggers the event.
    ///
    /// # Supported event names
    ///
    /// `"request"`, `"response"`, `"popup"`, `"download"`, `"console"`,
    /// `"filechooser"`, `"close"`, `"load"`, `"crash"`, `"pageerror"`,
    /// `"frameattached"`, `"framedetached"`, `"framenavigated"`, `"worker"`
    ///
    /// # Arguments
    ///
    /// * `event` - Event name (case-sensitive, matches Playwright protocol names).
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::InvalidArgument`] for unknown event names.
    /// Returns [`crate::error::Error::Timeout`] if the event does not fire within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-wait-for-event>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn expect_event(
        &self,
        event: &str,
        timeout: Option<f64>,
    ) -> Result<crate::protocol::EventWaiter<crate::protocol::EventValue>> {
        use crate::protocol::EventValue;
        use tokio::sync::oneshot;

        let timeout_ms = timeout.or(Some(30_000.0));

        match event {
            "request" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();

                self.subscribe_if_idle(&self.request).await;
                let inner_rx = self.request.wait();

                // select: drop the registry receiver when the caller times
                // out, or a stale FIFO waiter swallows the next event.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if let Ok(v) = v {
                                    let _ = tx.send(EventValue::Request(v));
                                }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "response" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();

                self.subscribe_if_idle(&self.response).await;
                let inner_rx = self.response.wait();

                // select: see the "request" arm.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if let Ok(v) = v {
                                    let _ = tx.send(EventValue::Response(v));
                                }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "popup" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.popup.wait();

                // select: see the "request" arm.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if let Ok(v) = v {
                                    let _ = tx.send(EventValue::Page(v));
                                }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "download" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.download.wait();

                // select: see the "request" arm.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if let Ok(v) = v {
                                    let _ = tx.send(EventValue::Download(v));
                                }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "console" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();

                self.subscribe_if_idle(&self.console).await;
                let inner_rx = self.console.wait();

                // The select is load-bearing: with FIFO waiters, a forwarding
                // task that merely awaits `inner_rx` keeps the registry's
                // sender alive after the caller's EventWaiter times out, and
                // that stale front-of-queue waiter would swallow the next
                // event, starving the live waiter behind it. Dropping
                // `inner_rx` the moment the outer receiver goes away lets the
                // registry's dead-sender skip do its job.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if let Ok(v) = v {
                                    let _ = tx.send(EventValue::ConsoleMessage(v));
                                }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "filechooser" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();

                self.subscribe_if_idle(&self.filechooser).await;
                let inner_rx = self.filechooser.wait();

                // select: see the "request" arm.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if let Ok(v) = v {
                                    let _ = tx.send(EventValue::FileChooser(v));
                                }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "close" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.close.wait();

                // select: see the "request" arm.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if v.is_ok() { let _ = tx.send(EventValue::Close); }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "load" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.load.wait();

                // select: see the "request" arm.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if v.is_ok() { let _ = tx.send(EventValue::Load); }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "crash" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.crash.wait();

                // select: see the "request" arm.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if v.is_ok() { let _ = tx.send(EventValue::Crash); }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "pageerror" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.pageerror.wait();

                // select: see the "request" arm.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            msg = inner_rx => {
                                if let Ok(msg) = msg { let _ = tx.send(EventValue::PageError(msg)); }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "frameattached" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.frameattached.wait();

                // select: see the "request" arm.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if let Ok(v) = v { let _ = tx.send(EventValue::Frame(v)); }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "framedetached" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.framedetached.wait();

                // select: see the "request" arm.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if let Ok(v) = v { let _ = tx.send(EventValue::Frame(v)); }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "framenavigated" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.framenavigated.wait();

                // select: see the "request" arm.
                tokio::spawn(
                    async move {
                        tokio::select! {
                            v = inner_rx => {
                                if let Ok(v) = v { let _ = tx.send(EventValue::Frame(v)); }
                            }
                            () = tx.closed() => {}
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            "worker" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::Worker>();
                self.worker_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(
                    async move {
                        if let Ok(v) = inner_rx.await {
                            let _ = tx.send(EventValue::Worker(v));
                        }
                    }
                    .in_current_span(),
                );

                Ok(crate::protocol::EventWaiter::new(rx, timeout_ms))
            }

            other => Err(Error::InvalidArgument(format!(
                "Unknown event name '{}'. Supported: request, response, popup, download, \
                 console, filechooser, close, load, crash, pageerror, \
                 frameattached, framedetached, framenavigated, worker",
                other
            ))),
        }
    }

    /// See: <https://playwright.dev/docs/api/class-page#page-event-request>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_request<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<Request> = Arc::new(move |request| Box::pin(handler(request)));

        self.subscribe_if_idle(&self.request).await;
        self.request.add_handler(handler);

        Ok(())
    }

    /// See: <https://playwright.dev/docs/api/class-page#page-event-request-finished>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_request_finished<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<Request> = Arc::new(move |request| Box::pin(handler(request)));

        self.subscribe_if_idle(&self.request_finished).await;
        self.request_finished.add_handler(handler);

        Ok(())
    }

    /// See: <https://playwright.dev/docs/api/class-page#page-event-request-failed>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_request_failed<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<Request> = Arc::new(move |request| Box::pin(handler(request)));

        self.subscribe_if_idle(&self.request_failed).await;
        self.request_failed.add_handler(handler);

        Ok(())
    }

    /// See: <https://playwright.dev/docs/api/class-page#page-event-response>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_response<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(ResponseObject) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<ResponseObject> =
            Arc::new(move |response| Box::pin(handler(response)));

        self.subscribe_if_idle(&self.response).await;
        self.response.add_handler(handler);

        Ok(())
    }

    /// Adds a listener for the `websocket` event.
    ///
    /// The handler will be called when a WebSocket request is dispatched.
    ///
    /// # Arguments
    ///
    /// * `handler` - The function to call when the event occurs
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-on-websocket>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_websocket<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(WebSocket) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler =
            Arc::new(move |ws: WebSocket| -> WebSocketHandlerFuture { Box::pin(handler(ws)) });
        self.websocket_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `worker` event.
    ///
    /// The handler is called when a new Web Worker is created in the page.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure called with the new [`Worker`] object
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-worker>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_worker<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Worker) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |w: Worker| -> WorkerHandlerFuture { Box::pin(handler(w)) });
        self.worker_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Registers a handler for the `close` event.
    ///
    /// The handler is called when the page is closed, either by calling `page.close()`,
    /// by the browser context being closed, or when the browser process exits.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure called with no arguments when the page closes
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-close>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_close<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<()> = Arc::new(move |()| Box::pin(handler()));
        self.close.add_handler(handler);
        Ok(())
    }

    /// Registers a handler for the `load` event.
    ///
    /// The handler is called when the page's `load` event fires, i.e. after
    /// all resources including stylesheets and images have finished loading.
    ///
    /// The server only sends `"load"` events after the first handler is registered
    /// (subscription is managed automatically).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure called with no arguments when the page loads
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-load>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_load<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<()> = Arc::new(move |()| Box::pin(handler()));
        // "load" events come via Frame's "loadstate" event, no subscription needed.
        self.load.add_handler(handler);
        Ok(())
    }

    /// Registers a handler for the `crash` event.
    ///
    /// The handler is called when the page crashes (e.g. runs out of memory).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure called with no arguments when the page crashes
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-crash>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_crash<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<()> = Arc::new(move |()| Box::pin(handler()));
        self.crash.add_handler(handler);
        Ok(())
    }

    /// Registers a handler for the `pageError` event.
    ///
    /// The handler is called when an uncaught JavaScript exception is thrown in the page.
    /// The handler receives the error message as a `String`.
    ///
    /// The server only sends `"pageError"` events after the first handler is registered
    /// (subscription is managed automatically).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the error message string
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-page-error>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_pageerror<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<String> = Arc::new(move |msg| Box::pin(handler(msg)));
        // "pageError" events come via BrowserContext, no subscription needed.
        self.pageerror.add_handler(handler);
        Ok(())
    }

    /// Registers a handler for the `popup` event.
    ///
    /// The handler is called when the page opens a popup window (e.g. via `window.open()`).
    /// The handler receives the new popup [`Page`] object.
    ///
    /// The server only sends `"popup"` events after the first handler is registered
    /// (subscription is managed automatically).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the popup Page
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-popup>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_popup<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Page) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<Page> = Arc::new(move |page| Box::pin(handler(page)));
        // "popup" events arrive via BrowserContext's "page" event when a page has an opener.
        self.popup.add_handler(handler);
        Ok(())
    }

    /// Registers a handler for the `frameAttached` event.
    ///
    /// The handler is called when a new frame (iframe) is attached to the page.
    /// The handler receives the attached [`Frame`](crate::protocol::Frame) object.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the attached Frame
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-frameattached>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_frameattached<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::Frame) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<crate::protocol::Frame> =
            Arc::new(move |frame| Box::pin(handler(frame)));
        self.frameattached.add_handler(handler);
        Ok(())
    }

    /// Registers a handler for the `frameDetached` event.
    ///
    /// The handler is called when a frame (iframe) is detached from the page.
    /// The handler receives the detached [`Frame`](crate::protocol::Frame) object.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the detached Frame
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-framedetached>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_framedetached<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::Frame) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<crate::protocol::Frame> =
            Arc::new(move |frame| Box::pin(handler(frame)));
        self.framedetached.add_handler(handler);
        Ok(())
    }

    /// Registers a handler for the `frameNavigated` event.
    ///
    /// The handler is called when a frame navigates to a new URL.
    /// The handler receives the navigated [`Frame`](crate::protocol::Frame) object.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the navigated Frame
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-event-framenavigated>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_framenavigated<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::Frame) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<crate::protocol::Frame> =
            Arc::new(move |frame| Box::pin(handler(frame)));
        self.framenavigated.add_handler(handler);
        Ok(())
    }
}
