// Playwright driver management
//
// Handles locating and managing the Playwright Node.js driver.
// Follows the same architecture as playwright-python, playwright-java, and playwright-dotnet.

use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get the path to the Playwright driver executable
///
/// This function attempts to locate the Playwright driver in the following order:
/// 1. Bundled driver downloaded by build.rs (PRIMARY - matches official bindings)
/// 2. PLAYWRIGHT_DRIVER_PATH environment variable (user override)
/// 3. PLAYWRIGHT_NODE_EXE and PLAYWRIGHT_CLI_JS environment variables (user override)
/// 4. Global npm installation (`npm root -g`) (development fallback)
/// 5. Local npm installation (`npm root`) (development fallback)
///
/// Returns a tuple of (node_executable_path, cli_js_path).
///
/// # Errors
///
/// Returns `Error::ServerNotFound` if the driver cannot be located in any of the search paths.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::server::driver::get_driver_executable;
///
/// let (node_exe, cli_js) = get_driver_executable()?;
/// println!("Node: {}", node_exe.display());
/// println!("CLI:  {}", cli_js.display());
/// # Ok::<(), playwright_rs::Error>(())
/// ```
pub fn get_driver_executable() -> Result<(PathBuf, PathBuf)> {
    // 1. Try bundled driver from build.rs (PRIMARY PATH - matches official bindings)
    if let Some(result) = try_bundled_driver()? {
        return Ok(result);
    }

    // 2. Try PLAYWRIGHT_DRIVER_PATH environment variable
    if let Some(result) = try_driver_path_env()? {
        return Ok(result);
    }

    // 3. Try PLAYWRIGHT_NODE_EXE and PLAYWRIGHT_CLI_JS environment variables
    if let Some(result) = try_node_cli_env()? {
        return Ok(result);
    }

    // 4. Try npm global installation (development fallback)
    if let Some(result) = try_npm_global()? {
        return Ok(result);
    }

    // 5. Try npm local installation (development fallback)
    if let Some(result) = try_npm_local()? {
        return Ok(result);
    }

    Err(Error::ServerNotFound)
}

/// Try to find bundled driver from build.rs
///
/// This is the PRIMARY path and matches how playwright-python, playwright-java,
/// and playwright-dotnet distribute their drivers.
fn try_bundled_driver() -> Result<Option<(PathBuf, PathBuf)>> {
    // Check if build.rs set the environment variables (compile-time)
    if let (Some(node_exe), Some(cli_js)) = (
        option_env!("PLAYWRIGHT_NODE_EXE"),
        option_env!("PLAYWRIGHT_CLI_JS"),
    ) {
        let node_path = PathBuf::from(node_exe);
        let cli_path = PathBuf::from(cli_js);

        if node_path.exists() && cli_path.exists() {
            return Ok(Some((node_path, cli_path)));
        }
    }

    // Fallback: Check PLAYWRIGHT_DRIVER_DIR and construct paths (compile-time)
    if let Some(driver_dir) = option_env!("PLAYWRIGHT_DRIVER_DIR") {
        let driver_path = PathBuf::from(driver_dir);
        let node_exe = if cfg!(windows) {
            driver_path.join("node.exe")
        } else {
            driver_path.join("node")
        };
        let cli_js = driver_path.join("package").join("cli.js");

        if node_exe.exists() && cli_js.exists() {
            return Ok(Some((node_exe, cli_js)));
        }
    }

    Ok(None)
}

/// Try to find driver from PLAYWRIGHT_DRIVER_PATH environment variable
///
/// User can set PLAYWRIGHT_DRIVER_PATH to a directory containing:
/// - node (or node.exe on Windows)
/// - package/cli.js
fn try_driver_path_env() -> Result<Option<(PathBuf, PathBuf)>> {
    if let Ok(driver_path) = std::env::var("PLAYWRIGHT_DRIVER_PATH") {
        let driver_dir = PathBuf::from(driver_path);
        let node_exe = if cfg!(windows) {
            driver_dir.join("node.exe")
        } else {
            driver_dir.join("node")
        };
        let cli_js = driver_dir.join("package").join("cli.js");

        if node_exe.exists() && cli_js.exists() {
            return Ok(Some((node_exe, cli_js)));
        }
    }

    Ok(None)
}

/// Try to find driver from PLAYWRIGHT_NODE_EXE and PLAYWRIGHT_CLI_JS environment variables
///
/// User can set both variables to explicitly specify paths.
fn try_node_cli_env() -> Result<Option<(PathBuf, PathBuf)>> {
    if let (Ok(node_exe), Ok(cli_js)) = (
        std::env::var("PLAYWRIGHT_NODE_EXE"),
        std::env::var("PLAYWRIGHT_CLI_JS"),
    ) {
        let node_path = PathBuf::from(node_exe);
        let cli_path = PathBuf::from(cli_js);

        if node_path.exists() && cli_path.exists() {
            return Ok(Some((node_path, cli_path)));
        }
    }

    Ok(None)
}

/// Try to find driver in npm global installation (development fallback)
fn try_npm_global() -> Result<Option<(PathBuf, PathBuf)>> {
    let output = Command::new("npm").args(["root", "-g"]).output();

    if let Ok(output) = output {
        if output.status.success() {
            let npm_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let node_modules = PathBuf::from(npm_root);
            if node_modules.exists() {
                if let Ok(paths) = find_playwright_in_node_modules(&node_modules) {
                    return Ok(Some(paths));
                }
            }
        }
    }

    Ok(None)
}

/// Try to find driver in npm local installation (development fallback)
fn try_npm_local() -> Result<Option<(PathBuf, PathBuf)>> {
    let output = Command::new("npm").args(["root"]).output();

    if let Ok(output) = output {
        if output.status.success() {
            let npm_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let node_modules = PathBuf::from(npm_root);
            if node_modules.exists() {
                if let Ok(paths) = find_playwright_in_node_modules(&node_modules) {
                    return Ok(Some(paths));
                }
            }
        }
    }

    Ok(None)
}

/// Find Playwright CLI in node_modules directory
///
/// Returns (node_executable, cli_js_path)
fn find_playwright_in_node_modules(node_modules: &Path) -> Result<(PathBuf, PathBuf)> {
    // Look for playwright or @playwright/test package
    let playwright_dirs = [
        node_modules.join("playwright"),
        node_modules.join("@playwright").join("test"),
    ];

    for playwright_dir in &playwright_dirs {
        if !playwright_dir.exists() {
            continue;
        }

        // Find cli.js in the package
        let cli_js = playwright_dir.join("cli.js");
        if !cli_js.exists() {
            continue;
        }

        // Find node executable from PATH
        if let Ok(node_exe) = find_node_executable() {
            return Ok((node_exe, cli_js));
        }
    }

    Err(Error::ServerNotFound)
}

/// Find the node executable in PATH or common locations
fn find_node_executable() -> Result<PathBuf> {
    // Try which/where command first
    #[cfg(not(windows))]
    let which_cmd = "which";
    #[cfg(windows)]
    let which_cmd = "where";

    if let Ok(output) = Command::new(which_cmd).arg("node").output() {
        if output.status.success() {
            let node_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !node_path.is_empty() {
                let path = PathBuf::from(node_path.lines().next().unwrap_or(&node_path));
                if path.exists() {
                    return Ok(path);
                }
            }
        }
    }

    // Try common locations
    #[cfg(not(windows))]
    let common_locations = [
        "/usr/local/bin/node",
        "/usr/bin/node",
        "/opt/homebrew/bin/node",
        "/opt/local/bin/node",
    ];

    #[cfg(windows)]
    let common_locations = [
        "C:\\Program Files\\nodejs\\node.exe",
        "C:\\Program Files (x86)\\nodejs\\node.exe",
    ];

    for location in &common_locations {
        let path = PathBuf::from(location);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(Error::LaunchFailed(
        "Node.js executable not found. Please install Node.js or set PLAYWRIGHT_NODE_EXE."
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_node_executable() {
        // This should succeed on any system with Node.js installed
        let result = find_node_executable();
        match result {
            Ok(node_path) => {
                println!("Found node at: {:?}", node_path);
                assert!(node_path.exists());
            }
            Err(e) => {
                println!(
                    "Node.js not found (expected if Node.js not installed): {:?}",
                    e
                );
                // Don't fail the test if Node.js is not installed
            }
        }
    }

    #[test]
    fn test_get_driver_executable() {
        // This test will pass if any driver source is available
        let result = get_driver_executable();
        match result {
            Ok((node, cli)) => {
                println!("Found Playwright driver:");
                println!("  Node: {:?}", node);
                println!("  CLI:  {:?}", cli);
                assert!(node.exists());
                assert!(cli.exists());
            }
            Err(Error::ServerNotFound) => {
                println!("Playwright driver not found (expected in some environments)");
                println!(
                    "This is OK - driver will be bundled at build time or can be installed via npm"
                );
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_bundled_driver_detection() {
        // Test that we can detect bundled driver if build.rs set env vars
        let result = try_bundled_driver();
        match result {
            Ok(Some((node, cli))) => {
                println!("Found bundled driver:");
                println!("  Node: {:?}", node);
                println!("  CLI:  {:?}", cli);
                assert!(node.exists());
                assert!(cli.exists());
            }
            Ok(None) => {
                println!("No bundled driver (expected during development)");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
}
