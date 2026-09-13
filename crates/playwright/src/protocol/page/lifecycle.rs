use super::{GotoOptions, Page, set_timeout_and_notify};
use crate::error::{Error, Result};
use crate::protocol::Worker;
use crate::protocol::browser_context::Viewport;
use crate::protocol::event_registry::EventRegistry;
use crate::server::channel::Channel;
use crate::server::channel_owner::{ChannelOwner, ChannelOwnerImpl, ParentOrConnection};
use crate::server::connection::{ConnectionExt, downcast_parent};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tracing::Instrument;

/// Lifecycle, timeouts, frames, and locator constructors.
impl Page {
    /// Creates a new Page from protocol initialization
    ///
    /// This is called by the object factory when the server sends a `__create__` message
    /// for a Page object.
    ///
    /// # Arguments
    ///
    /// * `parent` - The parent BrowserContext object
    /// * `type_name` - The protocol type name ("Page")
    /// * `guid` - The unique identifier for this page
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
        main_frame: crate::protocol::Frame,
    ) -> Result<Self> {
        // Check the parent BrowserContext's initializer for record_video before
        // moving `parent` into ChannelOwnerImpl. The Playwright server delivers
        // the video artifact GUID directly in the Page initializer's "video" field.
        let has_video = parent
            .initializer()
            .get("options")
            .and_then(|opts| opts.get("recordVideo"))
            .is_some();

        let video_artifact_guid: Option<String> = initializer
            .get("video")
            .and_then(|v| v.get("guid"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let base = ChannelOwnerImpl::new(
            ParentOrConnection::Parent(parent),
            type_name,
            guid,
            initializer,
        );

        // Initialize URL to about:blank

        // Initialize empty route handlers
        let route_handlers = Arc::new(Mutex::new(Vec::new()));

        // Initialize empty event handlers
        let websocket_handlers = Arc::new(Mutex::new(Vec::new()));
        let ws_route_handlers = Arc::new(Mutex::new(Vec::new()));

        // Initialize cached main frame as empty (will be populated on first access)

        // Extract viewport from initializer (may be null for no_viewport contexts)
        let initial_viewport: Option<Viewport> =
            base.initializer().get("viewportSize").and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    serde_json::from_value(v.clone()).ok()
                }
            });
        let viewport = Arc::new(RwLock::new(initial_viewport));

        let video = if has_video {
            let v = crate::protocol::Video::new();
            // Resolve the artifact from the initializer-provided GUID.
            if let Some(artifact_guid) = video_artifact_guid {
                let connection = base.connection();
                let v_clone = v.clone();
                tokio::spawn(
                    async move {
                        match connection.get_object(&artifact_guid).await {
                            Ok(artifact_arc) => v_clone.set_artifact(artifact_arc),
                            Err(e) => tracing::warn!(
                                "Failed to resolve video artifact {} from initializer: {}",
                                artifact_guid,
                                e
                            ),
                        }
                    }
                    .in_current_span(),
                );
            }
            Some(v)
        } else {
            None
        };

        Ok(Self {
            base,
            main_frame,
            route_handlers,
            download: EventRegistry::new("download"),
            dialog: EventRegistry::new("dialog"),
            dialog_closed: EventRegistry::new("dialogClosed"),
            request: EventRegistry::new("request"),
            request_finished: EventRegistry::new("requestFinished"),
            request_failed: EventRegistry::new("requestFailed"),
            response: EventRegistry::new("response"),
            websocket_handlers,
            ws_route_handlers,
            viewport,
            is_closed: Arc::new(AtomicBool::new(false)),
            default_timeout_ms: Arc::new(AtomicU64::new(crate::DEFAULT_TIMEOUT_MS.to_bits())),
            default_navigation_timeout_ms: Arc::new(AtomicU64::new(
                crate::DEFAULT_TIMEOUT_MS.to_bits(),
            )),
            binding_callbacks: Arc::new(Mutex::new(HashMap::new())),
            screencast_frame_handlers: Arc::new(Mutex::new(Vec::new())),
            screencast_artifact_guid: Arc::new(Mutex::new(None)),
            screencast_save_path: Arc::new(Mutex::new(None)),
            filechooser: EventRegistry::new("fileChooser"),
            console: EventRegistry::new("console"),
            close: EventRegistry::new("close"),
            load: EventRegistry::new("load"),
            crash: EventRegistry::new("crash"),
            pageerror: EventRegistry::new("pageError"),
            popup: EventRegistry::new("popup"),
            frameattached: EventRegistry::new("frameAttached"),
            framedetached: EventRegistry::new("frameDetached"),
            framenavigated: EventRegistry::new("frameNavigated"),
            worker_handlers: Arc::new(Mutex::new(Vec::new())),
            worker_waiters: Arc::new(Mutex::new(Vec::new())),
            console_messages_log: Arc::new(Mutex::new(Vec::new())),
            page_errors_log: Arc::new(Mutex::new(Vec::new())),
            workers_list: Arc::new(Mutex::new(Vec::new())),
            video,
            locator_handlers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Returns the channel for sending protocol messages
    ///
    /// Used internally for sending RPC calls to the page.
    fn channel(&self) -> &Channel {
        self.base.channel()
    }

    /// Returns the main frame of the page.
    ///
    /// The main frame is where navigation and DOM operations actually happen.
    ///
    /// This method also wires up the back-reference from the frame to the page so that
    /// `frame.page()`, `frame.locator()`, and `frame.get_by_*()` work correctly.
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn main_frame(&self) -> Result<crate::protocol::Frame> {
        Ok(self.main_frame_wired())
    }

    /// Clone of the construction-time main frame with the page back-reference
    /// wired, so `frame.page()` / `frame.locator()` work. Infallible: the
    /// frame is resolved when the Page is created.
    pub(crate) fn main_frame_wired(&self) -> crate::protocol::Frame {
        let frame = self.main_frame.clone();
        frame.set_page(self.clone());
        frame
    }

    /// Returns the current URL of the page.
    ///
    /// This returns the last committed URL, including hash fragments from anchor navigation.
    /// Initially, pages are at "about:blank".
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-url>
    pub fn url(&self) -> String {
        // The main frame is the source of truth for navigation, including
        // hash fragments from anchor navigation.
        self.main_frame.url()
    }

    /// Closes the page.
    ///
    /// This is a graceful operation that sends a close command to the page
    /// and waits for it to shut down properly.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has already been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-close>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn close(&self) -> Result<()> {
        // Send close RPC to server
        let result = self
            .channel()
            .send_no_result("close", serde_json::json!({}))
            .await;
        // Mark as closed regardless of error (best-effort)
        self.is_closed.store(true, Ordering::Relaxed);
        result
    }

    /// Returns whether the page has been closed.
    ///
    /// Returns `true` after `close()` has been called on this page, or after the
    /// page receives a close event from the server (e.g. when the browser context
    /// is closed).
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-is-closed>
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Relaxed)
    }

    /// Returns the page that opened this popup, or `None` if this page was not opened
    /// by another page.
    ///
    /// The opener is available from the page's initializer — it is the page that called
    /// `window.open()` or triggered a link with `target="_blank"`. Returns `None` for
    /// top-level pages that were not opened as popups.
    ///
    /// # Errors
    ///
    /// Returns error if the opener page GUID is present in the initializer but the
    /// object is not found in the connection registry.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-opener>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn opener(&self) -> Result<Option<Page>> {
        // The opener guid is stored in the page initializer as {"opener": {"guid": "..."}}.
        // It is set when the page is created as a popup; absent for non-popup pages.
        let opener_guid = self
            .base
            .initializer()
            .get("opener")
            .and_then(|v| v.get("guid"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        match opener_guid {
            None => Ok(None),
            Some(guid) => {
                let page = self.connection().get_typed::<Page>(&guid).await?;
                Ok(Some(page))
            }
        }
    }

    /// Returns all active web workers belonging to this page.
    ///
    /// Workers are tracked as they are created (`worker` event) and this method
    /// returns a snapshot of the current list.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-workers>
    pub fn workers(&self) -> Vec<Worker> {
        self.workers_list.lock().unwrap().clone()
    }

    /// Sets the default timeout for all operations on this page.
    ///
    /// The timeout applies to actions such as `click`, `fill`, `locator.wait_for`, etc.
    /// Pass `0` to disable timeouts.
    ///
    /// This stores the value locally so that subsequent action calls use it when
    /// no explicit timeout is provided, and also notifies the Playwright server
    /// so it can apply the same default on its side.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-set-default-timeout>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn set_default_timeout(&self, timeout: f64) {
        self.default_timeout_ms
            .store(timeout.to_bits(), Ordering::Relaxed);
        set_timeout_and_notify(self.channel(), "setDefaultTimeoutNoReply", timeout).await;
    }

    /// Sets the default timeout for navigation operations on this page.
    ///
    /// The timeout applies to navigation actions such as `goto`, `reload`,
    /// `go_back`, and `go_forward`. Pass `0` to disable timeouts.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-set-default-navigation-timeout>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn set_default_navigation_timeout(&self, timeout: f64) {
        self.default_navigation_timeout_ms
            .store(timeout.to_bits(), Ordering::Relaxed);
        set_timeout_and_notify(
            self.channel(),
            "setDefaultNavigationTimeoutNoReply",
            timeout,
        )
        .await;
    }

    /// Returns the current default action timeout in milliseconds.
    pub fn default_timeout_ms(&self) -> f64 {
        f64::from_bits(self.default_timeout_ms.load(Ordering::Relaxed))
    }

    /// Returns the current default navigation timeout in milliseconds.
    pub fn default_navigation_timeout_ms(&self) -> f64 {
        f64::from_bits(self.default_navigation_timeout_ms.load(Ordering::Relaxed))
    }

    /// Returns all frames in the page, including the main frame.
    ///
    /// Currently returns only the main (top-level) frame. Iframe enumeration
    /// is not yet implemented and will be added in a future release.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-frames>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn frames(&self) -> Result<Vec<crate::protocol::Frame>> {
        // Start with the main frame
        let main = self.main_frame().await?;
        Ok(vec![main])
    }

    /// Returns the browser context that the page belongs to.
    pub fn context(&self) -> Result<crate::protocol::BrowserContext> {
        downcast_parent::<crate::protocol::BrowserContext>(self)
            .ok_or_else(|| Error::ProtocolError("Page parent is not a BrowserContext".to_string()))
    }

    /// Returns the Clock object for this page's browser context.
    ///
    /// This is a convenience accessor that delegates to the parent context's clock.
    /// All clock RPCs are sent on the BrowserContext channel regardless of whether
    /// the Clock is obtained via `page.clock()` or `context.clock()`.
    ///
    /// # Errors
    ///
    /// Returns error if the page's parent is not a BrowserContext.
    ///
    /// See: <https://playwright.dev/docs/api/class-clock>
    pub fn clock(&self) -> Result<crate::protocol::clock::Clock> {
        Ok(self.context()?.clock())
    }

    /// Returns the `Video` object associated with this page, if video recording is enabled.
    ///
    /// Returns `Some(Video)` when the browser context was created with the `record_video`
    /// option; returns `None` otherwise.
    ///
    /// The `Video` shell is created eagerly. The underlying recording artifact is wired
    /// up when the Playwright server fires the internal `"video"` event (which typically
    /// happens when the page is first navigated). Calling [`crate::protocol::Video::save_as`] or
    /// [`crate::protocol::Video::path`] before the artifact arrives returns an error; close the page
    /// first to guarantee the artifact is ready.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-video>
    pub fn video(&self) -> Option<crate::protocol::Video> {
        self.video.clone()
    }

    /// Pauses script execution.
    ///
    /// Playwright will stop executing the script and wait for the user to either press
    /// "Resume" in the page overlay or in the debugger.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-pause>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn pause(&self) -> Result<()> {
        self.context()?.pause().await
    }

    /// Returns the page's title.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-title>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn title(&self) -> Result<String> {
        // Delegate to main frame
        let frame = self.main_frame().await?;
        frame.title().await
    }

    /// Returns the full HTML content of the page, including the DOCTYPE.
    ///
    /// This method retrieves the complete HTML markup of the page,
    /// including the doctype declaration and all DOM elements.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-content>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn content(&self) -> Result<String> {
        // Delegate to main frame
        let frame = self.main_frame().await?;
        frame.content().await
    }

    /// Sets the content of the page.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-set-content>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn set_content(
        &self,
        html: &str,
        options: impl Into<Option<GotoOptions>>,
    ) -> Result<()> {
        let options = options.into();
        let frame = self.main_frame().await?;
        frame.set_content(html, options).await
    }

    /// Creates a locator for finding elements on the page.
    ///
    /// Locators are the central piece of Playwright's auto-waiting and retry-ability.
    /// They don't execute queries until an action is performed.
    ///
    /// # Arguments
    ///
    /// * `selector` - CSS selector or other locating strategy
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-locator>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), selector = tracing::field::Empty))]
    pub fn locator(&self, selector: impl Into<String>) -> crate::protocol::Locator {
        let selector = selector.into();
        tracing::Span::current().record("selector", selector.as_str());
        let frame = self.main_frame_wired();

        crate::protocol::Locator::new(Arc::new(frame), selector, self.clone())
    }

    /// Creates a [`FrameLocator`](crate::protocol::FrameLocator) for an iframe on this page.
    ///
    /// The `selector` identifies the iframe element (e.g.
    /// `"iframe[name='content']"`). Pass `None` to search every frame in the
    /// subtree instead, so the iframe does not have to be located first; the
    /// rest of the locator still resolves inside a single frame, and matching
    /// elements in several of them is an error.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-frame-locator>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub fn frame_locator<'a>(
        &self,
        selector: impl Into<Option<&'a str>>,
    ) -> crate::protocol::FrameLocator {
        let frame = Arc::new(self.main_frame_wired());
        match selector.into() {
            Some(selector) => {
                crate::protocol::FrameLocator::new(frame, selector.to_string(), self.clone())
            }
            None => crate::protocol::FrameLocator::any_frame(frame, self.clone()),
        }
    }

    /// Returns a locator that matches elements containing the given text.
    ///
    /// By default, matching is case-insensitive and searches for a substring.
    /// Set `exact` to `true` for case-sensitive exact matching.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-text>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub fn get_by_text(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(crate::protocol::locator::get_by_text_selector(text, exact))
    }

    /// Returns a locator that matches elements by their associated label text.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-label>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub fn get_by_label(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(crate::protocol::locator::get_by_label_selector(text, exact))
    }

    /// Returns a locator that matches elements by their placeholder text.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-placeholder>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub fn get_by_placeholder(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(crate::protocol::locator::get_by_placeholder_selector(
            text, exact,
        ))
    }

    /// Returns a locator that matches elements by their alt text.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-alt-text>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub fn get_by_alt_text(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(crate::protocol::locator::get_by_alt_text_selector(
            text, exact,
        ))
    }

    /// Returns a locator that matches elements by their title attribute.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-title>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub fn get_by_title(&self, text: &str, exact: bool) -> crate::protocol::Locator {
        self.locator(crate::protocol::locator::get_by_title_selector(text, exact))
    }

    /// Returns a locator that matches elements by their test ID attribute.
    ///
    /// By default, uses the `data-testid` attribute. Call
    /// [`playwright.selectors().set_test_id_attribute()`](crate::protocol::Selectors::set_test_id_attribute)
    /// to change the attribute name.
    ///
    /// Always uses exact matching (case-sensitive).
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-test-id>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub fn get_by_test_id(&self, test_id: &str) -> crate::protocol::Locator {
        let attr = self.connection().selectors().test_id_attribute();
        self.locator(crate::protocol::locator::get_by_test_id_selector_with_attr(
            test_id, &attr,
        ))
    }

    /// Returns a locator that matches elements by their ARIA role.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-get-by-role>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub fn get_by_role(
        &self,
        role: crate::protocol::locator::AriaRole,
        options: Option<crate::protocol::locator::GetByRoleOptions>,
    ) -> crate::protocol::Locator {
        self.locator(crate::protocol::locator::get_by_role_selector(
            role, options,
        ))
    }

    /// Installs an opt-in fake of the File System Access API
    /// (`showSaveFilePicker` / `showOpenFilePicker`) on this page, so
    /// save/open flows are testable without a native picker dialog.
    ///
    /// Returns a [`FakeFileSystem`](crate::testing::FakeFileSystem) handle
    /// for seeding openable files, reading back saved bytes, and controlling
    /// the permission state. Install before the flow under test runs; see
    /// the [`testing`](crate::testing) module docs for the pattern. Pages
    /// that never call this keep the browser's real picker functions.
    ///
    /// This is a playwright-rs convenience with no upstream Playwright
    /// equivalent (upstream cannot drive the native pickers either; see
    /// <https://github.com/microsoft/playwright/issues/11288>).
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed or installing the script
    /// fails.
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn fake_file_system(&self) -> Result<crate::testing::FakeFileSystem> {
        crate::testing::FakeFileSystem::install(self).await
    }

    /// Brings this page to the front (activates the tab).
    ///
    /// Activates the page in the browser, making it the focused tab. This is
    /// useful in multi-page tests to ensure actions target the correct page.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Page has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-bring-to-front>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn bring_to_front(&self) -> Result<()> {
        self.channel()
            .send_no_result("bringToFront", serde_json::json!({}))
            .await
    }

    /// Clears all element highlights drawn by [`Locator::highlight`](crate::protocol::Locator::highlight).
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-hide-highlight>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn hide_highlight(&self) -> Result<()> {
        self.channel()
            .send_no_result("hideHighlight", serde_json::json!({}))
            .await
    }

    /// Forces garbage collection in the browser (Chromium only).
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-request-gc>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn request_gc(&self) -> Result<()> {
        self.channel()
            .send_no_result("requestGC", serde_json::json!({}))
            .await
    }

    /// Enters Playwright Inspector's interactive picker mode and resolves
    /// once the user clicks an element. The returned [`Locator`](crate::Locator) points at
    /// whatever element was clicked.
    ///
    /// This is the programmatic entry point to the same picker the
    /// Playwright Inspector and codegen tools use. It only resolves after
    /// a real DOM click — synthetic clicks (e.g. via `page.mouse.click`)
    /// do **not** complete the picker. To abort the picker without a
    /// click, call [`Page::cancel_pick_locator`] from a different async
    /// context.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-pick-locator>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn pick_locator(&self) -> Result<crate::protocol::Locator> {
        #[derive(serde::Deserialize)]
        struct PickLocatorResponse {
            selector: String,
        }
        let response: PickLocatorResponse = self
            .channel()
            .send("pickLocator", serde_json::json!({}))
            .await?;
        Ok(self.locator(&response.selector))
    }

    /// Cancels an in-progress [`Page::pick_locator`] call. Has no effect
    /// if the picker is not currently active.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-cancel-pick-locator>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn cancel_pick_locator(&self) -> Result<()> {
        self.channel()
            .send_no_result("cancelPickLocator", serde_json::json!({}))
            .await
    }

    /// Access the current origin's `localStorage`.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-local-storage>
    pub fn local_storage(&self) -> crate::protocol::WebStorage {
        crate::protocol::WebStorage::new(
            self.channel().clone(),
            crate::protocol::WebStorageKind::Local,
        )
    }

    /// Access the current origin's `sessionStorage`.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-session-storage>
    pub fn session_storage(&self) -> crate::protocol::WebStorage {
        crate::protocol::WebStorage::new(
            self.channel().clone(),
            crate::protocol::WebStorageKind::Session,
        )
    }
}
