//! The dogfood deploy gate: serve the Trunk-built landing page and assert,
//! through a real browser driven by playwright-rs, that it works as advertised.
//! Because the site is a Leptos CSR/WASM app, these assertions also prove the
//! WASM bundle actually boots and renders (a static-HTML check could not).
//!
//! Run after building the site:
//!   (cd crates/site && trunk build --release)
//!   cargo test --manifest-path crates/site-e2e/Cargo.toml
//!
//! Skips gracefully when `crates/site/dist` is absent so it never fails a run
//! that didn't build the site.

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::Router;
use playwright_rs::{Playwright, expect};
use tower_http::services::ServeDir;

fn dist_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../site/dist")
}

/// Serve `dist/` on an ephemeral port; returns the bound address and the
/// server task handle.
async fn serve(dist: &PathBuf) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new().fallback_service(ServeDir::new(dist));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind site server");
    let addr = listener.local_addr().expect("local addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve site");
    });
    (addr, handle)
}

#[tokio::test]
async fn landing_page_boots_and_shows_hero() {
    let dist = dist_dir();
    if !dist.join("index.html").exists() {
        eprintln!(
            "skipping dogfood test: {} not built — run `trunk build` in crates/site first",
            dist.display()
        );
        return;
    }

    let (addr, server) = serve(&dist).await;

    let pw = Playwright::launch().await.expect("launch playwright");
    let browser = pw.chromium().launch().await.expect("launch chromium");
    let page = browser.new_page().await.expect("new page");
    page.goto(&format!("http://{addr}"), None)
        .await
        .expect("navigate to site");

    // Auto-wait for Leptos to mount the hero — proves the WASM app booted.
    let hero = page.locator("#hero-title").await;
    expect(hero.clone())
        .to_be_visible()
        .await
        .expect("hero title should render");
    expect(hero)
        .to_contain_text("Playwright for Rust")
        .await
        .expect("hero title text");

    // Install section advertises the current crate version.
    let install = page.locator("#install").await;
    expect(install.clone())
        .to_be_visible()
        .await
        .expect("install section should render");
    expect(install)
        .to_contain_text("playwright-rs = \"0.13\"")
        .await
        .expect("install snippet should show the crate version");

    // Python<->Rust comparison shows both sides.
    let comparison = page.locator("#comparison").await;
    expect(comparison.clone())
        .to_be_visible()
        .await
        .expect("comparison section should render");
    expect(comparison.clone())
        .to_contain_text("sync_playwright")
        .await
        .expect("comparison should show the Python side");
    expect(comparison)
        .to_contain_text("Playwright::launch")
        .await
        .expect("comparison should show the Rust side");

    // Every advertised feature card renders.
    for id in [
        "#feature-locators",
        "#feature-assertions",
        "#feature-cross-browser",
        "#feature-routing",
        "#feature-tracing",
        "#feature-responsive",
    ] {
        let card = page.locator(id).await;
        expect(card)
            .to_be_visible()
            .await
            .unwrap_or_else(|e| panic!("feature card {id} should render: {e:?}"));
    }

    browser.close().await.ok();
    server.abort();
}
