// Integration tests for add_style_tag functionality
//
// Tests cover:
// - Page.add_style_tag() with inline content - CSS injection and verification
// - Multiple style tags - sequential CSS injection
// - Style persistence across navigation

mod test_server;

use playwright_rs::protocol::Playwright;
use test_server::TestServer;

#[tokio::test]
async fn test_add_style_tag_com_conteudo() {
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

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    page.add_style_tag(
        r#"
        body {
            background-color: rgb(255, 0, 0) !important;
        }
        "#,
        None,
    )
    .await
    .expect("Failed to add style tag");

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let bg_color = page
        .evaluate_value("window.getComputedStyle(document.body).backgroundColor")
        .await
        .expect("Failed to evaluate background color");

    assert!(
        bg_color.contains("rgb(255, 0, 0)") || bg_color.contains("rgb(255,0,0)"),
        "Background color should be red, got: {}",
        bg_color
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_style_tag_multiplos_estilos() {
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

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    page.add_style_tag(
        r#"
        body {
            font-size: 32px !important;
        }
        "#,
        None,
    )
    .await
    .expect("Failed to add first style tag");

    page.add_style_tag(
        r#"
        body {
            color: rgb(0, 255, 0) !important;
        }
        "#,
        None,
    )
    .await
    .expect("Failed to add second style tag");

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let font_size = page
        .evaluate_value("window.getComputedStyle(document.body).fontSize")
        .await
        .expect("Failed to evaluate font size");

    let color = page
        .evaluate_value("window.getComputedStyle(document.body).color")
        .await
        .expect("Failed to evaluate color");

    assert!(
        font_size.contains("32px"),
        "Font size should be 32px, got: {}",
        font_size
    );
    assert!(
        color.contains("rgb(0, 255, 0)") || color.contains("rgb(0,255,0)"),
        "Color should be green, got: {}",
        color
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}

#[tokio::test]
async fn test_add_style_tag_antes_de_navegar() {
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

    page.goto(&format!("{}/input.html", server.url()), None)
        .await
        .expect("Failed to navigate")
        .expect("Expected a response");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    page.add_style_tag(
        r#"
        body {
            margin: 0px !important;
            padding: 0px !important;
        }
        "#,
        None,
    )
    .await
    .expect("Failed to add style tag");

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let margin = page
        .evaluate_value("window.getComputedStyle(document.body).margin")
        .await
        .expect("Failed to evaluate margin");

    let padding = page
        .evaluate_value("window.getComputedStyle(document.body).padding")
        .await
        .expect("Failed to evaluate padding");

    assert!(
        margin.contains("0px"),
        "Margin should be 0px, got: {}",
        margin
    );
    assert!(
        padding.contains("0px"),
        "Padding should be 0px, got: {}",
        padding
    );

    browser.close().await.expect("Failed to close browser");
    server.shutdown();
}
