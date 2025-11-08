# Playwright-Rust Development Roadmap

**Vision:** Provide production-quality Rust bindings for Microsoft Playwright that match the API and quality of official language bindings (Python, Java, .NET), enabling broad community adoption.

**Architecture:** JSON-RPC communication with Playwright Node.js server (same as all official bindings)

**Status:** Phase 3 complete, planning Phase 4

---

## Overview

This roadmap outlines the path to a production-ready `playwright-rust` library. Each phase builds incrementally toward feature parity with playwright-python while maintaining strict API compatibility and comprehensive testing.

**Key Milestones:**
- ✅ **v0.1.0** - Phase 1 complete (Protocol Foundation)
- ✅ **v0.2.0** - Phase 2 complete (Browser API)
- ✅ **v0.3.0** - Phase 3 complete (Page Interactions)
- 🚀 **v0.4.0** - Phase 4 next (Options & ElementHandles)
- **v0.5.0** - Phase 5 (Advanced Testing Features)
- **v0.6.0** - Phase 6 (Production Hardening)
- **v1.0.0** - Stable release, ready for broad adoption

---

## Phase 1: Protocol Foundation ✅ Complete

**Goal:** Establish JSON-RPC communication with Playwright server and provide access to browser types.

**Status:** ✅ Complete - See [phase1-protocol-foundation.md](implementation-plans/phase1-protocol-foundation.md) and [Technical Summary](technical/phase1-technical-summary.md)

**Key Deliverables:**
- Playwright server download and lifecycle management
- Stdio pipe transport with length-prefixed JSON messages
- JSON-RPC connection layer with request/response correlation
- Protocol object factory (GUID-based object instantiation)
- Entry point: `Playwright::launch().await?`
- Access to `chromium()`, `firefox()`, `webkit()` browser types

---

## Phase 2: Browser API ✅ Complete

**Goal:** Implement browser launching and page lifecycle management.

**Status:** Complete - See [phase2-browser-api.md](implementation-plans/phase2-browser-api.md)

**Delivered:**
- Browser launching (`BrowserType::launch()`)
- Context management (`Browser::new_context()`)
- Page creation (`BrowserContext::new_page()`, `Browser::new_page()`)
- Lifecycle cleanup (close methods)
- Cross-browser testing (Chromium, Firefox, WebKit)

---

## Phase 3: Page Interactions ✅ Complete

**Goal:** Implement core page interactions (navigation, locators, actions) matching playwright-python API.

**Status:** ✅ Complete - See [phase3-page-interactions.md](./implementation-plans/phase3-page-interactions.md)

**Delivered:**
- Navigation: `page.goto()`, `page.reload()`, `page.title()`, URL tracking
- Locators: `page.locator(selector)` with auto-waiting and chaining
- Actions: `click()`, `dblclick()`, `fill()`, `clear()`, `press()`, `check()`, `uncheck()`, `hover()`
- Select and upload: `select_option()`, `set_input_files()`
- Queries: `text_content()`, `inner_text()`, `inner_html()`, `get_attribute()`, `input_value()`
- State queries: `is_visible()`, `is_enabled()`, `is_checked()`, `is_editable()`
- Locator chaining: `first()`, `last()`, `nth()`, `count()`
- Keyboard API: `keyboard.type_text()`, `keyboard.press()`, `keyboard.down()`, `keyboard.up()`, `keyboard.insert_text()`
- Mouse API: `mouse.move_to()`, `mouse.click()`, `mouse.dblclick()`, `mouse.down()`, `mouse.up()`, `mouse.wheel()`
- Screenshots: `page.screenshot()`, `page.screenshot_to_file()` (PNG format)
- Cross-browser testing: All features verified on Chromium, Firefox, WebKit
- Test infrastructure: Test server with custom HTML pages for deterministic testing

---

## Phase 4: Options & ElementHandles

**Goal:** Implement options pattern for actions/screenshots and ElementHandle protocol support (all deferred from Phase 3).

**Status:** Planning - See [phase4-advanced-features.md](./implementation-plans/phase4-advanced-features.md)

**Key Deliverables:**
- ElementHandle protocol support (deferred from Phase 3)
- Element screenshots: `locator.screenshot()` with ElementHandles
- Screenshot options: type, quality, full_page, clip, omit_background, mask
- Action options: ClickOptions, FillOptions, PressOptions with builder patterns
- SelectOption variants: value, label, index selection
- Navigation error handling and timeout tests
- Keyboard/Mouse options: delay, button, modifiers, steps
- All deferred Phase 3 items addressed

**API Preview (Phase 4):**
```rust
// Element screenshots (requires ElementHandle protocol)
let element = page.locator(".logo").await;
element.screenshot(None).await?;

// Screenshot options
use playwright::ScreenshotOptions;
page.screenshot(ScreenshotOptions::new()
    .screenshot_type(ScreenshotType::Jpeg)
    .quality(80)
    .full_page(true)
    .clip(ScreenshotClip { x: 0, y: 0, width: 800, height: 600 })
).await?;

// Click options
use playwright::ClickOptions;
button.click(ClickOptions::new()
    .button(MouseButton::Right)
    .modifiers(vec![KeyboardModifier::Shift])
    .position(Position { x: 10.0, y: 10.0 })
).await?;

// Select by label or index (not just value)
select.select_option(SelectOption::Label("Option 1")).await?;
select.select_option(SelectOption::Index(2)).await?;

// Assertions (if implemented in Phase 4)
use playwright::expect;
expect(page.locator(".success"))
    .to_be_visible()
    .await?;
```

---

## Phase 5: Advanced Testing Features

**Goal:** Implement advanced testing features including assertions, network interception, mobile emulation, and recording capabilities.

**Status:** Not Started

**Key Deliverables:**
- Assertions: `expect(locator).to_be_visible()` with auto-retry
- Network interception: Request/response handling
- Route mocking: `page.route()` API
- Mobile emulation: Device descriptors
- Videos: Recording support
- Tracing: Playwright trace integration
- Downloads: File download handling
- Dialogs: Alert/confirm/prompt handling

**API Preview (Phase 5):**
```rust
use playwright::expect;

// Assertions with auto-retry
expect(page.locator(".success-message"))
    .to_be_visible()
    .await?;

expect(page.locator("h1"))
    .to_have_text("Welcome")
    .await?;

// Network interception
page.route("**/api/**", |route| async move {
    route.fulfill(json!({
        "status": 200,
        "body": "mocked response"
    })).await
}).await?;

// Mobile emulation
let iphone = playwright.devices().get("iPhone 13")?;
let context = browser.new_context()
    .device(iphone)
    .await?;

// Video recording
let context = browser.new_context()
    .record_video_dir("videos/")
    .await?;
```

---

## Phase 6: Production Hardening

**Goal:** Polish for production use, comprehensive documentation, and prepare for broad community adoption.

**Status:** Not Started

**Key Deliverables:**
- Comprehensive test suite (unit, integration, cross-browser)
- Error handling refinement (helpful error messages)
- Performance optimization (benchmark suite)
- Documentation completeness (rustdoc for all public APIs)
- Examples covering all major features
- Migration guide from other Rust browser automation libraries
- Full Windows support (fix stdio cleanup issues)
- CI/CD pipeline hardening (Linux, macOS, Windows)
- Contributor guide
- Stability testing (memory leaks, resource cleanup)

---

## Post-1.0: Future Enhancements

**After v1.0.0 release, potential enhancements include:**

- **Protocol Code Generation** - Auto-generate Rust types from `protocol.yml`
- **Sync API Wrapper** - Optional blocking API for non-async codebases
- **Advanced Tracing** - Playwright inspector integration
- **Custom Browser Builds** - Support for custom Chromium/Firefox builds
- **Performance Optimization** - Connection pooling, caching
- **WebDriver BiDi Support** - When Playwright adds BiDi support
- **Component Testing** - Playwright component testing for Rust web frameworks
- **Visual Regression Testing** - Built-in visual diff capabilities

---

## Guiding Principles

Throughout all phases, we maintain:

1. **API Consistency** - Match playwright-python/JS/Java exactly
2. **Cross-Browser Parity** - All features work on Chromium, Firefox, WebKit
3. **Test-Driven Development** - Write tests first, comprehensive coverage
4. **Incremental Delivery** - Ship working code at end of each phase
5. **Production Quality** - Code quality suitable for broad adoption
6. **Documentation First** - Every feature documented with examples
7. **Community Focused** - Responsive to feedback, clear contribution path

---

## Release Strategy

### Version Numbering

- **0.x.y** - Pre-1.0, API may change between minor versions
- **1.0.0** - Stable API, ready for production
- **1.x.y** - Minor versions add features, maintain backward compatibility
- **2.0.0+** - Major versions may break API (avoid if possible)

### Publishing Cadence

- **Phase completions** - Publish to crates.io as `0.x.0`
- **Bug fixes** - Patch releases as `0.x.y`
- **Phase 5 complete** - Publish `1.0.0` to crates.io

### Communication

- **GitHub Releases** - Release notes for each version
- **Changelog** - Detailed change log in CHANGELOG.md
- **Blog Posts** - Major milestone announcements
- **Community Updates** - Regular progress updates

---

## How to Use This Roadmap

**For Contributors:**
- See current phase for what's being worked on
- Check "Not Started" phases for future opportunities
- Read implementation plans for detailed task breakdowns

**For Users:**
- Check phase status to see what features are available
- Use version numbers to understand stability
- Follow GitHub releases for updates

**For Planning:**
- Roadmap is updated after each phase completion
- Implementation plans created just-in-time (not all upfront)
- Phases may be adjusted based on learnings

**Just-In-Time Planning Approach:**

This roadmap provides high-level direction, but detailed implementation plans are created only when needed:

1. **Avoid over-planning** - Details will change as you learn
2. **Stay agile** - Respond to discoveries during implementation
3. **Focus on current work** - Don't spend time planning Phase 3 when Phase 1 isn't done
4. **Learn and adapt** - Each phase informs the next

Implementation plans are created when the previous phase is ~80% complete, allowing learnings to inform the next phase's approach.

---

**Last Updated:** 2025-11-08
