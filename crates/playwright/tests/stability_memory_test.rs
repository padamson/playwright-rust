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

mod common;
mod test_server;

use playwright_rs::protocol::Playwright;

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
async fn test_no_memory_leak_browser_cycles() {
    common::init_tracing();
    tracing::info!("\n=== Testing Memory Leaks: Browser Launch/Close Cycles ===\n");

    // Record initial memory
    let initial_memory = get_process_memory_mb().unwrap_or(0.0);
    tracing::info!("Initial memory: {:.2} MB", initial_memory);

    // Run 50 browser launch/close cycles (enough to detect memory growth trends)
    const CYCLES: usize = 50;
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
async fn test_no_memory_leak_page_cycles() {
    common::init_tracing();
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

    // Run 50 page creation/destruction cycles
    const CYCLES: usize = 50;
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
async fn test_no_memory_leak_context_cycles() {
    common::init_tracing();
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

    // Run 50 context creation/destruction cycles
    const CYCLES: usize = 50;
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
async fn test_rapid_browser_creation() {
    common::init_tracing();
    tracing::info!("\n=== Stress Test: Rapid Browser Creation ===\n");

    // Test that we can rapidly create and destroy browsers without resource exhaustion
    let playwright = Playwright::launch()
        .await
        .expect("Failed to launch Playwright");

    const RAPID_CYCLES: usize = 20;

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
