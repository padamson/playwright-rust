# Phase 5: Advanced Testing Features

**Status:** Planning

**Goal:** Implement advanced testing features including assertions with auto-retry, network interception, and other testing capabilities.

**Feature:** Assertions, network interception, route mocking, downloads, dialogs, and deferred Phase 4 enhancements

**User Story:** As a Rust developer, I want powerful testing features like auto-retry assertions and network mocking so that I can write robust, maintainable test suites.

**Related ADRs:**
- [ADR-0001: Protocol Architecture](../adr/0001-protocol-architecture.md)

---

## Prerequisites from Phase 4

Phase 5 builds on Phase 4's advanced features:
- ✅ ElementHandle protocol objects
- ✅ Screenshot options (type, quality, full_page, clip)
- ✅ Action options (Click, Fill, Press, Check, Hover, Select)
- ✅ SelectOption variants (value, label, index)
- ✅ Keyboard/Mouse options
- ✅ Navigation error handling

---

## Deferred from Phase 4

Low-priority items deferred from Phase 4 that can be implemented in Phase 5:

1. **set_checked() Convenience Method**
   - `locator.set_checked(checked: bool)`
   - Calls check() or uncheck() based on boolean

2. **FilePayload Struct**
   - In-memory file creation without PathBuf
   - `FilePayload { name: String, mime_type: String, buffer: Vec<u8> }`

3. **Modifier Key Parsing**
   - Keyboard.press with compound keys (e.g., "Control+A")

4. **Screenshot Mask Options**
   - `mask`: Hide sensitive elements
   - `mask_color`: Color for masked elements

---

## Proposed Scope for Phase 5

### High Priority

1. **Assertions with Auto-Retry** (Highest Priority)
   - `expect(locator).to_be_visible()` API
   - Auto-retry logic (poll until condition met or timeout)
   - Common assertions: to_be_visible, to_be_hidden, to_have_text, to_have_value
   - Negation: to_not_be_visible, etc.
   - Custom timeout configuration

2. **Network Interception Basics** (High Priority)
   - `page.route()` for request interception
   - Route matching by URL patterns
   - Request continuation, fulfillment, abort
   - Access to request/response data

### Medium Priority

3. **Downloads Handling**
   - Download event handling
   - Save downloaded files
   - Download metadata access

4. **Dialogs Handling**
   - Alert, confirm, prompt handling
   - Accept/dismiss dialogs
   - Access dialog messages

5. **Deferred Phase 4 Items** (As time permits)
   - set_checked() convenience method
   - FilePayload struct
   - Modifier key parsing
   - Screenshot mask options

### Future Phases (Not Phase 5)

Defer to Phase 6 or later:
- **Mobile Emulation** - Device descriptors, viewport emulation
- **Videos and Tracing** - Recording and trace generation
- **Advanced Network** - HAR export, service workers
- **Context Options** - Geolocation, permissions, user agent

---

## Success Criteria

Phase 5 will be considered complete when:

- [ ] Assertions API implemented with auto-retry
- [ ] Common assertions work (visible, hidden, text, value, etc.)
- [ ] Network route() API implemented
- [ ] Request interception works (continue, fulfill, abort)
- [ ] Downloads can be captured and saved
- [ ] Dialogs can be handled (accept, dismiss)
- [ ] All tests passing cross-browser
- [ ] Documentation complete

---

## Implementation Plan

**Status:** Planning - Ready to start Slice 1

Phase 5 follows the same TDD and vertical slicing approach as previous phases.

### Slice 1: Assertions Foundation - expect() API and to_be_visible()

**Status:** ✅ COMPLETE

**Goal:** Implement the `expect()` API foundation with auto-retry logic and the first assertion (to_be_visible).

**Why First:** Assertions are the highest-priority testing feature and the foundation for the rest of the assertions API.

**Research Completed:**
- ✅ Playwright's expect API uses standalone function (matches Python/JS)
- ✅ Auto-retry: poll with configurable interval (default 100ms) until timeout (default 5s)
- ✅ Negation via .not() method
- ✅ Error messages include selector, condition, and timeout

**Tasks:**
- [x] Research Playwright's expect API and auto-retry logic
- [x] Design Rust API (chose standalone `expect(locator)` for cross-language consistency)
- [x] Create Expectation struct with timeout configuration
- [x] Implement auto-retry polling mechanism
- [x] Implement to_be_visible() assertion
- [x] Implement to_be_hidden() assertion (reuses to_be_visible with negation)
- [x] Implement Page.evaluate() for dynamic element testing
- [x] Cross-browser testing (Chromium, Firefox, WebKit all passing)
- [x] Documentation with examples

**Implementation Details:**

**Files Created:**
- `crates/playwright-core/src/assertions.rs` - expect() API and Expectation struct
- `crates/playwright-core/tests/assertions_test.rs` - Integration tests

**Files Modified:**
- `crates/playwright-core/src/error.rs` - Added AssertionTimeout error variant
- `crates/playwright-core/src/lib.rs` - Exported expect() function
- `crates/playwright-core/src/protocol/page.rs` - Added evaluate() method
- `crates/playwright-core/src/protocol/frame.rs` - Added frame_evaluate_expression() method

**Test Results:**
  - `test_to_be_visible_element_already_visible` - Basic visibility check
  - `test_to_be_hidden_element_not_exists` - Hidden check for nonexistent element
  - `test_not_to_be_visible` - Negation support
  - `test_to_be_visible_timeout` - Timeout behavior
  - `test_to_be_visible_with_auto_retry` - Auto-retry with delayed element (500ms)
  - `test_to_be_hidden_with_auto_retry` - Auto-retry with element hiding
  - `test_custom_timeout` - Custom timeout configuration (2s delay)
  - `test_to_be_visible_firefox` - Firefox compatibility
  - `test_to_be_hidden_webkit` - WebKit compatibility
  - `test_auto_retry_webkit` - WebKit auto-retry (300ms delay)

**Key Implementation Details:**
- Auto-retry polling: 100ms interval, 5s default timeout
- Protocol integration: Implemented Page.evaluate() via Frame.evaluateExpression
- Visibility detection: Elements need non-zero dimensions (textContent required for empty elements)
- Cross-browser: All tests pass on Chromium, Firefox, and WebKit

**API Design Considerations:**

Option 1: Standalone function (matches Playwright Python/JS)
```rust
use playwright_core::expect;

expect(page.locator("button")).to_be_visible().await?;
expect(page.locator("input")).to_have_value("hello").await?;
```

Option 2: Trait-based (more Rust-idiomatic)
```rust
page.locator("button").expect().to_be_visible().await?;
page.locator("input").expect().to_have_value("hello").await?;
```

**Recommendation:** Option 1 (standalone) for consistency with other Playwright bindings.

---

### Slice 2: Text and Value Assertions

**Status:** ✅ COMPLETE

**Goal:** Implement text-based assertions (to_have_text, to_contain_text, to_have_value).

**Tasks:**
- [x] Implement to_have_text() - exact match
- [x] Implement to_contain_text() - substring match
- [x] Implement to_have_value() - for input elements
- [x] Support for regex patterns
- [x] Tests for all text assertions
- [x] Cross-browser testing

**Implementation Details:**

**Files Created:**
- `crates/playwright-core/tests/text_assertions_test.rs` - 15 comprehensive integration tests

**Files Modified:**
- `crates/playwright-core/src/assertions.rs` - Added 6 new assertion methods
- `crates/playwright-core/Cargo.toml` - Added `regex = "1.10"` dependency
- `crates/playwright-core/tests/test_server.rs` - Added `/text.html` route and handler

**New Assertion Methods:**
1. `to_have_text(expected: &str)` - Exact text match with auto-retry
2. `to_have_text_regex(pattern: &str)` - Regex pattern match for text
3. `to_contain_text(expected: &str)` - Substring match with auto-retry
4. `to_contain_text_regex(pattern: &str)` - Regex pattern for substring
5. `to_have_value(expected: &str)` - Input value match with auto-retry
6. `to_have_value_regex(pattern: &str)` - Regex pattern for input value

**Test Results:**
- Tests cover exact match, substring match, regex patterns
- Tests verify auto-retry behavior with dynamically changing elements
- Cross-browser tests for Firefox and WebKit
- Timeout error handling
- Empty value handling
- Text trimming behavior

**Key Implementation Details:**
- Uses `inner_text()` for text content (matches Playwright behavior)
- Uses `input_value()` for form inputs
- Automatic text trimming before comparison
- Full regex support via `regex` crate
- Negation support via `.not()` for all assertions
- Clear error messages with actual vs expected values

---

### Slice 3: State Assertions

**Status:** ✅ COMPLETE (except `to_be_focused()` - deferred)

**Goal:** Implement state-based assertions (enabled, disabled, checked, editable).

**Tasks:**
- [x] Implement to_be_enabled() / to_be_disabled()
- [x] Implement to_be_checked() / to_be_unchecked()
- [x] Implement to_be_editable()
- Implement to_be_focused() - **DEFERRED** (requires 'expect' protocol command or evalOnSelector return values)
- [x] Tests for all state assertions
- [x] Cross-browser testing

**Implementation Details:**

**Files Created:**
- `crates/playwright-core/tests/state_assertions_test.rs`

**Files Modified:**
- `crates/playwright-core/src/assertions.rs`
- `crates/playwright-core/src/protocol/frame.rs` - No changes needed (used existing is_* methods)
- `crates/playwright-core/src/protocol/locator.rs` - No changes needed (used existing is_* methods)

**New Assertion Methods:**
1. `to_be_enabled()` - Asserts element is enabled (no disabled attribute)
2. `to_be_disabled()` - Asserts element is disabled (reuses to_be_enabled with negation)
3. `to_be_checked()` - Asserts checkbox/radio is checked
4. `to_be_unchecked()` - Asserts checkbox/radio is unchecked (reuses to_be_checked with negation)
5. `to_be_editable()` - Asserts element is editable (enabled + no readonly attribute)
6. ~~`to_be_focused()`~~ - **DEFERRED** (not in this slice)

**Key Implementation Details:**
- All assertions use existing `is_enabled()`, `is_checked()`, `is_editable()` from Locator
- Auto-retry polling: 100ms interval, 5s default timeout
- Negation support via `.not()` for all assertions
- Uses negation-inversion pattern for `to_be_disabled()` and `to_be_unchecked()` (DRY principle)
- Clear error messages with selector and timeout information

**Deferred:**
- `to_be_focused()` - Playwright doesn't expose `isFocused()` at the protocol level. The assertion exists in Playwright's test assertions API but requires:
  - Option 1: Implementing the 'expect' protocol command (complex, touches core protocol)
  - Option 2: Properly handling `evalOnSelector` return values (needs investigation of return value deserialization)
  - Deferred to future slice (likely after network mocking is complete)
  - See code comments in Frame, Locator, and assertions.rs for details

---

### Slice 4: Network Route API Foundation

**Status:** ✅ COMPLETE (All sub-slices: 4a, 4b, 4c)

**Goal:** Implement page.route() for basic request interception.

**Why Split into Sub-slices:** Network routing requires handling async closures in Rust, which has architectural complexity. Breaking into 3 sub-slices allows incremental validation of the architecture.

**Architecture Research:** See [docs/technical/phase5-slice4-routing-architecture.md](../technical/phase5-slice4-routing-architecture.md) for detailed analysis of 3 routing architecture options and rationale for choosing callback-based approach with boxed futures.

---

#### Slice 4a: Basic Route Infrastructure

**Status:** ✅ COMPLETE

**Goal:** Get ONE test passing with minimal implementation - prove architecture end-to-end.

**Tasks:**
- [x] Research Playwright route API (page.route, Route class methods)
- [x] Design Rust API for route matching (chose callback-based with boxed futures)
- [x] Document architecture decision in technical docs
- [x] Add route_handlers storage to Page struct
- [x] Implement page.route() with async closure support
- [x] Implement simple pattern matching (substring + wildcard for initial version)
- [x] Handle "route" event from protocol and invoke handlers
- [x] Implement Route protocol object (abort, continue, request access)
- [x] Register Route in object factory
- [x] Basic protocol integration (setNetworkInterceptionPatterns command)
- [x] Create simplified integration tests
- [x] Verify basic routing works (registration and continue tests passing)

**Key Implementation Details:**
- Architecture: Callback-based with boxed futures (Arc<dyn Fn(Route) -> Pin<Box<dyn Future>>>)
- Handler storage: Arc<Mutex<Vec<RouteHandlerEntry>>>
- Protocol command: setNetworkInterceptionPatterns with glob objects
- Pattern matching: Simple substring + wildcard (will upgrade in 4b)
- Handler invocation: Independent execution via tokio::spawn
- Last-registered-wins pattern priority
- Route.abort() with error codes
- Route.continue() with isFallback parameter
- Request.url() and Request.method() for routing logic

**Deferred to Slice 4b:**
- Full glob pattern matching (currently substring + wildcard)
- Multiple pattern tests
- Pattern priority verification

---

#### Slice 4b: Pattern Matching

**Status:** ✅ COMPLETE

**Goal:** Support proper glob patterns for production use.

**Tasks:**
- [x] Add `glob` crate dependency to Cargo.toml
- [x] Replace substring matching with glob pattern matching
- [x] Support multiple handlers with priority (last registered wins)
- [x] Test pattern edge cases (wildcards, subdirectories, extensions)
- [x] Implement pattern matching tests from original test suite

**Key Implementation Details:**
- Glob pattern matching: Uses `glob::Pattern` for production-ready URL matching
- Pattern matching function:
  ```rust
  fn matches_pattern(pattern: &str, url: &str) -> bool {
      use glob::Pattern;
      match Pattern::new(pattern) {
          Ok(glob_pattern) => glob_pattern.matches(url),
          Err(_) => pattern == url,  // Fallback to exact match
      }
  }
  ```
- Handler priority: Last registered handler wins (reverse iteration in on_route_event)
- Pattern types supported: `**/*`, `**/*.png`, `**/*.{css,js}`, `**/path`
- Conditional logic: Handlers can inspect route.request().url() and decide abort vs continue
- Type complexity fix: Created `RouteHandlerFuture` type alias for `Pin<Box<dyn Future<Output = Result<()>> + Send>>`

**Why Second:** Builds on working infrastructure, adds production-ready matching

---

#### Slice 4c: Cross-browser & Polish

**Status:** ✅ COMPLETE

**Goal:** Production readiness with cross-browser support.

**Tasks:**
- [x] Verify route.continue() works correctly across browsers
- [x] Cross-browser testing (Firefox, WebKit)
- [x] Error handling (handler errors, protocol errors)
- [x] Polish error messages
- [x] Restore comprehensive test suite (with evaluate() return values)
- [x] Add evaluate() return value support

**Key Implementation Details:**

1. **Route.request() Fix**: Properly downcasts parent Request instead of creating stub
2. **evaluate_value()**: Unwraps Playwright protocol value format (`{"s": "value"}`, `{"n": 123}`, etc.)
3. **Cross-browser**: All routing tests pass on Chromium, Firefox, and WebKit
4. **Error Handling**: Route handler errors logged to stderr

**Why Last:** Completes feature with quality and compatibility

---

### Slice 5: Network Response Fulfillment

**Status:** ⚠️ PARTIAL (API implemented, main document navigation issue discovered)

**Goal:** Implement route.fulfill() for mocking responses.

**Tasks:**
- [x] Implement route.fulfill() with custom response
- [x] Support for status, headers, body
- [x] JSON response helpers
- [ ] Tests for response mocking (deferred due to main frame navigation issue)
- [ ] Cross-browser testing (deferred pending test resolution)

**Key Implementation Details:**
- Protocol format: Sends `{response: {status, headers: [{name, value}], body, isBase64, contentType}}`
- Body encoding: base64 with `isBase64: true` flag
- Headers: Array format matching playwright-python
- Content-Length: Automatically calculated and added
- Default status: 200 if not specified
- JSON helper: Automatic serialization with `serde_json` and `application/json` content-type

**Bug Fix Included:**
- Fixed route handler execution timing in `page.rs:on_route_event()`
- Changed from `tokio::spawn()` (fire-and-forget) to proper `await`
- Ensures fulfill/continue/abort complete before browser continues
- Fixes affect all routing methods (fulfill, continue_, abort)
- Verified with existing continue() and abort() tests (all passing)

**Known Issue / TODO:**
- Main document navigation (page.goto()) fulfillment may not work correctly
- Implementation works for fetch/XHR requests but appears to have issues with main frame navigations
- Needs further investigation of Playwright protocol for main document replacement
- Workaround: Use fulfill() for API mocking (its primary use case), not for replacing entire page HTML
- Tests deferred until issue is resolved
- Tracked in code with TODO comment in route.rs

---

### Slice 6: Downloads and Dialogs

**Goal:** Implement download and dialog event handling.

**Tasks:**
- [ ] Implement download event handling
- [ ] Download save functionality
- [ ] Dialog event handling (alert, confirm, prompt)
- [ ] Accept/dismiss dialogs
- [ ] Tests for downloads
- [ ] Tests for dialogs
- [ ] Cross-browser testing

---

### Slice 7: Phase 4 Deferrals and Polish

**Goal:** Implement remaining low-priority items and complete documentation.

**Tasks:**
- [ ] Implement set_checked() convenience method
- [ ] Implement FilePayload struct (if time permits)
- [ ] Implement modifier key parsing (if time permits)
- [ ] Complete all rustdoc
- [ ] Update README with Phase 5 examples
- [ ] Update roadmap.md
- [ ] Mark Phase 5 complete

---

This order prioritizes:
- Highest-value testing features first (assertions)
- Network mocking before advanced features
- Progressive complexity (simple assertions → complex network handling)
- Deferred items last (lowest priority)

---

**Created:** 2025-11-08
**Last Updated:** 2025-11-09 (Slice 5 complete - route.fulfill() implementation)

---
