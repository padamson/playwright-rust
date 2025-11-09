# Phase 6: Production Hardening - Implementation Plan

**Status:** 🚀 **IN PROGRESS**

**Goal:** Polish for production use, address deferred items, comprehensive documentation, and prepare for v1.0.0 release.

**User Story:** As a Rust developer, I want playwright-rust to be production-ready with comprehensive documentation, Windows support, and all deferred features completed, so I can confidently use it in production applications.

**Approach:** Vertical Slicing with focus on polish, documentation, and deferred items

---

## Deferred Items from Previous Phases

### High Priority Deferrals

1. **Windows Support** (Phase 1 defer ral)
   - Issue: Integration tests hang on Windows due to stdio pipe cleanup
   - Impact: Blocking Windows users from running tests
   - Location: `crates/playwright-core/src/transport.rs` cleanup logic

2. **to_be_focused() Assertion** (Phase 5 deferral)
   - Reason: Requires 'expect' protocol command or evalOnSelector return values
   - Impact: Missing one assertion from complete coverage
   - Location: `crates/playwright-core/src/assertions.rs:796`

3. **route.fulfill() Main Document Navigation** (Phase 5 known issue)
   - Issue: fulfill() works for fetch/XHR but not main document navigation
   - Impact: Limits response mocking use cases
   - Location: `crates/playwright-core/src/protocol/route.rs:167`

### Medium Priority Deferrals

4. **FilePayload Struct** (Phase 5 deferral)
   - Feature: Structured file upload with name, mimeType, buffer
   - Current: Basic PathBuf-based upload works
   - Impact: Low - basic functionality exists

5. **Transport Chunked Reading** (Performance optimization)
   - TODO: Consider chunked reading for very large messages (>32KB)
   - Location: `crates/playwright-core/src/transport.rs:254`

6. **GUID String Optimization** (Performance)
   - TODO: Avoid cloning GUID by restructuring Channel::new to accept &str
   - Location: `crates/playwright-core/src/channel_owner.rs:261`

### Low Priority Deferrals

7. **Browser Context Options** (Phase 2 deferral)
   - Feature: Full ContextOptions API (viewport, user agent, etc.)
   - Current: Minimal options (empty JSON)
   - Impact: Low - basic context creation works

8. **Route Continue Overrides** (Enhancement)
   - Feature: Modify headers/method/postData when continuing routes
   - Location: `crates/playwright-core/src/protocol/route.rs:127`

---

## Phase 6 Slices

### Slice 1: Windows Support and Stdio Cleanup 🚧 IN PROGRESS

**Goal:** Fix Windows integration test hangs and enable full Windows CI support.

**Why First:** Highest-priority deferral, blocking Windows users.

**Tasks:**
- [x] Research Windows stdio pipe cleanup behavior
- [x] Implement platform-specific cleanup logic (Windows vs Unix)
- [x] Test on macOS (Unix) - all tests pass
- [x] Enable Windows CI in GitHub Actions
- [x] Debug Windows browser launch hangs in CI - **Found root cause**
- [x] Implement browser launch flags for Windows CI
- [x] Add diagnostic logging to track execution
- [ ] Test fixes on local Windows machine
- [ ] Verify all integration tests pass on Windows CI
- [ ] Update README to remove Windows warning (when tests pass)
- [x] Document platform-specific behavior in code

**Files Modified:**
- `crates/playwright-core/src/server.rs` - Platform-specific cleanup in shutdown() and kill()
- `crates/playwright-core/src/transport.rs` - Documentation on platform-specific cleanup
- `crates/playwright-core/tests/windows_cleanup_test.rs` - New tests for cleanup verification
- `crates/playwright-core/src/protocol/browser_type.rs` - Added Windows CI browser flags
- `crates/playwright-core/src/protocol/browser.rs` - Added cleanup delay for Windows CI
- `crates/playwright-core/tests/browser_launch_integration.rs` - Added diagnostic logging
- `.github/workflows/test.yml` - Enabled Windows tests in CI
- `.github/workflows/test-debug.yml` - Created debug workflow for Windows testing
- `test-windows-local.ps1` - PowerShell script for local Windows testing
- `diagnose-windows.ps1` - PowerShell script for diagnosing browser processes
- `crates/playwright-core/src/protocol/browser_type_windows_fix.rs` - Alternative aggressive fixes (if needed)

**Implementation Summary:**

Windows CI browser launches hang due to complex interaction between Playwright server, browser processes, and the Windows CI environment. The issue is NOT with our Rust stdio cleanup (which works), but with how browsers themselves launch in the Windows CI environment.

**Solutions Implemented:**

1. **Platform-specific server cleanup in `server.rs`:** ✅ WORKING
   - On Windows: Explicitly close stdin, stdout, stderr before calling `kill()`
   - On Windows: Use timeout-based wait (5 seconds) to prevent permanent hangs
   - On Unix: Standard process termination
   - **Status:** Server cleanup tests pass, server doesn't hang

2. **Browser launch flags for Windows CI (browser_type.rs):** ❓ TESTING
   ```rust
   // Detects CI environment and adds flags
   if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
       // Adds: --no-sandbox, --disable-dev-shm-usage, --disable-gpu,
       // --disable-web-security, --disable-features=IsolateOrigins,site-per-process
   }
   ```
   - **Status:** Implemented but still hanging in CI

3. **Browser cleanup delay (browser.rs):** ❓ TESTING
   ```rust
   // After browser.close() on Windows CI
   tokio::time::sleep(Duration::from_millis(500)).await;
   ```
   - **Status:** Implemented but not preventing subsequent hangs

4. **Diagnostic logging:** ✅ WORKING
   - Added detailed logging to track test execution stages
   - Shows hang occurs at browser launch, not server startup

**Research Findings:**

1. **Playwright-python approach:**
   - Uses `asyncio.create_subprocess_exec()` with pipes
   - Has similar Windows CI issues reported (#1254, #1349, #723)
   - Common workarounds: context managers, memory limits, periodic restarts

2. **Known Windows-specific issues:**
   - Python asyncio has NotImplementedError with pipes in some environments
   - Windows handle inheritance problems with redirected stdio
   - Browser processes may not fully terminate between tests
   - GitHub Actions Windows runners have different sandboxing than local

3. **CI Behavior Observed:**
   - Workflow hangs at "Run single simple test" step
   - 5-minute timeout in workflow doesn't trigger (needs job-level timeout?)
   - Browser launch never completes or returns error
   - Manual cancellation required after 10+ minutes

**Local Windows Testing Options:**

1. **PowerShell test script (`test-windows-local.ps1`):**
   ```powershell
   $env:CI = "true"
   $env:GITHUB_ACTIONS = "true"
   cargo test --verbose --workspace -- --test-threads=1 --nocapture
   ```

2. **Process diagnostic script (`diagnose-windows.ps1`):**
   - Monitor browser processes
   - Kill orphaned processes
   - Test cleanup behavior

3. **Direct debugging:**
   - Set CI environment variables
   - Run specific test: `cargo test test_launch_chromium -- --nocapture`
   - Watch for: `[playwright-rust] Detected Windows CI environment, adding stability flags`

**Alternative Approaches (If Current Fix Fails):**

1. **More aggressive browser flags (`browser_type_windows_fix.rs`):**
   - `--single-process` - Run everything in one process
   - `--no-zygote` - Disable zygote process
   - `--no-first-run` - Skip first-run tasks
   - Pre-kill browser processes before launch

2. **Job-level timeout:**
   ```yaml
   jobs:
     test-windows:
       timeout-minutes: 30  # At job level instead of step level
   ```

3. **Test isolation:**
   - Run each test file separately
   - Kill browser processes between test files
   - Restart Playwright server between test files

4. **Browser-specific workarounds:**
   - Test only Chromium on Windows CI (skip Firefox/WebKit)
   - Use different launch method for Windows

**Current Status:**

- ✅ Server cleanup works (no hangs on server shutdown)
- ✅ Diagnostic logging implemented
- ❌ Browser launches still hang in Windows CI (despite flags)
- ❌ 5-minute timeout doesn't work (hangs indefinitely)
- ⏳ Awaiting local Windows testing to verify fixes

**Next Immediate Steps:**
1. Test on local Windows laptop with CI environment variables
2. If still hanging, implement more aggressive fixes
3. Consider job-level timeout or test isolation strategy
4. May need to temporarily disable Windows CI and document as known issue

**Success Criteria:** ⬜ Not Yet Met
- All tests pass on macOS (Unix) - ✅ verified locally
- All tests pass on Windows CI - ❌ hangs during browser launch
- No stdio cleanup hangs - ✅ for server, ❌ for browser processes
- Windows CI runs without timeouts - ❌ requires manual cancellation
- Code documented with platform differences - ✅

---

### Slice 2: to_be_focused() Assertion

**Goal:** Implement the deferred `to_be_focused()` assertion.

**Why Second:** Complete assertion API coverage, important for form testing.

**Research:**
- Check if Playwright added 'expect' protocol command (check protocol.yml)
- Review playwright-python implementation of to_be_focused()
- Determine if we can use JavaScript evaluation as workaround

**Tasks:**
- [ ] Research Playwright protocol for focus detection
- [ ] Write failing tests for to_be_focused() and not().to_be_focused()
- [ ] Implement focus detection (protocol or JS eval)
- [ ] Verify cross-browser (Chromium, Firefox, WebKit)
- [ ] Add rustdoc with examples

**Files to Create/Modify:**
- `crates/playwright-core/src/assertions.rs` - Implement to_be_focused()
- `crates/playwright-core/tests/state_assertions_test.rs` - Uncomment deferred tests

**Success Criteria:**
- to_be_focused() works on all browsers
- Tests pass for focused and unfocused elements
- API matches Playwright exactly

---

### Slice 3: Main Document Fulfillment Investigation

**Goal:** Investigate and fix route.fulfill() for main document navigation.

**Why Third:** Limits response mocking capabilities, user-facing issue.

**Research:**
- How does playwright-python handle main document fulfill()?
- What protocol messages are sent for page.goto() fulfillment?
- Is there a different approach for document vs fetch/XHR?

**Tasks:**
- [ ] Create test case for main document fulfillment
- [ ] Capture protocol messages for working (playwright-python) vs broken (Rust)
- [ ] Identify difference in protocol communication
- [ ] Implement fix
- [ ] Verify cross-browser
- [ ] Update route.rs documentation

**Files to Modify:**
- `crates/playwright-core/src/protocol/route.rs` - Fix fulfill logic
- `crates/playwright-core/tests/routing_test.rs` - Add main document test

**Success Criteria:**
- route.fulfill() works for main document navigation
- Tests verify HTML replacement works
- Documentation updated to reflect fix

**Alternative:** If unfixable, document as Playwright limitation with workaround.

---

### Slice 4: Documentation Completeness Audit

**Goal:** Ensure all public APIs have comprehensive rustdoc with examples.

**Why Fourth:** Essential for production use, helps users discover features.

**Tasks:**
- [ ] Audit all public APIs for rustdoc completeness
- [ ] Add missing documentation
- [ ] Add examples to all public methods
- [ ] Add links to Playwright docs for all methods
- [ ] Verify all examples compile with rustdoc test
- [ ] Generate docs and review for clarity

**Files to Audit:**
- All `crates/playwright-core/src/protocol/*.rs` files
- All `crates/playwright/src/api/*.rs` files (if they exist)

**Success Criteria:**
- 100% public API documentation coverage
- All examples compile and run
- cargo doc --open shows professional documentation

---

### Slice 5: Examples and Migration Guide

**Goal:** Create comprehensive examples and migration guide from other libraries.

**Why Fifth:** Lowers barrier to entry, helps users adopt playwright-rust.

**Tasks:**
- [ ] Create advanced examples (API mocking, file downloads, etc.)
- [ ] Create migration guide from:
  - headless_chrome
  - fantoccini
  - thirtyfour
- [ ] Document API differences from playwright-python
- [ ] Create "Getting Started" tutorial
- [ ] Add troubleshooting guide

**Files to Create:**
- `examples/advanced/` directory with complex examples
- `docs/migration-guides/` with comparison tables
- `docs/getting-started.md`
- `docs/troubleshooting.md`

**Success Criteria:**
- 5+ advanced examples covering common patterns
- Migration guides help users switch from other libraries
- Getting started guide onboards new users quickly

---

### Slice 6: Performance Optimization and Benchmarks

**Goal:** Optimize performance bottlenecks and establish benchmark suite.

**Why Sixth:** Important for production use, but not blocking.

**Tasks:**
- [ ] Create benchmark suite (criterion.rs)
- [ ] Benchmark: Browser launch time
- [ ] Benchmark: Page navigation
- [ ] Benchmark: Element queries
- [ ] Implement deferred optimizations:
  - Chunked reading for large messages
  - GUID string optimization (avoid cloning)
- [ ] Profile memory usage
- [ ] Document performance characteristics

**Files to Create:**
- `benches/` directory with criterion benchmarks
- `docs/performance.md` with results

**Success Criteria:**
- Benchmark suite runs in CI
- Performance comparable to playwright-python
- Optimizations implemented without regression

---

### Slice 7: Stability Testing and Error Handling

**Goal:** Verify resource cleanup, memory leaks, and error handling.

**Why Seventh:** Production polish, catch edge cases.

**Tasks:**
- [ ] Memory leak testing (long-running tests)
- [ ] Resource cleanup verification (file descriptors, processes)
- [ ] Error message quality audit
- [ ] Add context to error messages
- [ ] Test graceful shutdown on SIGTERM/SIGINT
- [ ] Test error recovery (network errors, browser crashes)

**Files to Modify:**
- `crates/playwright-core/src/error.rs` - Improve error messages
- Various protocol files - Add error context

**Success Criteria:**
- No memory leaks in long-running tests
- All resources cleaned up properly
- Error messages are helpful and actionable

---

### Slice 8: Low-Priority Enhancements (If Time Permits)

**Goal:** Implement nice-to-have deferred items.

**Tasks:**
- [ ] FilePayload struct for advanced file uploads
- [ ] BrowserContext options (viewport, user agent, etc.)
- [ ] Route continue overrides (headers, method, postData)
- [ ] Doctest infrastructure for runnable documentation

**Files to Create/Modify:**
- `crates/playwright-core/src/protocol/file_payload.rs`
- `crates/playwright-core/src/protocol/browser_context.rs` - ContextOptions
- `crates/playwright-core/src/protocol/route.rs` - ContinueOverrides

**Success Criteria:**
- Features implemented with tests
- Documentation updated
- No regression in existing functionality

---

### Slice 9: v0.6.0 Release Preparation

**Goal:** Prepare for v0.6.0 release to crates.io for friendly user feedback and real-world validation.

**Why Ninth:** Ship working code to get feedback before final v1.0 polish.

**Tasks:**
- [ ] Create CHANGELOG.md with all changes since v0.5.0
- [ ] Version bump to v0.6.0
- [ ] Final documentation review
- [ ] Final test pass (all platforms)
- [ ] Create GitHub release with notes
- [ ] Publish to crates.io
- [ ] Update README with installation instructions for v0.6.0

**Files to Create:**
- `CHANGELOG.md`
- Release notes in GitHub

**Success Criteria:**
- v0.6.0 published to crates.io
- Documentation is comprehensive enough for early adopters
- Examples work
- Ready for real-world testing in folio and other projects

---

## Success Criteria (Phase 6 Complete)

- ✅ Windows support fully working (tests pass on Windows)
- ✅ All deferred items addressed (HIGH and MEDIUM priority)
- ✅ 100% public API documentation coverage
- ✅ Comprehensive examples and migration guides
- ✅ Performance benchmarks established
- ✅ Stability testing passed (no leaks, clean shutdown)
- ✅ v0.6.0 published to crates.io
- ✅ Ready for real-world validation (folio integration, user feedback)

---

## Guiding Principles for Phase 6

1. **Good Enough for v0.6** - Polished enough for friendly users, not perfect
2. **User Experience** - Documentation and examples help early adopters
3. **Platform Parity** - Windows, macOS, Linux all first-class
4. **Performance** - Fast enough for CI/CD pipelines
5. **Stability** - No surprises, clean error handling
6. **Feedback Ready** - Ship to real users (folio) to inform Phase 7

**Phase 7 Note:** After v0.6.0 release, Phase 7 will focus on real-world validation, folio integration, user feedback incorporation, and final polish before v1.0.0.

---

**Created:** 2025-11-09
**Last Updated:** 2025-11-09 (Phase 6 planning complete)
