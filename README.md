# Playwright for Rust

> 🎭 Rust language bindings for [Microsoft Playwright](https://playwright.dev)

**Status:** 🚧 Active Development - Not yet ready for production use

## 🎯 Why playwright-rust?

Read our [WHY.md](WHY.md) to understand the vision, timing, and philosophy behind this project.

**TL;DR:** Rust is emerging as a serious web development language, with frameworks like Axum and Actix gaining traction. AI coding assistants are making Rust accessible to more developers. Test-Driven Development is experiencing a renaissance as the optimal way to work with AI agents.  **These trends are converging now, and they need production-quality E2E testing.** `playwright-rust` fills that gap by bringing Playwright's industry-leading browser automation to the Rust ecosystem.

## Roadmap and Goals

See [Development Roadmap](docs/roadmap.md) for plans and status of the development approach for `playwright-rust`.

**Goal:** Build this library to a production-quality state for broad adoption as `@playwright/rust` or `playwright-rs`. Provide official-quality Rust bindings for Microsoft Playwright, following the same architecture as [playwright-python](https://github.com/microsoft/playwright-python), [playwright-java](https://github.com/microsoft/playwright-java), and [playwright-dotnet](https://github.com/microsoft/playwright-dotnet).

## How It Works

`playwright-rust` follows Microsoft's proven architecture for language bindings:

```
┌──────────────────────────────────────────────┐
│ playwright-rs (Rust API)                     │
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

### API Design Philosophy

Following Playwright's cross-language consistency:

1. **Match Playwright API exactly** - Same method names, same semantics
2. **Idiomatic Rust** - Use Result<T>, async/await, builder patterns where appropriate
3. **Type safety** - Leverage Rust's type system for compile-time safety
4. **Auto-waiting** - Built-in smart waits like other Playwright implementations
5. **Testing-first** - Designed for reliable end-to-end testing

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
playwright-rs = "0.8"  # Auto-updates to latest 0.8.x
tokio = { version = "1", features = ["full"] }
```

See the [CHANGELOG](CHANGELOG.md) for version history and features.

### Browser Installation (Required)

**Important:** Browsers must be installed separately using the Playwright CLI.

The library bundles Playwright driver version **1.56.1**. You must install matching browser versions:

```bash
# Install all browsers (recommended)
npx playwright@1.56.1 install

# Or install specific browsers
npx playwright@1.56.1 install chromium firefox webkit
```

**Why version matters:** Each Playwright release expects specific browser builds. Using `playwright@1.56.1` ensures you get compatible browsers (chromium-1194, firefox-1495, webkit-2215).

**In CI/CD:** Add this to your GitHub Actions workflow:

```yaml
- name: Install Playwright Browsers
  run: npx playwright@1.56.1 install chromium firefox webkit --with-deps
```

The version constant is also available in code:

```rust
use playwright_rs::PLAYWRIGHT_VERSION;

println!("Install with: npx playwright@{} install", PLAYWRIGHT_VERSION);
```

**What happens if I don't install browsers?** You'll get a helpful error message with the correct install command when trying to launch a browser.

## Development

### Prerequisites

- Rust 1.85+
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

After building, install browsers as described in [Browser Installation](#browser-installation-required) above:

```bash
cargo build
npx playwright@1.56.1 install chromium firefox webkit
```

The build script automatically downloads the Playwright driver to `drivers/` (gitignored). CI handles browser installation automatically - see `.github/workflows/test.yml`.

**Platform Support:** ✅ Windows, macOS, Linux

### Running Tests

**Note:** This project uses [cargo-nextest](https://nexte.st/) for faster test execution. Install it once globally:
```bash
cargo install cargo-nextest
```

```bash
# All tests (recommended - faster)
cargo nextest run

# All tests (standard cargo)
cargo test

# Integration tests only (requires browsers)
cargo nextest run --test '*'

# Specific test
cargo nextest run test_launch_chromium

# With logging
RUST_LOG=debug cargo nextest run

# Doc-tests (nextest doesn't run these)
# See CLAUDE.md "Documentation Testing Strategy" for details

# Compile-only check (fast, used in pre-commit)
cargo test --doc --workspace

# Execute all ignored doctests (requires browsers, what CI does)
cargo test --doc --workspace -- --ignored

# Execute specific crate's doctests
cargo test --doc -p playwright-rs -- --ignored
```

### Running Examples

> **Note:** See [examples/](crates/playwright/examples/) for usage examples.

```bash
# Run a single example
cargo run --package playwright-rs --example basic

# Run all examples
for example in crates/playwright/examples/*.rs; do
    cargo run --package playwright-rs --example $(basename "$example" .rs) || exit 1
done
```

## Contributing

This project aims for **production-quality** Rust bindings matching Playwright's standards. Contributions should:

- Follow Playwright API conventions
- Include comprehensive tests
- Maintain type safety
- Document public APIs with examples
- Pass CI checks (fmt, clippy, tests)

## License

Apache-2.0 (same as Microsoft Playwright)

## Acknowledgments

- **Microsoft Playwright Team** - For the amazing browser automation framework
- **playwright-python** - API design reference
- **Folio Project** - Initial driver for development needs
