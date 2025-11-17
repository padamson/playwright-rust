// Windows CI Browser Launch Fix - Alternative Approach
//
// This module provides Windows-specific workarounds for browser launch hanging in CI.
// If the browser flags approach doesn't work, this provides more aggressive fixes.

use std::process::Command;

/// Kill all browser processes on Windows before launching new ones
#[cfg(windows)]
pub fn kill_browser_processes() {
    let browsers = vec!["chromium", "chrome", "firefox", "msedge", "webkit", "pw_"];

    for browser in browsers {
        let _ = Command::new("taskkill")
            .args(&["/F", "/IM", &format!("{}*.exe", browser), "/T"])
            .output();
    }

    // Small delay to ensure processes are fully terminated
    std::thread::sleep(std::time::Duration::from_millis(100));
}

/// Alternative browser launch arguments for Windows CI
pub fn get_windows_ci_args() -> Vec<String> {
    vec![
        // More aggressive flags
        "--no-sandbox".to_string(),
        "--disable-setuid-sandbox".to_string(),
        "--disable-dev-shm-usage".to_string(),
        "--disable-accelerated-2d-canvas".to_string(),
        "--no-first-run".to_string(),
        "--no-zygote".to_string(),
        "--single-process".to_string(),  // Run everything in one process
        "--disable-gpu".to_string(),
        "--disable-web-security".to_string(),
        "--disable-features=IsolateOrigins,site-per-process".to_string(),
        "--disable-blink-features=AutomationControlled".to_string(),
        "--disable-extensions".to_string(),
        "--disable-default-apps".to_string(),
        "--disable-backgrounding-occluded-windows".to_string(),
        "--disable-renderer-backgrounding".to_string(),
        "--disable-background-timer-throttling".to_string(),
        "--disable-ipc-flooding-protection".to_string(),
        "--password-store=basic".to_string(),
        "--use-mock-keychain".to_string(),
    ]
}

/// Alternative approach: Launch browser with explicit Windows settings
#[cfg(windows)]
pub fn prepare_windows_launch_environment() {
    // Set environment variables that might help
    std::env::set_var("PLAYWRIGHT_BROWSERS_PATH", "0");  // Use default path
    std::env::set_var("PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD", "0");
    std::env::set_var("DEBUG", "pw:api,pw:browser,pw:protocol");  // Enable debug logs

    // Kill any existing browser processes
    kill_browser_processes();
}
