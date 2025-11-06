# Phase 1: Protocol Foundation - Implementation Plan

**Feature:** JSON-RPC Protocol Client and Playwright Server Management

**User Story:** As a Rust developer, I want to launch the Playwright server and establish a JSON-RPC connection so that I can begin automating browsers.

**Related ADR:** TBD - Will create ADR for transport layer and async runtime decisions

**Approach:** Vertical Slicing with Test-Driven Development (TDD)

---

## Implementation Strategy

This implementation follows **vertical slicing** - each slice delivers end-to-end testable functionality that brings us closer to launching a browser.

**Architecture Reference:**
Based on research of playwright-python, playwright-java, and playwright-dotnet, all Microsoft Playwright bindings follow the same architecture:

1. **Transport Layer** - Length-prefixed JSON messages over stdio pipes
2. **Connection Layer** - JSON-RPC client with request/response correlation
3. **Driver Management** - Download and launch Playwright Node.js server
4. **Object Factory** - Instantiate typed objects from protocol messages

**Key Design Principles:**
- Match Microsoft's proven architecture exactly
- Use `tokio` for async runtime (Rust standard)
- Follow protocol message format from `protocol.yml`
- Length-prefixed message framing (4 bytes little-endian + JSON)
- GUID-based object references
- Event-driven architecture for protocol events

**Phase 1 Scope:**
This phase establishes the protocol foundation (server management, transport, connection, object factory, and entry point). Phase 1 ends when you can successfully launch the Playwright server and access `BrowserType` objects for Chromium, Firefox, and WebKit.

**Note:** Actual browser launching and cross-browser testing will be implemented in Phase 2. However, the protocol foundation built in Phase 1 is designed to support all three browsers from the start.

---

## Vertical Slices

### Slice 1: Walking Skeleton - Server Launch and Shutdown

**Status:** ✅ Complete (2025-11-05)

**User Value:** Can download Playwright server, launch it as a child process, and shut it down cleanly.

**Acceptance Criteria:**
- [x] Playwright driver is downloaded during build via `build.rs` from Azure CDN
- [x] Driver binaries are stored in `drivers/` directory (gitignored)
- [x] Platform detection works correctly (macOS x86_64/ARM64, Linux x86_64/ARM64, Windows x86_64)
- [x] Server process launches successfully via `node cli.js run-driver`
- [x] Process environment includes `PW_LANG_NAME=rust`, `PW_LANG_NAME_VERSION`, and `PW_CLI_DISPLAY_VERSION`
- [x] Server can be shut down gracefully without orphaning processes
- [x] Errors are handled with helpful messages (server not found, launch failure, etc.)
- [x] Fallback to `PLAYWRIGHT_DRIVER_PATH` environment variable if set
- [x] Fallback to npm-installed Playwright for development use

**Core Library Implementation (`playwright-core`):**
- [x] Create workspace structure: `crates/playwright-core/`
- [x] Add `Cargo.toml` with dependencies:
  - `tokio = { version = "1", features = ["full"] }`
  - `serde = { version = "1", features = ["derive"] }`
  - `serde_json = "1"`
  - `thiserror = "1"`
- [x] Define `src/error.rs` with `Error` enum:
  - `ServerNotFound`
  - `LaunchFailed`
  - `ConnectionFailed`
  - `TransportError`
  - `ProtocolError`
- [x] Create `src/driver.rs` module:
  - `get_driver_executable() -> Result<(PathBuf, PathBuf)>` - Returns (node_path, cli_js_path)
  - Try in order:
    1. Bundled driver in `drivers/` (from build.rs)
    2. `PLAYWRIGHT_DRIVER_PATH` environment variable
    3. npm global installation (development fallback)
    4. npm local installation (development fallback)
  - `find_node_executable() -> Result<PathBuf>` - Locate Node.js binary
  - Platform detection using `std::env::consts::{OS, ARCH}`
- [x] Create `src/server.rs` module:
  - `struct PlaywrightServer` - Wraps child process
  - `PlaywrightServer::launch() -> Result<Self>` - Launch server process
    - Command: `node <driver_path>/package/cli.js run-driver`
    - Set environment variables:
      - `PW_LANG_NAME=rust`
      - `PW_LANG_NAME_VERSION` (from `CARGO_PKG_RUST_VERSION`)
      - `PW_CLI_DISPLAY_VERSION` (from `CARGO_PKG_VERSION`)
    - Stdio: stdin=piped, stdout=piped, stderr=inherit
  - `PlaywrightServer::shutdown(self) -> Result<()>` - Graceful shutdown
  - `PlaywrightServer::kill(self) -> Result<()>` - Force kill (timeout fallback)
- [x] Export public API in `src/lib.rs`

**Core Library Unit Tests:**
- [x] Test `get_driver_executable()` returns valid path
- [x] Test bundled driver detection
- [x] Test `find_node_executable()` locates Node.js
- [x] Test `PlaywrightServer::launch()` spawns child process
- [x] Test `PlaywrightServer::shutdown()` terminates process
- [x] Test `PlaywrightServer::kill()` force kills process
- [x] Test error handling for driver not found

**Build System:**
- [x] Create `build.rs` script in `playwright-core/`:
  - Check if `drivers/` directory exists in workspace root
  - If not, download Playwright driver from Azure CDN
  - URL format: `https://playwright.azureedge.net/builds/driver/playwright-{version}-{platform}.zip`
  - Platform mapping:
    - macOS x86_64 → `mac`
    - macOS ARM64 → `mac-arm64`
    - Linux x86_64 → `linux`
    - Linux ARM64 → `linux-arm64`
    - Windows x86_64 → `win32_x64`
  - Extract to `drivers/playwright-{version}-{platform}/`
  - Contains: `node` binary and `package/` directory with `cli.js`
  - Set `PLAYWRIGHT_DRIVER_VERSION` env var for runtime
- [x] Add build dependencies to `Cargo.toml`:
  - `reqwest = { version = "0.12", features = ["blocking"] }`
  - `zip = "2.1"`
- [x] Add `drivers/` to `.gitignore`
- [x] Document build process in ADR and implementation plan

**Documentation:**
- [x] Rustdoc for all public types and functions
- [x] Example in doc comment showing server launch/shutdown
- [x] Link to Playwright docs for driver management
- [x] Document download strategy (build-time bundling matches official bindings)

**Notes:**
- **Decision:** Build-time download via `build.rs` (matches Python/Java/.NET approach)
  - ✅ **Matches official bindings** - All three bundle drivers in packages
  - ✅ Faster first run - No download delay when user runs code
  - ✅ Offline-friendly - Works without network after initial build
  - ✅ Simpler user experience - Just `cargo add playwright`
  - ⚠️ Requires network during build - Acceptable, common in Rust (like `cc` crate)
  - ⚠️ ~50MB download - Acceptable, same as other bindings
- Playwright version: Pin to specific version in `build.rs` (e.g., `1.56.0`)
  - Update version manually when updating crate
  - Document version compatibility in README
- Platform support: Start with macOS (x86_64, ARM64) and Linux (x86_64, ARM64)
  - Windows support in future release
  - Cross-compilation considerations for CI/CD
- Reference implementations:
  - Python: `setup.py` (`PlaywrightBDistWheelCommand`)
  - Java: `driver-bundle` module
  - .NET: `.csproj` Content directives

---

### Slice 2: Stdio Transport - Send and Receive Messages

**Status:** ✅ Complete (2025-11-05)

**User Value:** Can send JSON-RPC messages to Playwright server and receive responses over stdio pipes.

**Research Completed:** Analyzed transport implementations in playwright-python, playwright-java, and playwright-dotnet (2025-11-05)

**Acceptance Criteria:**
- [x] Messages are framed with 4-byte little-endian length prefix
- [x] JSON messages are serialized and sent to server stdin
- [x] Messages are read from server stdout with length prefix
- [x] Reader loop runs in background task without blocking (via async task)
- [x] Transport can be gracefully shut down (via drop or channel close)
- [x] Transport errors are propagated correctly

**Core Library Implementation (`playwright-core`):**
- [x] Create `src/transport.rs` module:
  - [x] `trait Transport` - Abstract transport interface
    - `async fn send(&mut self, message: JsonValue) -> Result<()>`
  - [x] `struct PipeTransport` - stdio pipe implementation
    - `stdin: ChildStdin` - stdin pipe
    - `stdout: ChildStdout` - stdout pipe
    - `message_tx: mpsc::UnboundedSender<JsonValue>` - Message channel
  - [x] `PipeTransport::new(stdin, stdout) -> (Self, Receiver)` - Constructor
  - [x] `PipeTransport::send(message: JsonValue) -> Result<()>` - Send implementation
  - [x] `PipeTransport::run()` - Async read loop (matches Python's `run()`)
  - [x] Graceful shutdown - Via dropping receiver channel (no explicit method needed)
- [x] Implement length-prefixed framing:
  - Write: `u32::to_le_bytes(len) + json_bytes`
  - Read: `read_exact(4 bytes) -> u32::from_le_bytes -> read_exact(len)`
- [x] Add message dispatch mechanism via `mpsc::unbounded_channel`
- [x] User spawns tokio task for read loop (matches Python pattern)

**Core Library Unit Tests:**
- [x] Test length prefix encoding (matches Python's little-endian format)
- [x] Test message framing format (4-byte LE + JSON)
- [x] Test send message with mock pipes
- [x] Test multiple messages in sequence
- [x] Test large messages (>32KB JSON, 100KB tested)
- [x] Test malformed length prefix (error handling)
- [x] Test broken pipe (server crash)
- [x] Test graceful shutdown (no messages lost)

**Integration Tests:**
- [x] Launch real Playwright server and create transport
- [x] Verify transport works with real process stdio (not just mock pipes)
- [x] Test transport handles server crash gracefully
- [ ] Verify server responds to protocol messages (deferred to Slice 5 - requires full initialization flow)
- [ ] Test concurrent message sending (deferred to Slice 5 - requires valid GUIDs from initialized objects)
- [ ] Test transport reconnection (future: beyond Phase 1)

**Integration Test Notes:**
- Basic integration tests verify transport layer works with real Playwright server process
- Full protocol interaction testing (sending JSON-RPC requests, validating responses) deferred to Slice 5
- Slice 4 implements infrastructure; Slice 5 implements initialization flow required for these tests
- Browser-specific testing (Chromium/Firefox/WebKit launch) deferred to Phase 2 (Browser API implementation)

**Documentation:**
- [x] Rustdoc for `Transport` trait and `PipeTransport`
- [x] Document length-prefix framing protocol (in code comments)
- [x] Example showing PipeTransport usage in rustdoc
- [x] Link to Python's PipeTransport for reference architecture

**Transport Implementation Research (2025-11-05):**

Based on analysis of all three official bindings, the transport layer follows these patterns:

**Message Framing (Identical across all bindings):**
- **4-byte little-endian length prefix** followed by JSON payload
- Python: `len(data).to_bytes(4, byteorder="little")`
- Java: Bit shifting `(v >>> 8) & 0xFF` for each byte
- .NET: Byte masks `(len >> 8) & 0xFF` for encoding

**Read Loop Patterns:**
- Python: Async loop with `readexactly(4)` for header, then `readexactly(length)` in 32KB chunks
- Java: Blocking thread with `DataInputStream.readInt()`, separate reader thread
- .NET: Async `ReadAsync()` with 1KB buffer, accumulate until message complete

**Dispatch Mechanisms:**
- Python: Direct callback `on_message(obj)` - matches Rust async model best
- Java: Blocking queue `incoming.put(message)` - thread-based
- .NET: Event `MessageReceived?.Invoke()` - async/await based

**Rust Implementation Strategy:**
- Follow **Python's async pattern** (closest to tokio's model)
- Use `tokio::io::AsyncReadExt::read_exact()` for framing
- Direct callback via channels (matches Python's `on_message`)
- Single async task for read loop (not separate threads)

**Key Code Pattern to Match:**
```python
# Python reference implementation
async def run(self):
    while not self._stopped:
        buffer = await self._proc.stdout.readexactly(4)
        length = int.from_bytes(buffer, byteorder="little")
        data = await self._proc.stdout.readexactly(length)
        obj = json.loads(data)
        self.on_message(obj)
```

**Notes:**
- Use `tokio::io::AsyncReadExt` and `AsyncWriteExt` for async I/O
- Match Python's chunked reading for large messages (32KB buffer)
- Use `tokio::sync::mpsc` for message dispatch (replaces Python's callback)
- Ensure reader loop exits cleanly on shutdown (use cancellation token)

**Lessons Learned (Post-Implementation 2025-11-05):**

1. **Generic Type Parameters Critical for Testing**
   - Made `PipeTransport<W, R>` generic over `AsyncWrite + AsyncRead`
   - Allows unit tests to use `tokio::io::duplex()` mock pipes
   - Production code uses `ChildStdin` and `ChildStdout` from real process
   - Key insight: Don't hardcode process types - use generics for testability

2. **Duplex Pipe Patterns for Bidirectional Testing**
   - Challenge: Single duplex pipe causes deadlocks when testing bidirectional I/O
   - Solution: Use **two separate duplex pipes**:
     - Pipe 1: Transport writes to `stdin_write`, test reads from `stdin_read`
     - Pipe 2: Test writes to `stdout_write`, transport reads from `stdout_read`
   - Pattern:
     ```rust
     let (stdin_read, stdin_write) = tokio::io::duplex(1024);
     let (stdout_read, stdout_write) = tokio::io::duplex(1024);
     let (transport, rx) = PipeTransport::new(stdin_write, stdout_read);
     ```

3. **Build Script Output Should Be Silent When Normal**
   - Initially: `cargo:warning=` for "driver already exists" (shown every build)
   - Fixed: Only show warnings when actually downloading or on errors
   - Rust convention: Quiet when everything is working correctly

4. **Integration Tests Validate Real-World Behavior**
   - Unit tests with mocks verify framing logic
   - Integration tests with real Playwright server verify:
     - Process stdio works differently than mock duplex pipes
     - Server communication patterns
     - Error handling with real process crashes
   - Both test types are essential - don't skip integration tests!

5. **Test Hierarchy: Unit → Integration → E2E**
   - **Unit tests** (8): Message framing, encoding, error handling (mock pipes)
   - **Integration tests** (3): Real server process, stdio communication, crash handling
   - **E2E tests** (deferred to Slice 4): Actual browser launch with Chromium/Firefox/WebKit
   - Clear separation of concerns at each test level

6. **Documentation of Design Patterns**
   - Downcasting and RAII need explicit explanation for future implementers
   - Don't assume developers know these patterns in Rust context
   - Link implementation patterns to official bindings (Python/Java/.NET)

7. **Shutdown via Channel Drop (No Explicit Method Needed)**
   - No explicit `shutdown()` method implemented
   - Shutdown pattern: Drop the receiver (`rx`) → `send()` in `run()` loop fails → loop exits
   - Idiomatic Rust: Use RAII (resource cleanup on drop) instead of explicit methods
   - Tested in `test_graceful_shutdown`: Verify loop exits when channel is dropped
   - Simpler than Python's explicit `close()` - Rust's ownership handles it automatically

---

### Slice 3: Connection - JSON-RPC Request/Response Correlation

**Status:** ✅ Complete (2025-11-06)

**User Value:** Can send JSON-RPC requests to Playwright server and await responses, with proper error handling.

**Acceptance Criteria:**
- [x] Each request has unique incrementing ID
- [x] Responses are correlated with requests by ID
- [x] Multiple concurrent requests are handled correctly
- [x] Protocol events (no ID) are distinguished from responses
- [x] Errors from server are propagated as Rust errors
- [x] Timeout handling for requests that never receive response (Note: Implemented as channel closed error when response never arrives)

**Core Library Implementation (`playwright-core`):**
- [x] Create `src/connection.rs` module:
  - `struct Connection<W, R>` - JSON-RPC client (generic over AsyncWrite/AsyncRead)
    - `transport: Arc<Mutex<PipeTransport<W, R>>>` - Underlying transport
    - `last_id: AtomicU32` - Request ID counter
    - `callbacks: Arc<Mutex<HashMap<u32, oneshot::Sender<Result<JsonValue>>>>>` - Pending requests
    - `message_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<Value>>>>` - Message receiver from transport
  - `Connection::new(transport: PipeTransport<W, R>, message_rx) -> Self`
  - `Connection::send_message(guid: &str, method: &str, params: JsonValue) -> Result<JsonValue>`
  - `Connection::dispatch(message: Message) -> Result<()>` - Handle incoming messages
  - `Connection::run()` - Async message dispatch loop (spawns transport loop internally)
- [x] Define protocol message types:
  - `struct Request { id: u32, guid: String, method: String, params: JsonValue }`
  - `struct Response { id: u32, result: Option<JsonValue>, error: Option<ErrorWrapper> }`
  - `struct Event { guid: String, method: String, params: JsonValue }`
  - `enum Message { Response(Response), Event(Event) }` - Discriminated union using `#[serde(untagged)]`
- [x] Implement request/response correlation:
  - Generate unique ID for each request using `AtomicU32::fetch_add`
  - Store `oneshot::Sender` in callbacks map
  - On response, complete the sender and remove from map
- [x] Implement event dispatch (logs events for now, full dispatch in Slice 4)

**Core Library Unit Tests:**
- [x] Test request ID increments correctly
- [x] Test dispatch returns response for matching ID (test_dispatch_response_success)
- [x] Test concurrent requests (test_concurrent_requests with 3 concurrent requests)
- [x] Test response with error field (test_dispatch_response_error)
- [x] Test dispatch routes responses correctly by ID
- [x] Test dispatch handles events (test_message_deserialization_event)
- [x] Test invalid ID error (test_dispatch_invalid_id)
- [x] Test message deserialization (Response vs Event)
- [x] Test error type parsing (TimeoutError, TargetClosedError, generic)

**Integration Tests:**
- [x] Test connection lifecycle with real Playwright server (test_connection_lifecycle_with_real_server)
- [x] Test error detection on server crash (test_connection_detects_server_crash_on_send)
- [ ] Test actual protocol messages with server (deferred to Slice 5 - requires full initialization flow)
- [ ] Test concurrent requests to real server (deferred to Slice 5 - requires valid object GUIDs from initialized objects)

**Documentation:**
- [x] Rustdoc for `Connection` and all message types
- [x] Document JSON-RPC protocol format in code comments
- [x] Examples showing request/response flow in rustdoc
- [x] Links to official Playwright bindings for reference

**Notes:**
- ✅ Used `tokio::sync::oneshot` for request/response completion
- ✅ Used `Arc<tokio::sync::Mutex<>>` for thread-safe shared state (async-safe)
- ✅ Timeout handling: Implemented via channel closed error when connection drops
- ✅ Event handling deferred to Slice 4 (currently logs events via tracing)

**Lessons Learned (Post-Implementation 2025-11-06):**

1. **Async Mutex Required for Async Operations**
   - Initially used `std::sync::Mutex` but caused compile errors with `.await`
   - Solution: Use `tokio::sync::Mutex` for any locks held across await points
   - `std::sync::Mutex` is fine for quick operations without awaits

2. **Generic Type Parameters for Testability**
   - Made `Connection<W, R>` generic over `AsyncWrite + AsyncRead`
   - Allows unit tests to use `tokio::io::duplex()` mock pipes
   - Production code uses real `ChildStdin` and `ChildStdout`
   - Same pattern as PipeTransport

3. **Untagged Enum for Protocol Message Discrimination**
   - Used `#[serde(untagged)]` on `enum Message { Response, Event }`
   - Serde automatically distinguishes based on presence of `id` field
   - Cleaner than manual field checking
   - Matches JSON-RPC protocol exactly

4. **Connection Spawns Transport Loop Internally**
   - `Connection::run()` spawns the transport read loop as a background task
   - Simplifies API - user only needs to spawn one loop, not two
   - Transport loop reads from stdio and sends to channel
   - Connection loop reads from channel and dispatches messages

5. **Integration Tests with Real Server**
   - Basic lifecycle test: server launches, connection starts, no panics
   - Error detection test: send after crash detects broken pipe fast (~150ms)
   - Full protocol tests deferred to Slice 4 (need object initialization)
   - Clear separation: unit tests for logic, integration tests for infrastructure

6. **Error Propagation Through Layers**
   - Transport errors (broken pipe, read failures) → `Error::TransportError`
   - Protocol errors (TimeoutError, TargetClosedError) → specific error variants
   - Channel closed → `Error::ChannelClosed`
   - Clear error boundaries at each layer

---

### Slice 4: Object Factory and Channel Owners

**Status:** ✅ Complete (2025-11-06)

**User Value:** Protocol objects (Browser, Page, etc.) are automatically created when server sends initializers, enabling the object model.

**Acceptance Criteria:**
- [x] Connection creates objects from protocol messages
- [x] Each object has a GUID and type
- [x] Objects are stored in connection's object registry
- [x] Events are routed to correct object by GUID
- [x] Object lifecycle is managed (creation, deletion via __create__, __dispose__, __adopt__)

**Core Library Implementation (`playwright-core`):**
- [x] Create `src/channel_owner.rs`:
  - `trait ChannelOwner` - Base for all protocol objects
    - `fn guid(&self) -> &str`
    - `fn on_event(&self, method: &str, params: JsonValue)`
    - `fn connection(&self) -> Arc<dyn ConnectionLike>`
    - `fn parent()`, `fn initializer()`, `fn channel()`, `fn dispose()`, `fn adopt()`, etc.
  - `struct ChannelOwnerImpl` - Reusable base implementation
- [x] Create `src/connection.rs` additions:
  - `trait ConnectionLike` - Object-safe connection interface
  - Object registry: `objects: Arc<Mutex<HashMap<String, Arc<dyn ChannelOwner>>>>`
  - Methods: `register_object()`, `unregister_object()`, `get_object()`
- [x] Create `src/channel.rs`:
  - `struct Channel` - RPC communication proxy
  - `fn send<P, R>()` - Generic typed RPC calls
- [x] Create `src/object_factory.rs`:
  - `fn create_object(parent: ParentOrConnection, type_name: String, guid: String, initializer: Value) -> Result<Arc<dyn ChannelOwner>>`
  - Match on `type_name`:
    - `"Playwright"` -> `Playwright::new()`
    - `"BrowserType"` -> `BrowserType::new()`
    - Future: `"Browser"`, `"BrowserContext"`, `"Page"`, etc. (Phase 2)
    - Unknown types return error with logging
- [x] Create protocol objects:
  - `src/protocol/mod.rs` - Protocol module
  - `src/protocol/playwright.rs` - Root Playwright object with chromium(), firefox(), webkit()
  - `src/protocol/browser_type.rs` - BrowserType object with name and executable_path
- [x] Update `Connection::dispatch()`:
  - Handle `__create__` messages via `handle_create()`
  - Handle `__dispose__` messages via `handle_dispose()`
  - Handle `__adopt__` messages via `handle_adopt()`
  - Call `create_object()` for new objects
  - Store in `objects` registry by GUID
  - Route events to object by GUID via `on_event()`

**Core Library Unit Tests:**
- [x] Connection unit tests (27 tests in connection.rs) - Request ID, dispatch, concurrent requests, error handling
- [x] Transport unit tests (8 tests in transport.rs) - Message framing, encoding, large messages
- [x] Server unit tests (2 tests in server.rs) - Launch, shutdown, kill
- [x] Driver unit tests (1 test in driver.rs) - Node executable detection
- Note: Object creation/registration tested via integration tests (require real Connection and server)

**Integration Tests:**
- [x] `test_connection_lifecycle_with_real_server` - Server launches, connection starts, no panics
- [x] `test_connection_detects_server_crash_on_send` - Broken pipe detection

**Integration Tests Deferred to Slice 5:**
The following tests require the full initialization flow (Playwright::launch()):
- [ ] Verify root "Playwright" object creation from server __create__ messages
- [ ] Verify "BrowserType" objects are initialized from server __create__ messages
- [ ] Test sending protocol requests with valid object GUIDs
- [ ] Test concurrent requests to real server
- [ ] Test server response to protocol messages
- [ ] Test full request/response cycle with object factory

**Rationale for Deferral:**
These tests require:
1. Complete initialization sequence (launch server → receive __create__ messages → build object tree)
2. Valid object GUIDs from initialized objects
3. `Playwright::launch()` API to orchestrate the flow

This functionality is implemented in Slice 5 (Entry Point), not Slice 4 (Object Factory).
Slice 4 provides the *infrastructure* (object factory, ChannelOwner, protocol handlers).
Slice 5 provides the *orchestration* (launch sequence, initialization, public API).

**Documentation:**
- [x] Rustdoc for `ChannelOwner` trait with complete implementation example
- [x] Rustdoc for `ChannelOwnerImpl` with usage pattern
- [x] Rustdoc for `Channel` with RPC examples
- [x] Rustdoc for `object_factory::create_object()` with usage
- [x] Rustdoc for `Playwright` and `BrowserType` protocol objects
- [x] Code comments explaining object lifecycle, downcasting, RAII patterns
- [x] Links to official Playwright implementations for reference

**Notes:**
- Start with minimal object types (Playwright, BrowserType) ✅
- Full Browser/Page implementation comes in Phase 2
- Use `Arc<dyn ChannelOwner>` for object references ✅
- **Downcasting**: Convert generic objects to specific types using `Any` trait ✅
  - Implemented via `as_any()` method returning `&dyn Any`
  - Example: `object.as_any().downcast_ref::<BrowserType>()`
  - Used in Playwright object to access BrowserType references

**Lessons Learned (Post-Implementation 2025-11-06):**

1. **Object-Safe Traits with Async Methods**
   - Challenge: `impl Future` in traits prevents `dyn Trait` usage
   - Solution: Use `Pin<Box<dyn Future>>` for object-safe async methods
   - Applied in `ConnectionLike::send_message()` to enable `Arc<dyn ConnectionLike>`

2. **Lifetime Management with Boxed Futures**
   - Challenge: String slices in async blocks cause lifetime issues with Box::pin
   - Solution: Convert to owned `String` before boxing the future
   - Pattern: Clone strings into the async block to satisfy 'static requirement

3. **Circular Dependencies Between Modules**
   - Challenge: Connection needs ChannelOwner, ChannelOwner needs Connection
   - Solution: Create `ConnectionLike` trait that Connection implements
   - Pattern: Use trait abstraction to break circular type dependencies

4. **Generic Type Parameters for Testability**
   - Continued from Slices 2-3: `Connection<W, R>` generic over AsyncWrite/AsyncRead
   - Enables both unit tests (mock duplex pipes) and integration tests (real server)
   - Pattern: Generic at low level, type alias for common case

5. **Downcasting Pattern for Protocol Objects**
   - Pattern: Store as `Arc<dyn ChannelOwner>`, downcast via `as_any()`
   - Example: `object.as_any().downcast_ref::<BrowserType>()` for concrete access
   - Matches pattern from official Playwright bindings (type-erased storage)

6. **Testing Strategy: Integration Over Unit**
   - Object creation/registration requires real Connection + server
   - Unit tests for isolated logic (message parsing, ID generation)
   - Integration tests for object lifecycle and protocol flow
   - Clear separation: what can be mocked vs. what needs real infrastructure

7. **Documentation as Code**
   - Complete doctest examples serve as both docs and tests (15 doctests passing)
   - Show full trait implementation pattern for future protocol objects
   - Provides reference for contributors adding Browser, Page, etc.

---

### Slice 5: Entry Point - Playwright::launch()

**Status:** Not Started

**User Value:** Can write `Playwright::launch().await?` to get a working Playwright instance with access to browser types.

**Acceptance Criteria:**
- [ ] `Playwright::launch()` returns `Result<Playwright>`
- [ ] Playwright instance provides access to `chromium()`, `firefox()`, `webkit()`
- [ ] Connection lifecycle is managed automatically
- [ ] Errors during initialization are propagated clearly
- [ ] Example code in README works end-to-end

**Core Library Implementation (`playwright-core`):**
- [ ] Create `src/playwright.rs`:
  - `pub struct Playwright` - Public API entry point
    - `connection: Arc<Connection>`
    - `chromium: BrowserType`
    - `firefox: BrowserType`
    - `webkit: BrowserType`
  - `impl Playwright`:
    - `pub async fn launch() -> Result<Self>`
    - `pub fn chromium(&self) -> &BrowserType`
    - `pub fn firefox(&self) -> &BrowserType`
    - `pub fn webkit(&self) -> &BrowserType`
- [ ] Implement launch flow:
  1. Download driver if needed
  2. Launch server process
  3. Create transport
  4. Create connection
  5. Start connection dispatch loop
  6. Wait for root "Playwright" object
  7. Extract BrowserType objects
  8. Return Playwright instance
- [ ] Export in `src/lib.rs`:
  - `pub use playwright::Playwright;`
  - `pub use error::Error;`

**Public API Crate (`playwright`):**
- [ ] Create `crates/playwright/` workspace member
- [ ] Add dependency on `playwright-core`
- [ ] Re-export public API in `src/lib.rs`:
  ```rust
  pub use playwright_core::{Playwright, Error};
  ```
- [ ] Add basic example in `examples/basic.rs`:
  ```rust
  use playwright::Playwright;

  #[tokio::main]
  async fn main() -> Result<(), Box<dyn std::error::Error>> {
      let playwright = Playwright::launch().await?;
      println!("Playwright launched successfully!");
      println!("Chromium: {:?}", playwright.chromium());
      Ok(())
  }
  ```

**Core Library Unit Tests:**
- [ ] Test `Playwright::launch()` returns Ok
- [ ] Test browser types are available
- [ ] Test launch with driver not found (error)
- [ ] Test launch with server crash (error)

**Integration Tests:**
- [ ] Test full launch flow with real server
- [ ] Verify all three browser types exist
- [ ] Test multiple Playwright instances
- [ ] Test graceful cleanup on drop

**Documentation:**
- [ ] Rustdoc for `Playwright` struct and methods
- [ ] Usage example in doc comments
- [ ] Update README.md with working example
- [ ] Document error scenarios

**Notes:**
- Consider implementing `Drop` for cleanup
- **RAII (Resource Acquisition Is Initialization)**: Automatic cleanup when objects go out of scope
  - Example: Browser automatically closes when `browser` variable is dropped
  - Implemented via Rust's `Drop` trait: `impl Drop for Browser { fn drop(&mut self) { ... } }`
  - Challenge: `Drop` is synchronous, but cleanup requires async calls to server
  - Solutions: Spawn background task in Drop, or require explicit `.close()` calls
  - Matches Python's context manager pattern (`with sync_playwright() as p:`)
- Connection dispatch loop should run in background task
- Need to handle Playwright object initialization timeout

---

## Slice Priority and Dependencies

| Slice | Priority | Depends On | Status |
|-------|----------|------------|--------|
| Slice 1: Server Launch | Must Have | None | ✅ Complete |
| Slice 2: Stdio Transport | Must Have | Slice 1 | ✅ Complete |
| Slice 3: Connection Layer | Must Have | Slice 2 | ✅ Complete |
| Slice 4: Object Factory | Must Have | Slice 3 | 🔄 Ready to Start |
| Slice 5: Entry Point | Must Have | Slice 4 | Not Started |

**Critical Path:** All slices are sequential and required for Phase 1 completion.

---

## Definition of Done

Phase 1 is complete when ALL of the following are true:

- [ ] All acceptance criteria from all slices are met
- [ ] Can run: `Playwright::launch().await?` successfully
- [ ] Can access `chromium()`, `firefox()`, `webkit()` browser types (objects exist, not yet launching browsers)
- [ ] All tests passing: `cargo test --workspace`
- [ ] Example code in README.md works
- [ ] Core library documentation complete: `cargo doc --open`
- [ ] Code formatted: `cargo fmt --check`
- [ ] No clippy warnings: `cargo clippy --workspace -- -D warnings`
- [ ] Cross-platform compatibility (macOS, Linux) - Windows optional
- [ ] README.md updated with Phase 1 status
- [ ] Playwright server downloads automatically on first run
- [ ] No unsafe code (or justified with SAFETY comments)
- [ ] Error messages are helpful and actionable

**Success Metric:** Can execute this code without errors:

```rust
use playwright::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    println!("Chromium: {:?}", playwright.chromium());
    println!("Firefox: {:?}", playwright.firefox());
    println!("WebKit: {:?}", playwright.webkit());
    Ok(())
}
```

**Note on Cross-Browser Testing:**
Phase 1 establishes the protocol foundation and provides access to all three `BrowserType` objects (Chromium, Firefox, WebKit). Actual browser launching (e.g., `chromium().launch().await?`) and comprehensive cross-browser testing will be implemented in Phase 2 (Browser API implementation). The architecture built in Phase 1 is designed from the ground up to support all three browsers equally.

---

## Learnings & Adjustments

### What's Working Well

**As of Slice 3 completion (2025-11-06):**

1. **Vertical Slicing Approach**
   - Each slice delivers end-to-end testable functionality
   - Clear dependencies between slices enable incremental progress
   - TDD workflow (Red → Green → Refactor) keeps quality high

2. **Generic Type Parameters**
   - `Transport<W, R>` and `Connection<W, R>` generic over AsyncWrite/AsyncRead
   - Enables both unit tests (mock duplex pipes) and integration tests (real server)
   - Excellent testability without sacrificing production performance

3. **Research-Driven Implementation**
   - Studying all three official bindings (Python, Java, .NET) before implementing
   - Identified common patterns (sequential IDs, oneshot channels, untagged enums)
   - Avoided pitfalls (std::sync::Mutex vs tokio::sync::Mutex)

4. **Cross-Platform Support**
   - CI validates on macOS, Ubuntu, and Windows
   - All 39 tests passing on all three platforms
   - Platform detection and driver download working correctly

### Challenges Encountered

1. **Async Mutex Requirements**
   - Initial use of `std::sync::Mutex` failed when holding locks across `.await`
   - Solution: Use `tokio::sync::Mutex` for async operations
   - Learned: Check if locks are held across await points

2. **Test Timeout Issues**
   - Initial crash detection test used passive 5s timeout
   - Solution: Actively send message to trigger broken pipe detection fast
   - Result: Test time reduced from 5s to ~150ms

3. **Transport Ownership in Connection**
   - Initially unclear whether transport should be spawned separately or owned by Connection
   - Solution: Connection owns transport and spawns its loop internally
   - Result: Simpler API - user only spawns Connection.run()

### Adjustments Made to Plan

1. **Deferred Test Clarification**
   - Originally said transport protocol tests "deferred to Slice 3"
   - Realized they need Slice 4 (object initialization for valid GUIDs)
   - Updated: All protocol interaction tests now correctly deferred to Slice 4

2. **Integration Test Strategy**
   - Planned full protocol tests in Slice 3
   - Realized we need object initialization first
   - Adjusted: Basic lifecycle tests in Slice 3, full protocol tests in Slice 4

3. **Message Loop Architecture**
   - Originally considered spawning transport and connection loops separately
   - Decided: Connection spawns transport loop internally
   - Benefit: Cleaner API, easier for users

### Lessons for Future Features

1. **Start with Research**
   - Always study official bindings first
   - Document patterns before implementing
   - Saves time and avoids design mistakes

2. **Generic for Testability**
   - Generic type parameters enable both unit and integration tests
   - Worth the complexity for excellent test coverage
   - Pattern: `PipeTransport<W, R>`, `Connection<W, R>`

3. **Defer Appropriately**
   - Be honest about what can't be tested yet
   - Don't try to test protocol interactions without object initialization
   - Clear deferral notes prevent confusion

4. **Fast Integration Tests**
   - Actively trigger conditions rather than waiting for timeouts
   - Example: Send message after crash to detect broken pipe
   - Result: Fast tests that still validate real behavior

---

## References

**Microsoft Playwright Protocol:**
- Protocol schema: `microsoft/playwright/packages/protocol/src/protocol.yml`
- Protocol docs: https://playwright.dev/docs/api

**Reference Implementations:**
- Python connection: `microsoft/playwright-python/playwright/_impl/_connection.py`
- Python transport: `microsoft/playwright-python/playwright/_impl/_transport.py`
- Java connection: `microsoft/playwright-java/playwright/src/main/java/com/microsoft/playwright/impl/Connection.java`
- Java transport: `microsoft/playwright-java/playwright/src/main/java/com/microsoft/playwright/impl/PipeTransport.java`

**Key Architectural Patterns:**
1. Length-prefixed message framing (4 bytes LE + JSON)
2. Request/response correlation via message ID
3. GUID-based object references
4. Event-driven architecture
5. Object factory pattern for protocol types

**Driver Bundling Strategy:**

Based on research of all three official Microsoft Playwright bindings (completed 2025-11-05), the driver distribution strategy is:

- **All official bindings bundle drivers** in their packages (Python wheel, Java JAR, .NET NuGet)
- **Build-time download** from Azure CDN: `https://playwright.azureedge.net/builds/driver/`
- **Platform-specific binaries** included (Node.js + Playwright package)
- **No separate installation** - users just install the package and it works

See **[ADR 0001: Driver Distribution Strategy](../adr/0001-protocol-architecture.md#driver-distribution-strategy)** for full details and rationale.
