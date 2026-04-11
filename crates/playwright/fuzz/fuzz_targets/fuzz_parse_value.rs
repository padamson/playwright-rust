#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

/// Fuzz the protocol value parser with arbitrary JSON.
///
/// parse_value handles recursive structures, circular references,
/// and special Playwright type tags (s, n, b, v, d, bi, etc.).
/// This target ensures it never panics on malformed input.
fuzz_target!(|data: &[u8]| {
    // Try to parse as JSON first — parse_value expects serde_json::Value
    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        // Should never panic, regardless of input shape
        let _ = playwright_rs::protocol::parse_value(&value, None);
    }
});
