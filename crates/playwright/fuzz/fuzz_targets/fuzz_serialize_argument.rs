#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

/// Fuzz the protocol argument serializer with arbitrary JSON.
///
/// serialize_argument converts Rust values into Playwright's wire format.
/// This target ensures it never panics on any valid JSON input.
fuzz_target!(|data: &[u8]| {
    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        let _ = playwright_rs::protocol::serialize_argument(&value);
    }
});
