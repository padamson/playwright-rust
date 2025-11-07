# Playwright Initialization Flow - Research and Implementation

**Date:** 2025-11-06
**Phase:** Phase 1 - Slice 5 (Entry Point)
**Status:** Implementation in Progress

## Overview

This document captures research into how official Microsoft Playwright language bindings implement the initialization flow, and how these findings inform the Rust implementation.

## Research Scope

Analyzed initialization patterns in three official bindings:
- **playwright-python** (Python)
- **playwright-java** (Java)
- **playwright-dotnet** (C#/.NET)

## Key Findings

### 1. Consistent Initialization Sequence

All three bindings follow the same pattern:

```
1. Launch server process (`playwright run-driver`)
2. Create transport (stdio pipes, length-prefixed JSON)
3. Create connection (manages protocol)
4. Send `initialize` message with SDK language
5. Receive Playwright object reference in response
6. Look up object from connection registry
7. Return Playwright instance
```

### 2. The "Root Object" Pattern

All bindings use a temporary **Root object** for initialization:

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

**Key characteristics:**
- Empty GUID (`""`)
- Not registered in object map
- Temporary - discarded after initialization
- Only purpose: send `initialize` message

### 3. Synchronous Initialization is Critical

The `initialize` message is **synchronous and blocking**:

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
  |                      (sends __create__ message)
  |                                |
  |<--- __create__ Playwright -----|
  |                                |
  |<--- response: {playwright: {guid: "..."}} ---|
  |                                |
```

**Critical insight:** By the time the response arrives, ALL objects already exist in the registry!

**This means:**
- ✅ No explicit waiting for `__create__` messages needed
- ✅ Just send `initialize`, await response, lookup object
- ✅ Server guarantees all children exist before responding

### 4. Response Format

Server response to `initialize` message:

```json
{
  "playwright": {
    "guid": "playwright"
  }
}
```

Extract the GUID and look it up from the connection's object registry.

### 5. Object Creation Order

Server creates objects in dependency order:

1. **BrowserType objects** (no dependencies)
   - `browserType@chromium`
   - `browserType@firefox`
   - `browserType@webkit`

2. **Playwright object** (references BrowserTypes)
   - GUID: `playwright`
   - Initializer contains GUID references to browser types:
   ```json
   {
     "chromium": { "guid": "browserType@chromium" },
     "firefox": { "guid": "browserType@firefox" },
     "webkit": { "guid": "browserType@webkit" }
   }
   ```

### 6. Connection Object Registry

All bindings maintain `HashMap<String, Object>` for GUID → object lookup:

- **Python:** `self._objects: Dict[str, ChannelOwner]`
- **Java:** `objects = new HashMap<String, ChannelOwner>()`
- **.NET:** `_objects = new ConcurrentDictionary<string, ChannelOwnerBase>()`

When `__create__` message arrives:
1. Parse type, GUID, initializer
2. Call object factory
3. Store in `objects[guid] = new_object`
4. Register with parent

### 7. No Explicit Waiting Mechanism

**Important:** None of the bindings poll or wait for `__create__` messages!

They rely on:
- `initialize` being synchronous
- Server creating all objects before responding
- Objects being in registry when response arrives

## Rust Implementation Plan

### Architecture

Based on the research, the Rust implementation follows this structure:

```
crates/
├── playwright-core/          # Protocol implementation (internal)
│   ├── protocol/
│   │   ├── root.rs          # Root object with initialize()
│   │   ├── playwright.rs    # Playwright object
│   │   └── browser_type.rs  # BrowserType objects
│   ├── connection.rs        # Connection with initialize_playwright()
│   └── server.rs            # Server management
└── playwright/              # Public API
    └── lib.rs              # High-level Playwright::launch()
```

### Implementation Steps (TDD)

1. **Create Root Object** (`protocol/root.rs`) ✅
   - Empty GUID
   - `initialize()` method sends message
   - Returns response with Playwright GUID

2. **Add Connection::initialize_playwright()** (`connection.rs`)
   - Create Root object
   - Call `root.initialize().await`
   - Extract Playwright GUID from response
   - Look up from registry
   - Return `Arc<Playwright>`

3. **Create Public API** (`playwright` crate)
   - `Playwright::launch()` orchestrates:
     - Launch server
     - Create transport
     - Create connection
     - Spawn message loop
     - Call `connection.initialize_playwright()`
     - Return wrapped Playwright instance

4. **Write Integration Tests**
   - Test with real server
   - Verify all browser types accessible
   - Test error handling

### Code Example (Target API)

```rust
use playwright::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;

    println!("Chromium: {}", playwright.chromium().name());
    println!("Firefox: {}", playwright.firefox().name());
    println!("WebKit: {}", playwright.webkit().name());

    Ok(())
}
```

## Key Design Decisions

### Decision 1: Follow Root Object Pattern

**Options:**
- A) Direct initialization without Root object
- B) Root object pattern (matches official bindings)

**Choice:** B - Root object pattern

**Rationale:**
- ✅ Matches all three official bindings
- ✅ Clean separation of concerns
- ✅ Type-safe message sending
- ✅ Proven architecture

### Decision 2: Synchronous Initialize (No Polling)

**Options:**
- A) Poll registry for Playwright object
- B) Trust synchronous `initialize` response

**Choice:** B - Synchronous trust

**Rationale:**
- ✅ Matches official bindings (none poll)
- ✅ Server guarantees objects exist
- ✅ Simpler implementation
- ✅ No race conditions

### Decision 3: Connection Lifetime Management

**Options:**
- A) Connection owned by Playwright object
- B) Separate Connection and Playwright lifetimes
- C) Connection as global/singleton

**Choice:** A - Playwright owns Connection

**Rationale:**
- ✅ Clear ownership semantics
- ✅ RAII cleanup when dropped
- ✅ Matches Python's context manager pattern
- ✅ No global state

### Decision 4: Server Process Lifetime

**Options:**
- A) Server killed when Connection drops
- B) Server outlives Connection
- C) Explicit `close()` method required

**Choice:** A - Server killed on drop

**Rationale:**
- ✅ RAII pattern (automatic cleanup)
- ✅ No orphaned processes
- ✅ Matches Python's context manager
- ⚠️ Requires careful Drop implementation (async cleanup challenge)

## Challenges and Solutions

### Challenge 1: Async Drop

**Problem:** Rust's `Drop` trait is synchronous, but server shutdown requires async.

**Solutions:**
- Option A: Spawn background task in Drop
- Option B: Require explicit `.close().await`
- Option C: Use blocking cleanup in Drop

**Recommendation:** Start with Option C (blocking), add Option B for graceful shutdown.

### Challenge 2: Object Downcasting

**Problem:** Connection stores `Arc<dyn ChannelOwner>`, need `Arc<Playwright>`.

**Solutions:**
- Option A: Clone the Playwright struct (requires Clone trait)
- Option B: Return `&Playwright` reference (lifetime issues)
- Option C: Store typed Arc in registry (requires generic registry)

**Recommendation:** Evaluate during implementation.

### Challenge 3: Server Not Responding

**Problem:** What if `initialize` never returns?

**Solution:** Add timeout to `channel.send()`:
```rust
tokio::time::timeout(
    Duration::from_secs(30),
    root.initialize()
).await??
```

## References

### Source Code

**Python:**
- `/playwright/_impl/_connection.py` - RootChannelOwner, Connection
- `/playwright/_impl/_transport.py` - PipeTransport
- `/playwright/sync_api/_context_manager.py` - Entry point

**Java:**
- `/playwright/src/main/java/com/microsoft/playwright/impl/Connection.java`
- `/playwright/src/main/java/com/microsoft/playwright/impl/PlaywrightImpl.java`

**.NET:**
- `/src/Playwright/Transport/Connection.cs`
- `/src/Playwright/Transport/StdIOTransport.cs`
- `/src/Playwright/Playwright.cs`

### Protocol Documentation

- Playwright Protocol: `microsoft/playwright/packages/protocol/src/protocol.yml`
- API Docs: https://playwright.dev/docs/api

---

**Last Updated:** 2025-11-06
**Contributors:** Paul Adamson, Claude Code
**Related ADR:** ADR-0001 Protocol Architecture, ADR-0002 Initialization Flow
