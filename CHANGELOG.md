# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.2] - 2025-12-24

### Added

- **Storage State Support** - `BrowserContextOptions` now supports `storageState` for session persistence (Issue #6)
  - `storage_state(StorageState)` - Load cookies and localStorage from inline object
  - `storage_state_path(String)` - Load storage state from JSON file
  - New types: `Cookie`, `LocalStorageItem`, `Origin`, `StorageState`
  - Enables authentication state persistence without re-login
  - Async file reading with proper error handling
- `Page::pause()` method for manual debugging (Issue #5)
  - Opens Playwright Inspector and pauses script execution
  - Delegates to new `BrowserContext::pause()` method

### Fixed

- Protocol serialization for methods with no arguments (fixed `ProtocolError` on `pause`)
- **Consistent Test Logging** - Refactored all integration tests to explicitly initialize tracing, ensuring protocol errors are captured and visible (Issue #4)

## [0.7.1] - 2025-12-24

### Added

- **Script Injection** - `BrowserContext.add_init_script()` for context-level script injection before page load
- **Script Injection** - `Page.add_init_script()` for page-level script injection before page load
- **Style Injection** - `Page.add_style_tag()` for injecting CSS into pages
  - `AddStyleTagOptions` struct with builder pattern
  - Support for inline `content` (CSS string)
  - Support for external `url` (stylesheet URL)
  - Support for `path` (load CSS from file)
  - Returns `ElementHandle` to the injected style tag

## [0.7.0] - 2025-11-16

### Added

- `Browser::is_connected()` method to check if the browser is still connected to the server (Issue #2)

### Changed

- **[BREAKING] Single-crate architecture** - Consolidated `playwright-core` into `playwright-rs` to match all official Playwright implementations (Python, Java, .NET, Node.js)
  - Merged all code from `playwright-core` into `playwright-rs` under `src/protocol/` and `src/server/`
  - Removed `playwright-core` crate from workspace
  - Updated all internal imports from `playwright_core::` to `playwright_rs::`
  - Server module now marked `#[doc(hidden)]` - exposed only for integration testing
  - **Migration**: Users of `playwright-rs` v0.6.x should see no API changes. Users of `playwright-core` should switch to `playwright-rs` (see deprecation notice in playwright-core v0.6.2)

### Fixed

- Resolved root cause of Issue #3 by eliminating two-crate complexity that caused workspace detection issues
- Updated all doctests to use consolidated crate structure
- Fixed integration test imports to use new module paths

### Internal

- All 248+ tests passing (library + integration + doctests)
- Maintained backward compatibility for public API
- Release workflow updated to publish single crate

**Related**: Issue #3, ADR 0003 (Single-Crate Architecture Decision)

## [0.6.1] - 2025-11-15

### Fixed

- **[Critical] Build script workspace detection** - Fixed issue #3 where `build.rs` failed to detect the correct workspace root when playwright-core is used as a crates.io dependency
  - Implemented robust three-tier detection strategy:
    1. Use `CARGO_WORKSPACE_DIR` (Rust 1.73+) to detect dependent project's workspace
    2. Walk up directory tree to find `Cargo.toml` with `[workspace]`
    3. Fallback to platform-specific cache directory (matches playwright-python behavior)
  - This fix unblocks usage of playwright-rust in downstream projects like Folio
  - Drivers now download to the correct location in all scenarios (workspace development, crates.io dependency, non-workspace projects)

## [0.6.0] - 2025-11-14

**First public release** of `playwright-rs` - Production-ready Rust bindings for Microsoft Playwright.

### Platform Support

- **Cross-platform**: Full support for Windows, macOS, and Linux
- **Cross-browser**: Chromium, Firefox, and WebKit
- **Windows-specific**: Platform-specific lifecycle management with Drop handler for proper stdio cleanup
- **CI-ready**: Automated cross-platform testing on all three operating systems

### Browser & Page Management

- Launch browsers (Chromium, Firefox, WebKit) in headless or headed mode
- Create isolated browser contexts
- Page lifecycle management (create, navigate, close)
- Automatic resource cleanup

### Page Navigation & Content

- Navigate to URLs with `page.goto()` (returns `Option<Response>` - `None` for data URLs and `about:blank`)
- Reload pages, get page title and URL
- Response status and metadata access

### Element Location & Interaction

- **Locators** with CSS selectors and auto-waiting
- **Locator chaining**: `first()`, `last()`, `nth()`, nested locators
- **Element queries**: `text_content()`, `inner_text()`, `inner_html()`, `get_attribute()`, `input_value()`
- **State queries**: `is_visible()`, `is_enabled()`, `is_checked()`, `is_editable()`
- **Actions**: `click()`, `dblclick()`, `fill()`, `clear()`, `press()`, `check()`, `uncheck()`, `hover()`
- **Advanced actions**: `select_option()` (by value/label/index), `set_input_files()`
- **Checkbox convenience**: `set_checked(bool)` for boolean-based check/uncheck

### Action Options (Builder Pattern)

- **Click options**: button, modifiers, position, force, trial, timeout, delay
- **Fill options**: force, timeout
- **Press options**: delay, timeout
- **Check options**: force, position, timeout, trial
- **Hover options**: force, modifiers, position, timeout, trial
- **Select options**: force, timeout

### Low-Level Input Control

- **Keyboard API**: `type_text()`, `press()`, `down()`, `up()`, `insert_text()` with delay options
- **Mouse API**: `move_to()`, `click()`, `dblclick()`, `down()`, `up()`, `wheel()` with button, click_count, delay, steps options

### Screenshots

- Page screenshots with `page.screenshot()`
- Element screenshots with `locator.screenshot()`
- **Screenshot options**: PNG/JPEG format, quality control, full-page, clip region, omit background
- Save to file or get bytes

### Assertions with Auto-Retry

- **Visibility**: `expect(locator).to_be_visible()`, `to_be_hidden()`
- **Text**: `to_have_text()`, `to_contain_text()` with regex pattern support
- **Value**: `to_have_value()` with regex pattern support
- **State**: `to_be_enabled()`, `to_be_disabled()`, `to_be_checked()`, `to_be_unchecked()`, `to_be_editable()`, `to_be_focused()`
- **Negation**: `.not()` for all assertions
- **Custom timeouts**: `.with_timeout()` configuration
- Default 5-second timeout with 100ms polling interval

### Network Interception

- **Route registration**: `page.route()` with async closure handlers
- **Request blocking**: `route.abort()`
- **Request modification**: `route.continue_()` with header/method/postData overrides
- **Response mocking**: `route.fulfill()` with custom status, headers, body (works for API/fetch requests)
- **JSON helpers**: Automatic serialization with `.json()`
- **Pattern matching**: Glob patterns (`**/*.png`, `**/*`, etc.)
- **Request inspection**: Access URL, method, headers in route handlers
- **Cross-browser**: Works on Chromium, Firefox, WebKit

### Event Handling

- **Downloads**: `page.on_download()` with `download.save_as()` and metadata access
- **Dialogs**: `page.on_dialog()` for alert/confirm/prompt with `accept()`/`dismiss()`

### Advanced Features

- **Browser context options**: viewport, user agent, locale, timezone, geolocation, mobile emulation, JavaScript control, offline mode
- **File uploads**: Basic PathBuf upload and advanced FilePayload with name/mimeType/buffer control
- **JavaScript evaluation**: `page.evaluate_value()` with return values

### Developer Experience

- **Comprehensive documentation**: 100% rustdoc coverage with examples, error docs, and Playwright links
- **cargo-nextest integration**: Faster test execution
- **Performance benchmarks**: Criterion.rs suite for regression detection
- **Cross-platform CI**: Tests run on Linux, macOS, and Windows

### Performance Optimizations

- **GUID storage**: Arc<str> optimization (5.5x faster clones, 2.0x faster lookups)
- **Message transport**: Chunked reading for large messages (>32KB) reduces memory pressure
- **Test suite**: 68% reduction in test count through combining related tests

### Known Limitations

- **route.fulfill() body transmission**: Response bodies are not transmitted to browsers due to a Playwright server limitation (tested with Playwright 1.56.1). Workaround: Mock at HTTP server level or wait for Playwright server update. The Rust implementation is correct and will work when Playwright fixes this issue.

### Migration Notes

- Navigation methods (`goto()`, `reload()`) return `Option<Response>` instead of `Response`
  - Playwright returns null for data URLs and `about:blank` (valid behavior, not an error)
  - Migration: `page.goto("https://example.com").await?.expect("response")` or use `if let Some(response) = page.goto(...).await? { ... }`

[Unreleased]: https://github.com/padamson/playwright-rust/compare/v0.7.2...HEAD
[0.7.2]: https://github.com/padamson/playwright-rust/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/padamson/playwright-rust/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/padamson/playwright-rust/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/padamson/playwright-rust/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/padamson/playwright-rust/releases/tag/v0.6.0
