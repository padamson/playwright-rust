# ADR 0001: Protocol Architecture - JSON-RPC over Stdio

**Status:** Accepted | **Phase 1:** ✅ Complete (2025-11-06)

**Date:** 2025-11-05

**Related Documents:**
- Implementation Plan: [Phase 1: Protocol Foundation](../implementation-plans/phase1-protocol-foundation.md)
- Microsoft Playwright Python: https://github.com/microsoft/playwright-python
- Microsoft Playwright Java: https://github.com/microsoft/playwright-java
- Microsoft Playwright .NET: https://github.com/microsoft/playwright-dotnet
- Protocol Schema: https://github.com/microsoft/playwright/tree/main/packages/protocol

---

## Context and Problem Statement

We need to implement Rust bindings for Microsoft Playwright that provide the same functionality as the official Python, Java, and .NET bindings. The core question is: **How should playwright-rust communicate with browsers?**

There are fundamentally two approaches:
1. **Reimplement browser protocols** (Chrome DevTools Protocol, Firefox Remote Protocol, WebKit Inspector Protocol)
2. **Reuse Playwright's server** (JSON-RPC to Node.js server that handles protocols)

### Requirements Summary

- **Functional Requirements:**
  - Must support all three browsers: Chromium, Firefox, WebKit
  - Must match Playwright API across all languages (API consistency)
  - Must handle browser automation (navigation, clicks, screenshots, etc.)
  - Must support auto-waiting and locators (Playwright's key features)
  - Must handle asynchronous operations (browser operations are inherently async)

- **Non-Functional Requirements:**
  - **Compatibility:** Match playwright-python/JS/Java API exactly
  - **Maintainability:** Minimize protocol implementation burden (solo/small team)
  - **API Ergonomics:** Idiomatic Rust with async/await
  - **Safety:** Type-safe bindings, proper error handling
  - **Performance:** Async I/O, efficient serialization
  - **Broad Adoption:** Build with quality suitable for broad adoption

- **Constraints:**
  - Must work with official Playwright server (don't fork)
  - Must follow Playwright's architecture patterns (proven by millions of users)
  - Limited development resources (community/solo developer initially)
  - Rust async ecosystem maturity (tokio is de facto standard)

### Current Architecture Context

- **Existing Codebase:** New project, no existing implementation
- **Target Dependencies:** tokio (async), serde (serialization), minimal external deps
- **Integration Points:** Playwright Node.js server, Playwright protocol (protocol.yml)

---

## Decision Drivers

Prioritized factors influencing this decision:

1. **Microsoft Compatibility** - Must follow Microsoft's proven architecture for adoption
2. **Cross-browser Support** - All three browsers without reimplementing protocols
3. **Maintainability** - Minimize protocol complexity, reuse Playwright's server
4. **API Consistency** - Match Python/JS/Java exactly for cross-language familiarity
5. **Feature Parity** - Automatic updates when Playwright server updates
6. **Development Speed** - Ship faster by reusing battle-tested server

---

## Options Considered

### Option 1: Reimplement Browser Protocols (Chrome DevTools Protocol, etc.)

**Description:**
Implement direct communication with browsers using Chrome DevTools Protocol (CDP), Firefox Remote Protocol, and WebKit Inspector Protocol. Similar to libraries like `chromiumoxide` (Rust) or `puppeteer` (JS).

**Key Implementation Details:**
- Implement WebSocket client for CDP (Chromium)
- Implement separate protocol clients for Firefox and WebKit
- Manually map high-level API to low-level protocol commands
- Handle browser lifecycle, connections, contexts manually
- Maintain protocol implementations as browsers update

**Code Example:**
```rust
// Direct CDP implementation
pub struct ChromiumBrowser {
    websocket: WebSocket,
}

impl ChromiumBrowser {
    pub async fn navigate(&self, url: &str) -> Result<()> {
        let command = json!({
            "id": self.next_id(),
            "method": "Page.navigate",
            "params": { "url": url }
        });
        self.websocket.send(command).await?;
        // Wait for response, handle events...
        Ok(())
    }
}
```

**Pros:**
- No dependency on Node.js or Playwright server
- Direct control over browser communication
- Potentially faster (no intermediary process)
- Can optimize for Rust-specific use cases

**Cons:**
- **Massive implementation burden** - 3 different browser protocols
- **No cross-browser abstraction** - different APIs for each browser
- **Constant maintenance** - protocols change with browser updates
- **Feature lag** - new Playwright features require re-implementation
- **Testing complexity** - must test against 3 browser implementations
- **Incompatible with Playwright ecosystem** - different API surface

**Dependencies Required:**
- `tokio-tungstenite` - WebSocket client
- Custom protocol implementations for CDP, Firefox, WebKit
- Browser driver management

---

### Option 2: JSON-RPC to Playwright Server (Microsoft Architecture)

**Description:**
Communicate with Playwright's Node.js server via JSON-RPC over stdio pipes. The server handles all browser protocol complexity and provides a unified cross-browser API. This is the architecture used by playwright-python, playwright-java, and playwright-dotnet.

**Key Implementation Details:**
- Launch Playwright server as child process (`node cli.js run-driver`)
- Communicate via stdio pipes with length-prefixed JSON messages
- JSON-RPC protocol with request/response correlation (message IDs)
- GUID-based object references (Browser, Page, etc.)
- Event-driven architecture for protocol events
- Protocol defined in `protocol.yml` (single source of truth)

**Code Example:**
```rust
// High-level Rust API
pub struct Page {
    connection: Arc<Connection>,
    guid: String,
}

impl Page {
    pub async fn goto(&self, url: &str) -> Result<Response> {
        let result = self.connection.send_message(
            &self.guid,
            "goto",
            json!({ "url": url })
        ).await?;

        Ok(Response::from_json(result)?)
    }
}

// Connection layer
pub struct Connection {
    transport: Arc<PipeTransport>,
    callbacks: Mutex<HashMap<u64, oneshot::Sender<JsonValue>>>,
}

impl Connection {
    pub async fn send_message(
        &self,
        guid: &str,
        method: &str,
        params: JsonValue,
    ) -> Result<JsonValue> {
        let id = self.next_id();
        let (tx, rx) = oneshot::channel();
        self.callbacks.lock().await.insert(id, tx);

        self.transport.send(json!({
            "id": id,
            "guid": guid,
            "method": method,
            "params": params,
        })).await?;

        rx.await.map_err(|_| Error::Timeout)?
    }
}
```

**Pros:**
- **Proven architecture** - Used by 4 official Playwright implementations
- **Cross-browser support** - Server handles Chromium, Firefox, WebKit
- **Feature parity** - Automatic updates when server updates
- **Minimal maintenance** - Server maintained by Microsoft Playwright team
- **API consistency** - Match Python/JS/Java exactly
- **Testing leverage** - Playwright's test suite validates server behavior
- **Broad community adoption path** - Same architecture as official bindings

**Cons:**
- Requires Node.js runtime (for Playwright server)
- Additional process overhead (server + Rust client)
- Slightly higher latency (IPC vs. direct WebSocket)
- Dependency on Playwright server releases

**Dependencies Required:**
- `tokio` (async runtime, process management, stdio I/O)
- `serde` + `serde_json` (JSON serialization)
- `thiserror` (error handling)
- Playwright server (downloaded at build/runtime)

---

### Option 3: Hybrid Approach (Playwright Server + Direct CDP Fallback)

**Description:**
Use Playwright server as primary method, but implement direct CDP support for advanced use cases or when Node.js is unavailable.

**Key Implementation Details:**
- Default to JSON-RPC protocol (Option 2)
- Provide fallback to direct CDP for Chromium-only use cases
- Maintain both code paths
- Feature flag to enable direct protocol

**Code Example:**
```rust
pub struct Playwright {
    backend: Backend,
}

enum Backend {
    Server(PlaywrightServer),
    Direct(DirectProtocol),
}

impl Playwright {
    pub async fn launch(config: Config) -> Result<Self> {
        let backend = if config.use_server {
            Backend::Server(PlaywrightServer::launch().await?)
        } else {
            Backend::Direct(DirectProtocol::connect().await?)
        };
        Ok(Self { backend })
    }
}
```

**Pros:**
- Flexibility for users without Node.js
- Can optimize direct path for Chromium-only scenarios
- Fallback if server has issues

**Cons:**
- **Double the maintenance burden** - Two protocol implementations
- **API surface complexity** - Different capabilities per backend
- **Testing complexity** - Must test both paths
- **Fragmentation** - Users have inconsistent experience
- **Still doesn't solve Firefox/WebKit** - Only Chromium works in direct mode

**Dependencies Required:**
- All dependencies from Option 1 + Option 2
- Feature flag management

---

## Comparison Matrix

### Feature Comparison

| Capability | Option 1: Direct | Option 2: JSON-RPC | Option 3: Hybrid | Weight | Notes |
|-----------|-----------------|-------------------|-----------------|--------|-------|
| **Cross-Browser** | 3 (requires 3 implementations) | 5 (server handles all) | 4 (server for 3, direct for Chrome) | **Critical** | All three browsers required |
| **API Consistency** | 2 (different from Playwright) | 5 (matches exactly) | 4 (mostly matches) | **Critical** | Match Python/JS/Java |
| **Maintenance Burden** | 1 (very high) | 5 (minimal) | 2 (high) | **Critical** | Solo/small team |
| **Feature Parity** | 2 (manual updates) | 5 (automatic) | 4 (server path auto) | **High** | Keep up with Playwright |
| **Broad Adoption** | 1 (incompatible) | 5 (same architecture) | 3 (partial) | **Critical** | Goal is broad community adoption |
| **Performance** | 4 (direct) | 4 (IPC overhead minimal) | 4 (same) | Medium | Both are fast enough |
| **Node.js Dependency** | 5 (none) | 3 (required) | 4 (optional) | Low | Node.js widely available |

### Dependency Comparison

| Dependency Aspect | Option 1 | Option 2 | Option 3 | Notes |
|------------------|----------|----------|----------|-------|
| **Rust Crates** | 10+ (WebSocket, protocol impls) | 3 (tokio, serde, thiserror) | 13+ | Option 2 is minimal |
| **System Dependencies** | Browser binaries | Node.js + Playwright | Both | Node.js easier to manage |
| **Binary Size Impact** | ~2-3 MB | ~1 MB | ~3-4 MB | Option 2 smallest |
| **Compile Time** | Slow (many deps) | Fast (few deps) | Slowest | Option 2 wins |
| **MSRV (Rust Version)** | 1.70 | 1.70 | 1.70 | All similar |
| **Stability Risk** | High (3 protocols change) | Low (server stable) | High | Server is battle-tested |

### Maintenance & Ecosystem

| Factor | Option 1 | Option 2 | Option 3 | Weight | Notes |
|--------|----------|----------|----------|--------|-------|
| **Implementation Effort** | 6-12 months | 1-2 months | 8-14 months | **Critical** | Time to first release |
| **Ongoing Maintenance** | High (protocol updates) | Low (server maintained) | Very High | **Critical** | Long-term burden |
| **Community Support** | None (new implementation) | High (Playwright community) | Split | High | Leverage existing community |
| **Breaking Changes Risk** | High (3 protocols evolve) | Low (protocol.yml stable) | High | High | API stability |
| **Broad Community Adoption Path** | None (incompatible) | Clear (same architecture) | Unclear | **Critical** | Primary goal |

---

## Decision Outcome

**Chosen Option:** Option 2 - JSON-RPC to Playwright Server (Microsoft Architecture)

**Rationale:**

We chose **Option 2 (JSON-RPC to Playwright Server)** because it is the **only option compatible with our goal of broad community adoption** and the **only practical option for a solo/small team to maintain**.

1. **Microsoft Compatibility (Critical Driver)**
   - playwright-python, playwright-java, and playwright-dotnet all use this architecture
   - Protocol defined in `protocol.yml` ensures consistency
   - Proven by millions of users across 4 languages
   - **This is the path to broad adoption** - using a different architecture would greatly impact adoption

2. **Maintainability (Critical Driver)**
   - Server maintained by Microsoft Playwright team (60+ engineers)
   - Automatic feature updates when server updates
   - No need to reimplement Chromium, Firefox, WebKit protocols
   - **Solo developer can build this** vs. 6-12 month effort for Option 1

3. **Cross-Browser Support (Critical Driver)**
   - Server provides unified API for all three browsers
   - Protocol abstraction handled by server
   - Testing leverage from Playwright's extensive test suite

4. **API Consistency (Critical Driver)**
   - Exact match with Python/JS/Java implementations
   - Users familiar with Playwright can use Rust bindings immediately
   - Documentation and examples translate directly

5. **Feature Parity (High Priority)**
   - New Playwright features available automatically
   - No lag time for re-implementation
   - Bug fixes in server benefit all language bindings

**Trade-offs Accepted:**

- **Node.js dependency** - Acceptable because Node.js is widely available and used by Playwright ecosystem
- **IPC overhead** - Minimal impact (microseconds vs. milliseconds for browser operations)
- **Additional process** - Server process is lightweight and well-managed
- **Server release dependency** - Acceptable because server is stable and well-maintained

---

## Consequences

### Positive Consequences

- **Fast implementation** - Can ship Phase 1 in weeks, not months
- **Microsoft-compatible** - Clear path to official adoption
- **Cross-browser support** - All three browsers from day one
- **Minimal maintenance** - Server maintained by Microsoft
- **API consistency** - Match Python/JS/Java exactly
- **Automatic updates** - New features come for free
- **Testing leverage** - Rely on Playwright's extensive test suite
- **Small binary** - Minimal Rust dependencies (tokio, serde, thiserror)
- **Fast compile times** - Few dependencies

### Negative Consequences

- **Node.js requirement** - Users must have Node.js installed
- **IPC latency** - Minor overhead for message passing (typically <1ms)
- **Server process** - Additional process to manage
- **Server version coupling** - Must stay compatible with server releases

### Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| Server protocol changes | Medium | Low | Protocol.yml is versioned, breaking changes are rare and documented |
| Node.js not available | Medium | Low | Node.js widely available, provide clear installation docs |
| Server crashes | Medium | Low | Playwright server is production-tested, handle gracefully and restart |
| IPC performance issues | Low | Very Low | Benchmark early, stdio pipes are very fast for JSON messages |

---

## Validation

### How This Decision Will Be Validated

- [x] Research all three official implementations (Python, Java, .NET) - **Completed 2025-11-05**
- [x] Research driver bundling strategy across all bindings - **Completed 2025-11-05**
- [x] Implement Phase 1: Slice 1 (Server launch) - **Completed 2025-11-05**
- [x] Implement Phase 1: Slice 2 (Transport layer) - **Completed 2025-11-05**
- [x] Implement Phase 1: Slice 3 (Connection layer) - **Completed 2025-11-06**
- [x] Implement Phase 1: Slice 4 (Object factory) - **Completed 2025-11-06**
- [x] Implement Phase 1: Slice 5 (Entry point) - **Completed 2025-11-06**
- [x] Test cross-browser compatibility (Chromium, Firefox, WebKit) - All three browser types accessible
- [x] Verify API matches playwright-python for Phase 1 features

### Success Criteria

- [x] Can launch Playwright server from Rust
- [x] Can send/receive JSON-RPC messages over stdio
- [x] Can correlate requests/responses with message IDs
- [x] Can create protocol objects (Playwright, BrowserType)
- [x] Can handle protocol lifecycle messages (__create__, __dispose__, __adopt__)
- [x] Can launch Playwright and access browser types
- [x] API matches playwright-python for Phase 1 features
- [x] Tests pass on macOS and Linux

**Note:** Additional testing and optimization deferred to [Phase 2 Implementation Plan](../implementation-plans/phase2-browser-api.md).

---

## Implementation Notes

### Code Changes Required

Phase 1 implementation (see [phase1-protocol-foundation.md](../implementation-plans/phase1-protocol-foundation.md)):

1. **Server Management** (`src/server.rs`) - ✅ Complete (2025-11-05)
   - Build-time driver download via `build.rs`
   - Launch server process: `node cli.js run-driver`
   - Set environment: `PW_LANG_NAME=rust`, `PW_LANG_NAME_VERSION`, `PW_CLI_DISPLAY_VERSION`

2. **Transport Layer** (`src/transport.rs`) - ✅ Complete (2025-11-05)
   - Stdio pipe communication
   - Length-prefixed message framing (4 bytes LE + JSON)
   - Async read/write with tokio
   - Generic over `AsyncWrite + AsyncRead` for testability
   - 8 unit tests + 3 integration tests (all passing)

   **Transport Implementation Details (Research 2025-11-05):**

   All three official bindings use identical message framing:
   - **4-byte little-endian length prefix** + JSON payload
   - Python: `len(data).to_bytes(4, byteorder="little")`
   - Java: Custom bit shifting for little-endian encoding
   - .NET: Byte masks for little-endian encoding

   Async patterns vary by language:
   - **Python**: Single async loop, `readexactly()`, direct callback
   - **Java**: Separate reader/writer threads, blocking queues
   - **.NET**: Async Tasks, event-based dispatch

   **Rust approach**: Match Python's async pattern (closest to tokio):
   - Use `tokio::io::AsyncReadExt::read_exact()` for length prefix
   - Single async task for read loop
   - `tokio::sync::mpsc` channel for message dispatch
   - Match Python's 32KB chunk size for large messages

3. **Connection Layer** (`src/connection.rs`) - ✅ Complete (2025-11-06)
   - JSON-RPC client with request/response correlation
   - Message ID generation and callback management (`AtomicU32`)
   - Request/response correlation using `tokio::sync::oneshot` channels
   - Event dispatch to protocol objects (logs for now, full dispatch in Slice 4)
   - Protocol message types: `Request`, `Response`, `Event`, `Message`
   - Error propagation: `TimeoutError`, `TargetClosedError`, generic `ProtocolError`
   - 9 unit tests + 2 integration tests (all passing)

   **Connection Implementation Details (2025-11-06):**

   All official bindings use similar patterns:
   - **Sequential request IDs** for correlation (starting from 0)
   - **Callback storage** using HashMap keyed by ID
   - **oneshot channels** (or equivalent) for async response completion
   - **Untagged enum** to distinguish responses (with ID) from events (without ID)

   **Rust approach**: Generic `Connection<W, R>` for testability
   - `AtomicU32` for thread-safe ID generation
   - `Arc<tokio::sync::Mutex<HashMap>>` for async-safe callback storage
   - `#[serde(untagged)]` enum for automatic message discrimination
   - Connection spawns transport loop internally (clean API)

4. **Object Factory** (`src/object_factory.rs`) - ✅ Complete (2025-11-06)
   - Type-to-constructor mapping (`create_object()` function)
   - Currently supports "Playwright" and "BrowserType"
   - Extensible for future types (Browser, Page, etc.)
   - Object registry in Connection (register/unregister/get)
   - Protocol message handlers: `__create__`, `__dispose__`, `__adopt__`
   - Event routing to objects by GUID

   **Object Factory Implementation Details (2025-11-06):**

   ChannelOwner pattern across all official bindings:
   - **Base trait/class** for all protocol objects
   - **GUID-based identity** for object lookup
   - **Parent-child hierarchy** for lifecycle management
   - **Channel proxy** for RPC communication
   - **Event handling** via `on_event()` method

   **Rust approach**:
   - `ChannelOwner` trait with `ChannelOwnerImpl` base struct
   - `ConnectionLike` trait for object-safe connection references
   - `Arc<dyn ChannelOwner>` for polymorphic object storage
   - Downcasting via `as_any()` for concrete type access
   - Protocol objects: `Playwright` (root) and `BrowserType`

5. **Entry Point** (`src/protocol/playwright.rs`) - ✅ Complete (2025-11-06)
   - `Playwright::launch()` - High-level API orchestrates full initialization
   - `Playwright::new()` - Internal constructor called by object factory
   - Access to `chromium()`, `firefox()`, `webkit()` browser types
   - Public API crate (`playwright`) re-exports for clean interface

### Documentation Updates

- [x] Document JSON-RPC protocol in rustdoc - All modules documented
- [x] Link to protocol.yml in rustdoc comments
- [x] Explain length-prefix framing in transport.rs
- [x] Document object lifecycle in channel_owner.rs
- [x] Provide examples in rustdoc and README

### Testing Strategy

- [x] Unit tests for message framing (transport.rs)
- [x] Unit tests for request/response correlation (connection.rs)
- [x] Integration tests with real Playwright server (57 tests passing)
- [x] Cross-browser support verified (all three browser types accessible)
- [x] Error handling tests (server crash detection, error propagation)

---

## References

**Microsoft Playwright Implementations:**
- **Python:** https://github.com/microsoft/playwright-python
  - Connection: `playwright/_impl/_connection.py`
  - Transport: `playwright/_impl/_transport.py`
  - Object Factory: `playwright/_impl/_object_factory.py`
- **Java:** https://github.com/microsoft/playwright-java
  - Connection: `com.microsoft.playwright.impl.Connection`
  - Transport: `com.microsoft.playwright.impl.PipeTransport`
- **.NET:** https://github.com/microsoft/playwright-dotnet
  - Connection: `src/Playwright/Transport/Connection.cs`

**Protocol Schema:**
- Protocol YAML: https://github.com/microsoft/playwright/blob/main/packages/protocol/src/protocol.yml
- Protocol Docs: https://playwright.dev/docs/api

**Playwright Architecture:**
- Blog: https://playwright.dev/docs/library#key-differences
- Protocol: https://github.com/microsoft/playwright/tree/main/packages/protocol

---

## Driver Distribution Strategy

**Date Added:** 2025-11-05 (Post-Research Update)

Based on comprehensive research of playwright-python, playwright-java, and playwright-dotnet, all three official Microsoft Playwright bindings follow the **same driver bundling approach**:

### How Official Bindings Distribute Drivers

**Python (playwright-python):**
- **Strategy:** Pre-bundled driver in wheel packages
- **Implementation:** Custom `PlaywrightBDistWheelCommand` in `setup.py` downloads driver binaries from Azure CDN during package build
- **Result:** Driver embedded in `playwright/driver/` within the installed pip package
- **User Experience:** No separate installation - `pip install playwright` includes everything

**Java (playwright-java):**
- **Strategy:** Bundled in `driver-bundle` JAR module
- **Implementation:** Maven module packages platform-specific driver binaries as JAR resources, extracted to temp directory at runtime
- **Result:** Driver included in Maven/Gradle dependency
- **User Experience:** Adding dependency automatically includes driver

**.NET (playwright-dotnet):**
- **Strategy:** Bundled in NuGet package
- **Implementation:** Platform-specific Node.js binaries and driver package embedded via `.csproj` Content directives
- **Result:** Driver included in `.playwright/` directory within package
- **User Experience:** NuGet package installation includes driver

### Common Pattern: Bundle, Don't Download

**Key Insight:** All three official bindings bundle the Playwright driver directly in their distribution packages. There is **no separate download step** for end users.

### Decision for Rust Implementation

**Chosen Approach:** Follow the official bindings pattern with Rust-specific adaptations.

**Implementation Strategy:**

1. **Build-Time Download** (Preferred for Cargo)
   - Create `build.rs` in `playwright-core` crate
   - Download Playwright driver from Azure CDN during `cargo build`
   - Extract to `drivers/` directory (gitignored)
   - Embed driver location in compiled binary via `include_str!` or `env!` macros

2. **Runtime Extraction** (If needed)
   - For platform-specific binaries, detect platform at runtime
   - Extract appropriate Node.js binary and driver package
   - Cache in known location (similar to `.playwright/` in .NET)

3. **Platform Support**
   - macOS (x86_64, ARM64)
   - Linux (x86_64, ARM64)
   - Windows (x86_64) - future support

**Download Source:** Azure CDN (`https://playwright.azureedge.net/builds/driver/`)

**Driver Version Strategy:**
- Pin to specific Playwright version in `Cargo.toml` or `build.rs`
- Start with latest stable (e.g., `1.56.0`)
- Document version compatibility

**Fallback Strategy:**
- Primary: Use bundled/downloaded driver in `drivers/`
- Fallback: Check `PLAYWRIGHT_DRIVER_PATH` environment variable
- Fallback: Check for globally installed Playwright via npm (for development)

**Trade-offs Accepted:**
- **Build-time network access** - Acceptable, common in Rust ecosystem (similar to `cc` crate downloading compilers)
- **Increased package size** - Acceptable, ~50MB for driver (similar to other bindings)
- **Platform detection complexity** - Acceptable, manageable with `std::env::consts::OS` and `ARCH`

### Alignment with Official Bindings

This approach **matches the architecture of all three official bindings**:
- ✅ Driver bundled with package
- ✅ No separate installation step for users
- ✅ Platform-specific binary selection
- ✅ User experience: `cargo add playwright` → everything works

### References

**Driver Download Implementations:**
- **Python:** https://github.com/microsoft/playwright-python/blob/main/setup.py (`PlaywrightBDistWheelCommand`)
- **Java:** https://github.com/microsoft/playwright-java/tree/main/driver-bundle (`driver-bundle` module)
- **.NET:** https://github.com/microsoft/playwright-dotnet/blob/main/src/Playwright/Playwright.csproj (`Content` directives)

**Azure CDN:**
- Base URL: `https://playwright.azureedge.net/builds/driver/`
- Path format: `playwright-{version}-{platform}.zip`

---

## Notes

**Open Questions:**
- Should we auto-generate Rust types from `protocol.yml`? (Future consideration, manual for now)

**Future Considerations:**
- Code generation from protocol.yml (like Java/C# implementations)
- Playwright trace viewer integration
- Playwright inspector integration
- Support for custom browser builds

---

**Author:** Paul Adamson

**Reviewers:** N/A (solo developer, community review via GitHub)

**Last Updated:** 2025-11-05
