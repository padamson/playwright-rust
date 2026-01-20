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

**⚠️ IMPORTANT:** Browsers are installed **automatically** after building the project!

When you run `cargo build`, the build script ([build.rs](crates/playwright/build.rs)) automatically:
1. Downloads the Playwright driver (version **1.56.1**) from Azure CDN
2. Extracts it to the appropriate location based on your setup:
   - **Workspace projects**: `drivers/playwright-1.56.1-<platform>/` in your workspace root
   - **Non-workspace projects**: Platform-specific cache directory (e.g., `~/.cache/playwright-rust/drivers/` on Linux/macOS)

The build script uses robust workspace detection to find the right location automatically.

After building, install browsers using the downloaded driver's CLI:

```bash
# Build the project (downloads Playwright 1.56.1 driver)
cargo build

# Install browsers using the driver's CLI
# macOS/Linux:
drivers/playwright-1.56.1-*/node drivers/playwright-1.56.1-*/package/cli.js install chromium firefox webkit

# Windows:
drivers\playwright-1.56.1-win32_x64\node.exe drivers\playwright-1.56.1-win32_x64\package\cli.js install chromium firefox webkit
```

**Platform-specific examples:**

```bash
# macOS (arm64):
drivers/playwright-1.56.1-mac-arm64/node drivers/playwright-1.56.1-mac-arm64/package/cli.js install chromium firefox webkit

# macOS (x64):
drivers/playwright-1.56.1-mac/node drivers/playwright-1.56.1-mac/package/cli.js install chromium firefox webkit

# Linux:
drivers/playwright-1.56.1-linux/node drivers/playwright-1.56.1-linux/package/cli.js install chromium firefox webkit
```

**Why this matters:**
- Playwright server 1.56.1 expects specific browser builds (chromium-1194, firefox-1495, webkit-2215)
- Using the driver's CLI ensures version compatibility
- The `drivers/` directory is gitignored, so each developer/CI environment installs its own

**Platform Support:**
- ✅ **Windows**: Full support with CI stability flags enabled (2025-11-09)
- ✅ **macOS**: Full support
- ✅ **Linux**: Full support

**Note:** CI automatically installs the correct browser versions - see `.github/workflows/test.yml`

**Verify installation:**
```bash
# Browsers are cached in:
# macOS: ~/Library/Caches/ms-playwright/
# Linux: ~/.cache/ms-playwright/
# Windows: %USERPROFILE%\AppData\Local\ms-playwright\

ls ~/Library/Caches/ms-playwright/
# Should show: chromium-1194, chromium_headless_shell-1194, firefox-1495, webkit-2215
```

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
