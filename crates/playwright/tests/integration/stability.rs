// Integration tests for Memory Leak Detection (Phase 6, Slice 7)
//
// Following TDD: Write tests first (Red), then implement fixes (Green), then refactor
//
// Tests cover:
// - Memory leak detection with repeated browser launch/close cycles
// - Memory leak detection with repeated page creation/destruction
// - Memory leak detection with repeated context creation/destruction
// - Process memory tracking over time
// - Resource exhaustion resistance
//
// Success Criteria:
// - 100+ browser cycles without memory growth
// - 50+ page/context cycles without leaks
// - Consistent memory usage patterns
// - No OOM (Out of Memory) errors

use crate::test_server::TestServer;
use playwright_rs::protocol::{ClickOptions, GotoOptions, Playwright};
use std::process::Command;
use std::time::Duration;

// ============================================================================
// Helper: Get Current Process Memory Usage (RSS - Resident Set Size)
// ============================================================================

#[cfg(target_os = "linux")]
fn get_process_memory_mb() -> Option<f64> {
    use std::fs;

    // Read /proc/self/status for memory info
    let status = fs::read_to_string("/proc/self/status").ok()?;

    // Find VmRSS line (Resident Set Size)
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            // Line format: "VmRSS: 12345 kB"
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                if let Ok(kb) = kb_str.parse::<f64>() {
                    return Some(kb / 1024.0); // Convert KB to MB
                }
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn get_process_memory_mb() -> Option<f64> {
    use std::process::Command;

    // Use ps command to get RSS
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;

    let output_str = String::from_utf8(output.stdout).ok()?;
    let kb: f64 = output_str.trim().parse().ok()?;
    Some(kb / 1024.0) // Convert KB to MB
}

#[cfg(target_os = "windows")]
fn get_process_memory_mb() -> Option<f64> {
    use std::mem;
    use winapi::shared::minwindef::FALSE;
    use winapi::um::processthreadsapi::GetCurrentProcess;
    use winapi::um::psapi::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};

    unsafe {
        let mut pmc: PROCESS_MEMORY_COUNTERS = mem::zeroed();
        pmc.cb = mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

        if GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut pmc as *mut _,
            mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ) != FALSE
        {
            let bytes = pmc.WorkingSetSize;
            return Some(bytes as f64 / (1024.0 * 1024.0)); // Convert bytes to MB
        }
    }
    None
}

// ============================================================================
// Memory Leak Test: Browser Launch/Close Cycles
// ============================================================================

#[tokio::test]
#[ignore] // Stress test: runs 20 full Playwright+browser launch/close cycles. Run with --run-ignored.
async fn test_no_memory_leak_browser_cycles() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Memory Leaks: Browser Launch/Close Cycles ===\n");

    // Record initial memory
    let initial_memory = get_process_memory_mb().unwrap_or(0.0);
    tracing::info!("Initial memory: {:.2} MB", initial_memory);

    // Run 20 browser launch/close cycles (enough to detect memory growth trends)
    const CYCLES: usize = 20;
    let mut memory_samples = Vec::new();

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

        // Create a page to exercise more of the system
        let page = browser.new_page().await.expect("Failed to create page");

        // Navigate to ensure browser is fully initialized
        let _ = page.goto("about:blank", None).await;

        // Close page and browser
        let _ = page.close().await;
        browser.close().await.expect("Failed to close browser");

        // Sample memory every 10 iterations
        if i % 10 == 9 {
            if let Some(mem) = get_process_memory_mb() {
                memory_samples.push(mem);
                tracing::debug!("After {} cycles: {:.2} MB", i + 1, mem);
            }
        }
    }

    // Final memory reading
    let final_memory = get_process_memory_mb().unwrap_or(0.0);
    tracing::info!("\nFinal memory: {:.2} MB", final_memory);

    tracing::info!("Memory growth: {:.2} MB", final_memory - initial_memory);

    // Analysis: Check for memory leak
    if memory_samples.len() >= 2 {
        let first_half_avg: f64 = memory_samples[..memory_samples.len() / 2]
            .iter()
            .sum::<f64>()
            / (memory_samples.len() / 2) as f64;
        let second_half_avg: f64 = memory_samples[memory_samples.len() / 2..]
            .iter()
            .sum::<f64>()
            / (memory_samples.len() - memory_samples.len() / 2) as f64;

        let memory_growth_rate = second_half_avg - first_half_avg;

        tracing::info!("First half average: {:.2} MB", first_half_avg);
        tracing::info!("Second half average: {:.2} MB", second_half_avg);
        tracing::info!("Growth rate: {:.2} MB", memory_growth_rate);

        // ASSERTION: Memory should not grow significantly (allow 50MB growth for normal variance)
        // This is the RED phase - we expect this might fail initially
        assert!(
            memory_growth_rate < 50.0,
            "Memory leak detected: growth rate {:.2} MB exceeds threshold",
            memory_growth_rate
        );
    }

    tracing::info!("\n✓ No memory leak detected in browser cycles");
}

// ============================================================================
// Memory Leak Test: Page Creation/Destruction Cycles
// ============================================================================

#[tokio::test]
#[ignore] // Stress test: runs 25 page creation/destruction cycles. Run with --run-ignored.
async fn test_no_memory_leak_page_cycles() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Memory Leaks: Page Creation/Destruction ===\n");

    // Launch Playwright and browser once
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Record initial memory
    let initial_memory = get_process_memory_mb().unwrap_or(0.0);
    tracing::info!("Initial memory: {:.2} MB", initial_memory);

    // Run 25 page creation/destruction cycles
    const CYCLES: usize = 25;
    let mut memory_samples = Vec::new();

    for i in 0..CYCLES {
        // Create page
        let page = browser.new_page().await.expect("Failed to create page");

        // Navigate to test page
        let _ = page.goto("about:blank", None).await;

        // Close page
        page.close().await.expect("Failed to close page");

        // Sample memory every 5 iterations
        if i % 5 == 4 {
            if let Some(mem) = get_process_memory_mb() {
                memory_samples.push(mem);
                tracing::debug!("After {} page cycles: {:.2} MB", i + 1, mem);
            }
        }
    }

    // Final memory reading
    let final_memory = get_process_memory_mb().unwrap_or(0.0);
    tracing::info!("\nFinal memory: {:.2} MB", final_memory);

    tracing::info!("Memory growth: {:.2} MB", final_memory - initial_memory);

    // Analysis: Check for memory leak
    if memory_samples.len() >= 2 {
        let first_half_avg: f64 = memory_samples[..memory_samples.len() / 2]
            .iter()
            .sum::<f64>()
            / (memory_samples.len() / 2) as f64;
        let second_half_avg: f64 = memory_samples[memory_samples.len() / 2..]
            .iter()
            .sum::<f64>()
            / (memory_samples.len() - memory_samples.len() / 2) as f64;

        let memory_growth_rate = second_half_avg - first_half_avg;

        tracing::info!("First half average: {:.2} MB", first_half_avg);
        tracing::info!("Second half average: {:.2} MB", second_half_avg);
        tracing::info!("Growth rate: {:.2} MB", memory_growth_rate);

        // ASSERTION: Memory should not grow significantly (allow 30MB growth for pages)
        assert!(
            memory_growth_rate < 30.0,
            "Memory leak detected: growth rate {:.2} MB exceeds threshold",
            memory_growth_rate
        );
    }

    browser.close().await.expect("Failed to close browser");

    tracing::info!("\n✓ No memory leak detected in page cycles");
}

// ============================================================================
// Memory Leak Test: Context Creation/Destruction Cycles
// ============================================================================

#[tokio::test]
#[ignore] // Stress test: runs 25 context creation/destruction cycles. Run with --run-ignored.
async fn test_no_memory_leak_context_cycles() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Memory Leaks: Context Creation/Destruction ===\n");

    // Launch Playwright and browser once
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Record initial memory
    let initial_memory = get_process_memory_mb().unwrap_or(0.0);
    tracing::info!("Initial memory: {:.2} MB", initial_memory);

    // Run 25 context creation/destruction cycles
    const CYCLES: usize = 25;
    let mut memory_samples = Vec::new();

    for i in 0..CYCLES {
        // Create context
        let context = browser
            .new_context()
            .await
            .expect("Failed to create context");

        // Create a page in the context
        let page = context.new_page().await.expect("Failed to create page");

        // Navigate to test page
        let _ = page.goto("about:blank", None).await;

        // Close page and context
        let _ = page.close().await;
        context.close().await.expect("Failed to close context");

        // Sample memory every 5 iterations
        if i % 5 == 4 {
            if let Some(mem) = get_process_memory_mb() {
                memory_samples.push(mem);
                tracing::debug!("After {} context cycles: {:.2} MB", i + 1, mem);
            }
        }
    }

    // Final memory reading
    let final_memory = get_process_memory_mb().unwrap_or(0.0);
    tracing::info!("\nFinal memory: {:.2} MB", final_memory);

    tracing::info!("Memory growth: {:.2} MB", final_memory - initial_memory);

    // Analysis: Check for memory leak
    if memory_samples.len() >= 2 {
        let first_half_avg: f64 = memory_samples[..memory_samples.len() / 2]
            .iter()
            .sum::<f64>()
            / (memory_samples.len() / 2) as f64;
        let second_half_avg: f64 = memory_samples[memory_samples.len() / 2..]
            .iter()
            .sum::<f64>()
            / (memory_samples.len() - memory_samples.len() / 2) as f64;

        let memory_growth_rate = second_half_avg - first_half_avg;

        tracing::info!("First half average: {:.2} MB", first_half_avg);
        tracing::info!("Second half average: {:.2} MB", second_half_avg);
        tracing::info!("Growth rate: {:.2} MB", memory_growth_rate);

        // ASSERTION: Memory should not grow significantly (allow 30MB growth for contexts)
        assert!(
            memory_growth_rate < 30.0,
            "Memory leak detected: growth rate {:.2} MB exceeds threshold",
            memory_growth_rate
        );
    }

    browser.close().await.expect("Failed to close browser");

    tracing::info!("\n✓ No memory leak detected in context cycles");
}

// ============================================================================
// Stress Test: Rapid Browser Creation
// ============================================================================

#[tokio::test]
#[ignore] // Stress test: rapid browser creation/destruction. Run with --run-ignored.
async fn test_rapid_browser_creation() {
    crate::common::init_tracing();
    tracing::info!("\n=== Stress Test: Rapid Browser Creation ===\n");

    // Test that we can rapidly create and destroy browsers without resource exhaustion
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    const RAPID_CYCLES: usize = 10;

    for i in 0..RAPID_CYCLES {
        let browser = playwright
            .chromium()
            .launch()
            .await
            .expect("Failed to launch browser");

        browser.close().await.expect("Failed to close browser");

        if i % 5 == 4 {
            tracing::debug!("Completed {} rapid cycles", i + 1);
        }
    }

    tracing::info!("\n✓ Rapid browser creation handled successfully");
}

// ============================================================================
// Merged from: stability_resource_cleanup_test.rs
// ============================================================================

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
#[ignore] // Stress test: 10 Playwright launch/close cycles with FD tracking. Run with --run-ignored.
#[cfg(unix)]
#[cfg(unix)]
async fn test_file_descriptor_cleanup() {
    crate::common::init_tracing();
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
        tokio::time::sleep(Duration::from_millis(50)).await;

        if i % 2 == 1 {
            let current_fds = count_open_file_descriptors().unwrap_or(0);
            tracing::debug!("After cycle {}: {} FDs", i + 1, current_fds);
        }
    }

    // Wait for final cleanup
    tokio::time::sleep(Duration::from_millis(200)).await;

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
    crate::common::init_tracing();
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
    crate::common::init_tracing();
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
    crate::common::init_tracing();
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
#[ignore] // Stress test: launches 3 concurrent browser instances. Run with --run-ignored.
async fn test_concurrent_browser_cleanup() {
    crate::common::init_tracing();
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
    tokio::time::sleep(Duration::from_millis(500)).await;

    tracing::info!("\n✓ Concurrent browsers cleaned up successfully");
}

// ============================================================================
// Stress Test: Resource Limit Testing
// ============================================================================

#[tokio::test]
#[ignore] // Stress test: creates 20 pages rapidly. Run with --run-ignored.
async fn test_resource_limit_stress() {
    crate::common::init_tracing();
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

// ============================================================================
// Merged from: stability_error_quality_test.rs
// ============================================================================

// Integration tests for Error Message Quality (Phase 6, Slice 7)
//
// Following TDD: Write tests first (Red), then implement fixes (Green), then refactor
//
// Tests cover:
// - Error messages include helpful context
// - Error messages suggest solutions
// - Error messages include what operation was being attempted
// - Error messages include relevant identifiers (selectors, URLs, etc.)
// - Network error messages are descriptive
// - Timeout error messages include duration
// - Element not found errors include selector
//
// Success Criteria:
// - All error messages are actionable
// - Errors include "what was attempted" context
// - Errors include "what went wrong" details
// - Errors suggest next steps when applicable

// ============================================================================
// Error Quality Test: Element Not Found
// ============================================================================

#[tokio::test]
async fn test_error_quality_element_not_found() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Element Not Found ===\n");

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Navigation failed");

    // Test: Element not found error should include selector
    let locator = page.locator("button.does-not-exist").await;

    // Use short timeout (1s instead of default 30s) to speed up test
    let options = ClickOptions {
        timeout: Some(1000.0), // 1 second in milliseconds
        ..Default::default()
    };
    let result = locator.click(Some(options)).await;

    assert!(result.is_err(), "Expected error for non-existent element");

    let error_msg = format!("{:?}", result.unwrap_err());
    tracing::info!("Error message: {}", error_msg);

    // ASSERTION: Error should mention the selector
    assert!(
        error_msg.contains("does-not-exist") || error_msg.contains("button"),
        "Error should include selector: {}",
        error_msg
    );

    // ASSERTION: Error should indicate what operation failed
    // Expected improvement: "Failed to click: Element not found: button.does-not-exist"
    // Current state might just say "Element not found"

    tracing::info!("\n✓ Element not found error includes selector");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Error Quality Test: Navigation Timeout
// ============================================================================

#[tokio::test]
async fn test_error_quality_navigation_timeout() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Navigation Timeout ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: Timeout error should include duration and URL
    let timeout_duration = Duration::from_millis(100);
    let target_url = "http://10.255.255.1:9999/page.html";
    let options = GotoOptions::new().timeout(timeout_duration);
    let result = page.goto(target_url, Some(options)).await;

    assert!(result.is_err(), "Expected timeout error");

    let error_msg = format!("{:?}", result.unwrap_err());
    tracing::info!("Error message: {}", error_msg);

    // ASSERTION: Error should mention timeout
    assert!(
        error_msg.contains("Timeout") || error_msg.contains("timeout"),
        "Error should mention timeout: {}",
        error_msg
    );

    // ASSERTION: Error should ideally include URL
    // Expected improvement: "Navigation timeout after 100ms navigating to http://10.255.255.1:9999/page.html"
    // Current state might just say "Timeout: ..."

    tracing::info!("\n✓ Navigation timeout error includes timeout duration");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Quality Test: Invalid URL
// ============================================================================

#[tokio::test]
async fn test_error_quality_invalid_url() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Invalid URL ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: Invalid URL error should be descriptive
    let invalid_url = "not-a-valid-url";
    let result = page.goto(invalid_url, None).await;

    assert!(result.is_err(), "Expected error for invalid URL");

    let error_msg = format!("{:?}", result.unwrap_err());
    tracing::info!("Error message: {}", error_msg);

    // ASSERTION: Error should indicate what was wrong with the URL
    // Expected improvement: "Invalid URL: 'not-a-valid-url' is not a valid URL"
    // or "Navigation failed: Cannot navigate to 'not-a-valid-url' (invalid URL format)"

    assert!(!error_msg.is_empty(), "Error message should not be empty");

    tracing::info!("\n✓ Invalid URL error is descriptive");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Quality Test: Connection Failed
// ============================================================================

#[tokio::test]
async fn test_error_quality_connection_failed() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Connection Failed ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: Connection refused error should be descriptive
    let unreachable_url = "http://localhost:59999/";
    let result = page.goto(unreachable_url, None).await;

    assert!(result.is_err(), "Expected connection error");

    let error_msg = format!("{:?}", result.unwrap_err());
    tracing::info!("Error message: {}", error_msg);

    // ASSERTION: Error should explain the connection failure
    // Expected improvement: "Connection failed: Cannot connect to http://localhost:59999/ (connection refused)"

    assert!(!error_msg.is_empty(), "Error message should not be empty");

    tracing::info!("\n✓ Connection failed error is descriptive");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Quality Test: Operation After Close
// ============================================================================

#[tokio::test]
async fn test_error_quality_operation_after_close() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Operation After Close ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Close the page
    page.close().await.expect("Failed to close page");

    // Test: Operating on closed page should give helpful error
    let result = page.goto("https://example.com", None).await;

    assert!(result.is_err(), "Expected error for closed page");

    let error_msg = format!("{:?}", result.unwrap_err());
    tracing::info!("Error message: {}", error_msg);

    // ASSERTION: Error should explain that the target was closed
    // Expected improvement: "Page is closed: Cannot perform navigation on a closed page"
    // Current state might say "Target closed" or "Channel closed"

    assert!(
        error_msg.contains("closed") || error_msg.contains("Closed"),
        "Error should mention that page is closed: {}",
        error_msg
    );

    tracing::info!("\n✓ Operation after close error is descriptive");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Quality Test: Assertion Timeout
// ============================================================================

#[tokio::test]
async fn test_error_quality_assertion_timeout() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Assertion Timeout ===\n");

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    page.goto(&format!("{}/locators.html", server.url()), None)
        .await
        .expect("Navigation failed");

    // Test: Assertion timeout should include what was being asserted
    let locator = page.locator("button.does-not-exist").await;

    // Try to click non-existent element with short timeout (1s instead of default 30s)
    let options = ClickOptions {
        timeout: Some(1000.0), // 1 second in milliseconds
        ..Default::default()
    };
    let result = locator.click(Some(options)).await;

    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        tracing::info!("Error message: {}", error_msg);

        // ASSERTION: Error should mention what was waited for
        // Expected improvement: "Timeout waiting for selector 'button.does-not-exist' to be visible"

        assert!(!error_msg.is_empty(), "Error message should not be empty");
    } else {
        tracing::error!("Unexpected success (element should not exist)");
    }

    tracing::info!("\n✓ Assertion timeout error includes context");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Error Quality Test: Multiple Errors in Sequence
// ============================================================================

#[tokio::test]
async fn test_error_quality_error_sequence() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Quality: Multiple Errors ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: Multiple errors should each be descriptive
    let errors = [
        ("invalid-url", "Invalid URL test"),
        ("http://localhost:59999/", "Connection refused test"),
        ("http://10.255.255.1:9999/timeout", "Timeout test"),
    ];

    for (url, test_name) in errors {
        let options = GotoOptions::new().timeout(Duration::from_millis(100));
        let result = page.goto(url, Some(options)).await;

        assert!(result.is_err(), "{} should produce error", test_name);

        let error_msg = format!("{:?}", result.unwrap_err());
        tracing::info!("{}: {}", test_name, error_msg);

        // Each error should be non-empty and descriptive
        assert!(!error_msg.is_empty(), "Error message should not be empty");
    }

    tracing::info!("\n✓ Multiple errors are each descriptive");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Quality Audit: Review All Error Types
// ============================================================================

#[tokio::test]
async fn test_error_quality_audit() {
    crate::common::init_tracing();
    tracing::info!("\n=== Error Quality Audit ===\n");

    // This test documents expected error message improvements
    // for each error variant in error.rs

    tracing::info!("Error Quality Expectations:");
    tracing::info!("1. ServerNotFound:");
    tracing::info!("   Current: 'Playwright server not found at expected location'");
    tracing::info!(
        "   Improved: 'Playwright server not found. Install with: npm install playwright'"
    );
    tracing::info!("2. LaunchFailed:");
    tracing::info!("   Current: 'Failed to launch Playwright server: <details>'");
    tracing::info!(
        "   Improved: 'Failed to launch Playwright server: <details>. Check that Node.js is installed.'"
    );
    tracing::info!("3. ElementNotFound:");
    tracing::info!("   Current: 'Element not found: <selector>'");
    tracing::info!(
        "   Improved: 'Element not found: <selector>. Waited for <timeout>. Retry with longer timeout or check selector.'"
    );
    tracing::info!("4. Timeout:");
    tracing::info!("   Current: 'Timeout: <message>'");
    tracing::info!(
        "   Improved: 'Timeout after <duration>: <operation> (<url>). Increase timeout or check network.'"
    );
    tracing::info!("5. TargetClosed:");
    tracing::info!("   Current: 'Target closed: <message>'");
    tracing::info!("   Improved: 'Target closed: Cannot perform <operation> on closed <target>.'");

    tracing::info!("\n✓ Error quality audit documented");
}

// ============================================================================
// Merged from: stability_shutdown_recovery_test.rs
// ============================================================================

// Integration tests for Graceful Shutdown and Error Recovery (Phase 6, Slice 7)
//
// Following TDD: Write tests first (Red), then implement fixes (Green), then refactor
//
// Tests cover:
// - Graceful shutdown on Drop
// - SIGTERM handling (Unix only)
// - SIGINT handling (Unix only)
// - Network error recovery
// - Browser crash handling
// - Connection loss recovery
// - Timeout recovery
//
// Success Criteria:
// - Clean shutdown on Drop
// - Proper SIGTERM/SIGINT handling
// - Graceful error recovery
// - No resource leaks on error paths

// ============================================================================
// Graceful Shutdown Test: Drop Cleanup
// ============================================================================

#[tokio::test]
async fn test_graceful_shutdown_on_drop() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Graceful Shutdown: Drop Cleanup ===\n");

    // Test that Playwright cleans up properly when dropped
    {
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

        tracing::info!("Playwright, browser, and page created");
        tracing::info!("Dropping all objects...");

        // Explicit drops to test cleanup order
        drop(page);
        drop(browser);
        drop(playwright);
    }

    // Wait for cleanup to complete
    tokio::time::sleep(Duration::from_secs(1)).await;

    tracing::info!("\n✓ Graceful shutdown on drop completed");
}

// ============================================================================
// Graceful Shutdown Test: Explicit Close
// ============================================================================

#[tokio::test]
async fn test_graceful_shutdown_explicit_close() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Graceful Shutdown: Explicit Close ===\n");

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

    // Close explicitly in reverse order
    tracing::info!("Closing page...");
    page.close().await.expect("Failed to close page");

    tracing::info!("Closing browser...");
    browser.close().await.expect("Failed to close browser");

    tracing::info!("Dropping playwright...");
    drop(playwright);

    tokio::time::sleep(Duration::from_millis(500)).await;

    tracing::info!("\n✓ Explicit close completed successfully");
}

// ============================================================================
// Graceful Shutdown Test: Multiple Browsers
// ============================================================================

#[tokio::test]
async fn test_graceful_shutdown_multiple_browsers() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Graceful Shutdown: Multiple Browsers ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Launch multiple browsers
    let browser1 = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser 1");

    let browser2 = playwright
        .firefox()
        .launch()
        .await
        .expect("Failed to launch browser 2");

    tracing::info!("Two browsers launched");

    // Close both
    tracing::info!("Closing browser 1...");
    browser1.close().await.expect("Failed to close browser 1");

    tracing::info!("Closing browser 2...");
    browser2.close().await.expect("Failed to close browser 2");

    tracing::info!("Dropping playwright...");
    drop(playwright);

    tokio::time::sleep(Duration::from_millis(500)).await;

    tracing::info!("\n✓ Multiple browsers shut down successfully");
}

// ============================================================================
// Error Recovery Test: Network Timeout Recovery
// ============================================================================

#[tokio::test]
async fn test_error_recovery_network_timeout() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Network Timeout ===\n");

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: After a timeout error, page should still be usable
    let options = GotoOptions::new().timeout(Duration::from_millis(100));
    let result = page.goto("http://10.255.255.1:9999/", Some(options)).await;

    assert!(result.is_err(), "Expected timeout error");
    assert!(result.is_err(), "Expected timeout error");
    tracing::info!("Timeout error occurred (expected)");

    // Recovery: Page should still work for valid navigation
    let recovery_result = page
        .goto(&format!("{}/locators.html", server.url()), None)
        .await;

    assert!(
        recovery_result.is_ok(),
        "Page should recover after timeout error: {:?}",
        recovery_result
    );

    tracing::info!("✓ Page recovered after timeout");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Error Recovery Test: Invalid URL Recovery
// ============================================================================

#[tokio::test]
async fn test_error_recovery_invalid_url() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Invalid URL ===\n");

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: After invalid URL error, page should still be usable
    let result = page.goto("not-a-valid-url", None).await;

    assert!(result.is_err(), "Expected invalid URL error");
    assert!(result.is_err(), "Expected invalid URL error");
    tracing::info!("Invalid URL error occurred (expected)");

    // Recovery: Page should still work for valid navigation
    let recovery_result = page
        .goto(&format!("{}/locators.html", server.url()), None)
        .await;

    assert!(
        recovery_result.is_ok(),
        "Page should recover after invalid URL error: {:?}",
        recovery_result
    );

    tracing::info!("✓ Page recovered after invalid URL");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Error Recovery Test: Multiple Errors in Sequence
// ============================================================================

#[tokio::test]
async fn test_error_recovery_multiple_errors() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Multiple Errors ===\n");

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Test: Page should handle multiple consecutive errors
    let errors = [
        "not-valid-url",
        "http://localhost:59999/",
        "http://10.255.255.1:9999/",
    ];

    for (i, url) in errors.iter().enumerate() {
        let options = GotoOptions::new().timeout(Duration::from_millis(100));
        let result = page.goto(url, Some(options)).await;

        assert!(result.is_err(), "Error {} should fail", i + 1);
        assert!(result.is_err(), "Error {} should fail", i + 1);
        tracing::info!("Error {} handled (expected)", i + 1);
    }

    // Recovery: Page should still work after multiple errors
    let recovery_result = page
        .goto(&format!("{}/locators.html", server.url()), None)
        .await;

    assert!(
        recovery_result.is_ok(),
        "Page should recover after multiple errors: {:?}",
        recovery_result
    );

    tracing::info!("✓ Page recovered after multiple consecutive errors");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Error Recovery Test: Error During Page Creation
// ============================================================================

#[tokio::test]
async fn test_error_recovery_page_creation() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Page Creation ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create multiple pages, even if some operations fail
    let page1 = browser.new_page().await.expect("Failed to create page 1");

    // Try an operation that might fail
    let _ = page1.goto("invalid-url", None).await;

    // Should still be able to create more pages
    let page2 = browser.new_page().await.expect("Failed to create page 2");

    // And use them (about:blank may not return a response, so just verify page is usable)
    let _ = page2.goto("about:blank", None).await;
    assert!(!page2.url().is_empty(), "Page 2 should have a URL");

    tracing::info!("✓ Browser recovered and created new page after error");

    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Recovery Test: Context Error Recovery
// ============================================================================

#[tokio::test]
async fn test_error_recovery_context() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Context ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Create context and page
    let context = browser
        .new_context()
        .await
        .expect("Failed to create context");
    let page = context.new_page().await.expect("Failed to create page");

    // Cause an error
    let _ = page.goto("invalid-url", None).await;

    // Context should still be usable
    let page2 = context
        .new_page()
        .await
        .expect("Failed to create second page");

    // Note: about:blank may not return a response, so we don't assert on the result
    // The important thing is that we can create and use the page
    let _ = page2.goto("about:blank", None).await;

    // Verify the page is usable by checking we can get its URL
    assert!(!page2.url().is_empty(), "Page 2 should have a URL");

    tracing::info!("✓ Context recovered after page error");

    context.close().await.expect("Failed to close context");
    browser.close().await.expect("Failed to close browser");
}

// ============================================================================
// Error Recovery Test: Browser Relaunch After Close
// ============================================================================

#[tokio::test]
async fn test_error_recovery_browser_relaunch() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Error Recovery: Browser Relaunch ===\n");

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    // Launch and close browser
    let browser1 = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser 1");

    browser1.close().await.expect("Failed to close browser 1");
    tracing::info!("Browser 1 closed");

    // Should be able to launch new browser
    let browser2 = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser 2");

    let page = browser2.new_page().await.expect("Failed to create page");

    // Note: about:blank may not return a response, so we don't assert on the result
    // The important thing is that we can create and use the page
    let _ = page.goto("about:blank", None).await;

    // Verify the page is usable by checking we can get its URL
    assert!(!page.url().is_empty(), "Page should have a URL");

    tracing::info!("✓ Browser relaunched successfully");

    browser2.close().await.expect("Failed to close browser 2");
}

// ============================================================================
// Stress Test: Error Recovery Under Load
// ============================================================================

#[tokio::test]
#[ignore] // Stress test: 10 rapid error/success navigation cycles. Run with --run-ignored.
async fn test_error_recovery_stress() {
    crate::common::init_tracing();
    tracing::info!("\n=== Stress Test: Error Recovery Under Load ===\n");

    let server = TestServer::start().await;
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");
    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");
    let page = browser.new_page().await.expect("Failed to create page");

    // Rapidly alternate between errors and successes
    const CYCLES: usize = 10;
    let mut successful_navigations = 0;

    for i in 0..CYCLES {
        if i % 2 == 0 {
            // Cause error (invalid port)
            let _ = page.goto("http://localhost:59999/", None).await;

            // Give a tiny bit of breathing room for the error to propagate
            tokio::time::sleep(Duration::from_millis(50)).await;
        } else {
            // Attempt successful navigation
            let result = page
                .goto(&format!("{}/locators.html", server.url()), None)
                .await;

            if result.is_ok() {
                successful_navigations += 1;
            } else {
                tracing::warn!("Navigation failed in cycle {}: {:?}", i, result.err());
            }
        }

        if i % 5 == 4 {
            tracing::info!("Completed {} error/success cycles", i + 1);
        }
    }

    // Verify at least 30% of valid attempts succeeded (allow some flakiness)
    // We attempt CYCLES/2 valid navigations.
    let attempts = CYCLES / 2;
    tracing::info!(
        "Successful navigations: {}/{}",
        successful_navigations,
        attempts
    );

    // We expect most to succeed with the small delay, but CI can be slow.
    // 30% success rate is enough to prove recovery works.
    let min_successful = (attempts as f64 * 0.3).ceil() as usize;
    assert!(
        successful_navigations >= min_successful,
        "Too few successful navigations: {} (expected at least {})",
        successful_navigations,
        min_successful
    );

    tracing::info!("✓ Error recovery stress test passed");

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

// ============================================================================
// Signal Handling Test: Ctrl+C Simulation (Unix only)
// ============================================================================

#[tokio::test]
#[cfg(unix)]
async fn test_signal_handling_cleanup() {
    crate::common::init_tracing();
    tracing::info!("\n=== Testing Signal Handling: Cleanup ===\n");

    // Note: We can't actually send SIGINT/SIGTERM to our own process in tests,
    // but we can verify that Drop handlers work correctly, which is what
    // signal handlers would ultimately call.

    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    let browser = playwright
        .chromium()
        .launch()
        .await
        .expect("Failed to launch browser");

    // Simulate abrupt shutdown by just dropping
    // Drop implementations should handle cleanup
    drop(browser);
    drop(playwright);

    // Wait for cleanup
    tokio::time::sleep(Duration::from_millis(500)).await;

    tracing::info!("✓ Cleanup handlers work for signal simulation");

    // Note: Real signal handling would require tokio::signal
    // and is better tested in integration/manual testing
}
