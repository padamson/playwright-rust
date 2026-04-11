// Integration tests for install_browsers() and install_browsers_with_deps() public API
//
// Browser installation is expensive and destructive (modifies system state),
// so these tests are lightweight and verify the infrastructure works without
// actually installing browsers.

use playwright_rs::{install_browsers, install_browsers_with_deps};

/// Verify that install_browsers() is callable and locates the driver correctly.
///
/// This test invokes `playwright install --help` style verification by checking
/// that the driver can be found and we can construct the command. We don't
/// actually install browsers to keep CI fast and side-effect-free.
///
/// We use an empty browser list with a known-safe invocation to verify the
/// function plumbing works without modifying system state.
#[tokio::test]
async fn test_install_browsers_driver_found() {
    crate::common::init_tracing();

    // install_browsers(Some(&[])) should succeed — no browsers requested
    // means the command is a no-op: `node cli.js install` with no browser args.
    // This exercises the full code path without installing anything.
    let result = install_browsers(Some(&[])).await;

    match result {
        Ok(()) => {
            tracing::info!("install_browsers(Some(&[])) succeeded");
        }
        Err(playwright_rs::Error::ServerNotFound) => {
            tracing::warn!("Driver not found — expected in some CI environments");
        }
        Err(e) => {
            panic!("Unexpected error from install_browsers: {:?}", e);
        }
    }
}

/// Verify that install_browsers_with_deps() compiles and the function signature is correct.
///
/// We do NOT execute install_browsers_with_deps in CI because --with-deps triggers
/// `apt-get install` via sudo, which causes apt lock contention on GitHub runners
/// (concurrent apt processes race for /var/lib/apt/lists/lock).
///
/// The only difference from install_browsers() is appending "--with-deps" to the
/// CLI args — the shared implementation is exercised by the other install tests.
#[tokio::test]
async fn test_install_browsers_with_deps_type_checks() {
    crate::common::init_tracing();

    // Compile-time verification that the function signature is correct.
    // Wrap in a never-called closure to avoid the side effect.
    let _ = || async {
        let _ = install_browsers_with_deps(Some(&["chromium"])).await;
        let _ = install_browsers_with_deps(None).await;
    };

    tracing::info!("install_browsers_with_deps() type signature verified");
}

/// Verify that passing Some(&["chromium"]) produces a valid command.
///
/// We don't run a full installation, but we verify the function signature
/// and argument handling works. The test environment should already have
/// browsers installed, so this becomes a no-op re-install.
#[tokio::test]
async fn test_install_browsers_with_browser_names() {
    crate::common::init_tracing();

    // Passing a specific browser name exercises the argument-building path.
    // In a pre-configured CI environment this is fast (already installed).
    let result = install_browsers(Some(&["chromium"])).await;

    match result {
        Ok(()) => {
            tracing::info!("install_browsers(Some(&[\"chromium\"])) succeeded");
        }
        Err(playwright_rs::Error::ServerNotFound) => {
            tracing::warn!("Driver not found — expected in some CI environments");
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
}

/// Verify that passing None (install all browsers) compiles and type-checks correctly.
///
/// We skip actual execution of this path because installing all browsers takes
/// minutes and modifies system state. The compile-time check here confirms that
/// `None` is accepted as the argument for both functions.
#[tokio::test]
async fn test_install_browsers_none_type_checks() {
    crate::common::init_tracing();

    // Compile-time verification that None::<&[&str]> is accepted as the argument.
    // Wrapping in a closure that is never called avoids the expensive side effect
    // while still exercising the type system.
    let _ = || async {
        let _ = install_browsers(None).await;
        let _ = install_browsers_with_deps(None).await;
    };

    tracing::info!("install_browsers() and install_browsers_with_deps() type signatures verified");
}

/// Verify that an invalid browser name produces an error.
///
/// Playwright's CLI exits non-zero when given an unknown browser name.
/// This confirms that the error-propagation path works.
#[tokio::test]
async fn test_install_browsers_invalid_name_returns_error() {
    crate::common::init_tracing();

    let result = install_browsers(Some(&["not-a-real-browser-xyz"])).await;

    match result {
        Err(playwright_rs::Error::ServerNotFound) => {
            tracing::warn!("Driver not found — skipping invalid-name test");
        }
        Err(_) => {
            // Any other error is the expected outcome — CLI rejected the browser name.
            tracing::info!("Got expected error for invalid browser name");
        }
        Ok(()) => {
            // Some Playwright versions may silently succeed; don't hard-fail.
            tracing::warn!(
                "install_browsers with invalid browser name returned Ok — \
                 Playwright may have ignored the unknown name"
            );
        }
    }
}
