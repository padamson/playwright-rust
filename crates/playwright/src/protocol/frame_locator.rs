//! FrameLocator for locating elements inside iframes.
//!
//! FrameLocator is a lightweight client-side object (like Locator) that represents
//! a view to an iframe on the page. It uses Playwright's `internal:control=enter-frame`
//! selector engine to cross iframe boundaries transparently.
//!
//! # Example
//!
//! ```ignore
//! use playwright_rs::Playwright;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!     let browser = playwright.chromium().launch().await?;
//!     let page = browser.new_page().await?;
//!
//!     page.goto("https://example.com", None).await?;
//!
//!     // Locate elements inside an iframe
//!     let frame = page.frame_locator("iframe#my-frame").await;
//!     frame.locator("button").click(None).await?;
//!
//!     // Use get_by_* methods inside the iframe
//!     frame.get_by_text("Submit", false).click(None).await?;
//!
//!     // Nested iframes
//!     let inner = frame.frame_locator("iframe.nested");
//!     inner.locator("h1").text_content().await?;
//!
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! See: <https://playwright.dev/docs/api/class-framelocator>

use crate::protocol::locator::{
    get_by_alt_text_selector, get_by_label_selector, get_by_placeholder_selector,
    get_by_role_selector, get_by_test_id_selector, get_by_text_selector, get_by_title_selector,
};
use crate::protocol::{AriaRole, Frame, GetByRoleOptions, Locator, Page};
use std::sync::Arc;

/// FrameLocator represents a view to an iframe on the page.
///
/// It is used to locate elements inside iframes. FrameLocator is not a
/// protocol object — it builds selector strings that include the special
/// `internal:control=enter-frame` directive, which the Playwright server's
/// selector engine understands to cross iframe boundaries.
///
/// See: <https://playwright.dev/docs/api/class-framelocator>
#[derive(Clone)]
pub struct FrameLocator {
    frame: Arc<Frame>,
    /// Selector ending with `>> internal:control=enter-frame`
    selector: String,
    page: Page,
}

impl FrameLocator {
    /// Creates a new FrameLocator from a frame selector.
    ///
    /// The `frame_selector` identifies the iframe element (e.g., `"iframe[name='content']"`).
    /// The resulting FrameLocator's internal selector appends `>> internal:control=enter-frame`.
    pub(crate) fn new(frame: Arc<Frame>, frame_selector: String, page: Page) -> Self {
        let selector = format!("{} >> internal:control=enter-frame", frame_selector);
        Self {
            frame,
            selector,
            page,
        }
    }

    /// Creates a [`Locator`] for elements inside this iframe.
    ///
    /// The resulting Locator's selector crosses the iframe boundary via the
    /// `internal:control=enter-frame` directive.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-locator>
    pub fn locator(&self, selector: &str) -> Locator {
        Locator::new(
            Arc::clone(&self.frame),
            format!("{} >> {}", self.selector, selector),
            self.page.clone(),
        )
    }

    /// Creates a nested [`FrameLocator`] for an iframe within this iframe.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-frame-locator>
    pub fn frame_locator(&self, selector: &str) -> FrameLocator {
        FrameLocator {
            frame: Arc::clone(&self.frame),
            selector: format!(
                "{} >> {} >> internal:control=enter-frame",
                self.selector, selector
            ),
            page: self.page.clone(),
        }
    }

    /// Returns a [`Locator`] for the iframe element itself (the `<iframe>` tag).
    ///
    /// This is the selector *without* the `>> internal:control=enter-frame` suffix.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-owner>
    pub fn owner(&self) -> Locator {
        // Strip the trailing " >> internal:control=enter-frame"
        let owner_selector = self
            .selector
            .strip_suffix(" >> internal:control=enter-frame")
            .unwrap_or(&self.selector)
            .to_string();
        Locator::new(Arc::clone(&self.frame), owner_selector, self.page.clone())
    }

    /// Returns a new FrameLocator matching the first iframe.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-first>
    pub fn first(&self) -> FrameLocator {
        self.nth(0)
    }

    /// Returns a new FrameLocator matching the last iframe.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-last>
    pub fn last(&self) -> FrameLocator {
        self.nth(-1)
    }

    /// Returns a new FrameLocator matching the nth iframe (0-indexed).
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-nth>
    pub fn nth(&self, index: i32) -> FrameLocator {
        // Insert nth before the enter-frame control:
        // "iframe >> internal:control=enter-frame"
        // becomes "iframe >> nth=N >> internal:control=enter-frame"
        let owner_selector = self
            .selector
            .strip_suffix(" >> internal:control=enter-frame")
            .unwrap_or(&self.selector);
        FrameLocator {
            frame: Arc::clone(&self.frame),
            selector: format!(
                "{} >> nth={} >> internal:control=enter-frame",
                owner_selector, index
            ),
            page: self.page.clone(),
        }
    }

    // ========================================================================
    // get_by_* convenience methods — delegate to self.locator(selector)
    // ========================================================================

    /// Locate by text content inside the iframe.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-get-by-text>
    pub fn get_by_text(&self, text: &str, exact: bool) -> Locator {
        self.locator(&get_by_text_selector(text, exact))
    }

    /// Locate by associated label text inside the iframe.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-get-by-label>
    pub fn get_by_label(&self, text: &str, exact: bool) -> Locator {
        self.locator(&get_by_label_selector(text, exact))
    }

    /// Locate by placeholder text inside the iframe.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-get-by-placeholder>
    pub fn get_by_placeholder(&self, text: &str, exact: bool) -> Locator {
        self.locator(&get_by_placeholder_selector(text, exact))
    }

    /// Locate by alt text inside the iframe.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-get-by-alt-text>
    pub fn get_by_alt_text(&self, text: &str, exact: bool) -> Locator {
        self.locator(&get_by_alt_text_selector(text, exact))
    }

    /// Locate by title attribute inside the iframe.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-get-by-title>
    pub fn get_by_title(&self, text: &str, exact: bool) -> Locator {
        self.locator(&get_by_title_selector(text, exact))
    }

    /// Locate by `data-testid` attribute inside the iframe.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-get-by-test-id>
    pub fn get_by_test_id(&self, test_id: &str) -> Locator {
        self.locator(&get_by_test_id_selector(test_id))
    }

    /// Locate by ARIA role inside the iframe.
    ///
    /// See: <https://playwright.dev/docs/api/class-framelocator#frame-locator-get-by-role>
    pub fn get_by_role(&self, role: AriaRole, options: Option<GetByRoleOptions>) -> Locator {
        self.locator(&get_by_role_selector(role, options))
    }
}

impl std::fmt::Debug for FrameLocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameLocator")
            .field("selector", &self.selector)
            .finish()
    }
}
