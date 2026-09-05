// Playwright driver management
//
// Handles locating and managing the Playwright Node.js driver.
// Follows the same architecture as playwright-python, playwright-java, and playwright-dotnet.

use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Get the path to the Playwright driver executable
///
/// The driver directory is located in this order:
/// 1. `PLAYWRIGHT_DRIVER_PATH` (user override: a directory holding `node`, or
///    `node.exe` on Windows, and `package/cli.js`)
/// 2. Bundled driver assembled by build.rs (the default; matches official bindings)
/// 3. User cache populated by `playwright-rs install` (stable across cargo install)
/// 4. Global npm installation (`npm root -g`) (development fallback)
/// 5. Local npm installation (`npm root`) (development fallback)
///
/// On top of that, `PLAYWRIGHT_NODE_EXE` replaces the runtime and
/// `PLAYWRIGHT_CLI_JS` replaces the CLI script, each independently. Setting
/// only `PLAYWRIGHT_NODE_EXE` runs the resolved driver's `cli.js` under a
/// different runtime, which is how to try the driver under Bun. A bare
/// command name such as `bun` is left for `PATH` lookup at spawn; anything
/// with a directory component must exist. When both are set no driver
/// directory is needed at all.
///
/// An override that is set but does not point at an existing file is an
/// error rather than a fall-through: silently spawning the bundled driver
/// would hide the misconfiguration. An exported-but-empty variable counts as
/// unset.
///
/// Returns a tuple of (node_executable_path, cli_js_path).
///
/// # Errors
///
/// Returns `Error::DriverMisconfigured` if an override environment variable
/// is set but does not point at an existing file, and `Error::ServerNotFound`
/// if no driver can be located in any of the search paths.
///
/// # Example
///
/// ```no_run
/// use playwright_rs::server::driver::get_driver_executable;
///
/// let (node_exe, cli_js) = get_driver_executable()?;
/// println!("Node: {}", node_exe.display());
/// println!("CLI:  {}", cli_js.display());
/// # Ok::<(), playwright_rs::Error>(())
/// ```
pub fn get_driver_executable() -> Result<(PathBuf, PathBuf)> {
    resolve_driver(|name| std::env::var(name).ok())
}

/// Resolution behind `get_driver_executable`, parameterized by the
/// process-environment lookup so it can be tested with a fake environment
/// instead of mutating the real one.
fn resolve_driver(env: impl Fn(&str) -> Option<String>) -> Result<(PathBuf, PathBuf)> {
    // Exported-but-empty counts as unset, the convention build.rs already
    // uses for PLAYWRIGHT_DRIVER_DIR.
    let env = |name: &str| env(name).filter(|value| !value.is_empty());

    let driver = resolve_driver_dir(&env)?;
    let runtime = env("PLAYWRIGHT_NODE_EXE")
        .map(runtime_override)
        .transpose()?;
    let cli = env("PLAYWRIGHT_CLI_JS")
        .map(|value| existing_file("PLAYWRIGHT_CLI_JS", value))
        .transpose()?;

    match (runtime, cli, driver) {
        (Some(runtime), Some(cli), _) => Ok((runtime, cli)),
        (runtime, cli, Some((driver_node, driver_cli))) => {
            Ok((runtime.unwrap_or(driver_node), cli.unwrap_or(driver_cli)))
        }
        (_, _, None) => Err(Error::ServerNotFound),
    }
}

/// The driver directory's `(node, cli.js)` pair: the `PLAYWRIGHT_DRIVER_PATH`
/// override first, then the bundled, user-cache, and npm locations. `None`
/// when nothing is found; an error only when the override is set but wrong.
fn resolve_driver_dir(env: &impl Fn(&str) -> Option<String>) -> Result<Option<(PathBuf, PathBuf)>> {
    if let Some(found) = try_driver_path_env(env)? {
        return Ok(Some(found));
    }
    if let Some(found) = try_bundled_driver()? {
        return Ok(Some(found));
    }
    if let Some(found) = try_user_cache_driver()? {
        return Ok(Some(found));
    }
    if let Some(found) = try_npm_global()? {
        return Ok(Some(found));
    }
    if let Some(found) = try_npm_local()? {
        return Ok(Some(found));
    }
    Ok(None)
}

/// The two files every driver directory holds: the runtime binary (`node`,
/// or `node.exe` for Windows) and the CLI script at `package/cli.js`.
fn driver_layout(driver_dir: &Path, windows: bool) -> (PathBuf, PathBuf) {
    let node_exe = driver_dir.join(if windows { "node.exe" } else { "node" });
    let cli_js = driver_dir.join("package").join("cli.js");
    (node_exe, cli_js)
}

/// `PLAYWRIGHT_NODE_EXE`: a bare command name (no directory component) is
/// left for `PATH` lookup at spawn; anything else must exist.
fn runtime_override(value: String) -> Result<PathBuf> {
    let is_bare_name = Path::new(&value).components().count() == 1;
    if is_bare_name {
        Ok(PathBuf::from(value))
    } else {
        existing_file("PLAYWRIGHT_NODE_EXE", value)
    }
}

fn existing_file(variable: &str, value: String) -> Result<PathBuf> {
    let path = PathBuf::from(value);
    if path.exists() {
        Ok(path)
    } else {
        Err(Error::DriverMisconfigured(format!(
            "{variable} is set to {} but that file does not exist",
            path.display()
        )))
    }
}

/// Try to find the bundled driver assembled by build.rs
///
/// This is the default path and matches how playwright-python, playwright-java,
/// and playwright-dotnet distribute their drivers. build.rs emits an empty
/// `PLAYWRIGHT_DRIVER_DIR` when the download was skipped or failed.
fn try_bundled_driver() -> Result<Option<(PathBuf, PathBuf)>> {
    let Some(driver_dir) = option_env!("PLAYWRIGHT_DRIVER_DIR").filter(|dir| !dir.is_empty())
    else {
        return Ok(None);
    };
    let (node_exe, cli_js) = driver_layout(Path::new(driver_dir), cfg!(windows));
    if node_exe.exists() && cli_js.exists() {
        Ok(Some((node_exe, cli_js)))
    } else {
        Ok(None)
    }
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

/// Resolution helper for `try_user_cache_driver` parameterized by cache root,
/// version, and platform, exposed at module scope so tests can drive it
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
    let (node_exe, cli_js) = driver_layout(&driver_dir, platform.starts_with("win32"));

    if node_exe.exists() && cli_js.exists() {
        Ok(Some((node_exe, cli_js)))
    } else {
        Ok(None)
    }
}

/// Try to find driver from the `PLAYWRIGHT_DRIVER_PATH` override: a directory
/// holding `node` (or `node.exe` on Windows) and `package/cli.js`.
///
/// Returns `None` when the variable is unset and an error when it is set but
/// the directory does not hold both files.
fn try_driver_path_env(
    env: &impl Fn(&str) -> Option<String>,
) -> Result<Option<(PathBuf, PathBuf)>> {
    let Some(driver_path) = env("PLAYWRIGHT_DRIVER_PATH") else {
        return Ok(None);
    };
    let driver_dir = PathBuf::from(driver_path);
    let (node_exe, cli_js) = driver_layout(&driver_dir, cfg!(windows));

    if node_exe.exists() && cli_js.exists() {
        Ok(Some((node_exe, cli_js)))
    } else {
        Err(Error::DriverMisconfigured(format!(
            "PLAYWRIGHT_DRIVER_PATH is set to {} but it does not contain both {} and package/cli.js",
            driver_dir.display(),
            if cfg!(windows) { "node.exe" } else { "node" }
        )))
    }
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
///   `Some(&[])` sends the same bare `install`, so it also installs the default
///   browsers; it is not a no-op probe.
///
/// Installs browsers only, on every platform — matching `npx playwright
/// install`. On Linux the browsers also need system libraries (libgtk,
/// libnss, etc.); use [`install_browsers_with_deps`] to install those
/// alongside, which runs the system package manager under `sudo`.
///
/// # Output
///
/// The installer's stdout and stderr are streamed to this process's stdout and
/// stderr as it runs, so download progress is visible and a stall is
/// distinguishable from progress. Callers that must not emit to the terminal
/// (a TUI, a captured test harness) should redirect the process's own streams.
/// The output is also retained and included in the error on failure.
///
/// # Errors
///
/// - [`Error::ServerNotFound`] if the Playwright driver cannot be located.
/// - [`Error::DriverMisconfigured`] if a driver override environment variable
///   is set but does not point at an existing file.
/// - [`Error::LaunchFailed`] if the installation process exits with a non-zero
///   status or fails to spawn.
///
/// # Example
///
/// ```no_run
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
/// Installing those libraries invokes the system package manager under `sudo`.
///
/// # Parameters
///
/// - `browsers` — optional slice of browser names. `None` installs all browsers.
///
/// # Output
///
/// Streams the installer's output live, like [`install_browsers`].
///
/// # Errors
///
/// - [`Error::ServerNotFound`] if the Playwright driver cannot be located.
/// - [`Error::DriverMisconfigured`] if a driver override environment variable
///   is set but does not point at an existing file.
/// - [`Error::LaunchFailed`] if the installation process exits with a non-zero
///   status or fails to spawn.
///
/// # Example
///
/// ```no_run
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

/// The driver CLI arguments for an install, in the order they are passed.
///
/// Split out as a pure function so the argument contract is unit-testable
/// without spawning a process: `--with-deps` appears only when asked for.
fn install_args(browsers: Option<&[&str]>, with_deps: bool) -> Vec<String> {
    let mut args = vec!["install".to_string()];
    if let Some(browser_list) = browsers {
        args.extend(browser_list.iter().map(|b| (*b).to_string()));
    }
    if with_deps {
        args.push("--with-deps".to_string());
    }
    args
}

/// Stream one of the child's pipes to `out`, keeping a copy for the error path.
///
/// Copies bytes rather than lines: the installer draws `\r`-updated progress
/// bars, which line buffering would hold back until the download finished.
async fn tee<R, W>(mut reader: R, mut out: W) -> Vec<u8>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut captured = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                // A closed/redirected stdout must not fail the install itself.
                let _ = out.write_all(&buf[..n]).await;
                let _ = out.flush().await;
                captured.extend_from_slice(&buf[..n]);
            }
            // No retry arm here on purpose: any `continue` on a reader that
            // keeps failing is an unbounded loop, and tokio's io driver
            // already absorbs EINTR for the pipes we hand it.
            Err(e) => {
                // Mark it rather than break quietly: the capture feeds the
                // failure message, and silently truncated diagnostics are the
                // exact problem this function exists to fix.
                captured.extend_from_slice(
                    format!("\n[playwright-rs: output truncated, read error: {e}]\n").as_bytes(),
                );
                break;
            }
        }
    }
    captured
}

/// Internal implementation shared by [`install_browsers`] and [`install_browsers_with_deps`].
async fn install_browsers_impl(browsers: Option<&[&str]>, with_deps_forced: bool) -> Result<()> {
    let (node_exe, cli_js) = get_driver_executable()?;

    let mut cmd = tokio::process::Command::new(&node_exe);
    cmd.arg(&cli_js);
    // --with-deps is opt-in on every platform, matching upstream. Linux used
    // to get it implicitly, which ran sudo apt-get for a call that only asked
    // for browsers and made the CLI's own --with-deps flag a no-op there.
    cmd.args(install_args(browsers, with_deps_forced));
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        Error::LaunchFailed(format!("Failed to spawn browser install process: {}", e))
    })?;

    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        return Err(Error::LaunchFailed(
            "browser install process is missing the stdio pipes it was configured with".to_string(),
        ));
    };

    // Stream both pipes while the child runs. Installing browsers takes minutes
    // and can stall on a slow mirror or a contended package manager; buffering
    // until exit makes a stall indistinguishable from progress.
    let (out_bytes, err_bytes, status) = tokio::join!(
        tee(stdout, tokio::io::stdout()),
        tee(stderr, tokio::io::stderr()),
        child.wait(),
    );

    let status = status.map_err(|e| {
        Error::LaunchFailed(format!("Browser install process was not reaped: {}", e))
    })?;

    if !status.success() {
        let stdout = String::from_utf8_lossy(&out_bytes);
        let stderr = String::from_utf8_lossy(&err_bytes);
        return Err(Error::LaunchFailed(format!(
            "Browser installation failed (exit code {:?}).\nstdout: {}\nstderr: {}",
            status.code(),
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
    fn install_args_request_system_deps_only_when_asked() {
        assert_eq!(
            install_args(Some(&["chromium"]), false),
            vec!["install", "chromium"],
            "--with-deps is opt-in upstream; it must never be added unasked"
        );
        assert_eq!(
            install_args(Some(&["chromium"]), true),
            vec!["install", "chromium", "--with-deps"]
        );
    }

    #[test]
    fn install_args_pass_browsers_through_in_order() {
        assert_eq!(
            install_args(Some(&["chromium", "firefox", "webkit"]), false),
            vec!["install", "chromium", "firefox", "webkit"]
        );
    }

    #[tokio::test]
    async fn tee_marks_a_read_error_instead_of_truncating_silently() {
        struct FailAfterFirstChunk(bool);
        impl tokio::io::AsyncRead for FailAfterFirstChunk {
            fn poll_read(
                mut self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
                buf: &mut tokio::io::ReadBuf<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                if self.0 {
                    return std::task::Poll::Ready(Err(std::io::Error::other("pipe died")));
                }
                self.0 = true;
                buf.put_slice(b"downloading...");
                std::task::Poll::Ready(Ok(()))
            }
        }

        let captured = tee(FailAfterFirstChunk(false), tokio::io::sink()).await;
        let text = String::from_utf8_lossy(&captured);
        assert!(text.starts_with("downloading..."), "keeps what it did read");
        assert!(
            text.contains("output truncated") && text.contains("pipe died"),
            "a lost pipe must be visible in the capture, not silent: {text}"
        );
    }

    #[tokio::test]
    async fn tee_streams_and_captures_the_same_bytes() {
        let input: &[u8] = b"downloading 12%\rdownloading 100%\ndone\n";
        let mut streamed = Vec::new();
        let captured = tee(input, &mut streamed).await;
        assert_eq!(captured, input, "capture feeds the failure message");
        assert_eq!(streamed, input, "streaming is what makes progress visible");
    }

    #[test]
    fn install_args_without_a_browser_list_install_the_defaults() {
        // Both spellings produce a bare `install`, which the driver reads as
        // "install the default browsers". Notably `Some(&[])` is *not* a no-op,
        // though an integration test long described it as one.
        assert_eq!(install_args(None, false), vec!["install"]);
        assert_eq!(install_args(Some(&[]), false), vec!["install"]);
        assert_eq!(install_args(None, true), vec!["install", "--with-deps"]);
    }

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
        // Passes if any driver source is available; ignores the real process
        // environment so a stale override in a developer's shell cannot fail it
        let result = resolve_driver(|_| None);
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
            .join("1.60.0")
            .join("playwright-1.60.0-linux");
        std::fs::create_dir_all(driver_subdir.join("package")).unwrap();
        std::fs::write(driver_subdir.join("node"), b"").unwrap();
        std::fs::write(driver_subdir.join("package").join("cli.js"), b"").unwrap();

        let (node, cli) = try_user_cache_driver_in(temp.path(), "1.60.0", "linux")
            .unwrap()
            .unwrap();
        assert!(node.exists());
        assert!(cli.exists());
    }

    #[test]
    fn try_user_cache_driver_in_returns_none_when_absent() {
        let temp = tempfile::tempdir().unwrap();
        let result = try_user_cache_driver_in(temp.path(), "1.60.0", "linux").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn bundled_driver_dir_lives_under_out_dir() {
        // Only meaningful for the default download location. CI relocates the
        // driver via PLAYWRIGHT_DRIVER_CACHE_DIR (cached on its own key) and
        // compile-only jobs skip the download entirely; in those modes the
        // OUT_DIR layout intentionally does not apply.
        if env!("PLAYWRIGHT_DRIVER_DIR_SOURCE") != "out_dir" {
            return;
        }
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
            .join("1.60.0")
            .join("playwright-1.60.0-win32_x64");
        std::fs::create_dir_all(driver_subdir.join("package")).unwrap();
        std::fs::write(driver_subdir.join("node.exe"), b"").unwrap();
        std::fs::write(driver_subdir.join("package").join("cli.js"), b"").unwrap();

        let (node, _cli) = try_user_cache_driver_in(temp.path(), "1.60.0", "win32_x64")
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

    fn fake_driver_dir(temp: &Path) -> (PathBuf, PathBuf) {
        let (node, cli) = driver_layout(temp, cfg!(windows));
        std::fs::create_dir_all(cli.parent().unwrap()).unwrap();
        std::fs::write(&node, b"").unwrap();
        std::fs::write(&cli, b"").unwrap();
        (node, cli)
    }

    fn env_of(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
    }

    fn s(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn driver_path_override_wins_over_the_bundled_driver() {
        let temp = tempfile::tempdir().unwrap();
        let (node, cli) = fake_driver_dir(temp.path());
        let env = env_of(&[("PLAYWRIGHT_DRIVER_PATH", &s(temp.path()))]);

        assert_eq!(resolve_driver(env).unwrap(), (node, cli));
    }

    #[test]
    fn node_exe_override_swaps_the_runtime_and_keeps_the_resolved_cli() {
        let temp = tempfile::tempdir().unwrap();
        let (_node, cli) = fake_driver_dir(temp.path());
        let other_runtime = temp.path().join("other-runtime");
        std::fs::write(&other_runtime, b"").unwrap();
        let env = env_of(&[
            ("PLAYWRIGHT_DRIVER_PATH", &s(temp.path())),
            ("PLAYWRIGHT_NODE_EXE", &s(&other_runtime)),
        ]);

        assert_eq!(resolve_driver(env).unwrap(), (other_runtime, cli));
    }

    #[test]
    fn cli_js_override_swaps_the_cli_and_keeps_the_resolved_runtime() {
        let temp = tempfile::tempdir().unwrap();
        let (node, _cli) = fake_driver_dir(temp.path());
        let other_cli = temp.path().join("other-cli.js");
        std::fs::write(&other_cli, b"").unwrap();
        let env = env_of(&[
            ("PLAYWRIGHT_DRIVER_PATH", &s(temp.path())),
            ("PLAYWRIGHT_CLI_JS", &s(&other_cli)),
        ]);

        assert_eq!(resolve_driver(env).unwrap(), (node, other_cli));
    }

    #[test]
    fn node_exe_and_cli_js_together_win_over_the_bundled_driver() {
        let temp = tempfile::tempdir().unwrap();
        let (node, cli) = fake_driver_dir(temp.path());
        let env = env_of(&[
            ("PLAYWRIGHT_NODE_EXE", &s(&node)),
            ("PLAYWRIGHT_CLI_JS", &s(&cli)),
        ]);

        assert_eq!(resolve_driver(env).unwrap(), (node, cli));
    }

    #[test]
    fn a_bare_command_name_runtime_is_left_to_path_lookup() {
        let temp = tempfile::tempdir().unwrap();
        let (_node, cli) = fake_driver_dir(temp.path());
        let env = env_of(&[
            ("PLAYWRIGHT_DRIVER_PATH", &s(temp.path())),
            ("PLAYWRIGHT_NODE_EXE", "bun"),
        ]);

        assert_eq!(resolve_driver(env).unwrap(), (PathBuf::from("bun"), cli));
    }

    #[test]
    fn an_empty_override_counts_as_unset() {
        let temp = tempfile::tempdir().unwrap();
        let (node, cli) = fake_driver_dir(temp.path());
        let env = env_of(&[
            ("PLAYWRIGHT_DRIVER_PATH", ""),
            ("PLAYWRIGHT_NODE_EXE", &s(&node)),
            ("PLAYWRIGHT_CLI_JS", &s(&cli)),
        ]);

        assert_eq!(resolve_driver(env).unwrap(), (node, cli));
    }

    #[test]
    fn a_runtime_override_pointing_at_a_missing_file_is_an_error() {
        let temp = tempfile::tempdir().unwrap();
        fake_driver_dir(temp.path());
        let missing = temp.path().join("no-such-runtime");
        let env = env_of(&[
            ("PLAYWRIGHT_DRIVER_PATH", &s(temp.path())),
            ("PLAYWRIGHT_NODE_EXE", &s(&missing)),
        ]);

        let err = resolve_driver(env).unwrap_err();

        assert!(
            matches!(&err, Error::DriverMisconfigured(msg) if msg.contains("PLAYWRIGHT_NODE_EXE")),
            "expected DriverMisconfigured naming the variable, got {err:?}"
        );
    }

    #[test]
    fn a_cli_override_pointing_at_a_missing_file_is_an_error() {
        let temp = tempfile::tempdir().unwrap();
        fake_driver_dir(temp.path());
        let missing = temp.path().join("no-such-cli.js");
        let env = env_of(&[
            ("PLAYWRIGHT_DRIVER_PATH", &s(temp.path())),
            ("PLAYWRIGHT_CLI_JS", &s(&missing)),
        ]);

        let err = resolve_driver(env).unwrap_err();

        assert!(
            matches!(&err, Error::DriverMisconfigured(msg) if msg.contains("PLAYWRIGHT_CLI_JS")),
            "expected DriverMisconfigured naming the variable, got {err:?}"
        );
    }

    #[test]
    fn a_driver_path_override_with_only_the_runtime_in_it_is_an_error() {
        let temp = tempfile::tempdir().unwrap();
        let (_node, cli) = fake_driver_dir(temp.path());
        std::fs::remove_file(&cli).unwrap();
        let env = env_of(&[("PLAYWRIGHT_DRIVER_PATH", &s(temp.path()))]);

        let err = resolve_driver(env).unwrap_err();

        assert!(
            matches!(&err, Error::DriverMisconfigured(msg) if msg.contains("package/cli.js")),
            "expected DriverMisconfigured naming the missing file, got {err:?}"
        );
    }

    #[test]
    fn a_driver_path_override_without_a_driver_in_it_is_an_error() {
        let temp = tempfile::tempdir().unwrap();
        let env = env_of(&[("PLAYWRIGHT_DRIVER_PATH", &s(temp.path()))]);

        let err = resolve_driver(env).unwrap_err();

        assert!(
            matches!(&err, Error::DriverMisconfigured(msg) if msg.contains("PLAYWRIGHT_DRIVER_PATH")),
            "expected DriverMisconfigured naming the variable, got {err:?}"
        );
    }

    #[test]
    fn without_overrides_resolution_still_reaches_the_bundled_driver() {
        let Some(bundled) = try_bundled_driver().unwrap() else {
            return; // compile-only builds skip the driver download
        };

        assert_eq!(resolve_driver(|_| None).unwrap(), bundled);
    }
}
