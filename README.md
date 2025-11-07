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

```rust
use playwright_core::protocol::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Launch Playwright
    let playwright = Playwright::launch().await?;

    // Launch a browser (Chromium, Firefox, or WebKit)
    let browser = playwright.chromium().launch().await?;

    // Create a page
    let page = browser.new_page().await?;

    // Navigate to a URL
    let response = page.goto("https://example.com", None).await?;
    println!("Response status: {}", response.status());
    println!("Page URL: {}", page.url());
    println!("Page title: {}", page.title().await?);

    // Cleanup
    page.close().await?;
    browser.close().await?;

    Ok(())
}
```

> **Note:** Element interaction (locators, click, fill) and assertions are coming soon.
> See [Development Roadmap](docs/roadmap.md) for details.

## Project Status

**What works now:**
- ✅ Launch browsers (Chromium, Firefox, WebKit)
- ✅ Create browser contexts and pages
- ✅ Page navigation (`goto()`, `reload()`, `title()`)
- ✅ URL tracking and response handling
- ✅ Proper lifecycle management and cleanup

**Coming next:** Locators, element interactions, screenshots

See [Development Roadmap](docs/roadmap.md) for the complete vision and timeline.

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

**Platform Support:**
- ✅ **macOS/Linux**: Full support - all tests pass
- ⚠️ **Windows**: Known issue - integration tests hang due to stdio pipe cleanup (Phase 1 deferred issue)
  - Unit tests work fine on Windows
  - CI runs unit tests only on Windows
  - Will be fixed when implementing proper cleanup (Browser::close() or later)

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

## Roadmap

See [Development Roadmap](docs/roadmap.md) for the complete vision and timeline.

## License

Apache-2.0 (same as Microsoft Playwright)

## Acknowledgments

- **Microsoft Playwright Team** - For the amazing browser automation framework
- **playwright-python** - API design reference
- **Folio Project** - Initial driver for development needs
