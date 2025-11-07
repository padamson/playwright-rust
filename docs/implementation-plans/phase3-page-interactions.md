# Phase 3: Page Interactions

**Status:** In Progress - Slice 1 (Navigation) COMPLETE ✅

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

**Tasks:**
- [ ] Implement Locator struct
  - Store selector, frame reference, options
  - Implement Clone (cheap Arc wrapper)
  - Protocol: selector serialization
- [ ] Implement `page.locator(selector, options)` method
  - Create Locator with main frame reference
  - Support CSS selectors initially
  - Add `LocatorOptions` (has_text, has)
- [ ] Implement locator chaining
  - `locator.locator(selector)` - sub-locator
  - `locator.first()` - first match
  - `locator.last()` - last match
  - `locator.nth(index)` - nth match
- [ ] Implement query methods
  - `locator.count()` - count matching elements
  - `locator.text_content()` - get text content
  - `locator.inner_text()` - get visible text
  - `locator.inner_html()` - get HTML
  - `locator.get_attribute(name)` - get attribute value
- [ ] Implement state query methods
  - `locator.is_visible()` - check visibility
  - `locator.is_enabled()` - check enabled state
  - `locator.is_checked()` - check checkbox state
  - `locator.is_editable()` - check editable state
- [ ] Protocol communication
  - Frame.locator protocol message
  - Query protocol messages (textContent, etc.)
  - State query protocol messages
  - Timeout handling for queries
- [ ] Tests
  - Test locator creation
  - Test locator chaining (first, last, nth)
  - Test count on multiple elements
  - Test text/HTML queries
  - Test attribute queries
  - Test state queries
  - Test selector not found errors
  - Cross-browser tests

**Files Created:**
- `crates/playwright/src/api/locator.rs` - Locator struct and methods
- `tests/locator_test.rs` - Locator integration tests

**Files Modified:**
- `crates/playwright/src/api/page.rs` - Add locator() method
- `crates/playwright/src/api/frame.rs` - Add locator protocol support
- `crates/playwright/src/api/options.rs` - Add LocatorOptions
- `crates/playwright-core/src/protocol/generated.rs` - Locator protocol types

**Acceptance Criteria:**
- Can create locators with selectors
- Locator chaining works correctly
- Query methods return expected values
- State queries return correct boolean values
- Element not found returns descriptive error
- All tests pass cross-browser

### Slice 3: Core Actions (Click, Fill, Press)

**Goal:** Implement the three most common user interactions with full options support.

**Why Third:** These are the most frequently used actions. Forms require fill, navigation requires click.

**Tasks:**
- [ ] Define option structs
  - `ClickOptions` with builder pattern
  - `FillOptions` with builder pattern
  - `PressOptions` with builder pattern
- [ ] Define enum types
  - `MouseButton` - Left, Right, Middle
  - `KeyboardModifier` - Alt, Control, ControlOrMeta, Meta, Shift
- [ ] Define position types
  - `Position { x: f64, y: f64 }`
- [ ] Implement `locator.click(options)`
  - Options: button, click_count, delay, force, modifiers, position, timeout, trial
  - Protocol: "click" message to Frame
  - Auto-waiting and actionability checks (server-side)
  - Return Result<()>
- [ ] Implement `locator.dblclick(options)`
  - Similar to click with click_count=2
  - Separate method for ergonomics
- [ ] Implement `locator.fill(text, options)`
  - Options: force, timeout
  - Protocol: "fill" message
  - Clears input before filling
  - Auto-waiting for editable element
- [ ] Implement `locator.clear(options)`
  - Options: force, timeout
  - Protocol: "clear" message
  - Clears input/textarea content
- [ ] Implement `locator.press(key, options)`
  - Options: delay (between keydown/keyup), timeout
  - Protocol: "press" message
  - Key names: "Enter", "Escape", "ArrowLeft", "Control+A", etc.
- [ ] Implement option serialization
  - Convert Duration to milliseconds
  - Convert enums to strings
  - Convert Position to {x, y} object
  - Filter out None values
  - Use camelCase for protocol
- [ ] Tests
  - Test click with default options
  - Test click with position, modifiers
  - Test double-click
  - Test fill text input
  - Test fill textarea
  - Test clear input
  - Test press Enter key
  - Test press with modifiers (Control+A)
  - Test timeout errors
  - Test element not found errors
  - Test disabled element errors
  - Cross-browser tests

**Files Modified:**
- `crates/playwright/src/api/locator.rs` - Add click, fill, press methods
- `crates/playwright/src/api/options.rs` - Add ClickOptions, FillOptions, PressOptions
- `crates/playwright/src/api/types.rs` - New file for MouseButton, KeyboardModifier, Position
- `crates/playwright/src/api/frame.rs` - Add action protocol support
- `tests/actions_test.rs` - New integration tests

**Acceptance Criteria:**
- Click successfully clicks elements
- Fill successfully fills form inputs
- Press successfully sends keyboard events
- Options work correctly (position, modifiers, timeout)
- Auto-waiting prevents clicking hidden elements
- Force option bypasses actionability checks
- Trial option does dry-run without action
- All tests pass cross-browser

### Slice 4: Checkbox and Hover Actions

**Goal:** Implement checkbox interactions and hover for dropdown testing.

**Why Fourth:** Checkboxes are common form elements. Hover needed for dropdown interactions.

**Tasks:**
- [ ] Implement `locator.check(options)`
  - Options: force, position, timeout, trial
  - Protocol: "check" message
  - Auto-waiting for checkbox/radio input
  - Idempotent (no-op if already checked)
- [ ] Implement `locator.uncheck(options)`
  - Options: force, position, timeout, trial
  - Protocol: "uncheck" message
  - Only works on checkboxes (not radio buttons)
  - Idempotent (no-op if already unchecked)
- [ ] Implement `locator.set_checked(checked, options)`
  - Convenience method that calls check() or uncheck()
- [ ] Implement `locator.hover(options)`
  - Options: force, modifiers, position, timeout, trial
  - Protocol: "hover" message
  - Auto-waiting for element to be stable
  - Triggers :hover CSS states
- [ ] Tests
  - Test check on unchecked checkbox
  - Test check is idempotent (already checked)
  - Test uncheck on checked checkbox
  - Test uncheck is idempotent (already unchecked)
  - Test set_checked(true) and set_checked(false)
  - Test check on radio button
  - Test uncheck fails on radio button
  - Test hover shows dropdown menu
  - Test hover with position option
  - Test timeout errors
  - Cross-browser tests

**Files Modified:**
- `crates/playwright/src/api/locator.rs` - Add check, uncheck, set_checked, hover methods
- `crates/playwright/src/api/options.rs` - Add CheckOptions, HoverOptions
- `tests/actions_test.rs` - Add checkbox and hover tests

**Acceptance Criteria:**
- Check/uncheck successfully toggle checkboxes
- Check works on radio buttons
- Uncheck fails gracefully on radio buttons
- Hover successfully triggers :hover states
- All tests pass cross-browser

### Slice 5: Select and File Upload

**Goal:** Implement dropdown selection and file upload functionality.

**Why Fifth:** Common form interactions. File upload needed for many test scenarios.

**Tasks:**
- [ ] Implement `locator.select_option(values, options)`
  - Accept single value or Vec of values
  - Options: force, timeout
  - Protocol: "selectOption" message
  - Support selecting by value, label, or index
  - Return Vec<String> of selected values
- [ ] Define file input types
  - `FilePayload { name, mime_type, buffer }`
  - `FileInput` enum: Path, Paths, Payload, Payloads
- [ ] Implement `locator.set_input_files(files, options)`
  - Options: timeout
  - Protocol: "setInputFiles" message
  - Accept PathBuf, Vec<PathBuf>, FilePayload, Vec<FilePayload>
  - Convert paths to file data (read bytes)
  - Base64 encode file contents for protocol
- [ ] Tests
  - Test select single option by value
  - Test select multiple options
  - Test select by label
  - Test select by index
  - Test file upload with single file
  - Test file upload with multiple files
  - Test file upload with FilePayload
  - Test clearing file input (empty array)
  - Test invalid file path error
  - Cross-browser tests

**Files Modified:**
- `crates/playwright/src/api/locator.rs` - Add select_option, set_input_files methods
- `crates/playwright/src/api/options.rs` - Add SelectOptionOptions, SetInputFilesOptions
- `crates/playwright/src/api/types.rs` - Add FilePayload, FileInput
- `tests/actions_test.rs` - Add select and file upload tests

**Acceptance Criteria:**
- Select option successfully changes dropdown value
- File upload successfully uploads files
- Multiple file upload works
- FilePayload allows in-memory file creation
- All tests pass cross-browser

### Slice 6: Keyboard and Mouse APIs

**Goal:** Implement low-level keyboard and mouse control for advanced interactions.

**Why Sixth:** Lower-level APIs for complex interactions. Less common than locator actions.

**Tasks:**
- [ ] Implement Keyboard struct
  - Accessed via `page.keyboard()`
  - Methods: down, up, press, type, insert_text
- [ ] Implement `keyboard.down(key)`
  - Protocol: "keyboardDown" message
  - Hold key until up() called
- [ ] Implement `keyboard.up(key)`
  - Protocol: "keyboardUp" message
  - Release previously pressed key
- [ ] Implement `keyboard.press(key, options)`
  - Options: delay (between down/up)
  - Protocol: calls keyboardDown then keyboardUp
  - Support modifier keys: "Control+A", "Shift+End"
- [ ] Implement `keyboard.type(text, options)`
  - Options: delay (between keystrokes)
  - Protocol: "keyboardType" message
  - Types text character by character
  - Does not press special keys
- [ ] Implement `keyboard.insert_text(text)`
  - Protocol: "keyboardInsertText" message
  - Insert text without key events (paste-like)
- [ ] Implement Mouse struct
  - Accessed via `page.mouse()`
  - Methods: move, click, dblclick, down, up, wheel
- [ ] Implement `mouse.move(x, y, options)`
  - Options: steps (intermediate move events)
  - Protocol: "mouseMove" message
  - Coordinates in CSS pixels relative to viewport
- [ ] Implement `mouse.click(x, y, options)`
  - Options: button, click_count, delay
  - Protocol: "mouseClick" message
  - Click at absolute coordinates
- [ ] Implement `mouse.dblclick(x, y, options)`
  - Similar to click with click_count=2
- [ ] Implement `mouse.down(options)` and `mouse.up(options)`
  - Options: button, click_count
  - Protocol: "mouseDown", "mouseUp"
  - For drag and drop interactions
- [ ] Implement `mouse.wheel(delta_x, delta_y)`
  - Protocol: "mouseWheel" message
  - Scroll by pixel delta
- [ ] Tests
  - Test keyboard.press sends key events
  - Test keyboard.type types text
  - Test keyboard.down/up for key hold
  - Test keyboard modifiers (Control+A selects all)
  - Test keyboard.insert_text (no key events)
  - Test mouse.move moves cursor
  - Test mouse.click at coordinates
  - Test mouse.down/up for drag
  - Test mouse.wheel for scrolling
  - Cross-browser tests

**Files Created:**
- `crates/playwright/src/api/keyboard.rs` - Keyboard struct and methods
- `crates/playwright/src/api/mouse.rs` - Mouse struct and methods
- `tests/keyboard_test.rs` - Keyboard integration tests
- `tests/mouse_test.rs` - Mouse integration tests

**Files Modified:**
- `crates/playwright/src/api/page.rs` - Add keyboard() and mouse() methods
- `crates/playwright/src/api/options.rs` - Add keyboard/mouse option structs

**Acceptance Criteria:**
- Keyboard methods successfully send key events
- Modifiers work correctly (Control+A, Shift+End)
- Type method types text character by character
- Mouse methods successfully control cursor
- Mouse coordinates work correctly
- All tests pass cross-browser

### Slice 7: Screenshots and Documentation

**Goal:** Implement screenshot capture and complete Phase 3 documentation.

**Why Last:** Screenshots are important but not blocking for other features. Documentation completes the phase.

**Tasks:**
- [ ] Define screenshot types
  - `ScreenshotType` enum: Png, Jpeg
  - `ScreenshotClip { x, y, width, height }`
- [ ] Implement `page.screenshot(options)` → Result<Vec<u8>>
  - Options: path, type, quality, full_page, clip, omit_background, mask, mask_color, timeout
  - Protocol: "screenshot" message
  - Server returns base64-encoded image data
  - Decode base64 to Vec<u8>
  - Optionally save to file path
  - Return bytes for in-memory usage
- [ ] Implement `locator.screenshot(options)` → Result<Vec<u8>>
  - Same options as page screenshot
  - Protocol: element-specific screenshot
  - Auto-waits for element to be visible
- [ ] Implement base64 decoding
  - Use `base64` crate
  - Handle decode errors
- [ ] Implement file saving
  - Use `tokio::fs::write` for async file I/O
  - Handle file write errors
- [ ] Tests
  - Test page screenshot with default options
  - Test page screenshot saves to file
  - Test full-page screenshot
  - Test screenshot with clip region
  - Test JPEG screenshot with quality
  - Test screenshot with omit_background (transparent PNG)
  - Test element screenshot
  - Test screenshot returns bytes
  - Test mask option (hide sensitive elements)
  - Cross-browser tests
- [ ] Documentation
  - Complete rustdoc for all Phase 3 APIs
  - Add examples to each method
  - Link to Playwright official docs
  - Update README.md with Phase 3 examples
  - Create examples/navigation.rs
  - Create examples/interactions.rs
  - Create examples/screenshots.rs
- [ ] Update roadmap
  - Mark Phase 3 as complete
  - Update Phase 4 status

**Files Created:**
- `examples/navigation.rs` - Navigation example
- `examples/interactions.rs` - Form interaction example
- `examples/screenshots.rs` - Screenshot example
- `tests/screenshot_test.rs` - Screenshot integration tests

**Files Modified:**
- `crates/playwright/src/api/page.rs` - Add screenshot method
- `crates/playwright/src/api/locator.rs` - Add screenshot method
- `crates/playwright/src/api/options.rs` - Add ScreenshotOptions
- `crates/playwright/src/api/types.rs` - Add ScreenshotType, ScreenshotClip
- `README.md` - Update with Phase 3 examples
- `docs/roadmap.md` - Mark Phase 3 complete

**Acceptance Criteria:**
- Page screenshots successfully capture visible content
- Full-page screenshots capture entire scrollable page
- Screenshots can be saved to file or returned as bytes
- JPEG quality option works correctly
- Clip region captures specified area
- Element screenshots capture single element
- All documentation is complete
- All examples run successfully
- README.md shows Phase 3 capabilities

---

**Created:** 2025-11-07
**Last Updated:** 2025-11-07

---
