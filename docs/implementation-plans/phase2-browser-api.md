# Phase 2: Browser API - Implementation Plan

**Status:** 📋 Planning

**Feature:** Browser launching, contexts, and page lifecycle

**User Story:** As a Rust developer, I want to launch browsers and create page objects so that I can prepare for browser automation (navigation and interaction come in Phase 3).

**Related ADRs:**
- [ADR-0001: Protocol Architecture](../adr/0001-protocol-architecture.md)
- [ADR-0002: Initialization Flow](../adr/0002-initialization-flow.md)

**Approach:** TBD - Will follow Phase 1's vertical slicing approach

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
   - Current: Verified on macOS and Linux only
   - Goal: Test driver download, stdio pipes, and full functionality on Windows
   - Priority: Medium

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

## Next Steps

1. Review Phase 1 learnings
2. Research browser launch in official bindings
3. Create detailed vertical slices
4. Write first failing test (TDD)

---

**Created:** 2025-11-06
**Last Updated:** 2025-11-06
