// Example: Clock API for deterministic time-based tests
//
// Playwright's Clock API replaces the page's Date and timer functions so tests
// can control time precisely. Useful when pages render timestamps, run
// setTimeout/setInterval, or animate based on elapsed time.
//
// Demonstrates:
// - page.clock().install(options) — replace real timers with a fake clock
// - clock.pause_at(time) — freeze Date.now() at a specific epoch
// - clock.fast_forward(ticks) — advance time (firing due timers)
// - clock.set_fixed_time(time) — freeze Date.now() without stopping timers
// - clock.resume() — return to real time after pause_at
//
// To run:
//   cargo run --package playwright-rs --example clock

use playwright_rs::Playwright;
use playwright_rs::protocol::ClockInstallOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Clock API — deterministic time control ===\n");

    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    // Install fake timers BEFORE navigating so page scripts see the fake clock.
    let clock = page.clock().expect("clock should be available");
    clock
        .install(Some(ClockInstallOptions { time: Some(0) }))
        .await?;

    // Load a simple HTML page that displays the current time.
    page.set_content(
        r#"<!DOCTYPE html>
        <html>
          <body>
            <div id="clock"></div>
            <script>
              function tick() {
                document.getElementById('clock').textContent =
                  new Date().toISOString();
              }
              tick();
              setInterval(tick, 1000);
            </script>
          </body>
        </html>"#,
        None,
    )
    .await?;

    // --- pause_at: freeze time at a specific moment ---
    println!(">> pause_at(1_700_000_000_000)  (2023-11-14T22:13:20Z)");
    clock.pause_at(1_700_000_000_000).await?;

    let now: String = page.evaluate_value("new Date().toISOString()").await?;
    println!("   Date:     {}", now);

    // --- fast_forward: advance 60 seconds; due setInterval timers fire ---
    println!("\n>> fast_forward(60_000)  (+60 seconds; setInterval handlers fire)");
    clock.fast_forward(60_000).await?;

    let now: String = page.evaluate_value("new Date().toISOString()").await?;
    println!("   Date:     {}", now);

    // The #clock element should reflect the advanced time (setInterval ran)
    let displayed = page
        .locator("#clock")
        .await
        .text_content()
        .await?
        .unwrap_or_default();
    println!("   #clock:   {}", displayed);

    // --- set_fixed_time: freeze Date.now() but let timers keep firing ---
    println!("\n>> set_fixed_time(0)  (Date.now() returns 0 but timers still tick)");
    clock.set_fixed_time(0).await?;
    let frozen: String = page.evaluate_value("Date.now().toString()").await?;
    println!("   Date.now() = {}", frozen);

    // --- resume: go back to real time after pause_at ---
    println!("\n>> resume()");
    clock.resume().await?;

    browser.close().await?;
    Ok(())
}
