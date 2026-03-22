// Locators example - Find, filter, and query elements
//
// Shows: Locator API, chaining, filtering, composition, get_by_* methods,
// element evaluation, bulk text retrieval, bounding boxes

use playwright_rs::Playwright;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto("https://example.com", None).await?;

    // Basic locator - find and query element
    let heading = page.locator("h1").await;
    let heading_text = heading.text_content().await?;
    let is_visible = heading.is_visible().await?;
    println!("Heading: {:?} (visible: {})", heading_text, is_visible);

    // Count matching elements
    let paragraphs = page.locator("p").await;
    let count = paragraphs.count().await?;
    println!("Found {} paragraphs", count);

    // Locator chaining - first, last, nth
    let first_para = paragraphs.first();
    let first_text = first_para.inner_text().await?;
    println!(
        "First paragraph: {}",
        first_text.lines().next().unwrap_or("")
    );

    // Nested locators - scope search within element
    let body = page.locator("body").await;
    let links = body.locator("a");
    let link_count = links.count().await?;
    println!("Found {} links in body", link_count);

    // --- Filtering & Composition (new in v0.8.7) ---

    // filter() - narrow results by text content
    let filtered = paragraphs.filter(playwright_rs::FilterOptions {
        has_text: Some("More information".into()),
        ..Default::default()
    });
    println!(
        "Paragraphs containing 'More information': {}",
        filtered.count().await?
    );

    // and_() - match elements satisfying both locators
    let h1 = page.locator("h1").await;
    let visible = page.locator(":visible").await;
    let visible_heading = h1.and_(&visible);
    println!("Visible h1 elements: {}", visible_heading.count().await?);

    // or_() - match elements satisfying either locator
    let h1 = page.locator("h1").await;
    let anchors = page.locator("a").await;
    let h1_or_a = h1.or_(&anchors);
    println!("h1 or <a> elements: {}", h1_or_a.count().await?);

    // --- Bulk text retrieval ---

    // all_inner_texts() - get text from all matching elements
    let all_texts = paragraphs.all_inner_texts().await?;
    for (i, text) in all_texts.iter().enumerate() {
        println!("Paragraph {}: {}...", i, &text[..text.len().min(50)]);
    }

    // --- Element evaluation ---

    // evaluate() - run JS with the element as first argument
    let tag_name: String = heading.evaluate("el => el.tagName", None::<()>).await?;
    println!("Heading tag name: {}", tag_name);

    // evaluate() with typed return
    let char_count: f64 = heading
        .evaluate("el => el.textContent.length", None::<()>)
        .await?;
    println!("Heading character count: {}", char_count);

    // evaluate_all() - run JS over all matching elements
    let lengths: Vec<f64> = paragraphs
        .evaluate_all("els => els.map(e => e.textContent.length)", None::<()>)
        .await?;
    println!("Paragraph lengths: {:?}", lengths);

    // --- Bounding box ---

    let bbox = heading.bounding_box().await?;
    if let Some(bbox) = bbox {
        println!(
            "Heading position: ({}, {}) size: {}x{}",
            bbox.x, bbox.y, bbox.width, bbox.height
        );
    }

    // --- get_by_* methods ---

    let heading = page.get_by_text("Example Domain", false).await;
    println!(
        "Found heading by text (visible: {})",
        heading.is_visible().await?
    );

    let link_by_text = body.get_by_text("Learn more", false);
    println!(
        "Found {} elements with 'Learn more' in body",
        link_by_text.count().await?
    );

    // --- wait_for() - wait for element state changes ---

    page.set_content(
        r#"<div id="target" style="display:none">Hello</div>
           <script>setTimeout(() => document.getElementById('target').style.display = '', 500)</script>"#,
        None,
    )
    .await?;

    let target = page.locator("#target").await;
    println!("Waiting for element to become visible...");
    target
        .wait_for(Some(playwright_rs::WaitForOptions {
            state: Some(playwright_rs::WaitForState::Visible),
            timeout: None,
        }))
        .await?;
    println!("Element is now visible: {}", target.is_visible().await?);

    // --- drag_to() - drag and drop between elements ---

    page.set_content(
        r#"<div id="source" draggable="true" style="width:50px;height:50px;background:blue">Drag</div>
           <div id="drop" style="width:100px;height:100px;background:gray">Drop here</div>
           <div id="result"></div>
           <script>
             document.getElementById('drop').addEventListener('drop', e => {
               e.preventDefault();
               document.getElementById('result').textContent = 'dropped';
             });
             document.getElementById('drop').addEventListener('dragover', e => e.preventDefault());
           </script>"#,
        None,
    )
    .await?;

    let source = page.locator("#source").await;
    let drop_zone = page.locator("#drop").await;
    source.drag_to(&drop_zone, None).await?;
    let result: String = page
        .locator("#result")
        .await
        .text_content()
        .await?
        .unwrap_or_default();
    println!("Drag result: {}", result);

    // --- page property - navigate from locator back to its page ---

    let locator = page.locator("body").await;
    let owner = locator.page()?;
    println!("Locator's page URL: {}", owner.url());

    browser.close().await?;
    Ok(())
}
