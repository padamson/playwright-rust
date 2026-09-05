// Playwright server management
//
// Handles downloading, launching, and managing the lifecycle of the Playwright
// Node.js server process.

use crate::server::driver::get_driver_executable;
use crate::{Error, Result};
use tokio::process::{Child, Command};
use tracing::Instrument;

/// Manages the Playwright server process lifecycle
///
/// The PlaywrightServer wraps a Node.js child process that runs the Playwright
/// driver. It communicates with the server via stdio pipes using JSON-RPC protocol.
///
/// # Example
///
/// ```no_run
/// # use playwright_rs::server::playwright_server::PlaywrightServer;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let server = PlaywrightServer::launch().await?;
/// // Use the server...
/// server.shutdown().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct PlaywrightServer {
    /// The Playwright server child process
    ///
    /// This is public to allow integration tests to access stdin/stdout pipes.
    /// In production code, you should use the Connection layer instead of
    /// accessing the process directly.
    pub process: Child,
}

impl PlaywrightServer {
    /// Launch the Playwright server process
    ///
    /// This will:
    /// 1. Check if the Playwright driver exists (download if needed)
    /// 2. Launch the server using `node <driver>/cli.js run-driver`
    /// 3. Set environment variable `PW_LANG_NAME=rust`
    ///
    /// # Errors
    ///
    /// Returns `Error::ServerNotFound` if the driver cannot be located,
    /// `Error::DriverMisconfigured` if a driver override environment variable
    /// is set but does not point at an existing file, and
    /// `Error::LaunchFailed` if the process fails to start.
    ///
    /// See: <https://playwright.dev/docs/api>
    pub async fn launch() -> Result<Self> {
        // Get the driver executable paths
        // The driver should already be downloaded by build.rs
        let (node_exe, cli_js) = get_driver_executable()?;

        // Launch the server process. Stderr is piped (not inherited)
        // because the Node driver writes terminal-capability queries
        // and other escape sequences to its stderr while alive. With
        // stderr inherited, those bytes clobber the user's tty and
        // break shell line-editing after a Ctrl-C while the driver is
        // still gracefully shutting down chromium (see #59). We drain
        // the piped stderr in a background task and forward each line
        // via `tracing::debug!` so users with tracing enabled can
        // still see driver diagnostics.
        let mut cmd = Command::new(&node_exe);
        cmd.arg(&cli_js)
            .arg("run-driver")
            .env("PW_LANG_NAME", "rust")
            .env("PW_LANG_NAME_VERSION", env!("CARGO_PKG_RUST_VERSION"))
            .env("PW_CLI_DISPLAY_VERSION", env!("CARGO_PKG_VERSION"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Put the Node driver in its own process group so a Ctrl-C in
        // the user's shell (which sends SIGINT to the foreground process
        // group) doesn't reach Node. When our process dies, Node's stdin
        // pipe closes and the driver runs `gracefullyProcessExitDoNotHang`
        // — a quiet, browser-aware shutdown. Without this isolation, Node
        // gets SIGINT'd alongside us and races a noisy EPIPE error path
        // that writes terminal-capability queries to stderr; the
        // terminal's responses then pollute bash's stdin buffer and
        // disrupt readline. See issue #59.
        // process_group is on tokio::process::Command directly (Unix
        // only). Pgid 0 means "make the child its own group leader"
        // (PGID == child PID).
        #[cfg(unix)]
        {
            cmd.process_group(0);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| Error::LaunchFailed(format!("Failed to spawn process: {}", e)))?;

        // Drain Node's stderr in a background task. Without an active
        // reader the kernel pipe buffer would eventually fill and
        // block the driver's writes; we don't want that. Bytes are
        // forwarded line-by-line via `tracing::debug!` so they're
        // accessible when needed without polluting the terminal.
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(
                async move {
                    use tokio::io::{AsyncBufReadExt, BufReader};
                    let mut lines = BufReader::new(stderr).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        tracing::debug!(target: "playwright_rs::driver_stderr", "{}", line);
                    }
                }
                .in_current_span(),
            );
        }

        // Check if process started successfully
        // Give it a moment to potentially fail
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(Error::LaunchFailed(format!(
                    "Server process exited immediately with status: {}",
                    status
                )));
            }
            Ok(None) => {
                // Process is still running, good!
            }
            Err(e) => {
                return Err(Error::LaunchFailed(format!(
                    "Failed to check process status: {}",
                    e
                )));
            }
        }

        Ok(Self { process: child })
    }

    /// How long to let the driver close its browsers before force-killing it.
    ///
    /// Teardown of a headed Chromium measures around 200ms; the bound is
    /// generous so a loaded machine still gets a clean exit, and it is only
    /// paid in full when the driver is genuinely wedged.
    pub const EXIT_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

    /// Wait for the driver to exit on its own, force-killing it if it doesn't.
    ///
    /// Blocking, so it can run from `Drop`. Call **after** closing the
    /// transport writer — that is what tells the driver to shut down; this
    /// only gives it the time to finish. Force-killing a driver that still
    /// owns browsers leaks them (see [`crate::protocol::Playwright`]'s `Drop`).
    ///
    /// Returns `true` if the driver exited by itself.
    pub fn wait_for_exit_blocking(&mut self, grace: std::time::Duration) -> bool {
        let deadline = std::time::Instant::now() + grace;
        loop {
            match self.process.try_wait() {
                // Already exited, or already reaped by the runtime.
                Ok(Some(_)) | Err(_) => return true,
                Ok(None) if std::time::Instant::now() >= deadline => break,
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(25)),
            }
        }

        tracing::warn!(
            "Playwright driver did not exit within {:?}; force-killing it. \
             Browsers it owns may survive as orphans.",
            grace
        );
        let _ = self.process.start_kill();
        false
    }

    /// Shut down the server gracefully
    ///
    /// Sends a shutdown signal to the server and waits for it to exit.
    ///
    /// # Platform-Specific Behavior
    ///
    /// **Windows**: Explicitly closes stdio pipes before killing the process to avoid
    /// hangs. On Windows, tokio uses a blocking threadpool for child process stdio,
    /// and failing to close pipes before terminating can cause the cleanup to hang
    /// indefinitely. Uses a timeout to prevent permanent hangs.
    ///
    /// **Unix**: Uses standard process termination with graceful wait.
    ///
    /// # Errors
    ///
    /// Returns an error if the shutdown fails or times out.
    pub async fn shutdown(mut self) -> Result<()> {
        #[cfg(windows)]
        {
            // Windows-specific cleanup: Close stdio pipes BEFORE killing process
            // This prevents hanging due to Windows' blocking threadpool for stdio
            drop(self.process.stdin.take());
            drop(self.process.stdout.take());
            drop(self.process.stderr.take());

            // The caller has closed the transport writer, so the driver is
            // already tearing its browsers down. Wait it out rather than
            // killing it — a killed driver leaves its browsers running.
            match tokio::time::timeout(Self::EXIT_GRACE, self.process.wait()).await {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(e)) => Err(Error::LaunchFailed(format!(
                    "Failed to wait for process: {}",
                    e
                ))),
                Err(_) => {
                    tracing::warn!(
                        "Playwright driver did not exit within {:?}; force-killing it. \
                         Browsers it owns may survive as orphans.",
                        Self::EXIT_GRACE
                    );
                    let _ = self.process.start_kill();
                    Err(Error::LaunchFailed(format!(
                        "Driver shutdown timed out after {:?}",
                        Self::EXIT_GRACE
                    )))
                }
            }
        }

        #[cfg(not(windows))]
        {
            // The caller has closed the transport writer, so the driver is
            // already tearing its browsers down. Wait it out rather than
            // killing it — a killed driver leaves its browsers running.
            match tokio::time::timeout(Self::EXIT_GRACE, self.process.wait()).await {
                Ok(_) => Ok(()),
                Err(_) => {
                    tracing::warn!(
                        "Playwright driver did not exit within {:?}; force-killing it. \
                         Browsers it owns may survive as orphans.",
                        Self::EXIT_GRACE
                    );
                    let _ = self.process.start_kill();
                    Err(Error::LaunchFailed(format!(
                        "Driver shutdown timed out after {:?}",
                        Self::EXIT_GRACE
                    )))
                }
            }
        }
    }

    /// Force kill the server process
    ///
    /// This should only be used if graceful shutdown fails.
    ///
    /// # Platform-Specific Behavior
    ///
    /// **Windows**: Closes stdio pipes before killing to prevent hangs.
    ///
    /// **Unix**: Standard force kill operation.
    ///
    /// # Errors
    ///
    /// Returns an error if the kill operation fails.
    pub async fn kill(mut self) -> Result<()> {
        #[cfg(windows)]
        {
            // Windows: Close pipes before killing
            drop(self.process.stdin.take());
            drop(self.process.stdout.take());
            drop(self.process.stderr.take());
        }

        self.process
            .kill()
            .await
            .map_err(|e| Error::LaunchFailed(format!("Failed to kill process: {}", e)))?;

        #[cfg(windows)]
        {
            // On Windows, wait with timeout
            let _ =
                tokio::time::timeout(std::time::Duration::from_secs(2), self.process.wait()).await;
        }

        #[cfg(not(windows))]
        {
            // On Unix, optionally wait (don't block)
            let _ =
                tokio::time::timeout(std::time::Duration::from_millis(500), self.process.wait())
                    .await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_launch_and_shutdown() {
        // This test will attempt to launch the Playwright server
        // If Playwright is not installed, it will try to download it
        let result = PlaywrightServer::launch().await;

        match result {
            Ok(server) => {
                tracing::info!("Server launched successfully!");
                // Clean shutdown
                let shutdown_result = server.shutdown().await;
                assert!(
                    shutdown_result.is_ok(),
                    "Shutdown failed: {:?}",
                    shutdown_result
                );
            }
            Err(Error::ServerNotFound) => {
                // This can happen if npm is not installed or download fails
                tracing::warn!(
                    "Could not launch server: Playwright not found and download may have failed"
                );
                tracing::warn!(
                    "To run this test, install Playwright manually: npm install playwright"
                );
                // Don't fail the test - this is expected in CI without Node.js
            }
            Err(Error::LaunchFailed(msg)) => {
                tracing::warn!("Launch failed: {}", msg);
                tracing::warn!("This may be expected if Node.js or npm is not installed");
                // Don't fail - expected in environments without Node.js
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_server_can_be_killed() {
        // Test that we can force-kill a server
        let result = PlaywrightServer::launch().await;

        if let Ok(server) = result {
            tracing::info!("Server launched, testing kill...");
            let kill_result = server.kill().await;
            assert!(kill_result.is_ok(), "Kill failed: {:?}", kill_result);
        } else {
            // Server didn't launch, that's okay for this test
            tracing::warn!("Server didn't launch (expected without Node.js/Playwright)");
        }
    }
}
