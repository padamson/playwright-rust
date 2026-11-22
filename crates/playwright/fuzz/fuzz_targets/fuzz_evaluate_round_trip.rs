#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

// Differential fuzz of the evaluate conversion: `parse_result` must be the
// left inverse of `serialize_argument`.
//
// The other targets in this directory assert only that the parsers do not
// panic. That is a weak oracle for a pure transform: a conversion can be
// completely wrong without ever panicking, and silently hand the caller
// corrupted data. This one holds the pair to something real, so it fails on
// wrongness rather than only on crashes.
//
// `tests/properties.rs` asserts the same invariant over generated values.
// This reaches shapes a generator biases away from, but only because the
// corpus outlives the run: `seeds/fuzz_evaluate_round_trip/` is committed
// and CI caches the grown `corpus/` between runs. Without both, the leading
// `from_slice` rejects most random bytes and a 60s budget rediscovers
// roughly what the property test already covers.
fuzz_target!(|data: &[u8]| {
    let Ok(value) = serde_json::from_slice::<Value>(data) else {
        return;
    };

    let wire = playwright_rs::protocol::serialize_argument(&value);
    let back = playwright_rs::protocol::parse_result(&wire["value"]);

    assert_eq!(
        js_number_normal_form(&back),
        js_number_normal_form(&value),
        "round-trip changed the value: {value:?} -> {wire:?} -> {back:?}"
    );
});

/// Rewrites every number to its `f64` form, recursively.
///
/// The wire format is JavaScript, which has one number type, so an integer
/// legitimately returns as a float of the same value and anything past
/// `2^53 - 1` is rounded. Normalizing both sides compares what the protocol
/// can actually carry instead of failing on `0` vs `0.0`.
fn js_number_normal_form(value: &Value) -> Value {
    match value {
        Value::Number(n) => n
            .as_f64()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        Value::Array(items) => Value::Array(items.iter().map(js_number_normal_form).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), js_number_normal_form(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}
