# Phase 3: Page Interactions

**Status:** COMPLETE - All 7 Slices Done ✅

**Goal:** Implement core page interactions (navigation, locators, actions) matching playwright-python API.

**Feature:** Navigate to URLs, find elements, and perform basic interactions

**User Story:** As a Rust developer, I want to navigate to web pages and interact with elements so that I can automate browser testing workflows.

**Related ADRs:**
- [ADR-0001: Protocol Architecture](../adr/0001-protocol-architecture.md)

---

## Prerequisites from Phase 2

Phase 3 builds on Phase 2's browser lifecycle management:
- ✅ Browser launching (all three browsers)
- ✅ Context and page creation
- ✅ Page objects at about:blank
- ✅ Lifecycle cleanup (close methods)

---

## Deferred from Phase 2

### Technical Improvements

1. **Windows Testing**
   - Current: Verified on macOS and Linux. Windows CI runs unit tests only (integration tests hang).
   - Issue: Integration tests hang on Windows after 60+ seconds when launching browsers
   - Root cause: Stdio pipe cleanup issue - Playwright server process doesn't terminate cleanly on Windows
   - Progress: ✅ Browser::close() implemented, but still hangs on Windows
   - Goal: Fix stdio pipe handling and implement proper cleanup
   - **When to re-enable full Windows CI**: After implementing explicit Drop for Playwright/Connection that:
     - Sends close/disconnect protocol messages to server
     - Waits for graceful server shutdown
     - Properly closes stdio pipes on Windows (different from Unix)
     - Kills child process if graceful shutdown times out
   - **Possible solutions**:
     1. Implement Drop for Playwright that calls a blocking cleanup method
     2. Add explicit `Playwright::disconnect()` method (like playwright-python)
     3. Better stdio pipe handling on Windows (tokio::process differences)
   - Priority: High (blocking full Windows support)
   - **Workaround**: CI runs `cargo test --lib` on Windows (unit tests only)

2. **Disposal Cleanup Refactor**
   - Current: Uses `tokio::spawn` for async unregister in `ChannelOwner::dispose()`
   - Goal: Refactor to fully synchronous disposal with background cleanup task
   - Rationale: All official bindings use synchronous disposal
   - Priority: Low (current approach works correctly)

3. **Error Message Improvements**
   - Current: Functional but terse error messages
   - Goal: Add context and suggestions to error messages
   - Priority: Low

### Testing Improvements

1. **IPC Performance Benchmarking**
   - Deferred from ADR-0001 validation checklist
   - Goal: Measure latency overhead (<5ms per operation expected)
   - Priority: Low (browser operations are 100+ms, IPC overhead negligible)

2. **Transport Reconnection**
   - Test reconnection scenarios after server crash/restart
   - Verify graceful degradation and recovery
   - Deferred from Phase 1 transport testing
   - Priority: Medium

### API Improvements

1. **Context Options API**
   - Phase 2 implemented minimal options (empty JSON)
   - Goal: Add full ContextOptions support:
     - Viewport configuration
     - User agent
     - Geolocation
     - Permissions
     - Locale/timezone
   - Priority: Medium (needed for mobile emulation in Phase 4)

2. **URL Tracking**
   - Phase 2: `page.url()` always returns "about:blank"
   - Goal: Track URL changes via page navigation events
   - Priority: High (required for Phase 3 navigation)

---

## Proposed Scope

### Core Features

1. **Navigation API**
   - `page.goto(url)` - Navigate to URL
   - `page.go_back()` - Navigate back
   - `page.go_forward()` - Navigate forward
   - `page.reload()` - Reload page
   - Navigation options: timeout, wait_until (load/domcontentloaded/networkidle)
   - Response handling

2. **Locators API**
   - `page.locator(selector)` - Create locator with auto-waiting
   - Selector strategies: CSS, text, XPath
   - Locator chaining
   - Auto-waiting and auto-retry

3. **Actions API**
   - `locator.click()` - Click element
   - `locator.fill(text)` - Fill input
   - `locator.type(text)` - Type with delays
   - `locator.press(key)` - Press keyboard key
   - `locator.select_option(value)` - Select dropdown option
   - `locator.check()` / `uncheck()` - Checkboxes
   - Action options: timeout, force, position

4. **Query API**
   - `locator.text_content()` - Get text
   - `locator.inner_text()` - Get visible text
   - `locator.inner_html()` - Get HTML
   - `locator.get_attribute(name)` - Get attribute
   - `locator.count()` - Count matching elements

5. **Waiting API**
   - `page.wait_for_selector(selector)` - Wait for element
   - `page.wait_for_url(pattern)` - Wait for URL match
   - `page.wait_for_load_state(state)` - Wait for page load state
   - `locator.wait_for()` - Wait for locator conditions

6. **Frame Support**
   - Basic frame handling
   - `page.frame_locator(selector)` - Locate iframe
   - Frame navigation

7. **Screenshots**
   - `page.screenshot()` - Capture screenshot
   - Options: path, full_page, clip, type (png/jpeg)

### Documentation

- Rustdoc for all public APIs
- Examples for navigation and interaction patterns
- Comparison with playwright-python API

### Testing

- Integration tests for navigation
- Tests for all action types
- Cross-browser tests
- Error handling tests (timeouts, element not found)

---

## Out of Scope (Future Phases)

- **Phase 4:** Assertions with auto-retry, network interception, route mocking, mobile emulation, videos, tracing, downloads, dialogs
- **Phase 5:** Production hardening, performance optimization, comprehensive documentation

---

## Success Criteria

- [ ] Can navigate to URLs
- [ ] Can find elements with locators
- [ ] Can perform basic actions (click, fill, type)
- [ ] Can query element content
- [ ] Can take screenshots
- [ ] Auto-waiting works correctly
- [ ] All tests passing with real browsers (macOS, Linux)
- [ ] Windows CI support (requires cleanup fixes from deferred items)
- [ ] Documentation complete
- [ ] Example code works

---

## Implementation Plan

Following Phase 2's successful vertical slicing approach, Phase 3 is divided into 7 focused slices. Each slice delivers working, tested functionality.

### Research Summary

**Cross-Language Analysis Completed:**
- ✅ Analyzed playwright-python, playwright-java, and TypeScript/JS implementations
- ✅ Identified common patterns: locator-based interactions, options pattern, auto-waiting
- ✅ Determined Rust idiomatic adaptations: builder pattern for options, enums for constants
- ✅ Established priority order based on usage frequency

**Key Findings:**
1. **Locator Architecture:** All bindings use locators as primary interaction mechanism with auto-waiting
2. **Options Pattern:** TypeScript uses object literals, Python uses kwargs, Java uses builder classes
3. **Rust Approach:** Builder pattern with `Option<T>` fields, consuming methods, `Default` trait
4. **Protocol Communication:** All options serialize to JSON-RPC, server handles timeouts and retries
5. **File Handling:** Screenshots/PDFs return bytes (Vec<u8>), optionally save to path
6. **Timeout Defaults:** 30 seconds for actions, configurable at page/context/action level

**Architecture Decision:**
- Use builder pattern for options (ergonomic and type-safe)
- Use enums for constrained values (MouseButton, KeyboardModifier, etc.)
- Use `Duration` instead of raw milliseconds (more idiomatic)
- Return `Result<T, Error>` with descriptive error types
- Delegate locator methods to frame (follows playwright-python pattern)

### Slice 1: Navigation and URL Tracking

**Goal:** Implement page navigation with URL tracking via protocol events.

**Why First:** Navigation is foundation for all interaction tests. URL tracking deferred from Phase 2.

**Status:** ✅ **COMPLETE**

**Implementation Notes:**
- URL tracking implemented by updating Page.url after successful navigation (not via events)
- Response data structure: headers are array of {name, value}, ok computed from status
- reload() is a Page method, not Frame method (unlike goto which delegates to Frame)
- Response objects have Request as parent (Request→Frame parent chain)
- Added retry loop for Response object availability (waits for __create__ message)
- Cross-browser testing verified: All navigation tests pass on Chromium, Firefox, and WebKit

**Tasks:**
- [x] Implement URL tracking in Page
  - ~~Subscribe to "Page.navigated" protocol event~~ (deferred - using direct update)
  - Update internal URL state on navigation
  - Fix `page.url()` to return actual URL (not "about:blank")
- [x] Implement `page.goto(url, options)`
  - Create `GotoOptions` struct with builder pattern
  - Fields: `timeout`, `wait_until` (load/domcontentloaded/networkidle/commit)
  - Serialize options to protocol format (camelCase)
  - Send "goto" protocol message to Frame (Page delegates to Frame)
  - Return Response object
- [x] Implement Response object
  - Fields: `url()`, `status()`, `ok()`, `headers()`
  - Protocol: deserialize from Response initializer (not direct RPC response)
  - Created Request and ResponseObject protocol objects
- [x] Add `page.title()` method
  - Protocol: "title" message via Frame
  - Returns String
- [x] Add `page.reload(options)` method
  - Same options as goto (timeout, wait_until)
  - Protocol: "reload" message to Page (not Frame!)
- [x] Tests
  - Test navigation to valid URL
  - Test URL updates correctly
  - Test page title
  - Test reload
  - ~~Test timeout error handling~~ (TODO: deferred - add to Phase 4 error handling slice)
  - Test response status codes
  - Cross-browser (Chromium, Firefox, WebKit) all passing

**Files Modified:**
- `crates/playwright-core/src/protocol/page.rs` - Added goto(), title(), reload() methods with URL tracking
- `crates/playwright-core/src/protocol/frame.rs` - Added goto() and title() methods (Page delegates to Frame for these)
- `crates/playwright-core/src/protocol/request.rs` - **NEW** - Request protocol object (parent for Response)
- `crates/playwright-core/src/protocol/response.rs` - **NEW** - ResponseObject protocol object
- `crates/playwright-core/src/protocol/mod.rs` - Exported Request and ResponseObject
- `crates/playwright-core/src/object_factory.rs` - Added Request and Response to factory
- `crates/playwright-core/tests/page_navigation_test.rs` - **NEW** -
- `crates/playwright/examples/basic.rs` - Updated to demonstrate navigation
- `README.md` - Updated to show navigation in quick example

**Acceptance Criteria:**
- `page.goto()` successfully navigates to URL
- `page.url()` returns correct current URL (not "about:blank")
- `page.title()` returns page title
- Response object provides status codes
- Timeout errors are descriptive
- All tests pass cross-browser

### Slice 2: Locators Foundation

**Goal:** Implement locator creation and basic protocol communication.

**Why Second:** Locators are foundation for all interactions. Modern Playwright uses locators, not direct selectors.

**Status:** ✅ **COMPLETE**

**Implementation Notes:**
- Locator is NOT a ChannelOwner - lightweight Arc<Frame> + selector wrapper
- All operations delegate to Frame with `strict=true` for single-element enforcement
- `page.locator()` is async (needs to resolve main frame)
- Locator chaining builds compound selectors (e.g., "p >> nth=0")
- Used `querySelectorAll` for count (returns element array)
- Followed TDD: Red (tests) → Green (implementation) → Refactor

**Tasks:**
- [x] Implement Locator struct
  - Store selector, frame reference
  - Implement Clone (cheap Arc wrapper)
  - Protocol: selector serialization
- [x] Implement `page.locator(selector)` method
  - Create Locator with main frame reference
  - Support CSS selectors
- [x] Implement locator chaining
  - `locator.locator(selector)` - sub-locator
  - `locator.first()` - first match
  - `locator.last()` - last match
  - `locator.nth(index)` - nth match
- [x] Implement query methods
  - `locator.count()` - count matching elements
  - `locator.text_content()` - get text content
  - `locator.inner_text()` - get visible text
  - `locator.inner_html()` - get HTML
  - `locator.get_attribute(name)` - get attribute value
- [x] Implement state query methods
  - `locator.is_visible()` - check visibility
  - `locator.is_enabled()` - check enabled state
  - `locator.is_checked()` - check checkbox state
  - `locator.is_editable()` - check editable state
- [x] Protocol communication
  - Frame delegate methods for all locator operations
  - Query protocol messages (querySelectorAll, textContent, innerText, innerHTML, getAttribute)
  - State query protocol messages (isVisible, isEnabled, isChecked, isEditable)
- [x] Tests
  - Test locator creation
  - Test locator chaining (first, last, nth)
  - Test nested locators
  - Test count on multiple elements
  - Test text/HTML queries
  - Test attribute queries (deferred - not in initial tests)
  - Test state queries
  - Cross-browser tests (Firefox, WebKit)

**Files Created:**
- `crates/playwright-core/src/protocol/locator.rs` - Locator struct and methods
- `crates/playwright-core/tests/locator_test.rs`

**Files Modified:**
- `crates/playwright-core/src/protocol/mod.rs` - Exported Locator
- `crates/playwright-core/src/protocol/page.rs` - Added locator() method (async), fixed doctest
- `crates/playwright-core/src/protocol/frame.rs` - Added 9 locator delegate methods (count, text_content, inner_text, inner_html, get_attribute, is_visible, is_enabled, is_checked, is_editable)
- `README.md` - Updated quick example to show locators
- `README.md` - Updated "What works now" section with locator features

**Acceptance Criteria:**
- ✅ Can create locators with selectors
- ✅ Locator chaining works correctly
- ✅ Query methods return expected values
- ✅ State queries return correct boolean values
- ✅ Cross-browser tests pass (Firefox, WebKit)

### Slice 3: Core Actions (Click, Fill, Press)

**Goal:** Implement the three most common user interactions with full options support.

**Why Third:** These are the most frequently used actions. Forms require fill, navigation requires click.

**Status:** ✅ **COMPLETE**

**Implementation Notes:**
- Implemented 5 core action methods: click(), dblclick(), fill(), clear(), press()
- All methods delegate to Frame with strict=true (single-element enforcement)
- Options support deferred - all methods accept Option<()> for now (no options implemented)
- Created test_server infrastructure with axum for robust integration testing
- Test server serves custom HTML pages for deterministic action testing
- Refactored locator_test.rs to use test_server for deterministic testing
- Action tests verify behavior where possible (click/dblclick verify text changes)
- Fill/clear/press tests only verify methods succeed (need input_value() for full verification)
- Added inline TODOs for input_value() implementation (needed in Slice 4)
- Cross-browser testing: All action tests pass on Chromium, Firefox, WebKit

**Tasks:**
- [x] Implement `locator.click(options)` - Basic implementation with Option<()>
- [x] Implement `locator.dblclick(options)` - Basic implementation with Option<()>
- [x] Implement `locator.fill(text, options)` - Basic implementation with Option<()>
- [x] Implement `locator.clear(options)` - Basic implementation with Option<()>
- [x] Implement `locator.press(key, options)` - Basic implementation with Option<()>
- [x] Frame delegate methods - Added 5 methods to Frame (click, dblclick, fill, clear, press)
- [x] Test infrastructure - Created test_server with axum for localhost testing
- [x] Tests - Created actions_test.rs with comprehensive coverage + cross-browser tests
- [x] Refactored locator_test.rs to use test_server
- [x] Added inline TODOs for input_value() implementation
- [x] Updated README.md to show actions working
- [x] Verified examples still work (actions.rs demonstrates click/dblclick)

**Deferred to Future Slices:**
- Options implementation (ClickOptions, FillOptions, PressOptions with builder pattern)
- Enum types (MouseButton, KeyboardModifier)
- Position types
- Option serialization
- Tests with options (position, modifiers, force, trial)
- Timeout and error handling tests

**Files Created:**
- `crates/playwright-core/tests/test_server.rs` - Axum test server for localhost testing
- `crates/playwright-core/tests/actions_test.rs` - Action integration tests

**Files Modified:**
- `crates/playwright-core/src/protocol/locator.rs` - Added 5 action methods + TODO for input_value()
- `crates/playwright-core/src/protocol/frame.rs` - Added 5 Frame delegate methods + TODO
- `crates/playwright-core/tests/locator_test.rs` - Refactored to use test_server
- `crates/playwright-core/tests/page_navigation_test.rs` - Updated TODO comment
- `crates/playwright/examples/actions.rs` - Already existed, demonstrates click/dblclick
- `README.md` - Updated to show actions in quick example
- `Cargo.toml` (playwright-core) - Added axum dev-dependencies

**Acceptance Criteria:**
- ✅ Click successfully clicks elements
- ✅ Dblclick successfully double-clicks elements
- ✅ Fill successfully fills form inputs (verified method succeeds)
- ✅ Clear successfully clears inputs (verified method succeeds)
- ✅ Press successfully sends keyboard events (verified method succeeds)
- ✅ All action tests pass cross-browser (Chromium, Firefox, WebKit)
- ✅ Test infrastructure robust (localhost test server with custom HTML)
- ⚠️ Behavioral verification partial (click/dblclick verify text, fill/clear/press need input_value())

### Slice 4: Checkbox and Hover Actions

**Goal:** Implement checkbox interactions and hover for dropdown testing.

**Why Fourth:** Checkboxes are common form elements. Hover needed for dropdown interactions.

**Status:** ✅ **COMPLETE**

**Implementation Notes:**
- Implemented 4 new methods: check(), uncheck(), hover(), input_value()
- All methods delegate to Frame with strict=true
- Options support deferred - all methods accept Option<()> for now
- Added checkbox.html and hover.html to test_server with checkboxes, radio buttons, hover tooltips
- Created checkbox_test.rs with comprehensive checkbox/hover tests
- Updated actions_test.rs to use input_value() for proper fill/clear/press verification
- All fill/clear/press tests now verify actual input values (no longer just method success)
- Cross-browser testing: All tests pass on Chromium, Firefox, WebKit
- check() and uncheck() are properly idempotent as per Playwright API

**Tasks:**
- [x] Implement `locator.check(options)` - Basic implementation with Option<()>
- [x] Implement `locator.uncheck(options)` - Basic implementation with Option<()>
- [x] Implement `locator.hover(options)` - Basic implementation with Option<()>
- [x] Implement `locator.input_value(options)` - Returns input/textarea/select values
- [x] Frame delegate methods - Added 4 methods to Frame (check, uncheck, hover, input_value)
- [x] Test infrastructure - Added checkbox.html and hover.html to test_server
- [x] Tests - Created checkbox_test.rs with comprehensive coverage + cross-browser tests
- [x] Updated actions_test.rs - Fill/clear/press tests now verify actual values with input_value()
- [x] Verified all existing tests still pass

**Deferred to Future Slices:**
- Options implementation (CheckOptions, HoverOptions with builder pattern)
- set_checked() convenience method (calls check/uncheck based on boolean)
- Test uncheck fails on radio button (currently Playwright server handles this)

**Files Created:**
- `crates/playwright-core/tests/checkbox_test.rs` - Checkbox and hover integration tests

**Files Modified:**
- `crates/playwright-core/src/protocol/locator.rs` - Added 4 methods (check, uncheck, hover, input_value)
- `crates/playwright-core/src/protocol/frame.rs` - Added 4 Frame delegate methods
- `crates/playwright-core/tests/test_server.rs` - Added checkbox.html and hover.html pages
- `crates/playwright-core/tests/actions_test.rs` - Updated fill/clear/press tests to verify values with input_value()
- `crates/playwright/examples/actions.rs` - Updated to demonstrate hover() and list all available form actions
- `README.md` - Added checkbox, hover, and input_value to quick example and "What works now" list

**Acceptance Criteria:**
- ✅ Check successfully checks unchecked checkboxes
- ✅ Check is idempotent (no-op on already checked)
- ✅ Uncheck successfully unchecks checked checkboxes
- ✅ Uncheck is idempotent (no-op on already unchecked)
- ✅ Check works on radio buttons
- ✅ Hover successfully triggers :hover CSS states (tooltip visibility)
- ✅ input_value() returns correct input/textarea values
- ✅ Fill/clear/press tests now verify actual values (not just method success)
- ✅ All tests pass cross-browser (Chromium, Firefox, WebKit)

### Slice 5: Select and File Upload

**Goal:** Implement dropdown selection and file upload functionality.

**Why Fifth:** Common form interactions. File upload needed for many test scenarios.

**Status:** ✅ **COMPLETE**

**Implementation Notes:**
- Implemented 4 new methods: select_option(), select_option_multiple(), set_input_files(), set_input_files_multiple()
- All methods delegate to Frame with strict=true
- Options support deferred - all methods accept Option<()> for now
- select_option() accepts single value string, returns Vec<String> of selected values
- select_option_multiple() accepts slice of strings, returns Vec<String>
- set_input_files() accepts PathBuf reference for single file
- set_input_files_multiple() accepts slice of PathBuf references for multiple files or empty array to clear
- File contents are read and base64-encoded for protocol transmission
- Protocol messages: "selectOption" with "options" parameter (array of {value: "..."})
- Protocol messages: "setInputFiles" with "payloads" parameter (array of {name, buffer})
- Added select.html and upload.html to test_server
- Created select_upload_test.rs with comprehensive tests
- Cross-browser testing: All tests pass on Chromium, Firefox, WebKit
- Added base64 dependency for file encoding

**Tasks:**
- [x] Implement `locator.select_option(value, options)` - Single value selection
- [x] Implement `locator.select_option_multiple(values, options)` - Multiple value selection
- [x] Implement `locator.set_input_files(file, options)` - Single file upload
- [x] Implement `locator.set_input_files_multiple(files, options)` - Multiple files or clear
- [x] Frame delegate methods - Added 4 methods (select, select_multiple, upload, upload_multiple)
- [x] Test infrastructure - Added select.html and upload.html to test_server
- [x] Tests - Created select_upload_test.rs with comprehensive coverage + cross-browser tests
- [x] File handling - Read files, base64 encode, send to protocol
- [x] Added base64 dependency to Cargo.toml
- [x] Added InvalidArgument error variant

**Deferred to Future Slices:**
- Options implementation (SelectOption with builder pattern for force, timeout)
- FilePayload struct for in-memory file creation (currently only supports PathBuf)
- Select by label or index (currently only supports value)
- FileInput enum for different input types

**Files Created:**
- `crates/playwright-core/tests/select_upload_test.rs` - Select and file upload integration tests

**Files Modified:**
- `crates/playwright-core/src/protocol/locator.rs` - Added 4 methods (select_option, select_option_multiple, set_input_files, set_input_files_multiple)
- `crates/playwright-core/src/protocol/frame.rs` - Added 4 Frame delegate methods
- `crates/playwright-core/src/error.rs` - Added InvalidArgument error variant
- `crates/playwright-core/tests/test_server.rs` - Added select.html and upload.html pages
- `crates/playwright-core/Cargo.toml` - Added base64 = "0.22" dependency

**Acceptance Criteria:**
- ✅ Select option successfully selects dropdown values
- ✅ Select option returns array of selected values
- ✅ Multiple select works with multiple values
- ✅ File upload successfully uploads single file
- ✅ File upload successfully uploads multiple files
- ✅ Clear file input works (empty array)
- ✅ All tests pass cross-browser (Chromium, Firefox, WebKit)

### Slice 6: Keyboard and Mouse APIs

**Goal:** Implement low-level keyboard and mouse control for advanced interactions.

**Why Sixth:** Lower-level APIs for complex interactions. Less common than locator actions.

**Status:** ✅ **COMPLETE**

**Implementation Notes:**
- Keyboard and Mouse are NOT ChannelOwners - lightweight wrappers holding Page reference
- All keyboard/mouse messages sent through Page channel (not Frame channel)
- dblclick uses mouseClick with clickCount=2 (not a separate protocol message)
- Keyboard methods: down, up, press, type_text, insert_text
- Mouse methods: move_to, click, dblclick, down, up, wheel
- Options support deferred - all methods accept Option<()> for now
- Created keyboard_mouse.html in test_server with input, clickable div, event handlers
- Created keyboard_mouse_test.rs with 11 comprehensive tests
- Cross-browser testing: All tests pass on Chromium, Firefox, WebKit
- Protocol messages: keyboardDown, keyboardUp, keyboardPress, keyboardType, keyboardInsertText
- Protocol messages: mouseMove, mouseClick, mouseDown, mouseUp, mouseWheel

**Tasks:**
- [x] Implement Keyboard struct
  - Accessed via `page.keyboard()`
  - Methods: down, up, press, type_text, insert_text
  - Delegates to Page protocol methods
- [x] Implement `keyboard.down(key)`
  - Protocol: "keyboardDown" message
  - Hold key until up() called
- [x] Implement `keyboard.up(key)`
  - Protocol: "keyboardUp" message
  - Release previously pressed key
- [x] Implement `keyboard.press(key, options)`
  - Options: deferred (Option<()>)
  - Protocol: "keyboardPress" message
  - Tested with Enter key, Shift+KeyA
- [x] Implement `keyboard.type_text(text, options)`
  - Options: deferred (Option<()>)
  - Protocol: "keyboardType" message
  - Types text character by character
- [x] Implement `keyboard.insert_text(text)`
  - Protocol: "keyboardInsertText" message
  - Insert text without key events (paste-like)
- [x] Implement Mouse struct
  - Accessed via `page.mouse()`
  - Methods: move_to, click, dblclick, down, up, wheel
  - Delegates to Page protocol methods
- [x] Implement `mouse.move_to(x, y, options)`
  - Options: deferred (Option<()>)
  - Protocol: "mouseMove" message
  - Coordinates in CSS pixels relative to viewport
- [x] Implement `mouse.click(x, y, options)`
  - Options: deferred (Option<()>)
  - Protocol: "mouseClick" message
  - Click at absolute coordinates
- [x] Implement `mouse.dblclick(x, y, options)`
  - Options: deferred (Option<()>)
  - Protocol: "mouseClick" with clickCount=2
- [x] Implement `mouse.down(options)` and `mouse.up(options)`
  - Options: deferred (Option<()>)
  - Protocol: "mouseDown", "mouseUp"
  - For drag and drop interactions
- [x] Implement `mouse.wheel(delta_x, delta_y)`
  - Protocol: "mouseWheel" message
  - Scroll by pixel delta
- [x] Tests
  - Test keyboard.type_text types text
  - Test keyboard.press sends key events (Enter)
  - Test keyboard.down/up for key hold (Shift+KeyA)
  - Test keyboard.insert_text (no key events)
  - Test mouse.move_to moves cursor
  - Test mouse.click at coordinates
  - Test mouse.dblclick at coordinates
  - Test mouse.down/up for drag simulation
  - Test mouse.wheel for scrolling
  - Cross-browser tests (Firefox, WebKit)

**Deferred to Future Slices:**
- Options implementation (KeyboardOptions, MouseOptions with builder pattern)
- Modifier key parsing for keyboard.press (e.g., "Control+A")
- Mouse button options (left, middle, right)
- Mouse steps option for smooth movement
- Delay option for keyboard typing

**Files Created:**
- `crates/playwright-core/src/protocol/keyboard.rs` - Keyboard struct with 5 methods
- `crates/playwright-core/src/protocol/mouse.rs` - Mouse struct with 6 methods
- `crates/playwright-core/tests/keyboard_mouse_test.rs` - 11 comprehensive tests

**Files Modified:**
- `crates/playwright-core/src/protocol/mod.rs` - Exported Keyboard and Mouse
- `crates/playwright-core/src/protocol/page.rs` - Added keyboard() and mouse() accessors, 11 internal protocol methods
- `crates/playwright-core/tests/test_server.rs` - Added keyboard_mouse.html page

**Acceptance Criteria:**
- ✅ Keyboard methods successfully send key events
- ✅ keyboard.type_text types text into inputs
- ✅ keyboard.press sends single key events
- ✅ keyboard.down/up allows key combinations (Shift+A)
- ✅ keyboard.insert_text inserts without key events
- ✅ Mouse methods successfully control cursor
- ✅ mouse.click clicks at absolute coordinates
- ✅ mouse.dblclick double-clicks at coordinates
- ✅ mouse.move_to moves cursor (coordinates tracked)
- ✅ mouse.down/up allows drag simulation
- ✅ mouse.wheel scrolls page
- ✅ All tests pass cross-browser (Chromium, Firefox, WebKit)

### Slice 7: Screenshots and Documentation ✅ COMPLETE

**Goal:** Implement screenshot capture and complete Phase 3 documentation.

**Why Last:** Screenshots are important but not blocking for other features. Documentation completes the phase.

**Implementation Summary:**
- ✅ Implemented page.screenshot() with base64 decoding
- ✅ Implemented page.screenshot_to_file() for saving screenshots
- ✅ Cross-browser screenshot tests (Chromium, Firefox, WebKit)
- ✅ Updated README.md with screenshot capability
- ⚠️ Deferred locator.screenshot() to Phase 4 (requires ElementHandle protocol)
- ⚠️ Deferred screenshot options to Phase 4 (ScreenshotType, ScreenshotClip, quality, full_page)

**Tasks Completed:**
- ✅ Implement base64 decoding
  - Using `base64` crate (BASE64_STANDARD engine)
  - Proper error handling for decode failures
- ✅ Implement `page.screenshot(options)` → Result<Vec<u8>>
  - Protocol: "screenshot" message to Page channel
  - Server returns base64-encoded PNG data
  - Decode base64 to Vec<u8>
  - Return bytes for in-memory usage
  - Currently uses PNG format (default)
- ✅ Implement file saving
  - Added `page.screenshot_to_file(path, options)` method
  - Use `tokio::fs::write` for async file I/O
  - Returns bytes AND saves to file
  - Proper error handling for file write errors
- ✅ Tests
  - ✅ Test page screenshot with default options
  - ✅ Test page screenshot saves to file
  - ✅ Test page screenshot returns bytes
  - ✅ Test screenshot cross-browser (Firefox, WebKit)
  - ✅ PNG magic bytes verification
- ✅ Documentation
  - ✅ Updated README.md with screenshot example
  - ✅ Added screenshot to feature list

**Deferred to Phase 4:**
- ⚠️ Screenshot options (type, quality, full_page, clip, omit_background, mask, etc.)
- ⚠️ `locator.screenshot()` - Requires ElementHandle protocol support
- ⚠️ ScreenshotType enum, ScreenshotClip struct
- ⚠️ Full-page, JPEG, clip region, omit_background tests
- ⚠️ Dedicated screenshot example (existing examples demonstrate core features)

**Files Created:**
- `crates/playwright-core/tests/screenshot_test.rs` - 5 page screenshot tests

**Files Modified:**
- `crates/playwright-core/src/protocol/page.rs` - Added screenshot() and screenshot_to_file() methods
- `crates/playwright-core/src/protocol/locator.rs` - Added TODO for screenshot (deferred)
- `crates/playwright-core/src/protocol/frame.rs` - Added TODO for locator_screenshot (deferred)
- `README.md` - Added screenshot capability to quick example and feature list

**Acceptance Criteria Met:**
- ✅ Page screenshots successfully capture visible content
- ✅ Screenshots can be saved to file or returned as bytes
- ✅ All tests pass (159 total tests)
- ✅ Cross-browser verified (Chromium, Firefox, WebKit)
- ✅ README.md shows screenshot capability
- ⚠️ Element screenshots deferred (requires ElementHandle protocol)
- ⚠️ Advanced options deferred (full_page, JPEG, clip, etc.)

---

**Created:** 2025-11-07
**Last Updated:** 2025-11-08

---
