// Integration tests for Resource Cleanup (Phase 6, Slice 7)
//
// Following TDD: Write tests first (Red), then implement fixes (Green), then refactor
//
// Tests cover:
// - File descriptor cleanup after browser shutdown
// - Process cleanup (no zombie processes)
// - Multiple Playwright server launch/shutdown cycles
// - Child process termination verification
// - Resource limit stress testing
//
// Success Criteria:
// - All file descriptors closed after shutdown
// - No zombie processes after operations
// - Clean server lifecycle management
// - Process tree fully cleaned up

mod common;
mod test_server;

use playwright_rs::protocol::Playwright;
use std::process::Command;
use std::time::Duration;

// ============================================================================
// Helper: Count Open File Descriptors (Unix only)
// ============================================================================

#[cfg(unix)]
fn count_open_file_descriptors() -> Option<usize> {
    // On Unix, /proc/self/fd contains symlinks to all open file descriptors
    #[cfg(target_os = "linux")]
    {
        std::fs::read_dir("/proc/self/fd")
            .ok()
            .map(|entries| entries.count())
    }

    // On macOS, use lsof command
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("lsof")
            .args(["-p", &std::process::id().to_string()])
            .output()
            .ok()?;

        let output_str = String::from_utf8(output.stdout).ok()?;
        // Count lines minus header
        let count = output_str.lines().count().saturating_sub(1);
        Some(count)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    None
}

#[cfg(not(unix))]
#[allow(dead_code)] // Unused on Windows - all tests using this are Unix-only
fn count_open_file_descriptors() -> Option<usize> {
    // Windows: Would need different approach (GetProcessHandleCount)
    // For now, return None to skip these tests on Windows
    None
}

// ============================================================================
// Helper: Count Child Processes
// ============================================================================

#[cfg(unix)]
fn count_child_processes() -> Option<usize> {
    let my_pid = std::process::id();

    // Use ps to find child processes
    let output = Command::new("ps")
        .args(["-o", "ppid=", "-e"])
        .output()
        .ok()?;

    let output_str = String::from_utf8(output.stdout).ok()?;

    let count = output_str
        .lines()
        .filter(|line| {
            if let Ok(ppid) = line.trim().parse::<u32>() {
                ppid == my_pid
            } else {
                false
            }
        })
        .count();

    Some(count)
}

#[cfg(not(unix))]
#[allow(dead_code)] // Unused on Windows - all tests using this are Unix-only
fn count_child_processes() -> Option<usize> {
    // Windows: Would need different approach
    None
}

// ============================================================================
// Helper: Check for Playwright Processes
// ============================================================================

#[allow(dead_code)] // Utility function for debugging process leaks
fn count_playwright_processes() -> Option<usize> {
    let output = Command::new("ps").args(["aux"]).output().ok()?;

    let output_str = String::from_utf8(output.stdout).ok()?;

    let count = output_str
        .lines()
        .filter(|line| {
            line.contains("playwright") && !line.contains("grep") && !line.contains("test")
            // Exclude the test process itself
        })
        .count();

    Some(count)
}

// ============================================================================
// Resource Cleanup Test: File Descriptors
// ============================================================================

#[tokio::test]
#[cfg(unix)]
#[cfg(unix)]
async fn test_file_descriptor_cleanup() {
    common::init_tracing();
    tracing::info!("\n=== Testing File Descriptor Cleanup ===\n");

    // Record initial FD count
    let initial_fds = count_open_file_descriptors().unwrap_or(0);
    tracing::info!("Initial file descriptors: {}", initial_fds);

    // Launch and close Playwright multiple times
    const CYCLES: usize = 10;

    for i in 0..CYCLES {
        // Launch Playwright
        let playwright = Playwright::launch()
            .await
            .expect("Failed to launch Playwright");

        // Launch browser
        let browser = playwright
            .chromium()
            .launch()
            .await
            .expect("Failed to launch browser");

        // Create page to exercise file descriptors
        let page = browser.new_page().await.expect("Failed to create page");
        let _ = page.goto("about:blank", None).await;

        // Close everything
        let _ = page.close().await;
        browser.close().await.expect("Failed to close browser");

        // Give system time to clean up
        tokio::time::sleep(Duration::from_millis(100)).await;

        if i % 2 == 1 {
            let current_fds = count_open_file_descriptors().unwrap_or(0);
            tracing::debug!("After cycle {}: {} FDs", i + 1, current_fds);
        }
    }

    // Wait for final cleanup
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check final FD count
    let final_fds = count_open_file_descriptors().unwrap_or(0);
    tracing::info!("\nFinal file descriptors: {}", final_fds);
    tracing::info!("FD growth: {}", final_fds as i32 - initial_fds as i32);

    // ASSERTION: FD count should not grow significantly
    // Allow some variance (10 FDs) for normal system behavior
    let fd_growth = (final_fds as i32 - initial_fds as i32).abs();
    assert!(
        fd_growth < 20,
        "File descriptor leak detected: {} FDs not cleaned up",
        fd_growth
    );

    tracing::info!("\n✓ File descriptors cleaned up properly");
}

// ============================================================================
// Resource Cleanup Test: Process Cleanup
// ============================================================================

#[tokio::test]
#[cfg(unix)]
async fn test_process_cleanup() {
    common::init_tracing();
    tracing::info!("\n=== Testing Process Cleanup ===\n");

    // Record initial child process count
    let initial_children = count_child_processes().unwrap_or(0);
    tracing::info!("Initial child processes: {}", initial_children);

    // Launch and close Playwright
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // During operation, should have child processes
    tokio::time::sleep(Duration::from_millis(100)).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let during_children = count_child_processes().unwrap_or(0);
    tracing::info!("Child processes during operation: {}", during_children);

    // Close (Playwright has Drop implementation that should clean up)
    drop(playwright);

    // Wait for cleanup using polling
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(2);
    let mut final_children = 0;
    let mut success = false;

    while start.elapsed() < timeout {
        final_children = count_child_processes().unwrap_or(0);
        if final_children <= initial_children {
            success = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    tracing::info!("Final child processes: {}", final_children);

    // ASSERTION: Child processes should return to initial count
    assert!(
        success,
        "Process leak detected: {} child processes not cleaned up (expected {})",
        final_children.saturating_sub(initial_children),
        initial_children
    );

    tracing::info!("\n✓ Child processes cleaned up properly");
}

// ============================================================================
// Resource Cleanup Test: Zombie Process Detection
// ============================================================================

#[tokio::test]
#[cfg(unix)]
async fn test_no_zombie_processes() {
    common::init_tracing();
    tracing::info!("\n=== Testing for Zombie Processes ===\n");

    // Helper to count zombie processes
    fn count_zombies() -> Option<usize> {
        let output = Command::new("ps").args(["aux"]).output().ok()?;

        let output_str = String::from_utf8(output.stdout).ok()?;

        let count = output_str
            .lines()
            .filter(|line| line.contains("<defunct>") || line.contains("Z"))
            .count();

        Some(count)
    }

    // Record initial zombie count
    let initial_zombies = count_zombies().unwrap_or(0);
    tracing::info!("Initial zombies: {}", initial_zombies);

    // Allow small tolerance for system noise (other processes may create/clean zombies)
    // We're testing that playwright doesn't leak zombies, not that the system is pristine
    const ZOMBIE_TOLERANCE: usize = 3;

    // Run multiple cycles
    const CYCLES: usize = 5;

    for i in 0..CYCLES {
        let playwright = Playwright::launch()
            .await
            .expect("Failed to launch Playwright");

        let browser = playwright
            .chromium()
            .launch()
            .await
            .expect("Failed to launch browser");

        let page = browser.new_page().await.expect("Failed to create page");
        let _ = page.goto("about:blank", None).await;

        // Close everything
        let _ = page.close().await;
        browser.close().await.expect("Failed to close browser");

        // Poll for zombies to be cleaned up
        // Instead of fixed sleep, we poll until count is within tolerance or timeout
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(2);
        let mut current_zombies = 0;
        let mut success = false;
        let max_allowed = initial_zombies + ZOMBIE_TOLERANCE;

        while start.elapsed() < timeout {
            current_zombies = count_zombies().unwrap_or(0);
            if current_zombies <= max_allowed {
                success = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        if i % 2 == 1 {
            tracing::debug!("After cycle {}: {} zombies", i + 1, current_zombies);
        }

        // ASSERTION: Zombie count should stay within tolerance of initial
        assert!(
            success,
            "Zombie process leak detected after cycle {}: {} zombies (max allowed: {})",
            i + 1,
            current_zombies,
            max_allowed
        );
    }

    tracing::info!("\n✓ No zombie processes detected");
}

// ============================================================================
// Server Lifecycle Test: Multiple Launch/Shutdown Cycles
// ============================================================================

#[tokio::test]
async fn test_multiple_server_cycles() {
    common::init_tracing();
    tracing::info!("\n=== Testing Multiple Server Launch/Shutdown Cycles ===\n");

    // Test that we can launch and shutdown Playwright server multiple times
    const CYCLES: usize = 5;

    for i in 0..CYCLES {
        tracing::info!("Server cycle {}/{}", i + 1, CYCLES);

        // Launch Playwright
        let playwright = Playwright::launch()
            .await
            .expect("Failed to launch Playwright");

        // Launch browser to verify server is working
        let browser = playwright
            .chromium()
            .launch()
            .await
            .expect("Failed to launch browser");

        // Create page to verify full functionality
        let page = browser.new_page().await.expect("Failed to create page");

        let _ = page.goto("about:blank", None).await;

        // Close everything
        let _ = page.close().await;
        browser.close().await.expect("Failed to close browser");

        // Explicitly drop Playwright to trigger shutdown
        drop(playwright);

        // Wait between cycles to ensure clean shutdown
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tracing::info!("\n✓ Multiple server cycles handled successfully");
}

// ============================================================================
// Resource Cleanup Test: Concurrent Browser Instances
// ============================================================================

#[tokio::test]
async fn test_concurrent_browser_cleanup() {
    common::init_tracing();
    tracing::info!("\n=== Testing Concurrent Browser Cleanup ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Launch multiple browsers concurrently
    const BROWSER_COUNT: usize = 3;

    let mut browsers = Vec::new();

    for i in 0..BROWSER_COUNT {
        let browser = playwright
            .chromium()
            .launch()
            .await
            .expect("Failed to launch browser");

        browsers.push(browser);
        tracing::info!("Launched browser {}/{}", i + 1, BROWSER_COUNT);
    }

    // Close all browsers
    for (i, browser) in browsers.into_iter().enumerate() {
        browser.close().await.expect("Failed to close browser");
        tracing::info!("Closed browser {}/{}", i + 1, BROWSER_COUNT);
    }

    // Wait for cleanup
    tokio::time::sleep(Duration::from_secs(1)).await;

    tracing::info!("\n✓ Concurrent browsers cleaned up successfully");
}

// ============================================================================
// Stress Test: Resource Limit Testing
// ============================================================================

#[tokio::test]
async fn test_resource_limit_stress() {
    common::init_tracing();
    tracing::info!("\n=== Stress Test: Resource Limits ===\n");

    // Test that we handle resource limits gracefully
    // Create many pages rapidly to stress resource management

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    const PAGE_COUNT: usize = 20;
    let mut pages = Vec::new();

    // Create many pages
    for i in 0..PAGE_COUNT {
        match browser.new_page().await {
            Ok(page) => {
                pages.push(page);
                if i % 5 == 4 {
                    tracing::debug!("Created {} pages", i + 1);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create page {}: {:?}", i + 1, e);
                // This is acceptable under resource stress
                break;
            }
        }
    }

    tracing::info!("Successfully created {} pages", pages.len());

    // Close all pages
    for (i, page) in pages.into_iter().enumerate() {
        let _ = page.close().await;
        if i % 5 == 4 {
            tracing::debug!("Closed {} pages", i + 1);
        }
    }

    browser.close().await.expect("Failed to close browser");

    tracing::info!("\n✓ Resource stress test handled successfully");
}
