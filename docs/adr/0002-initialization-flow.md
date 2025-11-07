# ADR 0002: Initialization Flow - Root Object and Synchronous Initialize

**Status:** Accepted | ✅ Implementation Complete

**Date:** 2025-11-06

**Related Documents:**
- [ADR 0001: Protocol Architecture](./0001-protocol-architecture.md)
- [Phase 1 Implementation Plan](../implementation-plans/phase1-protocol-foundation.md)
- [Initialization Flow Research](../technical/initialization-flow-research.md)
- Python: https://github.com/microsoft/playwright-python/blob/main/playwright/_impl/_connection.py
- Java: https://github.com/microsoft/playwright-java
- .NET: https://github.com/microsoft/playwright-dotnet

---

## Context and Problem Statement

After launching the Playwright server and establishing a connection, we need to initialize the protocol and obtain the root `Playwright` object with access to browser types. The question is: **How should playwright-rust initialize the connection and create the root Playwright object?**

Key challenges:
1. How to wait for the Playwright object to be created by the server?
2. How to handle the timing of `__create__` messages?
3. How to structure the initialization API?
4. How to manage object lifetimes?

## Decision Drivers

- **API Consistency:** Match official bindings (Python, Java, .NET)
- **Simplicity:** Avoid complex waiting/polling mechanisms
- **Type Safety:** Leverage Rust's type system
- **Reliability:** Proven patterns from official implementations
- **Testability:** Easy to test with real and mock servers

## Considered Options

### Option 1: Polling for Playwright Object

**Approach:**
```rust
// Launch server and connection
let connection = create_connection().await?;
spawn_message_loop(&connection);

// Poll registry until Playwright object appears
let timeout = Duration::from_secs(30);
let start = Instant::now();

loop {
    if let Ok(obj) = connection.get_object("playwright") {
        return Ok(obj);
    }
    if start.elapsed() > timeout {
        return Err(Error::Timeout);
    }
    sleep(Duration::from_millis(50)).await;
}
```

**Pros:**
- Simple to understand
- No special protocol messages

**Cons:**
- ❌ None of the official bindings use this pattern
- ❌ Wastes CPU cycles polling
- ❌ Arbitrary poll interval
- ❌ Race conditions possible
- ❌ No way to know when initialization is truly complete

### Option 2: Callback/Channel for Object Creation

**Approach:**
```rust
let (tx, rx) = oneshot::channel();

// Register callback for when Playwright object is created
connection.on_object_created("playwright", move |obj| {
    tx.send(obj);
});

// Wait for callback
let playwright = rx.await?;
```

**Pros:**
- Event-driven (no polling)
- Clear completion signal

**Cons:**
- ❌ Only Python uses this pattern (for sync API bridge)
- ❌ Requires callback registration infrastructure
- ❌ Timing issues if object created before callback registered
- ❌ More complex connection API

### Option 3: Root Object with Synchronous Initialize (Official Pattern)

**Approach:**
```rust
// Create temporary Root object
let root = Root::new(connection);

// Send initialize message (BLOCKS until response)
let response = root.initialize().await?;

// Extract Playwright GUID
let playwright_guid = response["playwright"]["guid"].as_str()?;

// Look up from registry (guaranteed to exist)
let playwright = connection.get_object(playwright_guid)?;
```

**Pros:**
- ✅ Matches ALL THREE official bindings
- ✅ No polling - synchronous request/response
- ✅ Server guarantees all objects exist before responding
- ✅ Simple and reliable
- ✅ Type-safe message sending via Channel
- ✅ Clean separation of concerns

**Cons:**
- Requires temporary Root object (minor complexity)

## Decision Outcome

**Chosen Option:** Option 3 - Root Object with Synchronous Initialize

### Rationale

This is the **only pattern used by all three official Microsoft Playwright bindings**:

**Python (`RootChannelOwner`):**
```python
class RootChannelOwner(ChannelOwner):
    def __init__(self, connection: "Connection") -> None:
        super().__init__(connection, "Root", "", {})  # Empty GUID

    async def initialize(self) -> "Playwright":
        return from_channel(
            await self._channel.send(
                "initialize",
                {"sdkLanguage": "python"},
            )
        )
```

**Java (`Root` inner class):**
```java
class Root extends ChannelOwner {
    PlaywrightImpl initialize() {
        JsonObject params = new JsonObject();
        params.addProperty("sdkLanguage", "java");

        JsonElement result = sendMessage("initialize", params, NO_TIMEOUT);

        String guid = result.getAsJsonObject()
            .getAsJsonObject("playwright")
            .get("guid")
            .getAsString();

        return connection.getExistingObject(guid);
    }
}
```

**.NET:**
```csharp
internal async Task<PlaywrightImpl> InitializePlaywrightAsync() {
    var args = new Dictionary<string, object?> {
        ["sdkLanguage"] = "csharp"
    };

    var jsonElement = await SendMessageToServerAsync(
        null,  // root object (no GUID)
        "initialize",
        args
    );

    return jsonElement.GetObject<PlaywrightImpl>("playwright", this)!;
}
```

### Key Insight: Synchronous Initialize

The `initialize` message is **synchronous and blocking**. The protocol flow is:

```
Client                           Server
  |                                |
  |---- initialize(sdkLanguage) -->|
  |                                |
  |                      (creates BrowserType objects)
  |                      (sends __create__ messages)
  |                                |
  |<--- __create__ BrowserType ----|
  |<--- __create__ BrowserType ----|
  |<--- __create__ BrowserType ----|
  |                                |
  |                      (creates Playwright object)
  |<--- __create__ Playwright -----|
  |                                |
  |<--- response: {playwright: {...}} ---|
```

**By the time the response arrives, ALL objects already exist in the registry!**

This means:
- ✅ No explicit waiting for `__create__` messages
- ✅ No polling loops
- ✅ No race conditions
- ✅ Server guarantees order

## Implementation

### 1. Root Object (`protocol/root.rs`)

```rust
pub struct Root {
    base: ChannelOwnerImpl,
}

impl Root {
    pub fn new(connection: Arc<dyn ConnectionLike>) -> Self {
        Self {
            base: ChannelOwnerImpl::new(
                ParentOrConnection::Connection(connection),
                "Root".to_string(),
                "".to_string(), // Empty GUID - not registered
                Value::Null,
            ),
        }
    }

    pub async fn initialize(&self) -> Result<Value> {
        self.channel()
            .send(
                "initialize",
                serde_json::json!({
                    "sdkLanguage": "rust"
                }),
            )
            .await
    }
}
```

**Key characteristics:**
- Empty GUID (`""`)
- Not registered in object registry
- Temporary - discarded after initialization
- Type-safe message sending via Channel

### 2. Connection Helper (`connection.rs`)

```rust
impl Connection {
    pub async fn initialize_playwright(
        &self
    ) -> Result<Arc<Playwright>> {
        // Create root object
        let root = Root::new(Arc::clone(self));

        // Send initialize (blocks until response)
        let response = root.initialize().await?;

        // Extract Playwright GUID
        let playwright_guid = response["playwright"]["guid"]
            .as_str()
            .ok_or_else(|| Error::ProtocolError(
                "initialize response missing playwright.guid".into()
            ))?;

        // Get from registry (guaranteed to exist)
        let playwright_obj = self.get_object(playwright_guid)?;

        // Downcast to Playwright type
        let playwright = playwright_obj
            .as_any()
            .downcast_ref::<Playwright>()
            .ok_or_else(|| Error::ProtocolError(
                "Object is not Playwright".into()
            ))?;

        // Clone to return owned Playwright
        Ok(Arc::new(playwright.clone()))
    }
}
```

### 3. High-Level API (`playwright` crate)

```rust
pub struct Playwright {
    inner: Arc<CorePlaywright>,
    connection: Arc<RealConnection>,
}

impl Playwright {
    pub async fn launch() -> Result<Self> {
        // 1. Launch server
        let mut server = PlaywrightServer::launch().await?;

        // 2. Create transport
        let stdin = server.process.stdin.take().unwrap();
        let stdout = server.process.stdout.take().unwrap();
        let (transport, message_rx) = PipeTransport::new(stdin, stdout);

        // 3. Create connection
        let connection = Arc::new(Connection::new(transport, message_rx));

        // 4. Spawn message loop
        let conn = Arc::clone(&connection);
        tokio::spawn(async move { conn.run().await });

        // 5. Initialize (synchronous, blocks until complete)
        let inner = connection.initialize_playwright().await?;

        Ok(Self { inner, connection })
    }

    pub fn chromium(&self) -> &BrowserType {
        self.inner.chromium()
    }

    pub fn firefox(&self) -> &BrowserType {
        self.inner.firefox()
    }

    pub fn webkit(&self) -> &BrowserType {
        self.inner.webkit()
    }
}
```

## Consequences

### Positive

- ✅ **API Consistency:** Matches all official bindings exactly
- ✅ **Simplicity:** No polling, no complex waiting
- ✅ **Reliability:** Proven pattern used by millions
- ✅ **Type Safety:** Strong typing via Root object and Channel
- ✅ **Testability:** Easy to test with real and mock servers
- ✅ **No Race Conditions:** Synchronous guarantees order
- ✅ **Clear Lifetime:** Root is temporary, Playwright owns connection

### Negative

- ⚠️ **Async Drop Challenge:** Server cleanup requires async (addressed below)
- ⚠️ **Arc Handling:** Need to handle `Arc<Playwright>` return type carefully

### Neutral

- Requires temporary Root object (minor additional code)
- Follows Microsoft's architecture (good for compatibility, may differ from pure Rust idioms)

## Implementation Notes

### Arc<Playwright> Handling

**Decision:** Clone the Playwright struct when returning from `initialize_playwright()`.

This is straightforward and works well. Future optimization (if needed) deferred to Phase 2.

### Initialization Timeout

No explicit timeout implemented in Phase 1. The connection layer will detect server failures via broken pipe errors. Explicit timeouts can be added in Phase 2 if needed.

### Cleanup on Drop

Phase 1 does not implement `Drop` for Playwright. Server process is killed when parent process exits. Graceful cleanup and explicit `close()` methods will be addressed in Phase 2 alongside Browser/Page lifecycle management.

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_root_object_creation() {
    let root = Root::new(mock_connection());
    assert_eq!(root.guid(), "");
    assert_eq!(root.type_name(), "Root");
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_initialize_with_real_server() {
    let server = PlaywrightServer::launch().await.unwrap();
    let (transport, rx) = create_transport(server);
    let connection = Arc::new(Connection::new(transport, rx));

    tokio::spawn(async move { connection.run().await });

    let playwright = connection.initialize_playwright().await.unwrap();

    assert_eq!(playwright.chromium().name(), "chromium");
    assert_eq!(playwright.firefox().name(), "firefox");
    assert_eq!(playwright.webkit().name(), "webkit");
}
```

## References

### Research Document

See [Initialization Flow Research](../technical/initialization-flow-research.md) for detailed analysis of all three official bindings.

### Source Code References

**Python:**
- `/playwright/_impl/_connection.py` - RootChannelOwner class
- `/playwright/_impl/_playwright.py` - Playwright initialization

**Java:**
- `/playwright/src/main/java/com/microsoft/playwright/impl/Connection.java` - Root inner class
- `/playwright/src/main/java/com/microsoft/playwright/impl/PlaywrightImpl.java`

**.NET:**
- `/src/Playwright/Transport/Connection.cs` - InitializePlaywrightAsync method
- `/src/Playwright/Playwright.cs` - CreateAsync method

## Related Decisions

- [ADR 0001: Protocol Architecture](./0001-protocol-architecture.md) - Base protocol design
- Phase 1 Slice 5 implementation follows this ADR

---

**Last Updated:** 2025-11-06
**Decision Makers:** Paul Adamson
**Status:** Accepted - ✅ Implementation Complete
