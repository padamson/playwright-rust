// Copyright 2026 Paul Adamson
// Licensed under the Apache License, Version 2.0
//
// Device descriptor types for browser emulation.
//
// See: <https://playwright.dev/docs/emulation>

use serde::Deserialize;

/// Viewport dimensions for a device descriptor.
#[derive(Clone, Debug, Deserialize)]
pub struct DeviceViewport {
    /// The viewport width in CSS pixels.
    pub width: i32,
    /// The viewport height in CSS pixels.
    pub height: i32,
}

/// Describes a device for browser emulation.
///
/// Use with `BrowserContext::new_context()` options to emulate a specific device,
/// matching the behavior of `playwright.devices["iPhone 13"]` in Python/JS.
///
/// Device descriptors are accessed by name from [`Playwright::devices`].
/// The name is the map key, not a field of the descriptor itself.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::Playwright;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///
///     let iphone = &playwright.devices()["iPhone 13"];
///     assert!(iphone.is_mobile);
///     assert!(iphone.has_touch);
///     assert_eq!(iphone.default_browser_type, "webkit");
///
///     playwright.shutdown().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-playwright#playwright-devices>
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDescriptor {
    /// The user-agent string for the device.
    pub user_agent: String,
    /// The viewport dimensions.
    pub viewport: DeviceViewport,
    /// The device pixel ratio (e.g., `3.0` for Retina displays).
    pub device_scale_factor: f64,
    /// Whether the device is a mobile device.
    pub is_mobile: bool,
    /// Whether the device supports touch input.
    pub has_touch: bool,
    /// The default browser type for the device: `"chromium"`, `"firefox"`, or `"webkit"`.
    pub default_browser_type: String,
}
