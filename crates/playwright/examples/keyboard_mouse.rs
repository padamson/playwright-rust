// Keyboard and Mouse example - Low-level input control
//
// Shows: keyboard (type, press, down/up, insert), mouse (move, click, wheel, drag)

use playwright_rs::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto("https://www.google.com", None).await?;

    // Keyboard example - type into search box
    let search = page.locator("textarea[name=q]").await;
    search.click(None).await?;

    let keyboard = page.keyboard();
    keyboard.type_text("Playwright Rust", None).await?;

    let value = search.input_value(None).await?;
    assert_eq!(value, "Playwright Rust");
    println!("Typed: {}", value);

    // Select all and replace (Ctrl+A / Cmd+A)
    #[cfg(target_os = "macos")]
    keyboard.down("Meta").await?;
    #[cfg(not(target_os = "macos"))]
    keyboard.down("Control").await?;

    keyboard.press("KeyA", None).await?;

    #[cfg(target_os = "macos")]
    keyboard.up("Meta").await?;
    #[cfg(not(target_os = "macos"))]
    keyboard.up("Control").await?;

    // Insert text (paste-like, no key events)
    keyboard.insert_text("Rust automation").await?;

    let new_value = search.input_value(None).await?;
    println!("Replaced with: {}", new_value);

    // Mouse example - click and scroll
    let mouse = page.mouse();

    // Move and click at coordinates
    mouse.move_to(300, 300, None).await?;
    mouse.click(400, 300, None).await?;
    println!("Mouse clicked at (400, 300)");

    // Drag simulation (down, move, up)
    mouse.down(None).await?;
    mouse.move_to(500, 300, None).await?;
    mouse.up(None).await?;
    println!("Simulated drag");

    // Scroll with wheel
    mouse.wheel(0, 100).await?;
    println!("Scrolled 100px");

    browser.close().await?;
    Ok(())
}
