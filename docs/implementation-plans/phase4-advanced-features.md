# Phase 4: Advanced Features

**Status:** Planning

**Goal:** Implement advanced Playwright features deferred from Phase 3, including ElementHandles, screenshots options, action options, assertions, and network interception.

**Feature:** ElementHandles, screenshot options, action options, assertions with auto-retry, network interception, and other advanced capabilities

**User Story:** As a Rust developer, I want access to advanced Playwright features so that I can write comprehensive browser automation and testing workflows.

**Related ADRs:**
- [ADR-0001: Protocol Architecture](../adr/0001-protocol-architecture.md)

---

## Prerequisites from Phase 3

Phase 4 builds on Phase 3's page interactions:
- ✅ Navigation (goto, reload)
- ✅ Locators and element queries
- ✅ Core actions (click, fill, press, check, hover)
- ✅ Select and file upload
- ✅ Keyboard and Mouse APIs
- ✅ Basic screenshots (page-level, PNG only)

---

## Deferred from Phase 3

The following items were deferred from Phase 3 Slices and need to be implemented in Phase 4:

### Slice 7 Deferrals: Screenshot Options and Element Screenshots

**ElementHandle Protocol Support** (High Priority)
- Required for `locator.screenshot()` - element-level screenshots
- ElementHandle is a protocol object representing single elements
- Needed for advanced element interactions
- **Discovery**: Frame.screenshot with selector isn't supported by Playwright protocol
- **Requirement**: Must implement ElementHandle protocol objects first

**Screenshot Options** (Medium Priority)
- ScreenshotType enum: Png, Jpeg
- ScreenshotClip struct: `{ x, y, width, height }`
- Options for page.screenshot() and locator.screenshot():
  - `type`: Png or Jpeg format
  - `quality`: JPEG quality (0-100)
  - `full_page`: Capture beyond viewport (full scrollable page)
  - `clip`: Capture specific region
  - `omit_background`: Transparent PNG
  - `mask`: Hide sensitive elements
  - `mask_color`: Color for masked elements
  - `timeout`: Screenshot timeout

**Screenshot Tests** (Medium Priority)
- Test full-page screenshot (captures beyond viewport)
- Test JPEG screenshot with quality option
- Test screenshot with clip region
- Test screenshot with omit_background (transparent PNG)
- Test element screenshot (locator.screenshot())
- Test mask option (hide sensitive elements)

**Screenshot Examples** (Low Priority)
- Consider creating examples/screenshots.rs
- Show page screenshot, element screenshot, options usage

### Slice 6 Deferrals: Keyboard and Mouse Options

**KeyboardOptions** (Low Priority)
- Options for keyboard.press() and keyboard.type_text()
- `delay`: Delay between key presses (milliseconds)
- Builder pattern with Option<T> fields

**MouseOptions** (Low Priority)
- Options for mouse methods
- `button`: MouseButton enum (left, middle, right)
- `click_count`: Number of clicks
- `delay`: Delay between mousedown and mouseup
- `steps`: Number of intermediate mousemove events for smooth movement
- Builder pattern with Option<T> fields

**Mouse/Keyboard Enhancements** (Low Priority)
- Modifier key parsing for keyboard.press (e.g., "Control+A", "Shift+Enter")
- MouseButton enum: Left, Middle, Right

### Slice 5 Deferrals: Select and Upload Options

**SelectOptions** (Low Priority)
- Options for select_option methods
- `force`: Skip actionability checks
- `timeout`: Selection timeout
- Builder pattern

**SelectOption Variants** (Medium Priority)
- Currently only supports value string selection
- Add support for label selection
- Add support for index selection
- SelectOption enum: Value(String), Label(String), Index(usize)

**FilePayload Struct** (Low Priority)
- In-memory file creation without PathBuf
- `FilePayload { name: String, mime_type: String, buffer: Vec<u8> }`
- Useful for testing without creating temp files

### Slice 4 Deferrals: Checkbox and Hover Options

**CheckOptions and HoverOptions** (Low Priority)
- Options for check(), uncheck(), hover()
- `force`: Skip actionability checks
- `timeout`: Action timeout
- `position`: Click position within element
- Builder pattern

**set_checked() Convenience Method** (Low Priority)
- `locator.set_checked(checked: bool)` - Calls check() or uncheck() based on boolean
- Idiomatic alternative to if/else with check/uncheck

### Slice 3 Deferrals: Core Action Options

**ClickOptions** (Medium Priority)
- Options for click() and dblclick()
- `button`: MouseButton enum (left, middle, right)
- `click_count`: Number of clicks
- `delay`: Delay between mousedown and mouseup
- `position`: Click position within element `{ x, y }`
- `modifiers`: Keyboard modifiers (Shift, Control, Alt, Meta)
- `force`: Skip actionability checks
- `no_wait_after`: Don't wait for navigation
- `timeout`: Action timeout
- `trial`: Perform checks without clicking
- Builder pattern with Option<T> fields

**FillOptions and PressOptions** (Low Priority)
- Options for fill(), clear(), press()
- `force`: Skip actionability checks
- `timeout`: Action timeout
- `no_wait_after`: Don't wait for navigation
- Builder pattern

**Position Types** (Low Priority)
- Position struct: `{ x: f64, y: f64 }`
- Used in click options, hover options

**Enum Types** (Low Priority)
- MouseButton enum: Left, Middle, Right
- KeyboardModifier enum: Shift, Control, Alt, Meta

**Options Tests** (Medium Priority)
- Test click with position option
- Test click with modifiers option
- Test click with force option
- Test trial option (performs checks without clicking)
- Test timeout and error handling

### Slice 1 Deferrals: Navigation Timeout Handling

**Navigation Error Handling** (High Priority)
- Test timeout error handling for goto()
- Test timeout error handling for reload()
- Verify error messages are descriptive
- Test wait_until option behavior

---

## Proposed Scope for Phase 4

### High Priority

1. **ElementHandle Protocol Support**
   - Implement ElementHandle protocol objects
   - Element-level screenshot support
   - Advanced element interactions

2. **Navigation Error Handling**
   - Timeout tests for goto(), reload()
   - Descriptive error messages
   - wait_until option validation

3. **Screenshot Options**
   - ScreenshotType enum (Png, Jpeg)
   - ScreenshotClip struct
   - Implement options for page.screenshot()
   - Implement locator.screenshot() with ElementHandles

### Medium Priority

4. **Action Options (Core)**
   - ClickOptions with builder pattern
   - Position types
   - MouseButton enum
   - Click options tests

5. **Select Option Variants**
   - SelectOption enum (Value, Label, Index)
   - Support label and index selection

6. **Screenshot Tests**
   - Full-page screenshots
   - JPEG format with quality
   - Clip region
   - Element screenshots

### Low Priority

7. **Remaining Action Options**
   - FillOptions, PressOptions
   - CheckOptions, HoverOptions
   - KeyboardOptions, MouseOptions
   - All with builder patterns

8. **Convenience Methods**
   - set_checked(bool) for checkboxes
   - Modifier key parsing for keyboard

9. **File Upload Enhancements**
   - FilePayload struct for in-memory files

### Future Phases (Not Phase 4)

- **Assertions** with auto-retry (expect API)
- **Network interception** and route mocking
- **Mobile emulation** (device descriptors, viewport, user agent)
- **Videos** and **tracing**
- **Downloads** and **dialogs**
- **Context options** (viewport, user agent, geolocation, permissions)

---

## Success Criteria

Phase 4 will be considered complete when:

- [x] ElementHandle protocol implemented ✅ (Slice 1)
- [x] locator.screenshot() works with ElementHandles ✅ (Slice 1)
- [x] Screenshot options fully implemented (type, quality, full_page, clip, etc.) ✅ (Slice 2)
- [x] Navigation timeout error handling tested ✅ (Slice 3)
- [x] ClickOptions with builder pattern implemented ✅ (Slice 4)
- [x] Action position and modifiers work correctly ✅ (Slice 4)
- [ ] SelectOption supports value, label, and index
- [ ] All deferred Phase 3 items addressed (implemented or explicitly re-deferred)
- [ ] All tests passing cross-browser
- [ ] Documentation updated

---

## Implementation Plan

**Status:** In Progress - Slices 1-4 Complete ✅, Ready for Slice 5

Phase 4 follows the same TDD and vertical slicing approach as Phase 3.

### Slice 1: ElementHandle Protocol & Element Screenshots ✅

**Status:** Complete (2025-11-08)

**Goal:** Implement ElementHandle as a ChannelOwner protocol object and enable element-level screenshots via locator.screenshot().

**Why First:** ElementHandles are required for locator.screenshot() (deferred from Phase 3). This is the highest-priority deferred item.

**Research Complete:** ✅
- ElementHandles are ChannelOwner protocol objects created via `__create__` messages
- Element screenshots use `ElementHandle.screenshot` channel method (NOT `Frame.screenshot`)
- Frame already returns ElementHandle GUIDs from `querySelectorAll` but we currently only use the count
- Pattern matches Request/Response object implementation

**Tasks:**
- [x] Create `element_handle.rs` protocol module
  - ElementHandle struct with ChannelOwnerImpl base
  - Implement ChannelOwner trait
  - Constructor: `new(parent, type_name, guid, initializer)`
  - Method: `screenshot(options) -> Result<Vec<u8>>`
  - Base64 decoding for screenshot data
- [x] Update `frame.rs` with query methods
  - `query_selector(selector) -> Result<Option<Arc<ElementHandle>>>`
  - `query_selector_all(selector) -> Result<Vec<Arc<ElementHandle>>>`
  - Helper to convert GUID responses to ElementHandle objects
- [x] Update `page.rs` with query delegates
  - `query_selector()` - delegates to main_frame
  - `query_selector_all()` - delegates to main_frame
- [x] Update `locator.rs`
  - Uncomment and implement `screenshot()` method
  - Use query_selector to get ElementHandle, call screenshot()
- [x] Update `object_factory.rs`
  - Add "ElementHandle" case in match statement
  - Call ElementHandle::new() with proper parent
- [x] Update `mod.rs`
  - Export ElementHandle module
- [x] Tests
  - Test query_selector returns ElementHandle
  - Test query_selector returns None when not found
  - Test query_selector_all returns multiple handles
  - Test ElementHandle.screenshot() directly
  - Test locator.screenshot() delegates to ElementHandle
  - test_locator_screenshot in screenshot_test.rs working
  - Cross-browser tests (Chromium, Firefox, WebKit)
- [x] Documentation
  - Removed debug statements from frame.rs
  - Added rustdoc to ElementHandle methods
  - Link to Playwright ElementHandle docs

**Files Created:**
- `crates/playwright-core/src/protocol/element_handle.rs`
- `crates/playwright-core/tests/element_handle_test.rs`

**Files Modified:**
- `crates/playwright-core/src/protocol/frame.rs`
- `crates/playwright-core/src/protocol/page.rs`
- `crates/playwright-core/src/protocol/locator.rs`
- `crates/playwright-core/src/protocol/mod.rs`
- `crates/playwright-core/src/object_factory.rs`
- `crates/playwright-core/src/error.rs` (added ElementNotFound variant)
- `crates/playwright-core/tests/screenshot_test.rs`
- `crates/playwright-core/tests/test_server.rs` (added `/locators.html` route)

**Acceptance Criteria:** ✅ All Met
- ✅ ElementHandle is a proper ChannelOwner protocol object
- ✅ query_selector methods work correctly
- ✅ ElementHandle.screenshot() captures element screenshots
- ✅ locator.screenshot() works via ElementHandle
- ✅ All tests pass cross-browser (Chromium, Firefox, WebKit)
- ✅ Debug statements removed

---

### Slice 2: Screenshot Options (Type, Quality, Full Page, Clip) ✅

**Status:** Complete (2025-11-08)

**Goal:** Implement ScreenshotOptions struct with builder pattern for page and element screenshots.

**Why Second:** Second-highest priority deferred item. Users need JPEG, full-page, and clip options.

**Tasks:**
- [x] Create ScreenshotOptions struct
- [x] Create ScreenshotType enum (Png, Jpeg)
- [x] Create ScreenshotClip struct
- [x] Implement builder pattern
- [x] Update page.screenshot() to accept ScreenshotOptions
- [x] Update ElementHandle.screenshot() to accept ScreenshotOptions
- [x] Update locator.screenshot() to accept ScreenshotOptions
- [x] Tests for all option combinations (combined into efficient tests)
- [x] Cross-browser tests (Chromium, Firefox, WebKit)

**Files Created:**
- `crates/playwright-core/src/protocol/screenshot.rs`
- `crates/playwright-core/tests/screenshot_options_test.rs`

**Files Modified:**
- `crates/playwright-core/src/protocol/mod.rs`
- `crates/playwright-core/src/protocol/page.rs`
- `crates/playwright-core/src/protocol/element_handle.rs`
- `crates/playwright-core/src/protocol/locator.rs`
- `crates/playwright-core/tests/screenshot_test.rs` (removed TODO)

**Acceptance Criteria:** ✅ All Met
- ✅ ScreenshotType enum (Png, Jpeg) with proper serialization
- ✅ ScreenshotClip struct for region capture
- ✅ ScreenshotOptions with builder pattern
- ✅ Support for type, quality, full_page, clip, omit_background options
- ✅ All screenshot methods accept options
- ✅ All tests pass cross-browser

---

### Slice 3: Navigation Error Handling ✅

**Status:** Complete (2025-11-08)

**Goal:** Add timeout tests and error handling for navigation methods.

**Why Third:** High-priority deferred item from Phase 3 Slice 1.

**Tasks:**
- [x] Test goto() timeout errors
- [x] Test reload() timeout errors
- [x] Test wait_until option behavior (Load, DomContentLoaded, NetworkIdle)
- [x] Verify descriptive error messages
- [x] Cross-browser error tests (Chromium, Firefox, WebKit)

**Files Created:**
- `crates/playwright-core/tests/navigation_errors_test.rs`

**Acceptance Criteria:** ✅ All Met
- ✅ Navigation timeout errors properly tested (unreachable URLs)
- ✅ Valid navigation with timeout options works
- ✅ Invalid URL errors handled correctly
- ✅ reload() timeout behavior tested
- ✅ wait_until options work (Load, DomContentLoaded, NetworkIdle)
- ✅ Error messages contain "timeout" for timeout errors
- ✅ All tests pass cross-browser (Chromium, Firefox, WebKit)

---

### Slice 4: ClickOptions with Builder Pattern ✅

**Status:** Complete (2025-11-08)

**Goal:** Implement ClickOptions with position, modifiers, button, force, trial options.

**Why Fourth:** Most commonly used action option. Foundation for other action options.

**Tasks:**
- [x] Create ClickOptions struct with builder
- [x] Create MouseButton enum (Left, Right, Middle)
- [x] Create KeyboardModifier enum (Alt, Control, Meta, Shift, ControlOrMeta)
- [x] Create Position struct
- [x] Update click() and dblclick() signatures to accept ClickOptions
- [x] Serialize options to protocol format
- [x] Tests with position, modifiers, button options
- [x] Test trial option (dry-run)
- [x] Test force option
- [x] Cross-browser tests (Chromium, Firefox, WebKit)

**Files Created:**
- `crates/playwright-core/src/protocol/click.rs`
- `crates/playwright-core/tests/click_options_test.rs`

**Files Modified:**
- `crates/playwright-core/src/protocol/mod.rs`
- `crates/playwright-core/src/protocol/locator.rs`
- `crates/playwright-core/src/protocol/frame.rs`
- `crates/playwright-core/tests/test_server.rs` (added `/click_options.html` route)

**Acceptance Criteria:** ✅ All Met
- ✅ MouseButton enum (Left, Right, Middle) with proper serialization
- ✅ KeyboardModifier enum (Alt, Control, Meta, Shift, ControlOrMeta)
- ✅ Position struct for click coordinates
- ✅ ClickOptions with builder pattern
- ✅ Support for button, click_count, delay, force, modifiers, no_wait_after, position, timeout, trial options
- ✅ click() and dblclick() methods accept ClickOptions
- ✅ Cross-browser compatibility verified (Chromium, Firefox, WebKit)

---

### Slice 5: Other Action Options

**Goal:** Implement remaining action options (Fill, Press, Check, Hover, Select, Keyboard, Mouse).

**Why Fifth:** Complete the options pattern for all deferred actions.

**Tasks:**
- [ ] FillOptions, PressOptions
- [ ] CheckOptions, HoverOptions
- [ ] SelectOptions (for select_option)
- [ ] KeyboardOptions (delay)
- [ ] MouseOptions (button, steps, delay)
- [ ] Update all action method signatures
- [ ] Comprehensive option tests

---

### Slice 6: SelectOption Variants

**Goal:** Support selection by value, label, or index (not just value).

**Why Sixth:** Medium-priority enhancement for select_option.

**Tasks:**
- [ ] Create SelectOption enum (Value, Label, Index)
- [ ] Update select_option() to accept SelectOption
- [ ] Update select_option_multiple() similarly
- [ ] Protocol serialization for each variant
- [ ] Tests for label and index selection

---

### Slice 7: Documentation and Polish

**Goal:** Complete Phase 4 documentation and address any remaining deferred items.

**Tasks:**
- [ ] Complete rustdoc for all new types
- [ ] Update README with Phase 4 examples
- [ ] Update roadmap.md
- [ ] Review all Phase 3 deferrals - confirm addressed
- [ ] Update CLAUDE.md if needed

---

This order prioritizes:
- High-impact features (ElementHandles, screenshot options)
- Deferred items blocking other features
- Most commonly used options first
- Progressive complexity (simple options → complex options)

---

**Created:** 2025-11-08
**Last Updated:** 2025-11-08

---
