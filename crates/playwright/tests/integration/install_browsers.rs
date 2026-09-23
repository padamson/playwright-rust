// Browser installation is expensive and modifies system state, so these tests
// only ask the driver for browsers CI has already installed.

use playwright_rs::{Error, install_browsers};

// An empty list is a bare `install`, which the driver reads as "install the
// default browsers", not as a no-op. It is cheap only because the workflow
// installed them already, so the driver finds every browser present and exits.
#[tokio::test]
async fn an_empty_browser_list_installs_the_defaults_without_error() {
    crate::common::init_tracing();

    install_browsers(Some(&[]))
        .await
        .expect("install_browsers(Some(&[])) against an already-installed browser set");
}

#[tokio::test]
async fn a_named_browser_that_is_already_installed_reinstalls_without_error() {
    crate::common::init_tracing();

    install_browsers(Some(&["chromium"]))
        .await
        .expect("install_browsers(Some(&[\"chromium\"])) against an installed Chromium");
}

// The driver exits non-zero for an unknown browser name; the error carries
// the driver's own output so the caller can see which name was rejected.
#[tokio::test]
async fn an_unknown_browser_name_is_a_launch_failure_naming_the_browser() {
    crate::common::init_tracing();

    let err = install_browsers(Some(&["not-a-real-browser-xyz"]))
        .await
        .expect_err("the driver should reject an unknown browser name");

    assert!(
        matches!(&err, Error::LaunchFailed(msg) if msg.contains("not-a-real-browser-xyz")),
        "expected LaunchFailed naming the rejected browser, got {err:?}"
    );
}
