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

**Current Phase:** ✅ Phase 1 Complete! → Phase 2 Starting Soon

### Phase 1: Protocol Foundation (✅ Complete!)
- [x] **Slice 1:** Server management (download, launch, lifecycle)
- [x] **Slice 2:** Transport layer (stdio, length-prefixed messages)
- [x] **Slice 3:** Connection layer (JSON-RPC request/response correlation)
- [x] **Slice 4:** Object factory and channel owners
- [x] **Slice 5:** Entry point (`Playwright::launch()` and initialization flow)

### Upcoming Phases
- [ ] **Phase 2:** Browser API (Browser, Context, Page lifecycle, `BrowserType::launch()`)
- [ ] **Phase 3:** Page Interactions (navigation, locators, actions)
- [ ] **Phase 4:** Advanced Features (assertions, network interception, mobile)
- [ ] **Phase 5:** Production Hardening (comprehensive testing, docs, polish)

See [Development Roadmap](docs/roadmap.md) and [Phase 1 Implementation Plan](docs/implementation-plans/phase1-protocol-foundation.md) for detailed plans.

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
- Node.js 18+ (for Playwright server)
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

# Run tests
cargo test

# Run examples (requires PLAYWRIGHT_DRIVER_PATH)
PLAYWRIGHT_DRIVER_PATH=./drivers/playwright-1.49.0-mac-arm64 \
    cargo run --package playwright --example basic
```

### Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# Run specific test
cargo test test_browser_launch

# With logging
RUST_LOG=debug cargo test
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

1. 🚧 **Phase 1:** Protocol Foundation (in progress)
2. **Phase 2:** Browser API (Browser, Context, Page lifecycle)
3. **Phase 3:** Page Interactions (navigation, locators, actions)
4. **Phase 4:** Advanced Features (assertions, network, mobile)
5. **Phase 5:** Production Hardening (testing, docs, polish)

See [Development Roadmap](docs/roadmap.md) for detailed phase descriptions and timelines.

## License

Apache-2.0 (same as Microsoft Playwright)

## Acknowledgments

- **Microsoft Playwright Team** - For the amazing browser automation framework
- **playwright-python** - API design reference
- **Folio Project** - Initial driver for development needs
