# Playwright-Rust Development Roadmap

**Vision:** Provide production-quality Rust bindings for Microsoft Playwright that match the API and quality of official language bindings (Python, Java, .NET), enabling broad community adoption.

**Architecture:** JSON-RPC communication with Playwright Node.js server (same as all official bindings)

**Status:** Phase 6 complete (2025-11-12), Phase 7 Slice 0 complete (2025-11-17)

---

## Overview

This roadmap outlines the path to a production-ready `playwright-rust` library. Each phase builds incrementally toward feature parity with playwright-python while maintaining strict API compatibility and comprehensive testing.

**Key Milestones:**
- ✅ **v0.1.0** - Phase 1 complete (Protocol Foundation)
- ✅ **v0.2.0** - Phase 2 complete (Browser API)
- ✅ **v0.3.0** - Phase 3 complete (Page Interactions)
- ✅ **v0.4.0** - Phase 4 complete (Options & ElementHandles)
- ✅ **v0.5.0** - Phase 5 complete (Advanced Testing Features) - 2025-11-09
- ✅ **v0.6.0** - Phase 6 complete (Production Hardening) - 2025-11-12
- 🚀 **v0.7.0** - Phase 7 Slice 0 complete (Single-Crate Architecture) - 2025-11-16
- 📋 **v1.0.0** - Phase 7 complete (Real-World Validation & Final Polish)

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

## Phase 4: Options & ElementHandles ✅ Complete

**Goal:** Implement options pattern for actions/screenshots and ElementHandle protocol support (all deferred from Phase 3).

**Status:** ✅ Complete (2025-11-08) - See [phase4-advanced-features.md](./implementation-plans/phase4-advanced-features.md)

**Delivered:**
- ElementHandle protocol support with ChannelOwner implementation
- Element screenshots: `locator.screenshot()` via ElementHandles
- Screenshot options: ScreenshotType (Png, Jpeg), quality, full_page, clip, omit_background
- ClickOptions with builder pattern: button, modifiers, position, force, trial, timeout
- Action options: FillOptions, PressOptions, CheckOptions, HoverOptions, SelectOptions
- SelectOption variants: select by value, label, or index
- Navigation error handling and timeout tests
- Keyboard/Mouse options: delay, button, click_count, steps
- All Phase 3 deferrals addressed and implemented
- Comprehensive cross-browser testing (Chromium, Firefox, WebKit)


---

## Phase 5: Advanced Testing Features ✅ Complete

**Goal:** Implement advanced testing features including assertions, network interception, and testing utilities.

**Status:** ✅ Complete (2025-11-09) - See [phase5-advanced-testing.md](./implementation-plans/phase5-advanced-testing.md)

**Delivered:**
- Assertions: `expect(locator).to_be_visible()` with auto-retry (5s default timeout, 100ms polling)
- Text assertions: `to_have_text()`, `to_contain_text()` with regex pattern support
- Value assertions: `to_have_value()` with regex pattern support
- State assertions: `to_be_enabled()`, `to_be_disabled()`, `to_be_checked()`, `to_be_unchecked()`, `to_be_editable()`
- Negation support: `.not()` for all assertions
- Custom timeouts: `.with_timeout()` configuration
- Network interception: `page.route()` with async closure handlers
- Route handling: `route.abort()`, `route.continue()`, `route.fulfill()`
- Response mocking: Custom status, headers, body (works for API/fetch, main document needs investigation)
- JSON helpers: `.json()` for automatic serialization
- Glob pattern matching: `**/*.png`, `**/*`, etc.
- Request data access: `route.request().url()`, `method()`
- Downloads: Event handling, save functionality, metadata access
- Dialogs: Alert/confirm/prompt handling with accept/dismiss
- Convenience methods: `locator.set_checked()` for boolean-based check/uncheck
- Cross-browser testing: All features verified on Chromium, Firefox, WebKit

---

## Phase 6: Production Hardening ✅ Complete

**Goal:** Polish for v0.6.0 release, address deferred items, comprehensive documentation.

**Status:** ✅ Complete (2025-11-12) - See [phase6-production-hardening.md](./implementation-plans/phase6-production-hardening.md)

**Delivered:**
- Windows support (stdio cleanup fix, CI stability flags)
- Complete assertion API (to_be_focused)
- Main document fulfillment investigation (Playwright server limitation documented)
- Documentation completeness (rustdoc 100% coverage for all public APIs)
- Performance optimization (benchmark suite, GUID Arc<str> optimization, chunked reading)
- Test suite optimization (cargo-nextest integration, test combining)
- Stability testing (memory leaks, resource cleanup, error handling)
- Low-priority enhancements (FilePayload, BrowserContext options, route continue overrides)
- v0.6.0 published to crates.io

---

## Phase 7: Real-World Validation & v1.0 Polish

**Goal:** Real-world testing in folio, incorporate user feedback, final polish before v1.0.0.

**Status:** 🚀 In Progress - See [phase7-real-world-validation.md](./implementation-plans/phase7-real-world-validation.md)

**Milestones by Slice:**
✅ Slice 0: Single-crate architecture consolidation, version bumped to 0.7.0 (playwright-core merged into playwright-rs)
- Slice 1: Folio integration & dogfooding
- Slice 2: Community feedback analysis
- Slice 3: Examples and documentation (informed by feedback)
- Slice 4: Performance optimization (data-driven)
- Slice 5: API polish and enhancements
- Slice 6: v1.0.0 release preparation

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
- **Phase 6 complete** - Publish `v0.6.0` for early adopter feedback and real-world testing
- **Phase 7 complete** - Publish `v1.0.0` after folio integration and user validation

### Communication

- **GitHub Releases** - Release notes for each version
- **Changelog** - Detailed change log in CHANGELOG.md
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

**Last Updated:** 2025-11-17
