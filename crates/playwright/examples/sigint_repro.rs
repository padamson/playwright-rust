// Reproducer for issue #59: hitting Ctrl-C after Playwright launches
// chromium leaves the terminal in non-canonical mode (arrow keys produce
// raw escape sequences, line editing broken).
//
// Used by the `tests/integration/sigint_termios.rs` integration test via
// the expect-based harness in `tests/integration/sigint_termios_harness.exp`.
//
// Prints `[sigint-repro] READY` on stderr once chromium is launched and
// the page has navigated, then sleeps for 30 seconds. The expect harness
// waits for the READY line, snapshots stty, sends Ctrl-C, snapshots stty,
// and asserts the two snapshots are byte-identical.
use playwright_rs::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[sigint-repro] launching playwright");
    let pw = Playwright::launch().await?;
    let browser = pw.chromium().launch().await?;
    let page = browser.new_page().await?;
    page.goto("data:text/html,<h1>repro</h1>", None).await?;

    eprintln!("[sigint-repro] READY");

    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

    browser.close().await?;
    Ok(())
}
