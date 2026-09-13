use super::Page;
use crate::error::Result;
use crate::server::channel_owner::ChannelOwner;

/// Input devices: keyboard, mouse and touchscreen.
impl Page {
    /// Returns the keyboard instance for low-level keyboard control.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-keyboard>
    pub fn keyboard(&self) -> crate::protocol::Keyboard {
        crate::protocol::Keyboard::new(self.clone())
    }

    /// Returns the mouse instance for low-level mouse control.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-mouse>
    pub fn mouse(&self) -> crate::protocol::Mouse {
        crate::protocol::Mouse::new(self.clone())
    }

    /// Returns the touchscreen instance for low-level touch input simulation.
    ///
    /// Requires a touch-enabled browser context (`has_touch: true` in
    /// [`BrowserContextOptions`](crate::protocol::browser_context::BrowserContext)).
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-touchscreen>
    pub fn touchscreen(&self) -> crate::protocol::Touchscreen {
        crate::protocol::Touchscreen::new(self.clone())
    }

    pub(crate) async fn keyboard_down(&self, key: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "keyboardDown",
                serde_json::json!({
                    "key": key
                }),
            )
            .await
    }

    pub(crate) async fn keyboard_up(&self, key: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "keyboardUp",
                serde_json::json!({
                    "key": key
                }),
            )
            .await
    }

    pub(crate) async fn keyboard_press(
        &self,
        key: &str,
        options: Option<crate::protocol::KeyboardOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "key": key
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("keyboardPress", params).await
    }

    pub(crate) async fn keyboard_type(
        &self,
        text: &str,
        options: Option<crate::protocol::KeyboardOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "text": text
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("keyboardType", params).await
    }

    pub(crate) async fn keyboard_insert_text(&self, text: &str) -> Result<()> {
        self.channel()
            .send_no_result(
                "keyboardInsertText",
                serde_json::json!({
                    "text": text
                }),
            )
            .await
    }

    // Internal mouse methods (called by Mouse struct)

    pub(crate) async fn mouse_move(
        &self,
        x: f64,
        y: f64,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "x": x,
            "y": y
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("mouseMove", params).await
    }

    pub(crate) async fn mouse_click(
        &self,
        x: f64,
        y: f64,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "x": x,
            "y": y
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("mouseClick", params).await
    }

    pub(crate) async fn mouse_dblclick(
        &self,
        x: f64,
        y: f64,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({
            "x": x,
            "y": y,
            "clickCount": 2
        });

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("mouseClick", params).await
    }

    pub(crate) async fn mouse_down(
        &self,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("mouseDown", params).await
    }

    pub(crate) async fn mouse_up(
        &self,
        options: Option<crate::protocol::MouseOptions>,
    ) -> Result<()> {
        let mut params = serde_json::json!({});

        if let Some(opts) = options {
            let opts_json = opts.to_json();
            if let Some(obj) = params.as_object_mut()
                && let Some(opts_obj) = opts_json.as_object()
            {
                obj.extend(opts_obj.clone());
            }
        }

        self.channel().send_no_result("mouseUp", params).await
    }

    pub(crate) async fn mouse_wheel(&self, delta_x: f64, delta_y: f64) -> Result<()> {
        self.channel()
            .send_no_result(
                "mouseWheel",
                serde_json::json!({
                    "deltaX": delta_x,
                    "deltaY": delta_y
                }),
            )
            .await
    }

    // Internal touchscreen method (called by Touchscreen struct)

    pub(crate) async fn touchscreen_tap(&self, x: f64, y: f64) -> Result<()> {
        self.channel()
            .send_no_result(
                "touchscreenTap",
                serde_json::json!({
                    "x": x,
                    "y": y
                }),
            )
            .await
    }

    /// Performs a drag from source selector to target selector.
    ///
    /// This is the page-level equivalent of `Locator::drag_to()`. It resolves
    /// both selectors in the main frame and performs the drag.
    ///
    /// # Arguments
    ///
    /// * `source` - A CSS selector for the element to drag from
    /// * `target` - A CSS selector for the element to drop onto
    /// * `options` - Optional drag options (positions, force, timeout, trial)
    ///
    /// # Errors
    ///
    /// Returns error if either selector does not resolve to an element, the
    /// drag action times out, or the page has been closed.
    ///
    /// See: <https://playwright.dev/docs/api/class-page#page-drag-and-drop>
    #[tracing::instrument(level = "info", skip_all, fields(guid = %self.guid()))]
    pub async fn drag_and_drop(
        &self,
        source: &str,
        target: &str,
        options: impl Into<Option<crate::protocol::DragToOptions>>,
    ) -> Result<()> {
        let options = options.into();
        let frame = self.main_frame().await?;
        frame.locator_drag_to(source, target, options).await
    }
}
