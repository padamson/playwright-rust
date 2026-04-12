# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **`ConsoleMessage` class** — captures JavaScript console output (`console.log`, `console.error`, etc.)
  - `type_()`, `text()`, `location()`, `page()` properties
  - `ConsoleMessageLocation` with url, line_number, column_number
  - `page.on_console(handler)` — page-level console event handler
  - `context.on_console(handler)` — context-level handler (fires for all pages)
  - Lazy subscription via `updateSubscription("console", true)`
- **`FileChooser` class** — handle file upload dialogs triggered by `<input type="file">`
  - `page()`, `element()`, `is_multiple()` properties
  - `set_files(files)` — set files on the input element
  - `page.on_filechooser(handler)` — event handler
  - `page.expect_file_chooser(timeout)` — returns `EventWaiter<FileChooser>`
- **`Selectors` class** — custom selector engines and test ID configuration
  - `playwright.selectors().register(name, script)` — register custom selector engine
  - `playwright.selectors().set_test_id_attribute(attr)` — change `get_by_test_id` attribute (default: `data-testid`)
  - Auto-propagates to all active BrowserContexts and new contexts

### Breaking Changes

- **`ConnectionLike` trait gains `selectors()` method** — internal server infrastructure, not user-facing API. Any code implementing `ConnectionLike` directly must add the new method.

### Fixed

- **unwrap() audit (closes #48)** — replaced bare `unwrap()` calls in library code with `expect()` (for infallible operations) or proper error handling (for protocol data). Remaining `unwrap()` calls are only mutex locks (`lock().unwrap()`) and test code.
- **15 broken rustdoc links** — all intra-doc links now resolve correctly (qualified paths for cross-module references)

## [0.10.0] - 2026-04-11

### Added

- **`BrowserContext::new_cdp_session(page)`** — creates a Chrome DevTools Protocol session (Chromium only)
  - `CDPSession::send(method, params)` — send any CDP command and receive the result as JSON
  - `CDPSession::detach()` — detach from the CDP session
  - `CDPSession` registered in the object factory for server-created sessions
  - See: <https://playwright.dev/docs/api/class-cdpsession>
- **`BrowserContext::tracing()`** — access the per-context Tracing object
  - `Tracing::start(options)` — begin trace recording (`tracingStart` + `tracingStartChunk`)
  - `Tracing::stop(options)` — stop trace recording (`tracingStopChunk` + `tracingStop`)
  - `TracingStartOptions` — `name`, `screenshots`, `snapshots` fields
  - `TracingStopOptions` — `path` field to export trace as a `.zip` archive
  - Artifact export wired through `Artifact::save_as` for path-based stop
  - See: <https://playwright.dev/docs/api/class-tracing>

- **`install_browsers()` / `install_browsers_with_deps()`** — programmatic browser installation (closes #46)
  - `install_browsers(None)` — install all browsers
  - `install_browsers(Some(&["chromium", "firefox"]))` — install specific browsers
  - `install_browsers_with_deps(browsers)` — also installs system dependencies (useful for CI)
  - Reuses the bundled Playwright driver — no `npx` required
- **Frame public API completion** — Frame now at **100%** coverage
  - `frame.locator(selector)` — create Locator scoped to a frame (was internal-only)
  - All 7 `get_by_*` methods (`get_by_text`, `get_by_label`, `get_by_role`, etc.)
  - `evaluate_handle(expression)` — returns ElementHandle from JS evaluation
  - `child_frames()` — returns child frames by scanning the connection registry
  - Properties: `name()`, `page()`, `parent_frame()`, `is_detached()`
  - `frame.page()` back-reference set lazily when Page accesses its main frame
- **`FrameLocator` class** — locate elements inside iframes using Playwright's `internal:control=enter-frame` selector engine
  - `page.frame_locator(selector)` / `locator.frame_locator(selector)` — entry points
  - `frame_locator.locator(selector)` — create Locator inside iframe
  - `frame_locator.frame_locator(selector)` — nested iframes
  - All 7 `get_by_*` methods (`get_by_text`, `get_by_label`, `get_by_role`, etc.)
  - `first()`, `last()`, `nth(index)` — composition for multiple matching iframes
  - `owner()` — Locator for the iframe element itself
- **`ConnectionExt` extension trait** — typed object retrieval via `connection.get_typed::<T>(guid).await?`, eliminating boilerplate `get_object` + `as_any().downcast_ref::<T>()` pattern
- **`downcast_parent<T>()` helper** — one-line parent type resolution replacing manual parent + downcast chains
- **`Error::TypeMismatch`** — structured error variant with guid, expected type, and actual type for better diagnostics

### Breaking Changes

- **MSRV bumped from 1.85 to 1.88** — transitive dependencies (`icu_*`, `image`, `time`, `zip`) now require Rust 1.88
- **`ConnectionLike` trait uses `#[async_trait]`** — methods migrated from manual `Pin<Box<dyn Future>>` returns to idiomatic `async fn`. Any code implementing `ConnectionLike` directly must update method signatures (internal server infrastructure, not user-facing API).

### Changed

- **Security & quality CI** — `cargo audit` and `cargo deny` run on every push to main and weekly; mutation testing moved from `test.yml` to dedicated `security.yml` with weekly schedule + release tag triggers; MSRV check (Rust 1.85) in `test.yml`
- **`deny.toml`** — license compliance (Apache-2.0 compatible), crate bans, source restrictions, duplicate detection
- **`cargo vet`** — supply chain review with trusted imports from 7 organizations; new dependencies require audit
- **SLSA provenance** — release artifacts include signed build attestations via `actions/attest-build-provenance`
- **Fuzz targets** — `cargo-fuzz` targets for `parse_value`, `serialize_argument`, `parse_result` (protocol parsing layer)
- **BrowserContext event handlers** — context-level event subscriptions (fire before page handlers)
  - `on_page(handler)` — fires when new page created in context
  - `on_close(handler)` — fires when context is closed
  - `on_request(handler)` / `on_request_finished(handler)` / `on_request_failed(handler)` — network events from any page
  - `on_response(handler)` — response events from any page
- **`expect_page()` / `expect_close()`** — promise-based event waiting with timeout
  - `expect_page(timeout)` — returns `EventWaiter<Page>` that resolves when a new page is created
  - `expect_close(timeout)` — returns `EventWaiter<()>` that resolves when the context closes
  - `EventWaiter<T>` — generic one-shot waiter backed by `tokio::sync::oneshot` with configurable timeout (default 30s)
- **`on_dialog(handler)`** — context-level dialog handler, fires before page handlers
- **`expose_function()` / `expose_binding()`** — JS→Rust callback bridge on both BrowserContext and Page
  - `expose_function(name, callback)` — inject a Rust function callable from JS as `window[name](...)`
  - `expose_binding(name, callback)` — same with source info (note: `needsHandle: true` not yet supported)
  - BindingCall protocol object for handling JS→Rust invocations
- Added `async-trait` as a dependency

## [0.9.0] - 2026-03-27

### Added

- **Back-reference properties** — navigate the protocol object hierarchy from child to parent
  - `dialog.page()` — returns the `Page` that owns the dialog (via protocol parent)
  - `download.page()` — returns the `Page` that triggered the download (stored at construction)
  - `response.request()` — returns the `Request` that triggered the response (via ResponseObject parent)
  - `response.frame()` — returns the `Frame` that initiated the request (delegates to `request.frame()`)
  - `request.frame()` — returns the `Frame` that initiated the request (eagerly resolved from initializer GUID)
- **Response server info** — inspect connection and TLS details
  - `response.security_details()` — TLS/SSL certificate info (`SecurityDetails`: issuer, protocol, subject_name, valid_from, valid_to); returns `None` for HTTP
  - `response.server_addr()` — server IP address and port (`RemoteAddr`); returns `None` for cached responses
  - `response.finished()` — wait for response to finish loading (currently returns immediately for goto/reload responses)
- **Request completion methods** — full request lifecycle access
  - `request.redirected_from()` / `request.redirected_to()` — navigate the redirect chain (eagerly resolved from initializer)
  - `request.response()` — get the matching `Response` via RPC
  - `request.sizes()` — resource size info (`RequestSizes`: request_body_size, request_headers_size, response_body_size, response_headers_size)
- **New types**: `SecurityDetails`, `RemoteAddr`, `RequestSizes` (exported from `playwright_rs::protocol`)
- **Page assertions** — `expect_page(&page)` now supports title and URL assertions
  - `to_have_title(expected)` / `to_have_title_regex(pattern)` — assert page title with auto-retry
  - `to_have_url(expected)` / `to_have_url_regex(pattern)` — assert page URL with auto-retry
  - `.not()` negation and `.with_timeout()` supported (matching locator assertion pattern)

### Breaking Changes

- **Response struct fields are now private** — `response.url`, `response.status`, `response.status_text`, `response.ok`, `response.headers` are no longer accessible as public fields. Use the existing accessor methods instead: `response.url()`, `response.status()`, `response.status_text()`, `response.ok()`, `response.headers()`. These methods were already available; only direct field access is removed.
- **`Download::from_artifact` is now `pub(crate)`** — this was an internal constructor not intended for public use.

### Fixed

- **Request parent type corrected** — Request's parent in the Playwright protocol is Page (not Frame as previously assumed). The `request.frame()` method now correctly resolves the frame from the initializer's `frame` GUID via the connection registry.

## [0.8.7] - 2026-03-24

### Added

- **Locator advanced methods** — `tap()`, `evaluate()`, `evaluate_all()`, `drag_to()`, `wait_for()`, `dispatch_event()`, `bounding_box()`, `scroll_into_view_if_needed()`
  - `tap(options)` — touch-tap on an element (requires `has_touch: true` context); `TapOptions` builder with `force`, `modifiers`, `position`, `timeout`, `trial`
  - `evaluate(expression, arg)` — run a JavaScript function with the element as first argument, returns typed `R: DeserializeOwned`
  - `evaluate_all(expression, arg)` — run a JavaScript function with all matching elements as an array, returns typed `R: DeserializeOwned`
  - `drag_to(target, options)` — drag this element to another; `DragToOptions` builder with `force`, `source_position`, `target_position`, `timeout`, `trial`
  - `wait_for(options)` — wait for element to reach a state (`Visible`, `Hidden`, `Attached`, `Detached`); `WaitForOptions` with `state` and `timeout`
  - `dispatch_event(type, event_init)` — fire DOM events with optional initialization data
  - `bounding_box()` — get element dimensions and position (x, y, width, height)
  - `scroll_into_view_if_needed()` — scroll element into viewport
  - `page` property — back-reference to the owning Page from any Locator
- **TLS backend features** — Expose `native-tls`, `rustls-tls-native-roots`, and `rustls-tls-webpki-roots` features for choosing TLS implementation (PR #41). Defaults to `native-tls`.
- **Locator filtering & composition** — `filter()`, `and_()`, `or_()` methods for narrowing and combining locators
  - `filter(FilterOptions)` — narrow by `has_text`, `has_not_text`, `has` (child locator), `has_not`
  - `and_(locator)` — match elements satisfying both locators
  - `or_(locator)` — match elements satisfying either locator
- **Locator interaction methods** — `focus()`, `blur()`, `press_sequentially()`, `all_inner_texts()`, `all_text_contents()`
  - `focus()` / `blur()` — set or remove keyboard focus on an element
  - `press_sequentially(text, options)` — type characters one by one with optional delay
  - `all_inner_texts()` / `all_text_contents()` — bulk text retrieval from all matching elements
  - `dispatch_event(type, event_init)` — fire DOM events with optional initialization data
  - `bounding_box()` — get element dimensions and position (x, y, width, height)
  - `scroll_into_view_if_needed()` — scroll element into viewport
- **BrowserContext runtime setters** — configure context after creation
  - `cookies(urls)` — retrieve cookies, optionally filtered by URL
  - `clear_cookies(options)` — remove cookies with optional name/domain/path filters
  - `set_extra_http_headers(headers)` — add HTTP headers to all requests
  - `grant_permissions(permissions, options)` — grant browser permissions (geolocation, camera, etc.)
  - `clear_permissions()` — revoke all granted permissions
  - `set_geolocation(geolocation)` — override device geolocation, or pass None to clear
  - `set_offline(offline)` — toggle offline mode
- **Page methods** — `bring_to_front()`, `viewport_size()`, `set_extra_http_headers()`, `emulate_media()`, `pdf()`, `add_script_tag()`
  - `bring_to_front()` — activate the page tab
  - `viewport_size()` — get current viewport dimensions (returns None if no_viewport context)
  - `set_extra_http_headers(headers)` — add HTTP headers to all page requests
  - `emulate_media(options)` — override CSS media type, color scheme, reduced motion, forced colors
  - `pdf(options)` — generate PDF (Chromium only), with full options (margins, scale, landscape, etc.)
  - `add_script_tag(options)` — inject JavaScript via URL, file path, or inline content
- **Page timeout & state** — `set_default_timeout()`, `set_default_navigation_timeout()`, `is_closed()`, `frames()`
  - `set_default_timeout(ms)` / `set_default_navigation_timeout(ms)` — configure default timeouts for actions and navigation
  - `is_closed()` — check if page has been closed (tracks close events from server)
  - `frames()` — list page frames (currently main frame only; iframe enumeration planned)
- **BrowserContext timeout defaults** — `set_default_timeout()`, `set_default_navigation_timeout()`
  - Propagates to all existing pages and newly created pages in the context
- **Response body access** — `body()`, `text()`, `json()`, `all_headers()`, `header_value()`, `headers_array()`
  - `body()` — response body as raw bytes
  - `text()` — response body as UTF-8 string
  - `json::<T>()` — parse response body as typed JSON (`T: DeserializeOwned`)
  - `all_headers()` — all response headers as HashMap (merges duplicates)
  - `header_value(name)` — get a single header value by name
  - `headers_array()` — all headers as `Vec<HeaderEntry>` preserving duplicates
- **Request properties** — `headers()`, `post_data()`, `post_data_buffer()`, `post_data_json()`, `failure()`, `all_headers()`, `header_value()`, `headers_array()`, `timing()`
  - `headers()` — request headers as HashMap (from initializer)
  - `post_data()` / `post_data_buffer()` — request body as string or bytes (base64-decoded)
  - `post_data_json::<T>()` — parse request body as typed JSON
  - `failure()` — error text if request failed (set on `requestFailed` event)
  - `all_headers()` / `header_value()` / `headers_array()` — full raw headers via RPC
  - `timing()` — `ResourceTiming` with 9 timing fields (extracted from Response on `requestFinished`)

### Changed

- **Playwright driver upgraded to 1.58.2** (from 1.56.1) — includes WebKit 26.0, Chromium 133, Firefox 135

### Fixed

- **WebKit `launchPersistentContext` now works** — Closes #39. Upgraded Playwright driver resolves "Browser started with no default context" error on macOS ARM64
- **docs.rs build** — Pin docs.rs to `nightly-2025-05-01` to work around `generic-array` 0.14 incompatibility with Rust 1.92+ (`doc_auto_cfg` removal)

## [0.8.6] - 2026-03-14

### Fixed

- **docs.rs build** — Skip Playwright driver download when building on docs.rs (no network access needed for documentation)
- **Imprecise dependency versions** — Pin workspace dependencies to minor versions (e.g., `serde = "1.0"` instead of `"1"`)

## [0.8.5] - 2026-03-14

### Added

- **`ignore_default_args` for persistent contexts** - Added `ignore_default_args` option to `BrowserContextOptions` for use with `launch_persistent_context_with_options()` (Issue #38)
  - `IgnoreDefaultArgs::Bool(true)` - Playwright does not pass its own default args
  - `IgnoreDefaultArgs::Array(vec)` - Filters out specific default arguments
  - Applies same `ignoreDefaultArgs` → `ignoreAllDefaultArgs` protocol normalization as `LaunchOptions`
  - Matches Playwright's official `launchPersistentContext` API
- **Page network event listeners** - Subscribe to network events on individual pages (PR #37)
  - `page.on_request(handler)` - Fires when a request is issued
  - `page.on_response(handler)` - Fires when a response is received
  - `page.on_request_finished(handler)` - Fires when a request completes successfully
  - `page.on_request_failed(handler)` - Fires when a request fails
  - Lazy subscription: events are only subscribed when a handler is registered
  - Works with iframes and sub-resources
- **Response accessor methods** - `response.status()`, `response.status_text()`, `response.url()` (PR #37)
- **`page.go_back()` / `page.go_forward()`** - History navigation with optional timeout and wait_until options
- **`page.set_content(html)`** - Set page HTML content directly, with optional timeout and wait_until options
- **`page.wait_for_load_state(state)`** - Wait for `load`, `domcontentloaded`, or `networkidle` states
- **`page.wait_for_url(url)`** - Wait for navigation to a matching URL (exact string or glob pattern)
- **`locator.is_hidden()` / `locator.is_disabled()`** - Negative state checks complementing `is_visible()` and `is_enabled()`
- **`to_have_screenshot()` visual regression assertion** (Issue #35)
  - `expect(locator).to_have_screenshot(path, options)` — compare locator screenshot against baseline
  - `expect_page(&page).to_have_screenshot(path, options)` — page-level screenshot comparison
  - Auto-creates baseline on first run, compares on subsequent runs
  - `max_diff_pixels` / `max_diff_pixel_ratio` — configurable tolerance
  - `threshold` — per-pixel color distance tolerance (default 0.2)
  - `animations: Disabled` — freeze CSS animations/transitions before capture
  - `mask` — overlay locators with pink (#FF00FF) to exclude dynamic content
  - `update_snapshots` — force baseline update
  - Generates diff image on failure highlighting differences in red
  - Auto-retry with timeout (default 5s), matching Playwright's assertion pattern

### Fixed

- Replace `unwrap()` with graceful error handling in network event dispatch (Issue #40)

## [0.8.4] - 2026-03-01

### Added

- **`get_by_*` locators** - Modern Playwright locator methods for finding elements by user-facing attributes
  - `get_by_text(text, exact)` - Find by text content
  - `get_by_label(text, exact)` - Find form controls by associated label
  - `get_by_placeholder(text, exact)` - Find inputs by placeholder text
  - `get_by_alt_text(text, exact)` - Find images by alt text
  - `get_by_title(text, exact)` - Find elements by title attribute
  - `get_by_test_id(test_id)` - Find elements by `data-testid` attribute (always exact)
  - `get_by_role(role, options)` - Find elements by ARIA role with optional name, checked, disabled, expanded, selected, level, pressed, include_hidden filters
  - All methods available on both `Page` and `Locator` (chainable)
  - Case-insensitive substring matching by default (`exact=false`), case-sensitive exact with `exact=true`
  - `AriaRole` enum with 81 ARIA roles for compile-time safety
  - `GetByRoleOptions` struct for role-based filtering
- **`connect_over_cdp`** - Connect to Chrome DevTools Protocol endpoints (Issue #32)
  - `browser_type.connect_over_cdp(endpoint_url, options)` - Connect to remote Chrome via CDP
  - Supports browserless, Chrome with `--remote-debugging-port`, and other CDP services
  - Accepts optional headers, timeout, and slow_mo options
  - Chromium-only (returns error for Firefox/WebKit)
- **`Locator.all()`** - Iterate over all matching elements (Issue #33)
  - `locator.all()` returns `Vec<Locator>`, one per matching element
  - Empty vec for non-matching selectors (no error)
  - Matches Playwright's `locator.all()` API
- **Improved error messages** - All locator methods now include the selector in error messages (Issue #33)
  - Timeout errors show `[selector: div.page-number > span:last-child]` instead of generic messages
  - Applied to all query methods (`text_content`, `get_attribute`, etc.) and action methods (`click`, `fill`, etc.)
- **BrowserContext proxy support** - Added `proxy` option to `BrowserContextOptions` for per-context proxy configuration (PR #29, Issue #28)
  - Enables rotating proxies without creating new browser instances
  - Supports HTTP and SOCKS proxies with optional authentication
- **Complete Route API** - Full network interception parity with Playwright (Issue #36)
  - `route.fallback(overrides)` - Continue to next matching handler (handler chaining)
  - `route.fetch(options)` - Fetch actual response for inspection/modification before fulfilling
  - `FetchResponse` type with `status()`, `ok()`, `headers()`, `body()`, `text()`, `json()` methods
  - `FetchOptions` builder for customizing fetch requests (method, headers, post_data, timeout)
- **Context-level routing** - `BrowserContext.route()`, `unroute()`, `unroute_all()` for routing across all pages in a context
- **Page unroute methods** - `page.unroute(pattern)` and `page.unroute_all()` for removing route handlers
- **APIRequestContext** - Internal implementation for `route.fetch()` via BrowserContext's request context
  - Handles fetch → fetchResponseBody → disposeAPIResponse protocol flow
  - Automatic base64 encoding/decoding for request and response bodies
- **`UnrouteBehavior` enum** - Control behavior when removing route handlers

### Fixed

- **`no_viewport(true)` / `--start-maximized` not working** - Fixed protocol field name for viewport disabling (Issue #34)
  - `no_viewport` now correctly serializes as `noDefaultViewport` (matching the Playwright protocol)
  - Previously serialized as `noViewport` which the server silently ignored
  - Enables `--start-maximized` with `no_viewport(true)` to produce maximized browser windows

## [0.8.3] - 2026-01-25

### Added

- **PLAYWRIGHT_VERSION constant** - Exposes bundled Playwright driver version (`1.56.1`) as a public constant for version-aware browser installation (Issue #27)
- **Helpful browser installation errors** - Detects missing browser errors and provides actionable guidance (Issue #27)
- **Page.content()** - Returns full HTML content of the page including DOCTYPE (Issue #23)
  - `page.content()` - Retrieves complete HTML markup
  - `frame.content()` - Frame-level implementation for consistency with Playwright API
- **Page.set_viewport_size()** - Dynamically resize viewport for responsive testing (Issue #24)
  - `page.set_viewport_size(viewport)` - Set viewport to specific width/height
  - Enables testing mobile, tablet, and desktop layouts within a single page session

### Fixed

- **page.url() hash navigation** - URL now correctly includes hash fragment after anchor clicks (Issue #26)
  - Frame now handles "navigated" events to track URL changes including hash updates
  - Page delegates to main frame for URL (matches playwright-python/JS behavior)

### Changed

- **Rust Edition 2024** - Upgraded to Rust Edition 2024, requiring Rust 1.85+
- **README documentation** - Added comprehensive browser installation section (Issue #25)

## [0.8.2] - 2026-01-19

### Added

- **Protocol Stubs** - Explicit protocol types for `Android`, `Electron`, `Tracing`, `APIRequestContext`, and `LocalUtils` to support valid registration and prevent "Unknown protocol type" warnings. (Implemented as stubs for future expansion)
- **Cookie & Storage Management** - Implemented `BrowserContext::storage_state()` and `BrowserContext::add_cookies()` (Issue #10)
- **Remote Connection** - Support for connecting to remote browsers via WebSocket
  - `BrowserType::connect(url, options)` implementation
  - `ConnectOptions` builder for connection configuration (headers, slow_mo, timeout)
  - WebSocket transport using `tokio-tungstenite`
  - Internal transport abstraction supporting both options (Pipe and WebSocket)
- **WebSocket Event Handling** - `Page::on_websocket()` for intercepting WebSocket connections (Slice 2)
  - `WebSocket` protocol object with events: `on_frame_sent`, `on_frame_received`, `on_close`, `on_error`
  - Access to WebSocket URL and state
- **File Upload Helpers** - `FilePayload::from_path` and `from_file` for automatic MIME type detection and easier file uploads.
- **Browser Context Options** - Added support for `RecordHar` and `RecordVideo` configuration (paths, dimensions, filters).
- **Service Worker Control** - Added `service_workers` option to `BrowserContextOptions`.
- **Error Handling** - Added `Error::context()` for richer error reporting.

### Breaking Changes

- **Error Enum**: Added `Error::Context` variant. Exhaustive matches on `Error` will need to handle this new variant.
- **BrowserContextOptions**: Added new public fields (`record_har`, `service_workers`, etc.). Code constructing this struct via struct literal syntax (e.g. `BrowserContextOptions { ... }`) will break; use `BrowserContextOptions::builder()` instead.

### Fixed

- **Event Deserialization** - Fixed `ProtocolError` when parsing `__dispose__` events by correctly handling optional `params` field (Issue #11)

## [0.8.1] - 2026-01-04

### Added

- **Persistent Contexts & App Mode** - Support for `launchPersistentContext` (Issue #9)
  - `BrowserType::launch_persistent_context(user_data_dir)`
  - `BrowserType::launch_persistent_context_with_options(user_data_dir, options)`
  - Full support for `--app=url` argument for standalone application windows
  - Persistent user data directories for saving session state (cookies, local storage) across runs
  - Initial page handling for app mode (automatically tracked in `context.pages()`)

## [0.8.0] - 2025-12-30

### Added

- **Typed Evaluate API** - Generic `Page::evaluate()` method with argument serialization and typed results (PR #8)
  - `Page::evaluate<T: Serialize, U: DeserializeOwned>(expression, arg)` - Fully typed JavaScript evaluation
  - `Frame::evaluate<T: Serialize>(expression, arg)` - Frame-level evaluation returning `serde_json::Value`
  - Argument serialization: Pass any `Serialize` type to JavaScript (primitives, structs, arrays, objects)
  - Result deserialization: Receive typed results with compile-time validation
  - Backward compatible: Original `evaluate_expression()` and `evaluate_value()` methods preserved
  - Comprehensive serialization module with Playwright protocol support
  - Special value handling: Infinity, NaN, -0, circular references, TypedArrays, Dates, BigInt
  - Example: `evaluate_typed.rs` demonstrating usage patterns

### Community Credit

- Implementation by @douglasob (Douglas Braga)

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

[Unreleased]: https://github.com/padamson/playwright-rust/compare/v0.10.0...HEAD
[0.10.0]: https://github.com/padamson/playwright-rust/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/padamson/playwright-rust/compare/v0.8.7...v0.9.0
[0.8.7]: https://github.com/padamson/playwright-rust/compare/v0.8.6...v0.8.7
[0.8.6]: https://github.com/padamson/playwright-rust/compare/v0.8.5...v0.8.6
[0.8.5]: https://github.com/padamson/playwright-rust/compare/v0.8.4...v0.8.5
[0.8.4]: https://github.com/padamson/playwright-rust/compare/v0.8.3...v0.8.4
[0.8.3]: https://github.com/padamson/playwright-rust/compare/v0.8.2...v0.8.3
[0.8.2]: https://github.com/padamson/playwright-rust/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/padamson/playwright-rust/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/padamson/playwright-rust/compare/v0.7.2...v0.8.0
[0.7.2]: https://github.com/padamson/playwright-rust/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/padamson/playwright-rust/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/padamson/playwright-rust/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/padamson/playwright-rust/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/padamson/playwright-rust/releases/tag/v0.6.0
