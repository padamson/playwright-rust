// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Selectors — custom selector engine registration
//
// Architecture Reference:
// - Python: playwright-python/playwright/_impl/_selectors.py
// - JavaScript: playwright/packages/playwright-core/src/client/selectors.ts
// - Docs: https://playwright.dev/docs/api/class-selectors
//
// IMPORTANT: Unlike most Playwright objects, Selectors is NOT a ChannelOwner.
// It does not have its own GUID or server-side representation. Instead, it is
// a pure client-side coordinator that:
// 1. Tracks registered selector engines and test ID attribute
// 2. Applies state to all registered BrowserContext objects via their channels
// 3. Stores engine definitions so they can be re-applied to new contexts
//
// This matches the Python and JavaScript implementations exactly.

//! Selectors — register custom selector engines and configure test ID attribute.
//!
//! Selectors can be used to install custom selector engines. Custom selector engines
//! are consulted when Playwright evaluates CSS, XPath, and other selectors.
//!
//! Unlike most Playwright objects, `Selectors` is a pure client-side coordinator;
//! it does not correspond to a server-side protocol object. It keeps track of
//! registered engines and the test ID attribute, and propagates changes to all
//! active browser contexts.
//!
//! # Example
//!
//! ```ignore
//! use playwright_rs::protocol::Playwright;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!
//!     // Access the shared Selectors instance
//!     let selectors = playwright.selectors();
//!
//!     // Register a custom selector engine
//!     let script = r#"
//!         {
//!             query(root, selector) {
//!                 return root.querySelector(selector);
//!             },
//!             queryAll(root, selector) {
//!                 return Array.from(root.querySelectorAll(selector));
//!             }
//!         }
//!     "#;
//!     selectors.register("tag", script, None).await?;
//!
//!     // Change the attribute used by get_by_test_id()
//!     selectors.set_test_id_attribute("data-custom-id").await?;
//!
//!     playwright.shutdown().await?;
//!     Ok(())
//! }
//! ```
//!
//! See: <https://playwright.dev/docs/api/class-selectors>

use crate::error::{Error, Result};
use crate::server::channel::Channel;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::Arc;

/// A registered selector engine definition.
#[derive(Clone, Debug)]
struct SelectorEngine {
    name: String,
    script: String,
    content_script: bool,
}

/// Inner shared state for `Selectors`.
struct SelectorsInner {
    /// Registered selector engine definitions (kept so they can be re-applied to new contexts).
    engines: Vec<SelectorEngine>,
    /// Names of registered engines (for duplicate detection).
    engine_names: HashSet<String>,
    /// Custom test ID attribute name, if overridden.
    test_id_attribute: Option<String>,
    /// Active browser contexts that need to receive selector updates.
    contexts: Vec<Channel>,
}

/// Selectors — manages custom selector engines and test ID attribute configuration.
///
/// An instance of Selectors is available via [`crate::protocol::Playwright::selectors()`].
///
/// Selector engines registered here are applied to all browser contexts. Register
/// engines **before** creating pages that will use them.
///
/// See: <https://playwright.dev/docs/api/class-selectors>
#[derive(Clone)]
pub struct Selectors {
    inner: Arc<Mutex<SelectorsInner>>,
}

impl Selectors {
    /// Creates a new, empty Selectors coordinator.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SelectorsInner {
                engines: Vec::new(),
                engine_names: HashSet::new(),
                test_id_attribute: None,
                contexts: Vec::new(),
            })),
        }
    }

    /// Registers a context's channel so it receives selector updates.
    ///
    /// Called by BrowserContext when it is created, so that:
    /// 1. All previously registered engines are applied to it immediately.
    /// 2. Future `register()` / `set_test_id_attribute()` calls reach it.
    pub async fn add_context(&self, channel: Channel) -> Result<()> {
        let (engines_snapshot, attr_snapshot) = {
            let mut inner = self.inner.lock();
            inner.contexts.push(channel.clone());
            (inner.engines.clone(), inner.test_id_attribute.clone())
        };

        // Re-apply all previously registered engines to this new context.
        for engine in &engines_snapshot {
            let params = serde_json::json!({
                "selectorEngine": {
                    "name": engine.name,
                    "source": engine.script,
                    "contentScript": engine.content_script,
                }
            });
            channel
                .send_no_result("registerSelectorEngine", params)
                .await?;
        }

        // Apply the current test ID attribute, if any.
        if let Some(attr) = attr_snapshot {
            channel
                .send_no_result(
                    "setTestIdAttributeName",
                    serde_json::json!({ "testIdAttributeName": attr }),
                )
                .await?;
        }

        Ok(())
    }

    /// Removes a context's channel when it is closed.
    ///
    /// Called by BrowserContext on close to avoid sending messages to dead channels.
    pub fn remove_context(&self, channel: &Channel) {
        let mut inner = self.inner.lock();
        inner.contexts.retain(|c| c.guid() != channel.guid());
    }

    /// Registers a custom selector engine.
    ///
    /// The script must evaluate to an object with `query` and `queryAll` methods:
    ///
    /// ```text
    /// {
    ///     query(root, selector) { return root.querySelector(selector); },
    ///     queryAll(root, selector) { return Array.from(root.querySelectorAll(selector)); }
    /// }
    /// ```
    ///
    /// After registration, use the engine with `page.locator("name=selector")`.
    ///
    /// # Arguments
    ///
    /// * `name` - Name to assign to the selector engine.
    /// * `script` - JavaScript string that evaluates to a selector engine factory.
    /// * `content_script` - Whether to run the engine in isolated content script mode.
    ///   Defaults to `false`.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - A selector engine with the same name is already registered
    /// - Any context rejects the registration (invalid script, etc.)
    ///
    /// See: <https://playwright.dev/docs/api/class-selectors#selectors-register>
    pub async fn register(
        &self,
        name: &str,
        script: &str,
        content_script: Option<bool>,
    ) -> Result<()> {
        let content_script = content_script.unwrap_or(false);

        let channels_snapshot = {
            let mut inner = self.inner.lock();

            if inner.engine_names.contains(name) {
                return Err(Error::ProtocolError(format!(
                    "Selector engine '{name}' is already registered"
                )));
            }

            inner.engine_names.insert(name.to_string());
            inner.engines.push(SelectorEngine {
                name: name.to_string(),
                script: script.to_string(),
                content_script,
            });

            inner.contexts.clone()
        };

        // The protocol expects { "selectorEngine": { "name": ..., "source": ..., "contentScript": ... } }
        let params = serde_json::json!({
            "selectorEngine": {
                "name": name,
                "source": script,
                "contentScript": content_script,
            }
        });

        // Broadcast to all active contexts.
        for channel in &channels_snapshot {
            channel
                .send_no_result("registerSelectorEngine", params.clone())
                .await?;
        }

        Ok(())
    }

    /// Returns the current test ID attribute name used by `get_by_test_id()` locators.
    ///
    /// Defaults to `"data-testid"`.
    pub fn test_id_attribute(&self) -> String {
        self.inner
            .lock()
            .test_id_attribute
            .clone()
            .unwrap_or_else(|| "data-testid".to_string())
    }

    /// Sets the attribute used by `get_by_test_id()` locators.
    ///
    /// By default, Playwright uses `data-testid`. Calling this method changes the
    /// attribute name for all current and future contexts.
    ///
    /// # Arguments
    ///
    /// * `attribute` - The attribute name to use as the test ID (e.g., `"data-custom-id"`).
    ///
    /// # Errors
    ///
    /// Returns error if any context rejects the update.
    ///
    /// See: <https://playwright.dev/docs/api/class-selectors#selectors-set-test-id-attribute>
    pub async fn set_test_id_attribute(&self, attribute: &str) -> Result<()> {
        let channels_snapshot = {
            let mut inner = self.inner.lock();
            inner.test_id_attribute = Some(attribute.to_string());
            inner.contexts.clone()
        };

        let params = serde_json::json!({ "testIdAttributeName": attribute });

        for channel in &channels_snapshot {
            channel
                .send_no_result("setTestIdAttributeName", params.clone())
                .await?;
        }

        Ok(())
    }
}

impl Default for Selectors {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Selectors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.lock();
        f.debug_struct("Selectors")
            .field("engines", &inner.engines)
            .field("test_id_attribute", &inner.test_id_attribute)
            .finish()
    }
}
