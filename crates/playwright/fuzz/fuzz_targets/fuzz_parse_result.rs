#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

/// Fuzz the protocol result parser with arbitrary JSON.
///
/// parse_result is the entry point for deserializing RPC responses.
/// It wraps parse_value but handles the "result" envelope.
fuzz_target!(|data: &[u8]| {
    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        let _ = playwright_rs::protocol::parse_result(&value);
    }
});
