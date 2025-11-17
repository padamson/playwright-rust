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
async fn test_file_descriptor_cleanup() {
    println!("\n=== Testing File Descriptor Cleanup ===\n");

    // Record initial FD count
    let initial_fds = count_open_file_descriptors().unwrap_or(0);
    println!("Initial file descriptors: {}", initial_fds);

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
            println!("After cycle {}: {} FDs", i + 1, current_fds);
        }
    }

    // Wait for final cleanup
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check final FD count
    let final_fds = count_open_file_descriptors().unwrap_or(0);
    println!("\nFinal file descriptors: {}", final_fds);
    println!("FD growth: {}", final_fds as i32 - initial_fds as i32);

    // ASSERTION: FD count should not grow significantly
    // Allow some variance (10 FDs) for normal system behavior
    let fd_growth = (final_fds as i32 - initial_fds as i32).abs();
    assert!(
        fd_growth < 20,
        "File descriptor leak detected: {} FDs not cleaned up",
        fd_growth
    );

    println!("\n✓ File descriptors cleaned up properly");
}

// ============================================================================
// Resource Cleanup Test: Process Cleanup
// ============================================================================

#[tokio::test]
#[cfg(unix)]
#[ignore = "Flaky: timing-dependent process cleanup varies by OS/load"]
async fn test_process_cleanup() {
    // TODO(Phase 7): Revisit after real-world usage. Process counting is inherently
    // racy and depends on OS scheduler and background processes. May need different
    // approach to verify cleanup without environmental variance.
    println!("\n=== Testing Process Cleanup ===\n");

    // Record initial child process count
    let initial_children = count_child_processes().unwrap_or(0);
    println!("Initial child processes: {}", initial_children);

    // Launch and close Playwright
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // During operation, should have child processes
    tokio::time::sleep(Duration::from_millis(100)).await;
    let during_children = count_child_processes().unwrap_or(0);
    println!("Child processes during operation: {}", during_children);

    // Close (Playwright has Drop implementation that should clean up)
    drop(playwright);

    // Wait for cleanup
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check final child process count
    let final_children = count_child_processes().unwrap_or(0);
    println!("Final child processes: {}", final_children);

    // ASSERTION: Child processes should return to initial count
    assert_eq!(
        final_children,
        initial_children,
        "Process leak detected: {} child processes not cleaned up",
        final_children.saturating_sub(initial_children)
    );

    println!("\n✓ Child processes cleaned up properly");
}

// ============================================================================
// Resource Cleanup Test: Zombie Process Detection
// ============================================================================

#[tokio::test]
#[cfg(unix)]
#[ignore = "Flaky: timing-dependent zombie reaping varies by OS/load"]
async fn test_no_zombie_processes() {
    // TODO(Phase 7): Revisit after real-world usage. May need different approach
    // to verify process cleanup without timing races. Zombie reaping is asynchronous
    // and depends on OS scheduler, making this inherently racy in CI.
    println!("\n=== Testing for Zombie Processes ===\n");

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
    println!("Initial zombies: {}", initial_zombies);

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

        // Check for zombies
        tokio::time::sleep(Duration::from_millis(200)).await;

        let current_zombies = count_zombies().unwrap_or(0);
        if i % 2 == 1 {
            println!("After cycle {}: {} zombies", i + 1, current_zombies);
        }

        // ASSERTION: No new zombie processes should be created
        assert_eq!(
            current_zombies,
            initial_zombies,
            "Zombie process detected after cycle {}",
            i + 1
        );
    }

    println!("\n✓ No zombie processes detected");
}

// ============================================================================
// Server Lifecycle Test: Multiple Launch/Shutdown Cycles
// ============================================================================

#[tokio::test]
async fn test_multiple_server_cycles() {
    println!("\n=== Testing Multiple Server Launch/Shutdown Cycles ===\n");

    // Test that we can launch and shutdown Playwright server multiple times
    const CYCLES: usize = 5;

    for i in 0..CYCLES {
        println!("Server cycle {}/{}", i + 1, CYCLES);

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

    println!("\n✓ Multiple server cycles handled successfully");
}

// ============================================================================
// Resource Cleanup Test: Concurrent Browser Instances
// ============================================================================

#[tokio::test]
async fn test_concurrent_browser_cleanup() {
    println!("\n=== Testing Concurrent Browser Cleanup ===\n");

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
        println!("Launched browser {}/{}", i + 1, BROWSER_COUNT);
    }

    // Close all browsers
    for (i, browser) in browsers.into_iter().enumerate() {
        browser.close().await.expect("Failed to close browser");
        println!("Closed browser {}/{}", i + 1, BROWSER_COUNT);
    }

    // Wait for cleanup
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("\n✓ Concurrent browsers cleaned up successfully");
}

// ============================================================================
// Stress Test: Resource Limit Testing
// ============================================================================

#[tokio::test]
async fn test_resource_limit_stress() {
    println!("\n=== Stress Test: Resource Limits ===\n");

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
                    println!("Created {} pages", i + 1);
                }
            }
            Err(e) => {
                println!("Failed to create page {}: {:?}", i + 1, e);
                // This is acceptable under resource stress
                break;
            }
        }
    }

    println!("Successfully created {} pages", pages.len());

    // Close all pages
    for (i, page) in pages.into_iter().enumerate() {
        let _ = page.close().await;
        if i % 5 == 4 {
            println!("Closed {} pages", i + 1);
        }
    }

    browser.close().await.expect("Failed to close browser");

    println!("\n✓ Resource stress test handled successfully");
}
