// Integration tests for PLAYWRIGHT_VERSION constant
//
// Tests that the PLAYWRIGHT_VERSION constant:
// 1. Is publicly accessible from the playwright-rs crate
// 2. Matches the expected version format (semver)
// 3. Matches the version defined in build.rs

use playwright_rs::PLAYWRIGHT_VERSION;

#[test]
fn test_playwright_version_exists() {
    // RED: This test should fail because PLAYWRIGHT_VERSION doesn't exist yet
    assert!(!PLAYWRIGHT_VERSION.is_empty());
}

#[test]
fn test_playwright_version_format() {
    // RED: Verify version follows semver format (X.Y.Z)
    let parts: Vec<&str> = PLAYWRIGHT_VERSION.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "Version should have exactly 3 parts (X.Y.Z)"
    );

    // Each part should be a valid number
    for part in parts {
        part.parse::<u32>()
            .expect("Version part should be a valid number");
    }
}

#[test]
fn test_playwright_version_matches_expected() {
    // RED: Verify version matches the current expected version (1.56.1)
    // This ensures the constant is properly synced with build.rs
    assert_eq!(PLAYWRIGHT_VERSION, "1.56.1");
}

#[test]
fn test_playwright_version_is_const() {
    // Verify PLAYWRIGHT_VERSION is a compile-time constant (not computed at runtime)
    // If this const declaration compiles, the test passes - PLAYWRIGHT_VERSION is a valid const
    const VERSION_AT_COMPILE_TIME: &str = PLAYWRIGHT_VERSION;
    // Compare against the runtime value to use the const (avoids dead_code)
    // and prove they're the same value
    let runtime_version: &str = PLAYWRIGHT_VERSION;
    assert_eq!(VERSION_AT_COMPILE_TIME, runtime_version);
}
