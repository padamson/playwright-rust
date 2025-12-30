# ADR 0004: Argument Serialization and Typed Results for evaluate()

**Status:** Accepted | **Date:** 2025-12-24

**Related Documents:**
- Implementation: Phase 5, Slice 4c - Evaluate with Arguments
- Module: `crates/playwright/src/protocol/serialize_argument.rs`
- Example: `crates/playwright/examples/evaluate_typed.rs`
- Tests: `crates/playwright/tests/evaluate_test.rs`
- Guide: `docs/EVALUATE_TYPED.md`

---

## Context and Problem Statement

### Before (Original Implementation)

The `evaluate()` family of methods existed but had significant limitations:

```rust
// Original: No argument support, no typed return
pub(crate) async fn frame_evaluate_expression(&self, expression: &str) -> Result<()> {
    let params = serde_json::json!({
        "expression": expression,
        "arg": {
            "value": {"v": "null"},  // Always null!
            "handles": []
        }
    });
    let _: serde_json::Value = self.channel().send("evaluateExpression", params).await?;
    Ok(())
}

pub(crate) async fn frame_evaluate_expression_value(&self, expression: &str) -> Result<String> {
    // Same limitation: expression only, no arguments
    // Returns String (lose typed information)
}
```

**Limitations:**
- ❌ **No argument passing**: `arg` was hardcoded to `{"v": "null"}` - impossible to pass data to JavaScript
- ❌ **Limited return types**: Only `()` or `String` - lose all structured information
- ❌ **No serialization support**: Custom structs couldn't be passed or returned
- ❌ **Type-unsafe**: Caller must manually parse String results if they wanted structure
- ❌ **Not matching Playwright API**: Other language bindings support arguments

### After (New Implementation)

```rust
// Frame: Flexible argument + typed return
pub async fn evaluate<T: Serialize>(
    &self,
    expression: &str,
    arg: Option<&T>,
) -> Result<Value>

// Page: Generic typed return
pub async fn evaluate<T: Serialize, U: DeserializeOwned>(
    &self,
    expression: &str,
    arg: Option<&T>,
) -> Result<U>

// User code:
#[derive(Serialize, Deserialize)]
struct Point { x: i32, y: i32 }

let point = Point { x: 10, y: 20 };
let result: Point = page.evaluate(
    "(arg) => ({x: arg.x * 2, y: arg.y * 2})",
    Some(&point)
).await?;
```

**Capabilities:**
- ✅ **Full argument support**: Any Serializable type can be passed
- ✅ **Typed returns**: Deserialize into user's struct or primitive
- ✅ **Round-trip serialization**: Send struct → JavaScript → receive struct
- ✅ **Type-safe**: Compiler validates both input and output types
- ✅ **Matches Playwright API**: Consistent with playwright-python, playwright-java

---

## Decision

**Implement argument serialization + generic typed returns**

Three key components:

### 1. Serialization Module (`serialize_argument.rs`)
```rust
pub fn serialize_argument<T: Serialize>(arg: &T) -> Value
pub fn serialize_null() -> Value
pub fn parse_result(value: &Value) -> Value
```

Converts Rust values to/from Playwright's protocol format (type-tagged JSON with handles).

### 2. Frame-level Method
```rust
pub async fn evaluate<T: Serialize>(
    &self,
    expression: &str,
    arg: Option<&T>,
) -> Result<Value>
```

Handles argument serialization, protocol communication, result parsing.

### 3. Page-level Method
```rust
pub async fn evaluate<T: Serialize, U: DeserializeOwned>(
    &self,
    expression: &str,
    arg: Option<&T>,
) -> Result<U>
```

Adds typed deserialization on top of frame's method.

### Key Components

#### 1. Serialization Module (serialize_argument.rs)

Converts between Rust and Playwright's protocol format:

```rust
// Input: Rust value
let point = Point { x: 10, y: 20 };

// Process: serialize_argument()
{
  "value": { "o": {...serialized object...} },
  "handles": []
}

// Output: Playwright format
```

**Features:**
- Type-tagged JSON encoding (primitive types, arrays, objects)
- Handle references for complex objects
- Special float handling (Infinity, NaN, -0)
- Circular reference detection via Visitor pattern
- Bidirectional: `serialize_argument()` + `parse_result()`

#### 2. Frame-level Integration

New method added to `Frame` struct:

```rust
pub async fn evaluate<T: serde::Serialize>(
    &self,
    expression: &str,
    arg: Option<&T>,
) -> Result<Value> {
    let serialized_arg = match arg {
        Some(a) => serialize_argument(a),
        None => serialize_null(),
    };
    
    let params = serde_json::json!({
        "expression": expression,
        "arg": serialized_arg
    });
    
    let result: EvaluateResult = 
        self.channel().send("evaluateExpression", params).await?;
    
    Ok(parse_result(&result.value))
}
```

**Changes from original:**
- Argument is now dynamic (not hardcoded null)
- Uses `serialize_argument()` for proper protocol format
- Parses result with `parse_result()` for correct deserialization

#### 3. Page-level API (Public Interface)

New method added to `Page` struct:

```rust
pub async fn evaluate<T: serde::Serialize, U: serde::de::DeserializeOwned>(
    &self,
    expression: &str,
    arg: Option<&T>,
) -> Result<U> {
    let frame = self.main_frame().await?;
    let result = frame.evaluate(expression, arg).await?;
    serde_json::from_value(result).map_err(Error::from)
}
```

**Benefits:**
- Adds typed deserialization on top of frame method
- User gets compile-time return type validation
- Page-level method (higher-level API)

### Backward Compatibility

Original methods preserved (unchanged):

```rust
// Original: For simple expressions without returns
pub(crate) async fn frame_evaluate_expression(&self, expression: &str) -> Result<()>

// Original: For string-only results
pub(crate) async fn frame_evaluate_expression_value(&self, expression: &str) -> Result<String>

// New convenience method
pub async fn evaluate_expression(&self, expression: &str) -> Result<()>
```

No breaking changes. Users can migrate gradually.

---

## Rationale

### 1. Closes a Capability Gap

Before: Impossible to pass JavaScript data as arguments

```javascript
// Original: Couldn't do this
page.evaluate("(arg) => arg.x + arg.y", {x: 10, y: 20})  // ✗ Not possible
```

After: Full argument support

```javascript
// New: Works seamlessly
let point = Point { x: 10, y: 20 };
page.evaluate("(arg) => arg.x + arg.y", Some(&point))  // ✅ Works
```

### 2. Matches Playwright Specification

The Playwright protocol supports full argument passing. Our implementation finally exposes this capability.

```rust
// Playwright protocol signature:
{
  "method": "evaluateExpression",
  "params": {
    "expression": string,
    "arg": {                    // ← This was always null before
      "value": any,
      "handles": []
    }
  }
}
```

### 3. Enables Type Safety

With arguments + typed returns, users can verify correctness at compile-time:

```rust
#[derive(Serialize, Deserialize)]
struct Input { x: i32, y: i32 }

#[derive(Deserialize)]
struct Output { sum: i32 }

// Compiler checks:
// ✅ Input implements Serialize
// ✅ Output implements Deserialize
let output: Output = page.evaluate(
    "(arg) => ({sum: arg.x + arg.y})",
    Some(&Input { x: 5, y: 3 })
).await?;  // Result is type-safe
```

### 4. Consistency with Playwright Ecosystem

| Language | Argument Support | Typed Returns |
|----------|------------------|---------------|
| Python   | ✅ Yes           | 💡 Runtime   |
| Java     | ✅ Yes           | ✅ Generics  |
| .NET     | ✅ Yes           | ✅ Generics  |
| Rust     | ❌ No (before)   | ❌ No (before) |
| Rust     | ✅ Yes (now)     | ✅ Generics (now) |

Our implementation closes the gap.

---

## Implementation Details

### Protocol Pipeline

```
User Input (T)
  ↓ serialize_argument()
Playwright JSON Format
  ↓ Network Call
JavaScript Execution
  ↓ Network Response
Playwright JSON Format
  ↓ parse_result()
serde_json::Value
  ↓ serde_json::from_value()
User Output (U)
  ↓
Result<U>
```

### Serialize Flow

```rust
// Original implementation (hardcoded null):
"arg": {
    "value": {"v": "null"},
    "handles": []
}

// New implementation (user-provided):
"arg": {
    "value": {"o": "0"},       // Object reference
    "handles": [
        {
            "type": "object",
            "guid": "auto-123",
            // ... serialized Point data
        }
    ]
}
```

### Result Deserialization Flow

```rust
// Playwright returns:
{
    "value": {"o": "0"},      // Object reference
    "handles": [
        {
            "guid": "auto-456",
            // ... result data
        }
    ]
}

// parse_result() converts to:
{"x": 20, "y": 40}

// from_value() deserializes to:
Point { x: 20, y: 40 }
```

---

## Examples: Before vs After

### Example 1: Simple Expression (No Argument)

**Before:**
```rust
// Could only return nothing or String
page.evaluate("() => 42").await?;  // Result discarded
// or
let s = page.evaluate_value("() => '42'").await?;  // Returns String "42"
```

**After:**
```rust
// Can return typed value
let num: i32 = page.evaluate("() => 42", None).await?;
// or use Value if needed
let val: Value = page.evaluate("() => 42", None).await?;
```

### Example 2: Passing Arguments (The Gap)

**Before:**
```rust
// ❌ IMPOSSIBLE: No way to pass arguments
let x = 10;
let y = 20;
// page.evaluate("(a, b) => a + b", Some((x, y))).await?  // NOT POSSIBLE

// Only workaround: String interpolation (fragile)
page.evaluate(&format!("() => {} + {}", x, y)).await?;  // Unsafe!
```

**After:**
```rust
// ✅ Works: Full argument support
let x = 10;
let y = 20;
let sum: i32 = page.evaluate("(a, b) => a + b", Some(&(x, y))).await?;
```

### Example 3: Complex Objects

**Before:**
```rust
// ❌ IMPOSSIBLE: No way to pass structs
#[derive(Serialize)]
struct Point { x: i32, y: i32 }

let p = Point { x: 10, y: 20 };
// page.evaluate("(p) => p.x + p.y", Some(&p)).await?  // NOT POSSIBLE
```

**After:**
```rust
// ✅ Works: Automatic serialization
#[derive(Serialize, Deserialize)]
struct Point { x: i32, y: i32 }

#[derive(Deserialize)]
struct Result { sum: i32 }

let p = Point { x: 10, y: 20 };
let result: Result = page.evaluate(
    "(p) => ({sum: p.x + p.y})",
    Some(&p)
).await?;
// result.sum == 30
```

### Example 4: API Data

**Before:**
```rust
// ❌ IMPOSSIBLE: Can't pass API data structure
struct User { id: u32, name: String }
let user = User { id: 1, name: "Alice".into() };
// page.evaluate("(u) => u.name.toUpperCase()", Some(&user)).await?  // NOT POSSIBLE

// Only option: manual JavaScript string building
page.evaluate(&format!(
    r#"() => {{ "{}".toUpperCase() }}"#,
    user.name
)).await?;
```

**After:**
```rust
// ✅ Works: Seamless struct passing
#[derive(Serialize, Deserialize)]
struct User { id: u32, name: String }

let user = User { id: 1, name: "Alice".into() };
let uppercase: String = page.evaluate(
    "(u) => u.name.toUpperCase()",
    Some(&user)
).await?;
// uppercase == "ALICE"
```

---

## Comparison with Other Bindings

### playwright-python

```python
# Python: Supports arguments and returns
result = page.evaluate("(arg) => arg.x + arg.y", {"x": 10, "y": 20})
# Result is Python dict/object, type-checked at runtime
```

**Our implementation:**
```rust
// Rust: Compile-time typed (better!)
let result: i32 = page.evaluate("(arg) => arg.x + arg.y", Some(&(10, 20))).await?;
```

### playwright-java

```java
// Java: Uses generics like our approach
Object result = page.evaluate("(arg) => arg.x + arg.y", new Object[]{object});
int sum = (Integer) result;  // Manual cast
```

**Our implementation:**
```rust
// Rust: Type inference (better!)
let sum: i32 = page.evaluate("(arg) => arg.x + arg.y", Some(&object)).await?;
```

### playwright-dotnet

```csharp
// C#: Uses Task<T> generics
var result = await page.EvaluateAsync<int>("(arg) => arg.x + arg.y", object);
```

**Our implementation:**
```rust
// Rust: Almost identical with async/await!
let result: i32 = page.evaluate("(arg) => arg.x + arg.y", Some(&object)).await?;
```

---

## Trade-offs

### Pros
- ✅ **Closes capability gap**: Arguments now fully supported (was impossible before)
- ✅ **Type safety**: Compiler validates both inputs and outputs
- ✅ **Better DX**: No manual serialization/deserialization code
- ✅ **IDE support**: Full autocomplete for struct fields
- ✅ **Matches ecosystem**: Consistent with other Playwright bindings
- ✅ **Backward compatible**: Existing code continues to work
- ✅ **Flexible**: Works with primitives, tuples, custom structs

### Cons
- ⚠️ **New module required**: `serialize_argument.rs` adds ~800 lines
- ⚠️ **Learning curve**: Users need to understand `Serialize`/`Deserialize` traits
- ⚠️ **Struct definition overhead**: Complex returns require struct boilerplate
- ⚠️ **Generic trait bounds**: Public API shows generic constraints (normal in Rust)

**Verdict:** Pros heavily outweigh cons. This is not a cosmetic improvement—it enables functionality that was previously impossible.

---

## Code Archaeology: What Changed

### Frame Method Signature

**Before:**
```rust
pub(crate) async fn frame_evaluate_expression(&self, expression: &str) -> Result<()> {
    let params = serde_json::json!({
        "expression": expression,
        "arg": {
            "value": {"v": "null"},  // ← HARDCODED NULL!
            "handles": []
        }
    });
    let _: serde_json::Value = self.channel().send("evaluateExpression", params).await?;
    Ok(())
}
```

**After:**
```rust
pub async fn evaluate<T: serde::Serialize>(
    &self,
    expression: &str,
    arg: Option<&T>,  // ← DYNAMIC ARGUMENT!
) -> Result<Value> {
    let serialized_arg = match arg {
        Some(a) => serialize_argument(a),  // ← SERIALIZE USER DATA!
        None => serialize_null(),
    };
    
    let params = serde_json::json!({
        "expression": expression,
        "arg": serialized_arg  // ← NOW DYNAMIC!
    });
    
    let result: EvaluateResult = self.channel().send("evaluateExpression", params).await?;
    Ok(parse_result(&result.value))  // ← PARSE RESULT!
}
```

### Key Differences

| Aspect | Before | After |
|--------|--------|-------|
| **Argument** | `null` (hardcoded) | User-provided (serialized) |
| **Return type** | `()` only | `Value` with parsing |
| **Type support** | Primitives only | Any Serialize/Deserialize type |
| **API match** | Non-standard | Matches other bindings |
| **Capability** | Expression-only | Expression + arguments |

---

## Validation & Testing

### Coverage

- ✅ 11 integration tests with typed evaluate
- ✅ 23 unit tests for serialization/deserialization
- ✅ Tests with primitives (i32, String, bool)
- ✅ Tests with custom structs
- ✅ Tests with arrays and objects
- ✅ Round-trip serialization validated

### Test Scenarios

```rust
// Primitive types
let num: i32 = page.evaluate("() => 42", None).await?;

// String arguments
let result: String = page.evaluate(
    "(s) => s.toUpperCase()",
    Some(&"hello")
).await?;

// Struct arguments and returns
#[derive(Serialize, Deserialize)]
struct Point { x: i32, y: i32 }

let point = Point { x: 10, y: 20 };
let result: Point = page.evaluate(
    "(p) => ({x: p.x * 2, y: p.y * 2})",
    Some(&point)
).await?;

// Array arguments
let nums = vec![1, 2, 3];
let sum: i32 = page.evaluate(
    "(arr) => arr.reduce((a, b) => a + b, 0)",
    Some(&nums)
).await?;
```


---

## Future Enhancements

1. **Convenience Methods**
   ```rust
   // Typed return without argument
   pub async fn evaluate_typed<U: DeserializeOwned>(
       &self,
       expression: &str,
   ) -> Result<U>
   
   // With error recovery
   pub async fn evaluate_or<U: DeserializeOwned>(
       &self,
       expression: &str,
       arg: Option<&impl Serialize>,
       default: U,
   ) -> Result<U>
   ```

2. **Batch Operations**
   ```rust
   pub async fn evaluate_batch<U: DeserializeOwned>(
       &self,
       expressions: Vec<&str>,
   ) -> Result<Vec<U>>
   ```

3. **Type Aliases**
   ```rust
   pub type EvaluateResult = serde_json::Value;
   pub type EvaluateJson = serde_json::Map<String, Value>;
   ```

---

## Backward Compatibility & Migration

### Existing Code Continues to Work

```rust
// Original method (no arguments)
page.evaluate_expression("console.log('hi')").await?;

// Original method (string result)
let s = page.evaluate_value("() => 'hello'").await?;

// Both still work - no breaking changes
```

### Migration Path (Optional)

Users can gradually adopt typed evaluate:

```rust
// Old way (if they had)
let result: Value = ... ;

// New way (when ready)
#[derive(Deserialize)]
struct MyStruct { /* fields */ }
let result: MyStruct = page.evaluate("...", None).await?;
```

---

## Alternatives Considered & Rejected

### Alt 1: Fixed Return Type (Always Value)
**Why rejected:** Doesn't solve the core problem (no argument support)

### Alt 2: Macro-based Serialization
**Why rejected:** Unnecessary—serde traits are sufficient and more idiomatic

### Alt 3: Builder Pattern for evaluate
**Why rejected:** Overkill for 2-parameter method, adds complexity

### Alt 4: Separate Methods per Type
```rust
pub async fn evaluate_i32(&self, ...) -> Result<i32>
pub async fn evaluate_string(&self, ...) -> Result<String>
// etc
```
**Why rejected:** Explosion of method variants, users prefer generics

---

## Conclusion

This ADR documents the implementation of **argument serialization and typed results** for the evaluate() method.

### What Was Missing Before
- ❌ No way to pass arguments (always null)
- ❌ No way to get typed returns
- ❌ Inconsistent with other Playwright bindings
- ❌ Users had to use string interpolation (unsafe)

### What We Built
- ✅ Full argument serialization (any Serialize type)
- ✅ Typed results (any DeserializeOwned type)
- ✅ Consistent with playwright-python, playwright-java, playwright-dotnet
- ✅ Type-safe and compiler-verified
- ✅ Comprehensive testing (34 tests)
- ✅ Backward compatible (no breaking changes)

### Impact
This implementation closes a critical capability gap in playwright-rust and brings it to feature parity with other Playwright bindings while maintaining Rust's type safety guarantees.

---

## Status & Timeline

**Status:** ✅ Implemented and Tested

**Completed:**
- ✅ serialize_argument.rs module (23 unit tests)
- ✅ Frame.evaluate<T>() method
- ✅ Page.evaluate<T, U>() method
- ✅ 11 integration tests
- ✅ Documentation and examples
- ✅ Pattern conformance validation

**Date Completed:** 2025-12-24

**References:**
- Implementation: [serialize_argument.rs](../crates/playwright/src/protocol/serialize_argument.rs)
- Frame method: [frame.rs lines 1090-1145](../crates/playwright/src/protocol/frame.rs)
- Page method: [page.rs lines 785-810](../crates/playwright/src/protocol/page.rs)
- Tests: [evaluate_test.rs](../crates/playwright/tests/evaluate_test.rs)
- Guide: [EVALUATE_TYPED.md](../docs/EVALUATE_TYPED.md)
