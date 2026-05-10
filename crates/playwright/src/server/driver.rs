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
/// 2. User cache populated by `playwright-rs install` (stable across cargo install)
/// 3. PLAYWRIGHT_DRIVER_PATH environment variable (user override)
/// 4. PLAYWRIGHT_NODE_EXE and PLAYWRIGHT_CLI_JS environment variables (user override)
/// 5. Global npm installation (`npm root -g`) (development fallback)
/// 6. Local npm installation (`npm root`) (development fallback)
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
    if let Some(result) = try_bundled_driver()? {
        return Ok(result);
    }

    if let Some(result) = try_user_cache_driver()? {
        return Ok(result);
    }

    if let Some(result) = try_driver_path_env()? {
        return Ok(result);
    }

    if let Some(result) = try_node_cli_env()? {
        return Ok(result);
    }

    if let Some(result) = try_npm_global()? {
        return Ok(result);
    }

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

/// Try to find driver in the user cache populated by `playwright-rs install`.
///
/// The CLI bootstrap drops the driver at
/// `<cache>/playwright-rust/<version>/playwright-<version>-<platform>/`,
/// which survives `cargo install` cleanup of the build's `target/`. The
/// version and platform come from compile-time env vars emitted by build.rs.
fn try_user_cache_driver() -> Result<Option<(PathBuf, PathBuf)>> {
    let Some(cache_dir) = dirs::cache_dir() else {
        return Ok(None);
    };
    let (Some(version), Some(platform)) = (
        option_env!("PLAYWRIGHT_DRIVER_VERSION"),
        option_env!("PLAYWRIGHT_DRIVER_PLATFORM"),
    ) else {
        return Ok(None);
    };
    try_user_cache_driver_in(&cache_dir, version, platform)
}

/// Resolution helper for `try_user_cache_driver` parameterised by cache root,
/// version, and platform — exposed at module scope so tests can drive it
/// with a `tempfile::tempdir()`.
fn try_user_cache_driver_in(
    cache_root: &Path,
    version: &str,
    platform: &str,
) -> Result<Option<(PathBuf, PathBuf)>> {
    let driver_dir = cache_root
        .join("playwright-rust")
        .join(version)
        .join(format!("playwright-{}-{}", version, platform));

    let node_exe = if platform.starts_with("win32") {
        driver_dir.join("node.exe")
    } else {
        driver_dir.join("node")
    };
    let cli_js = driver_dir.join("package").join("cli.js");

    if node_exe.exists() && cli_js.exists() {
        Ok(Some((node_exe, cli_js)))
    } else {
        Ok(None)
    }
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

    if let Ok(output) = output
        && output.status.success()
    {
        let npm_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let node_modules = PathBuf::from(npm_root);
        if node_modules.exists()
            && let Ok(paths) = find_playwright_in_node_modules(&node_modules)
        {
            return Ok(Some(paths));
        }
    }

    Ok(None)
}

/// Try to find driver in npm local installation (development fallback)
fn try_npm_local() -> Result<Option<(PathBuf, PathBuf)>> {
    let output = Command::new("npm").args(["root"]).output();

    if let Ok(output) = output
        && output.status.success()
    {
        let npm_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let node_modules = PathBuf::from(npm_root);
        if node_modules.exists()
            && let Ok(paths) = find_playwright_in_node_modules(&node_modules)
        {
            return Ok(Some(paths));
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

    if let Ok(output) = Command::new(which_cmd).arg("node").output()
        && output.status.success()
    {
        let node_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !node_path.is_empty() {
            let path = PathBuf::from(node_path.lines().next().unwrap_or(&node_path));
            if path.exists() {
                return Ok(path);
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

/// Install Playwright browsers programmatically.
///
/// Finds the bundled Playwright driver and runs:
/// `<driver>/node <driver>/package/cli.js install [browsers...]`
///
/// # Parameters
///
/// - `browsers` — optional slice of browser names (e.g. `&["chromium", "firefox"]`).
///   Pass `None` to install all browsers (equivalent to `npx playwright install`).
///   Pass `Some(&[])` for a no-op invocation that validates the driver is reachable.
///
/// On Linux, `--with-deps` is automatically appended so that required system
/// libraries (libgtk, libnss, etc.) are installed alongside the browser binaries.
/// Use [`install_browsers_with_deps`] to force this flag on other platforms.
///
/// # Errors
///
/// - [`Error::ServerNotFound`] if the Playwright driver cannot be located.
/// - [`Error::LaunchFailed`] if the installation process exits with a non-zero
///   status or fails to spawn.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::install_browsers;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Install only Chromium
///     install_browsers(Some(&["chromium"])).await?;
///
///     // Install all browsers
///     install_browsers(None).await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/browsers#installing-browsers>
pub async fn install_browsers(browsers: Option<&[&str]>) -> Result<()> {
    install_browsers_impl(browsers, /* with_deps_forced */ false).await
}

/// Install Playwright browsers and their system dependencies.
///
/// Identical to [`install_browsers`] but always passes `--with-deps` to the
/// Playwright CLI, regardless of the current operating system. This is the
/// recommended call for CI environments where system libraries may be missing.
///
/// # Parameters
///
/// - `browsers` — optional slice of browser names. `None` installs all browsers.
///
/// # Errors
///
/// - [`Error::ServerNotFound`] if the Playwright driver cannot be located.
/// - [`Error::LaunchFailed`] if the installation process exits with a non-zero
///   status or fails to spawn.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::install_browsers_with_deps;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     install_browsers_with_deps(Some(&["chromium", "firefox"])).await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/browsers#installing-browsers>
pub async fn install_browsers_with_deps(browsers: Option<&[&str]>) -> Result<()> {
    install_browsers_impl(browsers, /* with_deps_forced */ true).await
}

/// Internal implementation shared by [`install_browsers`] and [`install_browsers_with_deps`].
async fn install_browsers_impl(browsers: Option<&[&str]>, with_deps_forced: bool) -> Result<()> {
    let (node_exe, cli_js) = get_driver_executable()?;

    let mut cmd = tokio::process::Command::new(&node_exe);
    cmd.arg(&cli_js).arg("install");

    if let Some(browser_list) = browsers {
        for browser in browser_list {
            cmd.arg(browser);
        }
    }

    // Pass --with-deps on Linux automatically (needed for system libraries),
    // or when the caller explicitly requested it via install_browsers_with_deps.
    if with_deps_forced || cfg!(target_os = "linux") {
        cmd.arg("--with-deps");
    }

    let output = cmd.output().await.map_err(|e| {
        Error::LaunchFailed(format!("Failed to spawn browser install process: {}", e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(Error::LaunchFailed(format!(
            "Browser installation failed (exit code {:?}).\nstdout: {}\nstderr: {}",
            output.status.code(),
            stdout.trim(),
            stderr.trim(),
        )));
    }

    Ok(())
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
                tracing::info!("Found node at: {:?}", node_path);
                assert!(node_path.exists());
            }
            Err(e) => {
                tracing::warn!(
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
                tracing::info!("Found Playwright driver:");
                tracing::info!("  Node: {:?}", node);
                tracing::info!("  CLI:  {:?}", cli);
                assert!(node.exists());
                assert!(cli.exists());
            }
            Err(Error::ServerNotFound) => {
                tracing::warn!("Playwright driver not found (expected in some environments)");
                tracing::warn!(
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
                tracing::info!("Found bundled driver:");
                tracing::info!("  Node: {:?}", node);
                tracing::info!("  CLI:  {:?}", cli);
                assert!(node.exists());
                assert!(cli.exists());
            }
            Ok(None) => {
                tracing::info!("No bundled driver (expected during development)");
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn try_user_cache_driver_in_resolves_when_files_present() {
        let temp = tempfile::tempdir().unwrap();
        let driver_subdir = temp
            .path()
            .join("playwright-rust")
            .join("1.59.1")
            .join("playwright-1.59.1-linux");
        std::fs::create_dir_all(driver_subdir.join("package")).unwrap();
        std::fs::write(driver_subdir.join("node"), b"").unwrap();
        std::fs::write(driver_subdir.join("package").join("cli.js"), b"").unwrap();

        let (node, cli) = try_user_cache_driver_in(temp.path(), "1.59.1", "linux")
            .unwrap()
            .unwrap();
        assert!(node.exists());
        assert!(cli.exists());
    }

    #[test]
    fn try_user_cache_driver_in_returns_none_when_absent() {
        let temp = tempfile::tempdir().unwrap();
        let result = try_user_cache_driver_in(temp.path(), "1.59.1", "linux").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn bundled_driver_dir_lives_under_out_dir() {
        let dir = env!("PLAYWRIGHT_DRIVER_DIR");
        let sep = std::path::MAIN_SEPARATOR;
        let build_marker = format!("{sep}build{sep}playwright-rs");
        let out_marker = format!("{sep}out{sep}");
        assert!(
            dir.contains(&build_marker) && dir.contains(&out_marker),
            "PLAYWRIGHT_DRIVER_DIR should sit under target/<profile>/build/playwright-rs-<hash>/out, got: {dir}"
        );
    }

    #[test]
    fn try_user_cache_driver_in_uses_node_exe_for_windows_platforms() {
        let temp = tempfile::tempdir().unwrap();
        let driver_subdir = temp
            .path()
            .join("playwright-rust")
            .join("1.59.1")
            .join("playwright-1.59.1-win32_x64");
        std::fs::create_dir_all(driver_subdir.join("package")).unwrap();
        std::fs::write(driver_subdir.join("node.exe"), b"").unwrap();
        std::fs::write(driver_subdir.join("package").join("cli.js"), b"").unwrap();

        let (node, _cli) = try_user_cache_driver_in(temp.path(), "1.59.1", "win32_x64")
            .unwrap()
            .unwrap();
        assert!(
            node.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(".exe")
        );
    }
}
