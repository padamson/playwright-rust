//! WebStorage: per-origin `localStorage` / `sessionStorage` access.
//!
//! Obtained via [`Page::local_storage`](crate::protocol::Page::local_storage) /
//! [`Page::session_storage`](crate::protocol::Page::session_storage). Reads and
//! writes the current origin's storage directly through the Page channel (not via
//! `page.evaluate`), matching playwright-python's `WebStorage`.
//!
//! ```no_run
//! # use playwright_rs::Playwright;
//! # async fn ex() -> playwright_rs::Result<()> {
//! # let pw = Playwright::launch().await?;
//! # let browser = pw.chromium().launch().await?;
//! # let page = browser.new_page().await?;
//! page.goto("https://example.com", None).await?;
//! let storage = page.local_storage();
//! storage.set_item("token", "abc123").await?;
//! assert_eq!(storage.get_item("token").await?, Some("abc123".to_string()));
//! # Ok(())
//! # }
//! ```
//!
//! See: <https://playwright.dev/docs/api/class-webstorage>

use crate::error::Result;
use crate::protocol::page::Page;
use crate::server::channel_owner::ChannelOwner;
use serde_json::json;

/// Which storage area a [`WebStorage`] handle targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebStorageKind {
    /// `window.localStorage` — persists across sessions for the origin.
    Local,
    /// `window.sessionStorage` — cleared when the tab/context closes.
    Session,
}

impl WebStorageKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            WebStorageKind::Local => "local",
            WebStorageKind::Session => "session",
        }
    }
}

/// Read/write access to a page's `localStorage` or `sessionStorage` for the
/// current origin.
///
/// Obtained from [`Page::local_storage`](crate::protocol::Page::local_storage)
/// and [`Page::session_storage`](crate::protocol::Page::session_storage).
///
/// See: <https://playwright.dev/docs/api/class-webstorage>
#[derive(Debug, Clone)]
pub struct WebStorage {
    page: Page,
    kind: WebStorageKind,
}

impl WebStorage {
    pub(crate) fn new(page: Page, kind: WebStorageKind) -> Self {
        Self { page, kind }
    }

    /// Returns the value for `name`, or `None` if the key is not set.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The page has been closed
    /// - Communication with the browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-webstorage#web-storage-get-item>
    pub async fn get_item(&self, name: &str) -> Result<Option<String>> {
        #[derive(serde::Deserialize)]
        struct R {
            #[serde(default)]
            value: Option<String>,
        }
        let r: R = self
            .page
            .channel()
            .send(
                "webStorageGetItem",
                json!({ "kind": self.kind.as_str(), "name": name }),
            )
            .await?;
        Ok(r.value)
    }

    /// Sets `name` to `value`.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The page has been closed
    /// - Communication with the browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-webstorage#web-storage-set-item>
    pub async fn set_item(&self, name: &str, value: &str) -> Result<()> {
        self.page
            .channel()
            .send_no_result(
                "webStorageSetItem",
                json!({ "kind": self.kind.as_str(), "name": name, "value": value }),
            )
            .await
    }

    /// Removes `name` from storage (no-op if absent).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The page has been closed
    /// - Communication with the browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-webstorage#web-storage-remove-item>
    pub async fn remove_item(&self, name: &str) -> Result<()> {
        self.page
            .channel()
            .send_no_result(
                "webStorageRemoveItem",
                json!({ "kind": self.kind.as_str(), "name": name }),
            )
            .await
    }

    /// Removes all entries from this storage area.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The page has been closed
    /// - Communication with the browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-webstorage#web-storage-clear>
    pub async fn clear(&self) -> Result<()> {
        self.page
            .channel()
            .send_no_result("webStorageClear", json!({ "kind": self.kind.as_str() }))
            .await
    }

    /// Returns all `(name, value)` entries currently in this storage area.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - The page has been closed
    /// - Communication with the browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-webstorage#web-storage-items>
    pub async fn items(&self) -> Result<Vec<(String, String)>> {
        #[derive(serde::Deserialize)]
        struct Item {
            name: String,
            value: String,
        }
        #[derive(serde::Deserialize)]
        struct R {
            items: Vec<Item>,
        }
        let r: R = self
            .page
            .channel()
            .send("webStorageItems", json!({ "kind": self.kind.as_str() }))
            .await?;
        Ok(r.items.into_iter().map(|i| (i.name, i.value)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::WebStorageKind;

    #[test]
    fn kind_as_str_maps_each_variant() {
        assert_eq!(WebStorageKind::Local.as_str(), "local");
        assert_eq!(WebStorageKind::Session.as_str(), "session");
    }
}
