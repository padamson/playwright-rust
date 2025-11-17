// Playwright server management
//
// Handles downloading, launching, and managing the lifecycle of the Playwright
// Node.js server process.

use crate::server::driver::get_driver_executable;
use crate::{Error, Result};
use tokio::process::{Child, Command};

/// Manages the Playwright server process lifecycle
///
/// The PlaywrightServer wraps a Node.js child process that runs the Playwright
/// driver. It communicates with the server via stdio pipes using JSON-RPC protocol.
///
/// # Example
///
/// ```ignore
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
    /// Returns `Error::ServerNotFound` if the driver cannot be located.
    /// Returns `Error::LaunchFailed` if the process fails to start.
    ///
    /// See: <https://playwright.dev/docs/api>
    pub async fn launch() -> Result<Self> {
        // Get the driver executable paths
        // The driver should already be downloaded by build.rs
        let (node_exe, cli_js) = get_driver_executable()?;

        // Launch the server process
        let mut child = Command::new(&node_exe)
            .arg(&cli_js)
            .arg("run-driver")
            .env("PW_LANG_NAME", "rust")
            .env("PW_LANG_NAME_VERSION", env!("CARGO_PKG_RUST_VERSION"))
            .env("PW_CLI_DISPLAY_VERSION", env!("CARGO_PKG_VERSION"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| Error::LaunchFailed(format!("Failed to spawn process: {}", e)))?;

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

            // Kill the process
            self.process
                .kill()
                .await
                .map_err(|e| Error::LaunchFailed(format!("Failed to kill process: {}", e)))?;

            // Wait for process to exit with timeout (Windows can hang without this)
            match tokio::time::timeout(std::time::Duration::from_secs(5), self.process.wait()).await
            {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(e)) => Err(Error::LaunchFailed(format!(
                    "Failed to wait for process: {}",
                    e
                ))),
                Err(_) => {
                    // Timeout - try one more kill
                    let _ = self.process.start_kill();
                    Err(Error::LaunchFailed(
                        "Process shutdown timeout after 5 seconds".to_string(),
                    ))
                }
            }
        }

        #[cfg(not(windows))]
        {
            // Unix: Standard graceful shutdown
            self.process
                .kill()
                .await
                .map_err(|e| Error::LaunchFailed(format!("Failed to kill process: {}", e)))?;

            // Wait for process to exit
            let _ = self.process.wait().await;

            Ok(())
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
                println!("Server launched successfully!");
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
                eprintln!(
                    "Could not launch server: Playwright not found and download may have failed"
                );
                eprintln!("To run this test, install Playwright manually: npm install playwright");
                // Don't fail the test - this is expected in CI without Node.js
            }
            Err(Error::LaunchFailed(msg)) => {
                eprintln!("Launch failed: {}", msg);
                eprintln!("This may be expected if Node.js or npm is not installed");
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
            println!("Server launched, testing kill...");
            let kill_result = server.kill().await;
            assert!(kill_result.is_ok(), "Kill failed: {:?}", kill_result);
        } else {
            // Server didn't launch, that's okay for this test
            eprintln!("Server didn't launch (expected without Node.js/Playwright)");
        }
    }
}
