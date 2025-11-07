# Phase 2: Browser API - Implementation Plan

**Status:** 🚀 In Progress (Slice 3/7 Complete)

**Feature:** Browser launching, contexts, and page lifecycle

**User Story:** As a Rust developer, I want to launch browsers and create page objects so that I can prepare for browser automation (navigation and interaction come in Phase 3).

**Related ADRs:**
- [ADR-0001: Protocol Architecture](../adr/0001-protocol-architecture.md)
- [ADR-0002: Initialization Flow](../adr/0002-initialization-flow.md)

**Approach:** Vertical slicing with TDD (Red → Green → Refactor), following Phase 1 pattern

**Progress:** 3/7 slices complete (43%)

---

## Overview

Phase 2 builds on Phase 1's protocol foundation to implement browser launching and page lifecycle management. This enables users to:
- Launch browsers (Chromium, Firefox, WebKit)
- Create browser contexts (isolated sessions)
- Create page objects (empty pages at about:blank)
- Basic lifecycle management (close browsers/contexts/pages)

**Note:** Navigation (`page.goto()`), element interaction (clicks, typing), and locators are Phase 3. Phase 2 only creates the browser/context/page objects.

## Prerequisites from Phase 1

✅ Protocol foundation complete:
- JSON-RPC communication working
- Object factory and ChannelOwner pattern established
- Connection lifecycle management
- Playwright initialization flow
- Access to browser types

## Deferred from Phase 1

### Technical Improvements

1. **Disposal Cleanup Refactor**
   - Current: Uses `tokio::spawn` for async unregister in `ChannelOwner::dispose()`
   - Goal: Refactor to fully synchronous disposal with background cleanup task
   - Rationale: All official bindings use synchronous disposal
   - Priority: Low (current approach works correctly)

2. **Windows Testing**
   - Current: Verified on macOS and Linux only. Windows CI disabled (tests hang).
   - Issue: Tests hang on Windows after 60+ seconds in `playwright_launch.rs` tests
   - Root cause: Likely stdio pipe cleanup issue on Windows (process doesn't terminate cleanly)
   - Goal: Fix stdio pipe handling on Windows and implement proper cleanup
   - **When to re-enable Windows CI**: After implementing explicit Drop for Playwright/Connection that:
     - Sends close protocol messages to server
     - Waits for graceful shutdown
     - Properly closes stdio pipes on Windows
     - Kills child process if graceful shutdown fails
   - Priority: High (blocking Windows support)

3. **Error Message Improvements**
   - Current: Functional but terse error messages
   - Goal: Add context and suggestions to error messages
   - Priority: Low

### Testing Improvements

1. **IPC Performance Benchmarking**
   - Deferred from ADR-0001 validation checklist
   - Goal: Measure latency overhead (<5ms per operation expected)
   - Priority: Low (browser operations are 100+ms, IPC overhead negligible)

2. **Advanced Concurrent Requests Testing**
   - Test multiple concurrent requests to different objects (Browser, Context, Page)
   - Verify responses are correctly correlated when arriving out of order
   - Test complex protocol message sequences (browser launch, page create, navigation)
   - Deferred from Phase 1: connection_integration.rs and transport integration tests

3. **Transport Reconnection**
   - Test reconnection scenarios after server crash/restart
   - Verify graceful degradation and recovery
   - Deferred from Phase 1 transport testing

4. **Protocol Error Handling**
   - Test intentionally invalid requests to verify error propagation
   - Ensure protocol errors from server are properly converted to Rust errors
   - Deferred from connection_integration.rs

## Proposed Scope

### Core Features

1. **BrowserType::launch()** - Launch browser process
   - Launch options (headless, args, etc.)
   - Browser object creation
   - Browser lifecycle management

2. **Browser object** - Represents browser instance
   - `new_context()` - Create browser context
   - `new_page()` - Shortcut for default context + page
   - `close()` - Graceful shutdown
   - `contexts()` - List contexts
   - Events: close

3. **BrowserContext object** - Isolated browser session
   - `new_page()` - Create page in context
   - `close()` - Close all pages
   - `pages()` - List pages
   - Events: page, close

4. **Page object** - Web page instance (initially at about:blank)
   - `close()` - Close page
   - `url()` - Get current URL (returns "about:blank" initially)
   - `is_closed()` - Check if page is closed
   - Events: close

**Note:** Navigation (`goto()`), content (`title()`, `content()`), and interactions are Phase 3.

### Documentation

- Rustdoc for all public APIs
- Examples for common patterns
- Migration guide from Phase 1

### Testing

- Unit tests for each object type
- Integration tests with real browser launching
- Cross-browser tests (all three browsers)
- Error handling tests

## Out of Scope (Future Phases)

- **Phase 3:** Navigation (`page.goto()`), locators (`page.locator()`), actions (click, type, fill)
- **Phase 4:** Screenshots, network interception, assertions, content APIs
- **Phase 5:** Mobile emulation, advanced features

Phase 2 is strictly about **object lifecycle** - creating and closing Browser/Context/Page objects. The actual web automation (navigation, interaction) comes in Phase 3.

## Open Questions

1. How to handle async Drop for Browser/Page? (Same pattern as Playwright from Phase 1?)
2. Launch options API design - builder pattern or options struct?
3. Should we implement Drop for Browser/Context/Page for auto-cleanup?

## Success Criteria

- [ ] Can launch all three browsers (Chromium, Firefox, WebKit)
- [ ] Can create browser contexts
- [ ] Can create pages
- [ ] Can close browsers/contexts/pages gracefully
- [ ] All tests passing with real browsers
- [ ] Documentation complete
- [ ] Example code works

## Implementation Slices

### Slice 1: Browser Object Foundation ✅

**Goal:** Create Browser protocol object that can be instantiated from server messages

**Tasks:**
- [x] Create `protocol/browser.rs` with Browser struct
- [x] Implement ChannelOwner trait for Browser
- [x] Add Browser to object factory type dispatch
- [x] Parse initializer (version, name fields)
- [x] Integration test (compile-time verification)

**Files:**
- New: `crates/playwright-core/src/protocol/browser.rs`
- Modify: `crates/playwright-core/src/object_factory.rs`
- Modify: `crates/playwright-core/src/protocol/mod.rs`
- New: `crates/playwright-core/tests/browser_creation.rs`

**Tests:**
```rust
#[test]
fn test_browser_type_exists() {
    // Verifies Browser implements ChannelOwner
    // Passes: ✅
}
```

**Definition of Done:**
- ✅ Browser struct exists
- ✅ Can be created from `__create__` message
- ✅ Registered in object factory
- ✅ Compile-time test passes

---

### Slice 2: Launch Options API ✅

**Goal:** Create LaunchOptions with builder pattern and normalization

**Tasks:**
- [x] Create `api/launch_options.rs` with full option set
- [x] Implement builder pattern methods
- [x] Implement normalize() for protocol compatibility
- [x] Unit tests for builder and normalization

**Files:**
- New: `crates/playwright-core/src/api/launch_options.rs`
- New: `crates/playwright-core/src/api/mod.rs`
- Modify: `crates/playwright-core/src/lib.rs`

**Tests:**
```rust
#[test]
fn test_launch_options_builder()
#[test]
fn test_launch_options_normalize_env()
#[test]
fn test_launch_options_normalize_ignore_default_args()
```

**Definition of Done:**
- ✅ LaunchOptions has all 17+ fields
- ✅ Builder pattern works
- ✅ Normalization matches protocol format
- ✅ Unit tests pass (7 tests)

---

### Slice 3: BrowserType::launch() ✅

**Goal:** Implement browser launching with real server integration

**Tasks:**
- [x] Add `launch()` method to BrowserType
- [x] Add `launch_with_options()` method
- [x] Handle launch RPC via Channel
- [x] Parse response and retrieve Browser from registry
- [x] Integration test with real browser launch

**Files:**
- Modified: `crates/playwright-core/src/protocol/browser_type.rs`
- New: `crates/playwright-core/tests/browser_launch_integration.rs`

**Tests:**
```rust
#[tokio::test]
async fn test_launch_chromium() // ✅ Passing
#[tokio::test]
async fn test_launch_with_headless_option() // ✅ Passing
#[tokio::test]
async fn test_launch_all_three_browsers() // ✅ Passing
```

**Definition of Done:**
- ✅ Can launch Chromium with default options
- ✅ Can launch with custom options
- ✅ Can launch Firefox and WebKit
- ✅ Browser object accessible after launch
- ✅ Integration tests pass with real browsers

**Key Implementation Details:**
- Used `Channel::send()` for "launch" RPC call
- `LaunchOptions::normalize()` converts options to protocol format
- Response contains `{ browser: { guid: "..." } }`
- Retrieved Browser from connection registry via `get_object()`
- Downcast Arc<dyn ChannelOwner> to Browser using `as_any()`

**Gotchas Discovered:**
- ⚠️ Browser versions must match Playwright server version
- Server 1.49.0 requires: `npx playwright@1.49.0 install`
- Documented in README.md to prevent "Executable doesn't exist" errors
- Updated CI workflow to install matching browsers automatically

**CI Updates:**
- Added browser installation step: `npx playwright@1.49.0 install chromium firefox webkit --with-deps`
- Added browser caching to speed up CI runs
- Cache key: `${{ runner.os }}-playwright-browsers-1.49.0`

---

### Slice 4: Browser::close() ⏸️

**Goal:** Implement graceful browser shutdown

**Tasks:**
- [ ] Add `close()` method to Browser
- [ ] Send "close" RPC to server
- [ ] Handle server cleanup response
- [ ] Test close with real browser

**Files:**
- Modify: `crates/playwright-core/src/protocol/browser.rs`
- Modify: `crates/playwright-core/tests/browser_launch_integration.rs`

**Tests:**
```rust
#[tokio::test]
async fn test_browser_close()
#[tokio::test]
async fn test_close_cleans_up_resources()
```

**Definition of Done:**
- Can close browser gracefully
- Server process terminates
- Tests verify cleanup works

---

### Slice 5: BrowserContext Object ⏸️

**Goal:** Create BrowserContext protocol object

**Tasks:**
- [ ] Create `protocol/browser_context.rs`
- [ ] Implement ChannelOwner for BrowserContext
- [ ] Add to object factory
- [ ] Create ContextOptions struct
- [ ] Implement `Browser::new_context()`
- [ ] Integration test

**Files:**
- New: `crates/playwright-core/src/protocol/browser_context.rs`
- New: `crates/playwright-core/src/api/context_options.rs`
- Modify: `crates/playwright-core/src/protocol/browser.rs`
- Modify: `crates/playwright-core/src/object_factory.rs`

**Tests:**
```rust
#[tokio::test]
async fn test_new_context()
#[tokio::test]
async fn test_new_context_with_options()
```

**Definition of Done:**
- BrowserContext object exists
- Can create context from browser
- Context options work
- Tests pass

---

### Slice 6: Page Object ⏸️

**Goal:** Create Page protocol object with basic methods

**Tasks:**
- [ ] Create `protocol/page.rs`
- [ ] Implement ChannelOwner for Page
- [ ] Add to object factory
- [ ] Implement `BrowserContext::new_page()`
- [ ] Implement `Browser::new_page()` (convenience)
- [ ] Add basic page methods: `url()`, `is_closed()`
- [ ] Integration test

**Files:**
- New: `crates/playwright-core/src/protocol/page.rs`
- Modify: `crates/playwright-core/src/protocol/browser_context.rs`
- Modify: `crates/playwright-core/src/protocol/browser.rs`
- Modify: `crates/playwright-core/src/object_factory.rs`

**Tests:**
```rust
#[tokio::test]
async fn test_new_page()
#[tokio::test]
async fn test_browser_new_page_convenience()
#[tokio::test]
async fn test_page_url_initially_blank()
```

**Definition of Done:**
- Page object exists
- Can create page from context
- Can create page from browser (convenience)
- Basic page methods work
- Tests pass

---

### Slice 7: Cleanup and Documentation ⏸️

**Goal:** Finalize Phase 2 with docs and examples

**Tasks:**
- [ ] Update public API exports in `playwright` crate
- [ ] Write rustdoc for all public APIs
- [ ] Create example: `examples/browser_lifecycle.rs`
- [ ] Update README with Phase 2 features
- [ ] Run full test suite
- [ ] Update Phase 2 status to Complete

**Files:**
- Modify: `crates/playwright/src/lib.rs`
- New: `crates/playwright/examples/browser_lifecycle.rs`
- Modify: `README.md`

**Tests:**
- All existing tests still pass
- New example runs successfully
- Doc tests compile

**Definition of Done:**
- All APIs documented
- Example demonstrates browser lifecycle
- All tests pass
- Phase 2 marked complete

---

## Next Steps

1. ✅ Research browser launch protocol (completed)
2. ✅ Break into vertical slices (completed)
3. ✅ Complete Slice 1: Browser Object Foundation
4. ✅ Complete Slice 2: Launch Options API
5. ✅ Complete Slice 3: BrowserType::launch()
6. Start Slice 4: Browser::close()

---

**Created:** 2025-11-06
**Last Updated:** 2025-11-07
**Slice 1 Completed:** 2025-11-07
**Slice 2 Completed:** 2025-11-07
**Slice 3 Completed:** 2025-11-07
