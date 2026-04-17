// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Clock — fake timer / time-manipulation API
//
// Architecture Reference:
// - Python: playwright-python/playwright/_impl/_clock.py
// - JavaScript: playwright/packages/playwright-core/src/client/clock.ts
// - Docs: https://playwright.dev/docs/api/class-clock

//! Clock — manipulate fake timers for deterministic time-dependent tests
//!
//! The Clock object is accessible via [`crate::protocol::BrowserContext::clock`] or
//! [`crate::protocol::Page::clock`]. All RPC calls are sent on the BrowserContext channel.
//!
//! # Example
//!
//! ```ignore
//! use playwright_rs::protocol::{Playwright, ClockInstallOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let playwright = Playwright::launch().await?;
//!     let browser = playwright.chromium().launch().await?;
//!     let context = browser.new_context().await?;
//!     let page = context.new_page().await?;
//!
//!     let clock = context.clock();
//!
//!     // Install fake timers, optionally setting an initial time (ms since epoch)
//!     clock.install(Some(ClockInstallOptions { time: Some(0) })).await?;
//!
//!     // Freeze time at a fixed point
//!     clock.set_fixed_time(1_000_000).await?;
//!
//!     // Verify via evaluate
//!     let now: f64 = page.evaluate_value("Date.now()").await?.parse()?;
//!     assert_eq!(now as u64, 1_000_000);
//!
//!     // Advance time by 5 seconds
//!     clock.fast_forward(5_000).await?;
//!
//!     // Pause at a specific instant
//!     clock.pause_at(2_000_000).await?;
//!
//!     // Resume normal flow
//!     clock.resume().await?;
//!
//!     context.close().await?;
//!     browser.close().await?;
//!     Ok(())
//! }
//! ```
//!
//! See: <https://playwright.dev/docs/api/class-clock>

use crate::error::Result;
use crate::server::channel::Channel;

/// Options for [`Clock::install`].
///
/// See: <https://playwright.dev/docs/api/class-clock#clock-install>
#[derive(Debug, Clone, Default)]
pub struct ClockInstallOptions {
    /// Initial time for the fake clock in milliseconds since the Unix epoch.
    /// When `None`, the clock starts at the current real time.
    pub time: Option<u64>,
}

/// Playwright Clock — provides fake timer control for deterministic tests.
///
/// All methods send RPC calls on the owning BrowserContext channel.
///
/// See: <https://playwright.dev/docs/api/class-clock>
#[derive(Clone)]
pub struct Clock {
    channel: Channel,
}

impl std::fmt::Debug for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Clock").finish_non_exhaustive()
    }
}

impl Clock {
    /// Creates a new Clock backed by the given BrowserContext channel.
    pub fn new(channel: Channel) -> Self {
        Self { channel }
    }

    /// Installs fake timers, replacing the browser's built-in clock APIs
    /// (`Date`, `setTimeout`, `setInterval`, etc.) with controlled equivalents.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional configuration; set `time` to fix the starting epoch
    ///   timestamp in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-clock#clock-install>
    pub async fn install(&self, options: Option<ClockInstallOptions>) -> Result<()> {
        let mut params = serde_json::json!({});
        if let Some(opts) = options
            && let Some(time) = opts.time
        {
            params["timeNumber"] = serde_json::Value::Number(time.into());
        }
        self.channel.send_no_result("clockInstall", params).await
    }

    /// Advances the fake clock by the given number of milliseconds, firing any
    /// timers that fall within that range.
    ///
    /// # Arguments
    ///
    /// * `ticks` - Number of milliseconds to advance the clock.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Clock is not installed
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-clock#clock-fast-forward>
    pub async fn fast_forward(&self, ticks: u64) -> Result<()> {
        self.channel
            .send_no_result(
                "clockFastForward",
                serde_json::json!({ "ticksNumber": ticks }),
            )
            .await
    }

    /// Pauses the fake clock at the given epoch timestamp (milliseconds).
    ///
    /// No timers fire and time does not advance until [`Clock::resume`] is called.
    ///
    /// # Arguments
    ///
    /// * `time` - Epoch timestamp in milliseconds to pause at.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Clock is not installed
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-clock#clock-pause-at>
    pub async fn pause_at(&self, time: u64) -> Result<()> {
        self.channel
            .send_no_result("clockPauseAt", serde_json::json!({ "timeNumber": time }))
            .await
    }

    /// Resumes the fake clock after it was paused via [`Clock::pause_at`].
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-clock#clock-resume>
    pub async fn resume(&self) -> Result<()> {
        self.channel
            .send_no_result("clockResume", serde_json::json!({}))
            .await
    }

    /// Freezes `Date.now()` and related APIs at the given epoch timestamp
    /// (milliseconds), without affecting timer scheduling.
    ///
    /// # Arguments
    ///
    /// * `time` - Epoch timestamp in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-clock#clock-set-fixed-time>
    pub async fn set_fixed_time(&self, time: u64) -> Result<()> {
        self.channel
            .send_no_result(
                "clockSetFixedTime",
                serde_json::json!({ "timeNumber": time }),
            )
            .await
    }

    /// Updates the system time reported by `Date` and related APIs without
    /// freezing the clock or affecting timer scheduling.
    ///
    /// # Arguments
    ///
    /// * `time` - Epoch timestamp in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-clock#clock-set-system-time>
    pub async fn set_system_time(&self, time: u64) -> Result<()> {
        self.channel
            .send_no_result(
                "clockSetSystemTime",
                serde_json::json!({ "timeNumber": time }),
            )
            .await
    }
}
