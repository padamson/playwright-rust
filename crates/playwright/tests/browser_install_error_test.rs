// Integration tests for helpful browser installation error messages
//
// Tests that when browsers are not installed, users get helpful error messages
// with the correct installation command.

use playwright_rs::{Error, PLAYWRIGHT_VERSION};

/// Test that verifies Error::BrowserNotInstalled variant exists
#[test]
fn test_browser_not_installed_error_type_exists() {
    // Create a BrowserNotInstalled error
    let error = Error::BrowserNotInstalled {
        browser_name: "chromium".to_string(),
        message: "Looks like Playwright Test or Playwright was just installed or updated."
            .to_string(),
        playwright_version: PLAYWRIGHT_VERSION.to_string(),
    };

    // Verify the error contains helpful information
    let error_message = error.to_string();
    assert!(error_message.contains("chromium"));
    assert!(error_message.contains("npx playwright"));
}

/// Test that BrowserNotInstalled error includes the PLAYWRIGHT_VERSION constant
#[test]
fn test_browser_not_installed_includes_version() {
    let error = Error::BrowserNotInstalled {
        browser_name: "firefox".to_string(),
        message: "Looks like Playwright Test or Playwright was just installed or updated."
            .to_string(),
        playwright_version: PLAYWRIGHT_VERSION.to_string(),
    };

    let error_message = error.to_string();

    // Error should include the Playwright version
    assert!(
        error_message.contains(PLAYWRIGHT_VERSION),
        "Error message should contain Playwright version {}",
        PLAYWRIGHT_VERSION
    );
}

/// Test that BrowserNotInstalled error includes installation command
#[test]
fn test_browser_not_installed_includes_install_command() {
    let error = Error::BrowserNotInstalled {
        browser_name: "webkit".to_string(),
        message: "Looks like Playwright Test or Playwright was just installed or updated."
            .to_string(),
        playwright_version: PLAYWRIGHT_VERSION.to_string(),
    };

    let error_message = error.to_string();

    // Error should include the install command with version
    let expected_command = format!("npx playwright@{} install", PLAYWRIGHT_VERSION);
    assert!(
        error_message.contains(&expected_command),
        "Error message should contain install command: {}",
        expected_command
    );
}

/// Test that the error message is user-friendly
#[test]
fn test_browser_not_installed_error_is_helpful() {
    let error = Error::BrowserNotInstalled {
        browser_name: "chromium".to_string(),
        message: "Looks like Playwright Test or Playwright was just installed or updated."
            .to_string(),
        playwright_version: PLAYWRIGHT_VERSION.to_string(),
    };

    let error_message = error.to_string();

    // Should mention the specific browser
    assert!(error_message.contains("chromium"));

    // Should explain what to do
    assert!(error_message.contains("install") || error_message.contains("Install"));

    // Should provide command
    assert!(error_message.contains("npx playwright"));
}

/// Test error message for different browsers
#[test]
fn test_browser_not_installed_different_browsers() {
    let browsers = vec!["chromium", "firefox", "webkit"];

    for browser in browsers {
        let error = Error::BrowserNotInstalled {
            browser_name: browser.to_string(),
            message: format!("Browser '{}' is not installed", browser),
            playwright_version: PLAYWRIGHT_VERSION.to_string(),
        };

        let error_message = error.to_string();
        assert!(
            error_message.contains(browser),
            "Error for {} should mention the browser name",
            browser
        );
    }
}
