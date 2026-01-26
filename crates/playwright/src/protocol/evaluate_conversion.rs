//! Value conversion for Playwright's evaluate() method
//!
//! This module handles bidirectional conversion between Rust types and Playwright's
//! JSON protocol format for the evaluate() expression method.
//!
//! # Functions
//!
//! - `serialize_argument<T>()` - Converts Rust arguments to protocol format
//! - `serialize_null()` - Serializes None/null values
//! - `parse_value()` - Deserializes protocol responses
//! - `parse_result()` - Convenience wrapper for result deserialization
//!
//! # Protocol Format
//!
//! Playwright uses a type-tagged JSON format where each value includes type information:
//! - `{"v": "null"}` - Null values
//! - `{"v": "undefined"}` - Undefined values
//! - `{"b": true}` - Boolean values
//! - `{"n": 42}` - Number values (int or float)
//! - `{"s": "hello"}` - String values
//! - `{"d": "2025-12-25T00:00:00.000Z"}` - Date values (ISO 8601 format in UTC)
//! - `{"bi": "12345678901234567890"}` - BigInt values (as strings)
//! - `{"u": "https://example.com"}` - URL values (as strings)
//! - `{"e": {"m": "msg", "n": "name", "s": "stack"}}` - Error objects
//! - `{"ta": {"b": "base64...", "k": "ui8"}}` - TypedArray values (base64 encoded)
//! - `{"a": [...], "id": 0}` - Arrays (with circular reference tracking)
//! - `{"o": [...], "id": 1}` - Objects (with circular reference tracking)
//! - `{"v": "Infinity"}`, `{"v": "NaN"}` - Special float values
//!
//! # Example
//!
//! ```ignore
//! use playwright_rs::protocol::{serialize_argument, parse_result};
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct Result {
//!     sum: i32,
//! }
//!
//! // Serialize argument for evaluate
//! let arg = 5;
//! let serialized = serialize_argument(&arg);
//!
//! // After sending to Playwright and getting response back...
//! // Deserialize result from evaluate
//! let response_value = serde_json::json!({"n": 10});
//! let deserialized: i32 = serde_json::from_value(parse_result(&response_value))
//!     .expect("Failed to deserialize result");
//! ```
//!
//! # Implementation Notes
//!
//! Based on playwright-python's serialize_value and parse_value implementations:
//! <https://github.com/microsoft/playwright-python/blob/main/playwright/_impl/_js_handle.py>

use base64::{Engine as _, engine::general_purpose};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Serializes a value following Playwright's protocol specification.
///
/// Playwright expects values in specific formats:
/// - `null`: `{"v": "null"}`
/// - `undefined`: `{"v": "undefined"}`
/// - Boolean: `{"b": true}` or `{"b": false}`
/// - Number: `{"n": 42}` or `{"n": 3.14}`
/// - String: `{"s": "hello"}`
/// - Date: `{"d": "2025-12-25T00:00:00.000Z"}` (ISO 8601 UTC)
/// - BigInt: `{"bi": "12345678901234567890"}` (as string)
/// - URL: `{"u": "https://example.com"}` (as string)
/// - Error: `{"e": {"m": "message", "n": "name", "s": "stack"}}`
/// - TypedArray: `{"ta": {"b": "base64...", "k": "ui8"}}`
/// - Array: `{"a": [...], "id": 0}`
/// - Object: `{"o": [{"k": "name", "v": ...}], "id": 1}`
/// - Special floats: `{"v": "Infinity"}`, `{"v": "-Infinity"}`, `{"v": "-0"}`, `{"v": "NaN"}`
///
/// The `id` field is used for circular reference tracking.
fn serialize_value(value: &Value, visitor: &mut Visitor) -> Value {
    // Handle null
    if value.is_null() {
        return json!({"v": "null"});
    }

    // Handle boolean
    if let Some(b) = value.as_bool() {
        return json!({"b": b});
    }

    // Handle number
    if let Some(n) = value.as_f64() {
        // Check for special float values
        if n.is_infinite() {
            if n.is_sign_positive() {
                return json!({"v": "Infinity"});
            } else {
                return json!({"v": "-Infinity"});
            }
        }
        if n.is_nan() {
            return json!({"v": "NaN"});
        }
        // Check for negative zero
        if n == 0.0 && n.is_sign_negative() {
            return json!({"v": "-0"});
        }
        return json!({"n": n});
    }

    // Handle string
    if let Some(s) = value.as_str() {
        return json!({"s": s});
    }

    // Handle array
    if let Some(arr) = value.as_array() {
        // Check if already visited (circular reference)
        let value_ptr = value as *const Value as usize;
        if let Some(ref_id) = visitor.visited.get(&value_ptr) {
            return json!({"ref": ref_id});
        }

        // Mark as visited
        let id = visitor.next_id();
        visitor.visited.insert(value_ptr, id);

        // Serialize array elements
        let serialized: Vec<Value> = arr
            .iter()
            .map(|item| serialize_value(item, visitor))
            .collect();

        return json!({"a": serialized, "id": id});
    }

    // Handle object
    if let Some(obj) = value.as_object() {
        // Check if already visited (circular reference)
        let value_ptr = value as *const Value as usize;
        if let Some(ref_id) = visitor.visited.get(&value_ptr) {
            return json!({"ref": ref_id});
        }

        // Mark as visited
        let id = visitor.next_id();
        visitor.visited.insert(value_ptr, id);

        // Serialize object properties
        let serialized: Vec<Value> = obj
            .iter()
            .map(|(key, val)| {
                json!({
                    "k": key,
                    "v": serialize_value(val, visitor)
                })
            })
            .collect();

        return json!({"o": serialized, "id": id});
    }

    // Default to undefined
    json!({"v": "undefined"})
}

/// Tracks visited objects for circular reference detection.
struct Visitor {
    visited: HashMap<usize, usize>,
    id_counter: usize,
}

impl Visitor {
    fn new() -> Self {
        Self {
            visited: HashMap::new(),
            id_counter: 0,
        }
    }

    fn next_id(&mut self) -> usize {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }
}

/// Serializes an argument for Playwright's evaluateExpression method.
///
/// This is the main entry point for serializing arguments. It wraps the serialized
/// value with the required "value" and "handles" fields.
///
/// # Arguments
///
/// * `arg` - A value that implements `serde::Serialize` or a `serde_json::Value`
///
/// # Returns
///
/// A JSON object with "value" and "handles" fields in Playwright's format:
/// ```json
/// {
///   "value": { /* serialized value */ },
///   "handles": []
/// }
/// ```
///
/// # Examples
///
/// ```
/// use playwright_rs::protocol::serialize_argument;
/// use serde_json::json;
///
/// // String argument
/// let arg = serialize_argument(&json!("hello"));
/// // Returns: {"value": {"s": "hello"}, "handles": []}
///
/// // Number argument
/// let arg = serialize_argument(&json!(42));
/// // Returns: {"value": {"n": 42}, "handles": []}
///
/// // Object argument
/// let arg = serialize_argument(&json!({"name": "test"}));
/// // Returns: {"value": {"o": [{"k": "name", "v": {"s": "test"}}], "id": 0}, "handles": []}
/// ```
pub fn serialize_argument<T: serde::Serialize>(arg: &T) -> Value {
    let json_value = serde_json::to_value(arg).unwrap_or(Value::Null);
    let mut visitor = Visitor::new();
    let value = serialize_value(&json_value, &mut visitor);

    json!({
        "value": value,
        "handles": []
    })
}

/// Convenience function to serialize None/null as an argument.
///
/// # Returns
///
/// A JSON object representing null: `{"value": {"v": "null"}, "handles": []}`
pub fn serialize_null() -> Value {
    json!({
        "value": {"v": "null"},
        "handles": []
    })
}

/// Parses a value returned by Playwright's evaluateExpression method.
///
/// This function deserializes values from Playwright's protocol format back
/// into standard Rust/JSON values. It handles the same types as serialization:
///
/// - `{"v": "null"}` → `Value::Null`
/// - `{"v": "undefined"}` → `Value::Null`
/// - `{"b": true}` → `Value::Bool(true)`
/// - `{"n": 42}` → `Value::Number(42)`
/// - `{"s": "hello"}` → `Value::String("hello")`
/// - `{"d": "2025-12-25T00:00:00.000Z"}` → `Value::String("2025-12-25T00:00:00.000Z")`
/// - `{"bi": "12345678901234567890"}` → `Value::String("12345678901234567890")`
/// - `{"u": "https://example.com"}` → `Value::String("https://example.com")`
/// - `{"e": {...}}` → `Value::Object` with error details
/// - `{"ta": {...}}` → `Value::Array` of decoded values
/// - `{"a": [...]}` → `Value::Array([...])`
/// - `{"o": [...]}` → `Value::Object({...})`
/// - Special values: `"Infinity"`, `"-Infinity"`, `"NaN"`, `"-0"`
///
/// # Arguments
///
/// * `value` - The wrapped value from Playwright
/// * `refs` - Optional map for tracking circular references
///
/// # Returns
///
/// The parsed value as a `serde_json::Value`
///
/// # Examples
///
/// ```
/// use playwright_rs::protocol::parse_value;
/// use serde_json::json;
///
/// // Parse a string
/// let result = parse_value(&json!({"s": "hello"}), None);
/// assert_eq!(result, json!("hello"));
///
/// // Parse a number
/// let result = parse_value(&json!({"n": 42}), None);
/// assert_eq!(result, json!(42));
///
/// // Parse a boolean
/// let result = parse_value(&json!({"b": true}), None);
/// assert_eq!(result, json!(true));
/// ```
pub fn parse_value(value: &Value, refs: Option<&mut HashMap<usize, Value>>) -> Value {
    let mut local_refs = HashMap::new();
    let refs = match refs {
        Some(r) => r,
        None => &mut local_refs,
    };

    // Handle null input
    if value.is_null() {
        return Value::Null;
    }

    // Must be an object with type indicators
    if let Some(obj) = value.as_object() {
        // Handle circular reference
        if let Some(ref_id) = obj.get("ref").and_then(|v| v.as_u64()) {
            return refs.get(&(ref_id as usize)).cloned().unwrap_or(Value::Null);
        }

        // Handle special "v" values (null, undefined, special floats)
        if let Some(v) = obj.get("v").and_then(|v| v.as_str()) {
            return match v {
                "null" | "undefined" => Value::Null,
                "Infinity" => {
                    // Return as number if possible, otherwise as null
                    serde_json::Number::from_f64(f64::INFINITY)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                }
                "-Infinity" => serde_json::Number::from_f64(f64::NEG_INFINITY)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
                "NaN" => serde_json::Number::from_f64(f64::NAN)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
                "-0" => serde_json::Number::from_f64(-0.0)
                    .map(Value::Number)
                    .unwrap_or(json!(0.0)),
                _ => Value::Null,
            };
        }

        // Handle boolean
        if let Some(b) = obj.get("b").and_then(|v| v.as_bool()) {
            return json!(b);
        }

        // Handle number
        if let Some(n) = obj.get("n") {
            return n.clone();
        }

        // Handle string
        if let Some(s) = obj.get("s").and_then(|v| v.as_str()) {
            return json!(s);
        }

        // Handle date (ISO 8601 UTC string)
        if let Some(d) = obj.get("d").and_then(|v| v.as_str()) {
            // Node.js Date objects are always in UTC, store as ISO 8601 string
            return json!(d);
        }

        // Handle BigInt (stored as string to preserve precision)
        if let Some(bi) = obj.get("bi").and_then(|v| v.as_str()) {
            return json!(bi);
        }

        // Handle URL (stored as string)
        if let Some(u) = obj.get("u").and_then(|v| v.as_str()) {
            return json!(u);
        }

        // Handle error objects
        if let Some(error_obj) = obj.get("e").and_then(|v| v.as_object()) {
            let mut result = serde_json::Map::new();
            if let Some(message) = error_obj.get("m").and_then(|v| v.as_str()) {
                result.insert("m".to_string(), json!(message));
            }
            if let Some(name) = error_obj.get("n").and_then(|v| v.as_str()) {
                result.insert("n".to_string(), json!(name));
            }
            if let Some(stack) = error_obj.get("s").and_then(|v| v.as_str()) {
                result.insert("s".to_string(), json!(stack));
            }
            return Value::Object(result);
        }

        // Handle TypedArray (base64 encoded)
        if let Some(ta_obj) = obj.get("ta").and_then(|v| v.as_object()) {
            let (Some(encoded), Some(kind)) = (
                ta_obj.get("b").and_then(|v| v.as_str()),
                ta_obj.get("k").and_then(|v| v.as_str()),
            ) else {
                return Value::Null;
            };

            let Ok(decoded) = general_purpose::STANDARD.decode(encoded) else {
                return Value::Null;
            };
            // Return as array of decoded values
            let mut result_array = Vec::new();
            match kind {
                "ui8" | "ui8c" => {
                    // Unsigned 8-bit
                    for byte in decoded {
                        result_array.push(json!(byte as u32));
                    }
                }
                "i8" => {
                    // Signed 8-bit
                    for byte in decoded {
                        result_array.push(json!(byte as i8 as i32));
                    }
                }

                "ui16" => {
                    // Unsigned 16-bit
                    for chunk in decoded.chunks(2) {
                        if chunk.len() == 2 {
                            let value = u16::from_le_bytes([chunk[0], chunk[1]]);
                            result_array.push(json!(value as u32));
                        }
                    }
                }
                "i16" => {
                    // Signed 16-bit
                    for chunk in decoded.chunks(2) {
                        if chunk.len() == 2 {
                            let value = i16::from_le_bytes([chunk[0], chunk[1]]);
                            result_array.push(json!(value as i32));
                        }
                    }
                }
                "i32" => {
                    // Signed 32-bit
                    for chunk in decoded.chunks(4) {
                        if chunk.len() == 4 {
                            let value =
                                i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                            result_array.push(json!(value as i64));
                        }
                    }
                }
                "ui32" => {
                    // Unsigned 32-bit
                    for chunk in decoded.chunks(4) {
                        if chunk.len() == 4 {
                            let value =
                                u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                            result_array.push(json!(value as u64));
                        }
                    }
                }
                "f32" => {
                    // 32-bit floating point
                    for chunk in decoded.chunks(4) {
                        if chunk.len() == 4 {
                            let value =
                                f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                            result_array.push(json!(value));
                        }
                    }
                }
                "f64" => {
                    // 64-bit floating point
                    for chunk in decoded.chunks(8) {
                        if chunk.len() == 8 {
                            let value = f64::from_le_bytes([
                                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5],
                                chunk[6], chunk[7],
                            ]);
                            result_array.push(json!(value));
                        }
                    }
                }
                _ => {
                    // For other types, return as array of bytes
                    for byte in decoded {
                        result_array.push(json!(byte));
                    }
                }
            }
            return json!(result_array);
        }

        // Handle array
        if let Some(arr) = obj.get("a").and_then(|v| v.as_array()) {
            // Store reference if has id
            let result_arr: Vec<Value> = arr
                .iter()
                .map(|item| parse_value(item, Some(refs)))
                .collect();

            let result = json!(result_arr);

            if let Some(id) = obj.get("id").and_then(|v| v.as_u64()) {
                refs.insert(id as usize, result.clone());
            }

            return result;
        }

        // Handle object
        if let Some(props) = obj.get("o").and_then(|v| v.as_array()) {
            let mut result_obj = serde_json::Map::new();

            for prop in props {
                if let Some(prop_obj) = prop.as_object() {
                    if let (Some(key), Some(val)) = (
                        prop_obj.get("k").and_then(|v| v.as_str()),
                        prop_obj.get("v"),
                    ) {
                        result_obj.insert(key.to_string(), parse_value(val, Some(refs)));
                    }
                }
            }

            let result = Value::Object(result_obj);

            if let Some(id) = obj.get("id").and_then(|v| v.as_u64()) {
                refs.insert(id as usize, result.clone());
            }

            return result;
        }
    }

    // Default to null for unrecognized formats
    Value::Null
}

/// Parses a result from Playwright's evaluate methods.
///
/// This is a convenience wrapper around `parse_value` for parsing
/// evaluation results. It handles the common case where you receive
/// a result from `evaluateExpression` or similar methods.
///
/// # Arguments
///
/// * `result` - The result value from Playwright
///
/// # Returns
///
/// The parsed value as a `serde_json::Value`
///
/// # Examples
///
/// ```
/// use playwright_rs::protocol::parse_result;
/// use serde_json::json;
///
/// // Parse evaluation result
/// let result = parse_result(&json!({"s": "hello"}));
/// assert_eq!(result, json!("hello"));
/// ```
pub fn parse_result(result: &Value) -> Value {
    parse_value(result, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PI: f64 = std::f64::consts::PI;

    #[test]
    fn test_serialize_null() {
        let result = serialize_argument(&json!(null));
        assert_eq!(
            result,
            json!({
                "value": {"v": "null"},
                "handles": []
            })
        );
    }

    #[test]
    fn test_serialize_boolean() {
        let result = serialize_argument(&json!(true));
        assert_eq!(
            result,
            json!({
                "value": {"b": true},
                "handles": []
            })
        );

        let result = serialize_argument(&json!(false));
        assert_eq!(
            result,
            json!({
                "value": {"b": false},
                "handles": []
            })
        );
    }

    #[test]
    fn test_serialize_number() {
        let result = serialize_argument(&json!(42));
        let value = &result["value"];
        assert_eq!(value["n"].as_f64().unwrap(), 42.0);
        assert_eq!(result["handles"], json!([]));

        let result = serialize_argument(&json!(PI));
        let value = &result["value"];
        assert_eq!(value["n"].as_f64().unwrap(), PI);
        assert_eq!(result["handles"], json!([]));
    }

    #[test]
    fn test_serialize_special_floats() {
        // Note: serde_json serializes special floats as null by default
        // This test documents the behavior - in practice, special floats
        // would need to be handled before serialization or passed as strings

        // Test that regular floats work
        let result = serialize_argument(&json!(1.5));
        let value = &result["value"];
        assert_eq!(value["n"].as_f64().unwrap(), 1.5);

        // Test zero
        let result = serialize_argument(&json!(0.0));
        let value = &result["value"];
        assert_eq!(value["n"].as_f64().unwrap(), 0.0);
    }

    #[test]
    fn test_serialize_string() {
        let result = serialize_argument(&json!("hello"));
        assert_eq!(
            result,
            json!({
                "value": {"s": "hello"},
                "handles": []
            })
        );
    }

    #[test]
    fn test_serialize_array() {
        let result = serialize_argument(&json!([1, 2, 3]));

        assert_eq!(result["handles"], json!([]));
        let value = &result["value"];
        assert!(value["a"].is_array());
        assert_eq!(value["id"], 0);

        let items = value["a"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["n"].as_f64().unwrap(), 1.0);
        assert_eq!(items[1]["n"].as_f64().unwrap(), 2.0);
        assert_eq!(items[2]["n"].as_f64().unwrap(), 3.0);
    }

    #[test]
    fn test_serialize_object() {
        let result = serialize_argument(&json!({
            "name": "test",
            "value": 42
        }));

        assert_eq!(result["handles"], json!([]));
        let value = &result["value"];
        assert!(value["o"].is_array());
        assert_eq!(value["id"], 0);

        let props = value["o"].as_array().unwrap();
        assert_eq!(props.len(), 2);

        // Properties can be in any order
        let mut found_name = false;
        let mut found_value = false;

        for prop in props {
            if prop["k"] == "name" {
                assert_eq!(prop["v"], json!({"s": "test"}));
                found_name = true;
            }
            if prop["k"] == "value" {
                assert_eq!(prop["v"]["n"].as_f64().unwrap(), 42.0);
                found_value = true;
            }
        }

        assert!(found_name);
        assert!(found_value);
    }

    #[test]
    fn test_serialize_nested_array() {
        let result = serialize_argument(&json!([1, "test", true, [2, 3]]));

        let value = &result["value"];
        let items = value["a"].as_array().unwrap();
        assert_eq!(items.len(), 4);

        // First three items
        assert_eq!(items[0]["n"].as_f64().unwrap(), 1.0);
        assert_eq!(items[1], json!({"s": "test"}));
        assert_eq!(items[2], json!({"b": true}));

        // Fourth item is nested array
        assert_eq!(items[3]["id"], 1); // Second object gets id=1
        let nested = items[3]["a"].as_array().unwrap();
        assert_eq!(nested.len(), 2);
        assert_eq!(nested[0]["n"].as_f64().unwrap(), 2.0);
        assert_eq!(nested[1]["n"].as_f64().unwrap(), 3.0);
    }

    #[test]
    fn test_serialize_nested_object() {
        let result = serialize_argument(&json!({
            "outer": {
                "inner": "value"
            }
        }));

        let value = &result["value"];
        assert_eq!(value["id"], 0);

        let props = value["o"].as_array().unwrap();
        assert_eq!(props.len(), 1);
        assert_eq!(props[0]["k"], "outer");

        let inner_obj = &props[0]["v"];
        assert_eq!(inner_obj["id"], 1);

        let inner_props = inner_obj["o"].as_array().unwrap();
        assert_eq!(inner_props.len(), 1);
        assert_eq!(inner_props[0]["k"], "inner");
        assert_eq!(inner_props[0]["v"], json!({"s": "value"}));
    }

    #[test]
    fn test_serialize_mixed_types() {
        let result = serialize_argument(&json!({
            "string": "hello",
            "number": 42,
            "boolean": true,
            "null": null,
            "array": [1, 2, 3],
            "object": {"nested": "value"}
        }));

        let value = &result["value"];
        assert!(value["o"].is_array());

        let props = value["o"].as_array().unwrap();
        assert_eq!(props.len(), 6);
    }

    #[test]
    fn test_serialize_null_helper() {
        let result = serialize_null();
        assert_eq!(
            result,
            json!({
                "value": {"v": "null"},
                "handles": []
            })
        );
    }

    // ===== Deserialization Tests =====

    #[test]
    fn test_parse_null() {
        let result = parse_value(&json!({"v": "null"}), None);
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn test_parse_undefined() {
        let result = parse_value(&json!({"v": "undefined"}), None);
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn test_parse_boolean() {
        let result = parse_value(&json!({"b": true}), None);
        assert_eq!(result, json!(true));

        let result = parse_value(&json!({"b": false}), None);
        assert_eq!(result, json!(false));
    }

    #[test]
    fn test_parse_number() {
        let result = parse_value(&json!({"n": 42}), None);
        assert_eq!(result.as_f64().unwrap(), 42.0);

        let result = parse_value(&json!({"n": PI}), None);
        assert_eq!(result.as_f64().unwrap(), PI);
    }

    #[test]
    fn test_parse_special_floats() {
        // Note: serde_json cannot represent special float values in JSON
        // They get converted to null. This documents the behavior.

        // Infinity - serde_json will return null for special floats
        let result = parse_value(&json!({"v": "Infinity"}), None);
        // serde_json::Number::from_f64 returns None for special floats
        assert!(result.is_null());

        // -Infinity
        let result = parse_value(&json!({"v": "-Infinity"}), None);
        assert!(result.is_null());

        // NaN
        let result = parse_value(&json!({"v": "NaN"}), None);
        assert!(result.is_null());

        // -0 can be represented
        let result = parse_value(&json!({"v": "-0"}), None);
        assert!(result.is_number());
    }

    #[test]
    fn test_parse_string() {
        let result = parse_value(&json!({"s": "hello"}), None);
        assert_eq!(result, json!("hello"));

        let result = parse_value(&json!({"s": "world"}), None);
        assert_eq!(result, json!("world"));
    }

    #[test]
    fn test_parse_date() {
        let result = parse_value(&json!({"d": "2025-12-25T00:00:00.000Z"}), None);
        assert_eq!(result, json!("2025-12-25T00:00:00.000Z"));

        let result = parse_value(&json!({"d": "2025-12-25T10:30:45.123Z"}), None);
        assert_eq!(result, json!("2025-12-25T10:30:45.123Z"));
    }

    #[test]
    fn test_parse_bigint() {
        let result = parse_value(&json!({"bi": "12345678901234567890"}), None);
        assert_eq!(result, json!("12345678901234567890"));

        let result = parse_value(&json!({"bi": "9007199254740991"}), None);
        assert_eq!(result, json!("9007199254740991"));
    }

    #[test]
    fn test_parse_url() {
        let result = parse_value(&json!({"u": "https://example.com"}), None);
        assert_eq!(result, json!("https://example.com"));

        let result = parse_value(&json!({"u": "https://example.com/path?query=1"}), None);
        assert_eq!(result, json!("https://example.com/path?query=1"));
    }

    #[test]
    fn test_parse_error() {
        let result = parse_value(
            &json!({
                "e": {
                    "m": "Something went wrong",
                    "n": "TypeError",
                    "s": "Error: at line 1"
                }
            }),
            None,
        );

        let obj = result.as_object().unwrap();
        assert_eq!(
            obj.get("m").and_then(|v| v.as_str()),
            Some("Something went wrong")
        );
        assert_eq!(obj.get("n").and_then(|v| v.as_str()), Some("TypeError"));
        assert_eq!(
            obj.get("s").and_then(|v| v.as_str()),
            Some("Error: at line 1")
        );
    }

    #[test]
    fn test_parse_typed_array_ui8() {
        // Array [1, 2, 3, 4, 5]
        let values: Vec<u8> = vec![1, 2, 3, 4, 5];
        let base64_encoded = general_purpose::STANDARD.encode(&values);

        let result = parse_value(&json!({"ta": {"b": base64_encoded, "k": "ui8"}}), None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), values.len());
        for (i, &expected) in values.iter().enumerate() {
            assert_eq!(arr[i].as_u64().unwrap(), expected as u64);
        }
    }

    #[test]
    fn test_parse_typed_array_ui8c() {
        // ui8c (Uint8ClampedArray) should behave same as ui8
        let values: Vec<u8> = vec![1, 2, 3, 4, 5];
        let base64_encoded = general_purpose::STANDARD.encode(&values);

        let result = parse_value(&json!({"ta": {"b": base64_encoded, "k": "ui8c"}}), None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), values.len());
        assert_eq!(arr[0].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_parse_typed_array_i8() {
        // Array [-1, 127, -128, 0, 1]
        let values: Vec<i8> = vec![-1, 127, -128, 0, 1];
        let bytes: Vec<u8> = values.iter().map(|&v| v as u8).collect();
        let base64_encoded = general_purpose::STANDARD.encode(&bytes);

        let result = parse_value(&json!({"ta": {"b": base64_encoded, "k": "i8"}}), None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), values.len());
        for (i, &expected) in values.iter().enumerate() {
            assert_eq!(arr[i].as_i64().unwrap(), expected as i64);
        }
    }

    #[test]
    fn test_parse_typed_array_ui16() {
        // Uint16Array [1, 256, 65535]
        let values: Vec<u16> = vec![1, 256, 65535];
        let mut bytes = Vec::new();
        for &v in &values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let base64_encoded = general_purpose::STANDARD.encode(&bytes);

        let result = parse_value(&json!({"ta": {"b": base64_encoded, "k": "ui16"}}), None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), values.len());
        for (i, &expected) in values.iter().enumerate() {
            assert_eq!(arr[i].as_u64().unwrap(), expected as u64);
        }
    }

    #[test]
    fn test_parse_typed_array_i16() {
        // Int16Array [1, -1, 32767, -32768]
        let values: Vec<i16> = vec![1, -1, 32767, -32768];
        let mut bytes = Vec::new();
        for &v in &values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let base64_encoded = general_purpose::STANDARD.encode(&bytes);

        let result = parse_value(&json!({"ta": {"b": base64_encoded, "k": "i16"}}), None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), values.len());
        for (i, &expected) in values.iter().enumerate() {
            assert_eq!(arr[i].as_i64().unwrap(), expected as i64);
        }
    }

    #[test]
    fn test_parse_typed_array_ui32() {
        // Uint32Array [1, 256, 4294967295]
        let values: Vec<u32> = vec![1, 256, 4294967295];
        let mut bytes = Vec::new();
        for &v in &values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let base64_encoded = general_purpose::STANDARD.encode(&bytes);

        let result = parse_value(&json!({"ta": {"b": base64_encoded, "k": "ui32"}}), None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), values.len());
        for (i, &expected) in values.iter().enumerate() {
            assert_eq!(arr[i].as_u64().unwrap(), expected as u64);
        }
    }

    #[test]
    fn test_parse_typed_array_i32() {
        // Int32Array [1, -1, 2147483647, -2147483648]
        let values: Vec<i32> = vec![1, -1, 2147483647, -2147483648];
        let mut bytes = Vec::new();
        for &v in &values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let base64_encoded = general_purpose::STANDARD.encode(&bytes);

        let result = parse_value(&json!({"ta": {"b": base64_encoded, "k": "i32"}}), None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), values.len());
        for (i, &expected) in values.iter().enumerate() {
            assert_eq!(arr[i].as_i64().unwrap(), expected as i64);
        }
    }

    #[test]
    fn test_parse_typed_array_f32() {
        // Float32Array [1.0, -1.0, 3.14]
        let values: Vec<f32> = vec![1.0, -1.0, PI as f32];
        let mut bytes = Vec::new();
        for &v in &values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let base64_encoded = general_purpose::STANDARD.encode(&bytes);

        let result = parse_value(&json!({"ta": {"b": base64_encoded, "k": "f32"}}), None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), values.len());
        for (i, &expected) in values.iter().enumerate() {
            assert!((arr[i].as_f64().unwrap() - expected as f64).abs() < 0.01);
        }
    }

    #[test]
    fn test_parse_typed_array_f64() {
        // Float64Array [1.0, -1.0, 3.141592653589793]
        let values: Vec<f64> = vec![1.0, -1.0, PI];
        let mut bytes = Vec::new();
        for &v in &values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let base64_encoded = general_purpose::STANDARD.encode(&bytes);

        let result = parse_value(&json!({"ta": {"b": base64_encoded, "k": "f64"}}), None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), values.len());
        for (i, &expected) in values.iter().enumerate() {
            assert!((arr[i].as_f64().unwrap() - expected).abs() < 0.0000001);
        }
    }

    #[test]
    fn test_parse_typed_array_empty() {
        // Base64 encoded empty array
        let result = parse_value(&json!({"ta": {"b": "", "k": "ui8"}}), None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 0);
    }

    #[test]
    fn test_parse_typed_array_invalid_base64() {
        // Invalid base64 should return null
        let result = parse_value(&json!({"ta": {"b": "not-valid-base64!", "k": "ui8"}}), None);
        assert!(result.is_null());
    }

    #[test]
    fn test_parse_typed_array_unknown_kind() {
        // Unknown kind should default to byte array
        let result = parse_value(&json!({"ta": {"b": "AQIDBAU=", "k": "unknown"}}), None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 5);
        assert_eq!(arr[0].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_parse_typed_array_missing_fields() {
        // Missing "b" field
        let result = parse_value(&json!({"ta": {"k": "ui8"}}), None);
        assert!(result.is_null());

        // Missing "k" field
        let result = parse_value(&json!({"ta": {"b": "AQIDBAU="}}), None);
        assert!(result.is_null());
    }

    #[test]
    fn test_parse_circular_reference() {
        // Test array with circular reference
        let mut refs = HashMap::new();
        refs.insert(5, json!([1, 2, 3]));

        let result = parse_value(&json!({"ref": 5}), Some(&mut refs));
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_i64().unwrap(), 1);
    }

    #[test]
    fn test_parse_array() {
        let input = json!({
            "a": [
                {"n": 1},
                {"n": 2},
                {"n": 3}
            ],
            "id": 0
        });

        let result = parse_value(&input, None);
        assert!(result.is_array());

        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_f64().unwrap(), 1.0);
        assert_eq!(arr[1].as_f64().unwrap(), 2.0);
        assert_eq!(arr[2].as_f64().unwrap(), 3.0);
    }

    #[test]
    fn test_parse_object() {
        let input = json!({
            "o": [
                {"k": "name", "v": {"s": "John"}},
                {"k": "age", "v": {"n": 30}}
            ],
            "id": 0
        });

        let result = parse_value(&input, None);
        assert!(result.is_object());

        let obj = result.as_object().unwrap();
        assert_eq!(obj.get("name").and_then(|v| v.as_str()), Some("John"));
        assert_eq!(obj.get("age").and_then(|v| v.as_f64()), Some(30.0));
    }

    #[test]
    fn test_parse_nested_array() {
        let input = json!({
            "a": [
                {"n": 1},
                {"s": "test"},
                {"b": true},
                {
                    "a": [
                        {"n": 2},
                        {"n": 3}
                    ],
                    "id": 1
                }
            ],
            "id": 0
        });

        let result = parse_value(&input, None);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 4);

        assert_eq!(arr[0].as_f64().unwrap(), 1.0);
        assert_eq!(arr[1].as_str().unwrap(), "test");
        assert!(arr[2].as_bool().unwrap());

        let nested = arr[3].as_array().unwrap();
        assert_eq!(nested.len(), 2);
        assert_eq!(nested[0].as_f64().unwrap(), 2.0);
        assert_eq!(nested[1].as_f64().unwrap(), 3.0);
    }

    #[test]
    fn test_parse_nested_object() {
        let input = json!({
            "o": [
                {
                    "k": "outer",
                    "v": {
                        "o": [
                            {"k": "inner", "v": {"s": "value"}}
                        ],
                        "id": 1
                    }
                }
            ],
            "id": 0
        });

        let result = parse_value(&input, None);
        let obj = result.as_object().unwrap();
        let outer = obj.get("outer").unwrap().as_object().unwrap();
        assert_eq!(outer.get("inner").and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn test_parse_result() {
        // Test the convenience wrapper
        let result = parse_result(&json!({"s": "hello"}));
        assert_eq!(result, json!("hello"));

        let result = parse_result(&json!({"n": 42}));
        assert_eq!(result.as_f64().unwrap(), 42.0);
    }

    #[test]
    fn test_roundtrip_serialization() {
        // Test that we can serialize and deserialize values
        let original = json!({
            "name": "test",
            "value": 42,
            "active": true,
            "items": [1, 2, 3]
        });

        // Serialize
        let serialized = serialize_argument(&original);
        let serialized_value = &serialized["value"];

        // Deserialize
        let deserialized = parse_value(serialized_value, None);

        // Compare (note: object property order may differ)
        assert!(deserialized.is_object());
        let obj = deserialized.as_object().unwrap();
        assert_eq!(obj.get("name").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(obj.get("value").and_then(|v| v.as_f64()), Some(42.0));
        assert_eq!(obj.get("active").and_then(|v| v.as_bool()), Some(true));

        let items = obj.get("items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(items.len(), 3);
    }
}
