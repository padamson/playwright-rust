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

- [ ] ElementHandle protocol implemented
- [ ] locator.screenshot() works with ElementHandles
- [ ] Screenshot options fully implemented (type, quality, full_page, clip, etc.)
- [ ] Navigation timeout error handling tested
- [ ] ClickOptions with builder pattern implemented
- [ ] Action position and modifiers work correctly
- [ ] SelectOption supports value, label, and index
- [ ] All deferred Phase 3 items addressed (implemented or explicitly re-deferred)
- [ ] All tests passing cross-browser
- [ ] Documentation updated

---

## Implementation Plan

**Status:** Planning - Detailed slice breakdown TBD

Phase 4 will follow the same TDD and vertical slicing approach as Phase 3. Slices will be determined when Phase 4 implementation begins.

**Suggested Slice Order:**
1. ElementHandle protocol + locator.screenshot()
2. Screenshot options (type, quality, full_page, clip)
3. Navigation error handling
4. ClickOptions with builder pattern
5. Other action options (Fill, Press, Check, Hover)
6. SelectOption variants
7. Keyboard/Mouse options and enhancements

This order prioritizes:
- High-impact features (ElementHandles, screenshot options)
- Deferred items blocking other features
- Most commonly used options first
- Progressive complexity (simple options → complex options)

---

**Created:** 2025-11-08
**Last Updated:** 2025-11-08

---
