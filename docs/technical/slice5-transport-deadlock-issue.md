# Slice 5 Transport Deadlock Issue

**Date:** 2025-11-06
**Status:** Debugging in Progress
**Issue:** Transport locking causes deadlock during initialization

## Problem Summary

The `Connection::initialize_playwright()` implementation times out after 30 seconds because of a **deadlock in the transport locking strategy**.

## Root Cause

### Current Architecture

```rust
// In Connection struct
transport: Arc<Mutex<PipeTransport<W, R>>>

// In Connection::run() - spawns transport loop
let transport_handle = tokio::spawn(async move {
    let mut transport = transport.lock().await;  // LOCKS HERE
    if let Err(e) = transport.run().await {      // KEEPS LOCK FOREVER
        tracing::error!("Transport error: {}", e);
    }
});

// In Connection::send_message() - tries to send
self.transport.lock().await.send(request_value).await?;  // DEADLOCK!
```

### The Deadlock

1. `Connection::run()` spawns a task that **locks the transport mutex**
2. The lock is held for the **entire duration** of `transport.run()` (which runs forever)
3. `Connection::send_message()` tries to **acquire the same lock** to send messages
4. **Deadlock**: send_message waits forever for a lock that will never be released

## Debug Trace

```
[DEBUG] Root object registered, sending initialize message
[DEBUG] Sending message: id=0, guid='', method='initialize'
[DEBUG] Request JSON: {"guid":"","id":0,"method":"initialize","params":{"sdkLanguage":"rust"}}
<hangs here - never sees "Message sent successfully">
<timeout after 30 seconds>
```

The `send()` call in line 361 of connection.rs never completes because it's waiting for the transport lock.

## Solution Options

### Option 1: Split Send/Receive (Recommended)

Separate the transport into send and receive channels:

```rust
pub struct Connection<W, R> {
    // Send side - needs mutex for concurrent sends
    transport_send: Arc<Mutex<W>>,
    // Receive side - exclusive access for run loop
    transport_recv: Option<R>,  // Take out when starting run()
    // ... rest of fields
}

impl Connection {
    pub async fn run(&self) {
        // Take ownership of receive side
        let mut recv = self.transport_recv.lock().await.take()
            .expect("run() can only be called once");

        // Read loop doesn't need to lock anything
        loop {
            let message = read_message(&mut recv).await?;
            self.dispatch(message).await?;
        }
    }

    pub async fn send_message(&self, ...) -> Result<Value> {
        // Only locks send side
        self.transport_send.lock().await.write_message(...).await?;
    }
}
```

### Option 2: Use Channels for Sending

Instead of locking transport for sends, use a channel:

```rust
pub struct Connection<W, R> {
    send_tx: mpsc::UnboundedSender<Value>,  // Send messages here
    // Transport owns stdin, listens on channel
}
```

### Option 3: RwLock (Not Recommended)

Use `RwLock` instead of `Mutex` - but this doesn't really help since we need exclusive write access anyway.

## Recommended Fix (Option 1)

### Step 1: Refactor PipeTransport

Split stdin/stdout access:

```rust
// Keep PipeTransport as-is for construction
pub struct PipeTransport<W, R> { ... }

impl<W, R> PipeTransport<W, R> {
    pub fn split(self) -> (W, R) {
        (self.stdin, self.stdout)
    }
}
```

### Step 2: Refactor Connection

```rust
pub struct Connection<W, R> {
    last_id: AtomicU32,
    callbacks: Arc<Mutex<HashMap<u32, oneshot::Sender<Result<Value>>>>>,

    // NEW: Split transport
    stdin: Arc<Mutex<W>>,              // For sending
    message_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<Value>>>>,  // For receiving

    objects: Arc<Mutex<HashMap<String, Arc<dyn ChannelOwner>>>>,
}

impl<W, R> Connection<W, R> {
    pub fn new(transport: PipeTransport<W, R>, message_rx: mpsc::UnboundedReceiver<Value>) -> Self {
        let (stdin, _stdout) = transport.split();  // stdout handled by transport loop

        Self {
            stdin: Arc::new(Mutex::new(stdin)),
            message_rx: Arc::new(Mutex::new(Some(message_rx))),
            // ...
        }
    }

    pub async fn send_message(&self, ...) -> Result<Value> {
        // Serialize message
        let message_bytes = serialize_with_length_prefix(&request)?;

        // Lock ONLY for the write operation
        self.stdin.lock().await.write_all(&message_bytes).await?;

        // Rest of code unchanged
    }

    pub async fn run(&self) {
        // Spawn transport loop OUTSIDE of any locks
        let transport_task = /* spawn reading from stdout */;

        // Message dispatch loop (no locking issues)
        let mut message_rx = self.message_rx.lock().await.take().unwrap();
        while let Some(message) = message_rx.recv().await {
            self.dispatch(message).await?;
        }
    }
}
```

Actually, re-examining the code, the transport already creates a message channel. So the issue is simpler - we just need to not lock the transport during run(). Let me trace through the actual PipeTransport design...

Looking at `PipeTransport::new()`, it returns `(Self, mpsc::UnboundedReceiver<Value>)`. So the transport sends messages to a channel. The `run()` method reads from stdout and sends to that channel. The issue is that `run()` needs exclusive access to stdout, but we're locking the entire PipeTransport struct.

### Simpler Fix

The PipeTransport already has the right design! The issue is just how Connection uses it.

**Current (Wrong):**
```rust
// Connection wraps entire transport in mutex
transport: Arc<Mutex<PipeTransport<W, R>>>

// run() locks transport forever
let mut transport = transport.lock().await;
transport.run().await;  // Never unlocks

// send_message() can't acquire lock
self.transport.lock().await.send(...).await;  // Deadlock!
```

**Fixed:**
```rust
// Split stdin from the transport for independent locking
pub struct Connection<W, R> {
    stdin: Arc<Mutex<W>>,  // Only stdin needs mutex (for concurrent sends)
    // stdout is owned by transport.run() - no sharing needed
}
```

The fix is to extract stdin from PipeTransport and put it in Connection, while letting run() take ownership of the transport (or at least stdout).

## Next Steps

1. Refactor `PipeTransport` to allow extracting stdin separately
2. Update `Connection` to only mutex-wrap stdin
3. Let `Connection::run()` take ownership of receive side (or use the already-provided message_rx channel)
4. Test that sends and receives work concurrently
5. Re-run initialization test

## Files to Modify

- `crates/playwright-core/src/transport.rs` - Add method to split or access stdin separately
- `crates/playwright-core/src/connection.rs` - Refactor locking strategy
- Tests remain the same - should pass after fix

## Learnings

- **Mutexes and long-running tasks don't mix** - If a task needs exclusive access for its entire lifetime, it should own the resource, not borrow it through a mutex
- **Send and receive are independent** - stdin and stdout don't conflict, so they shouldn't share a mutex
- **The transport already has the right design** - It creates a channel for receives. We just need to use it correctly.

---

**Status:** Issue identified, solution designed, ready to implement in next session
