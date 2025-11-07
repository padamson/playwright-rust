# Playwright for Rust

> 🎭 Rust language bindings for [Microsoft Playwright](https://playwright.dev)

**Status:** 🚧 Active Development - Not yet ready for production use

## Vision

Provide official-quality Rust bindings for Microsoft Playwright, following the same architecture as [playwright-python](https://github.com/microsoft/playwright-python), [playwright-java](https://github.com/microsoft/playwright-java), and [playwright-dotnet](https://github.com/microsoft/playwright-dotnet).

**Goal:** Build this library to a production-quality state for broad adoption as `@playwright/rust` or `playwright-rust`.

## How It Works

`playwright-rust` follows Microsoft's proven architecture for language bindings:

```
┌──────────────────────────────────────────────┐
│ playwright-rust (Rust API)                   │
│ - High-level, idiomatic Rust API             │
│ - Async/await with tokio                     │
│ - Type-safe bindings                         │
└─────────────────────┬────────────────────────┘
                      │ JSON-RPC over stdio
┌─────────────────────▼────────────────────────┐
│ Playwright Server (Node.js/TypeScript)       │
│ - Browser automation logic                   │
│ - Cross-browser protocol abstraction         │
│ - Maintained by Microsoft Playwright team    │
└─────────────────────┬────────────────────────┘
                      │ Native protocols
        ┌─────────────┼─────────────┐
        ▼             ▼             ▼
    Chromium      Firefox       WebKit
```

This means:
- ✅ **Full feature parity** with Playwright (JS/Python/Java/.NET)
- ✅ **Cross-browser support** (Chromium, Firefox, WebKit)
- ✅ **Automatic updates** when Playwright server updates
- ✅ **Minimal maintenance** - protocols handled by Microsoft's server
- ✅ **Production-tested** architecture used by millions

## Quick Example

### Phase 1: Protocol Foundation (✅ Complete!)

```rust
use playwright::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Launch Playwright and access browser types
    let playwright = Playwright::launch().await?;

    println!("Chromium: {}", playwright.chromium().executable_path());
    println!("Firefox: {}", playwright.firefox().executable_path());
    println!("WebKit: {}", playwright.webkit().executable_path());

    Ok(())
}
```

### Target API (Phase 2+)

> **Note:** Browser launching, page interactions, and assertions will be implemented in future phases.

```rust
use playwright::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;

    // Launch browser (Phase 2 - coming soon)
    let browser = playwright
        .chromium()
        .launch()
        .headless(true)
        .await?;

    // Create page
    let page = browser.new_page().await?;

    // Navigate and interact
    page.goto("https://playwright.dev").await?;

    // Use Playwright-style locators (Phase 3)
    let title = page.locator("h1").text_content().await?;
    println!("Title: {}", title);

    // Playwright assertions (Phase 4)
    playwright::expect(page.locator(".hero__title"))
        .to_be_visible()
        .await?;

    // Take screenshot
    page.screenshot()
        .path("screenshot.png")
        .await?;

    // Cleanup
    browser.close().await?;

    Ok(())
}
```

## Project Status

**Current Phase:** 🚀 Phase 2 in Progress (Slice 3/7 Complete)

### Phase 1: Protocol Foundation (✅ Complete!)
- [x] **Slice 1:** Server management (download, launch, lifecycle)
- [x] **Slice 2:** Transport layer (stdio, length-prefixed messages)
- [x] **Slice 3:** Connection layer (JSON-RPC request/response correlation)
- [x] **Slice 4:** Object factory and channel owners
- [x] **Slice 5:** Entry point (`Playwright::launch()` and initialization flow)

**Result:** JSON-RPC communication working, Playwright server integration complete. See [Phase 1 Technical Summary](docs/technical/phase1-technical-summary.md).

### Phase 2: Browser API (🚀 In Progress - 3/7 Slices Complete)
- [x] **Slice 1:** Browser object foundation (protocol object, ChannelOwner, object factory)
- [x] **Slice 2:** Launch options API (LaunchOptions struct, builder pattern, normalization)
- [x] **Slice 3:** BrowserType::launch() (RPC implementation, integration tests) ✅
- [ ] **Slice 4:** Browser::close() (graceful shutdown)
- [ ] **Slice 5:** BrowserContext object (contexts, isolation)
- [ ] **Slice 6:** Page object (page creation, basic methods)
- [ ] **Slice 7:** Documentation and examples

**Goal:** Enable browser launching and page lifecycle management. See [Phase 2 Implementation Plan](docs/implementation-plans/phase2-browser-api.md).

**Latest:** Can now launch browsers! All three browsers (Chromium, Firefox, WebKit) can be launched with the `BrowserType::launch()` API.

### Upcoming Phases
- [ ] **Phase 3:** Page Interactions (navigation, locators, actions)
- [ ] **Phase 4:** Advanced Features (assertions, network interception, mobile)
- [ ] **Phase 5:** Production Hardening (comprehensive testing, docs, polish)

See [Development Roadmap](docs/roadmap.md) for detailed phase descriptions.

## Installation

**Not yet published to crates.io** - Library is under active development.

Once published:
```toml
[dependencies]
playwright = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Development

### Prerequisites

- Rust 1.70+
- Node.js 18+ (for Playwright server and browser installation)
- tokio async runtime

### Building from Source

```bash
# Clone repository
git clone https://github.com/YOUR_USERNAME/playwright-rust.git
cd playwright-rust

# Install pre-commit hooks
pip install pre-commit
pre-commit install

# Build
cargo build
```

### Installing Browsers

**⚠️ IMPORTANT:** Browser versions must match the Playwright server version!

The Playwright server bundled in `drivers/` is version **1.49.0**. You must install matching browsers:

```bash
# Install browsers matching Playwright 1.49.0
npx playwright@1.49.0 install chromium firefox webkit
```

**Why this matters:**
- Playwright server 1.49.0 expects specific browser builds (e.g., chromium build 1148)
- If you run `npx playwright install` without version, you'll get the latest browsers
- Mismatched versions will cause "Executable doesn't exist" errors during tests

**Note:** CI automatically installs the correct browser versions - see `.github/workflows/test.yml`

**Verify installation:**
```bash
# Browsers are cached in:
# macOS: ~/Library/Caches/ms-playwright/
# Linux: ~/.cache/ms-playwright/
# Windows: %USERPROFILE%\AppData\Local\ms-playwright\

ls ~/Library/Caches/ms-playwright/
# Should show: chromium-1148, chromium_headless_shell-1148, firefox-1466, webkit-2104
```

### Running Tests

```bash
# All tests
cargo test

# Integration tests only (requires browsers)
cargo test --test '*'

# Specific test
cargo test test_launch_chromium

# With logging
RUST_LOG=debug cargo test
```

### Running Examples

```bash
# Set driver path and run example
PLAYWRIGHT_DRIVER_PATH=./drivers/playwright-1.49.0-mac-arm64 \
    cargo run --package playwright --example basic
```

## API Design Philosophy

Following Playwright's cross-language consistency:

1. **Match Playwright API exactly** - Same method names, same semantics
2. **Idiomatic Rust** - Use Result<T>, async/await, builder patterns where appropriate
3. **Type safety** - Leverage Rust's type system for compile-time safety
4. **Auto-waiting** - Built-in smart waits like other Playwright implementations
5. **Testing-first** - Designed for reliable end-to-end testing

## Comparison with Alternatives

| Library | Protocol | Cross-Browser | Playwright Compatible |
|---------|----------|---------------|----------------------|
| **playwright-rust** | JSON-RPC to Playwright | ✅ All 3 | ✅ Official API |
| fantoccini | WebDriver | ✅ Via drivers | ❌ Different API |
| thirtyfour | WebDriver | ✅ Via drivers | ❌ Different API |
| chromiumoxide | CDP | ❌ Chrome only | ❌ Different API |
| headless_chrome | CDP | ❌ Chrome only | ❌ Different API |

## Contributing

This project aims for **production-quality** Rust bindings matching Playwright's standards. Contributions should:

- Follow Playwright API conventions
- Include comprehensive tests
- Maintain type safety
- Document public APIs with examples
- Pass CI checks (fmt, clippy, tests)

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Roadmap to Broad Adoption

1. ✅ **Phase 1:** Protocol Foundation (complete!)
2. 🚀 **Phase 2:** Browser API - in progress (2/7 slices)
3. **Phase 3:** Page Interactions (navigation, locators, actions)
4. **Phase 4:** Advanced Features (assertions, network, mobile)
5. **Phase 5:** Production Hardening (testing, docs, polish)

See [Phase 2 Implementation Plan](docs/implementation-plans/phase2-browser-api.md) for current progress and [Development Roadmap](docs/roadmap.md) for overall timeline.

## License

Apache-2.0 (same as Microsoft Playwright)

## Acknowledgments

- **Microsoft Playwright Team** - For the amazing browser automation framework
- **playwright-python** - API design reference
- **Folio Project** - Initial driver for development needs
