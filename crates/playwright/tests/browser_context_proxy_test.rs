// Tests for BrowserContext proxy option
//
// Verifies that proxy settings can be configured on browser contexts.

use playwright_rs::protocol::{BrowserContextOptions, ProxySettings, Viewport};

#[test]
fn test_proxy_settings_serialization() {
    // Verify ProxySettings serializes correctly to match Playwright's expected format
    let proxy = ProxySettings {
        server: "http://proxy.example.com:8080".to_string(),
        bypass: Some(".example.com, localhost".to_string()),
        username: Some("user".to_string()),
        password: Some("secret".to_string()),
    };

    let json = serde_json::to_value(&proxy).expect("Failed to serialize");

    assert_eq!(json["server"], "http://proxy.example.com:8080");
    assert_eq!(json["bypass"], ".example.com, localhost");
    assert_eq!(json["username"], "user");
    assert_eq!(json["password"], "secret");
}

#[test]
fn test_proxy_settings_minimal() {
    // Verify only server is required, optional fields are skipped
    let proxy = ProxySettings {
        server: "socks5://proxy:1080".to_string(),
        bypass: None,
        username: None,
        password: None,
    };

    let json = serde_json::to_value(&proxy).expect("Failed to serialize");

    assert_eq!(json["server"], "socks5://proxy:1080");
    assert!(json.get("bypass").is_none());
    assert!(json.get("username").is_none());
    assert!(json.get("password").is_none());
}

#[test]
fn test_browser_context_options_with_proxy() {
    // Verify proxy can be set via builder pattern
    let options = BrowserContextOptions::builder()
        .proxy(ProxySettings {
            server: "http://localhost:8888".to_string(),
            bypass: None,
            username: None,
            password: None,
        })
        .build();

    assert!(options.proxy.is_some());
    assert_eq!(options.proxy.unwrap().server, "http://localhost:8888");
}

#[test]
fn test_browser_context_options_serialization_with_proxy() {
    // Verify the full options struct serializes correctly with proxy
    let options = BrowserContextOptions::builder()
        .viewport(Viewport {
            width: 1920,
            height: 1080,
        })
        .proxy(ProxySettings {
            server: "http://proxy:3128".to_string(),
            bypass: Some(".internal.com".to_string()),
            username: None,
            password: None,
        })
        .build();

    let json = serde_json::to_value(&options).expect("Failed to serialize");

    // Verify viewport is present
    assert_eq!(json["viewport"]["width"], 1920);
    assert_eq!(json["viewport"]["height"], 1080);

    // Verify proxy is present and correctly formatted
    assert_eq!(json["proxy"]["server"], "http://proxy:3128");
    assert_eq!(json["proxy"]["bypass"], ".internal.com");
}

#[test]
fn test_proxy_settings_backward_compat_import() {
    // Verify ProxySettings can be imported from api module (backward compatibility)
    use playwright_rs::api::ProxySettings as ApiProxySettings;

    let proxy = ApiProxySettings {
        server: "http://test:8080".to_string(),
        bypass: None,
        username: None,
        password: None,
    };

    assert_eq!(proxy.server, "http://test:8080");
}
