//! The dogfood deploy gate: serve the Trunk-built landing page and drive it
//! with playwright-rs, asserting it works as advertised. Because the site is a
//! Leptos CSR/WASM app, these assertions also prove the WASM bundle boots and
//! that its interactive widgets actually react (a static-HTML check could not).
//!
//! The steps are written the way you would test a real app: wait for the SPA
//! to render (auto-waiting locators, no sleeps), perform user interactions and
//! assert the resulting state, then check key content. Each step also writes an
//! element screenshot to `crates/site/dist/receipts/steps/`, and the whole run
//! is traced to `dist/receipts/trace.zip`; the page's walkthrough surfaces both.
//! Those artifacts are byproducts. The assertions are the gate.
//!
//! Run after building the site:
//!   (cd crates/site && trunk build)
//!   cargo test --manifest-path crates/site-e2e/Cargo.toml
//!
//! Skips gracefully when `crates/site/dist` is absent.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use axum::Router;
use playwright_rs::protocol::{
    Animations, AriaSnapshotOptions, Page, Playwright, ScreenshotOptions, StartHarOptions,
    TracingStartOptions, TracingStopOptions,
};
use playwright_rs::{expect, expect_page};
use tower_http::services::ServeDir;

fn dist_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../site/dist")
}

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

/// Write an element screenshot of `selector` to the step file. An element
/// screenshot scrolls the element into view and frames it tightly, so each
/// step's receipt is distinct (a viewport screenshot of adjacent sections looks
/// nearly identical).
async fn shot(page: &Page, steps: &Path, file: &str, selector: &str) {
    // Freeze CSS animations/transitions so the receipt captures the settled
    // state. This consumes the `animations` option that dogfooding this very
    // site added to playwright-rs.
    let opts = ScreenshotOptions::builder()
        .animations(Animations::Disabled)
        .build();
    let bytes = page
        .locator(selector)
        .await
        .screenshot(Some(opts))
        .await
        .unwrap_or_else(|e| panic!("screenshot {selector}: {e:?}"));
    std::fs::write(steps.join(file), bytes)
        .unwrap_or_else(|e| panic!("write step screenshot {file}: {e:?}"));
}

#[tokio::test]
async fn landing_page_works_as_advertised() {
    let dist = dist_dir();
    if !dist.join("index.html").exists() {
        eprintln!(
            "skipping dogfood test: {} not built. Run `trunk build` in crates/site first.",
            dist.display()
        );
        return;
    }
    // Write receipts into the site's `public/receipts/` source dir (not dist/).
    // Trunk's copy-dir re-copies it into dist on every build, so receipts
    // survive `trunk serve` rebuilds and show up with hot reload.
    let receipts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../site/public/receipts");
    let steps = receipts.join("steps");
    std::fs::create_dir_all(&steps).expect("create receipts/steps dir");

    let (addr, server) = serve(&dist).await;

    let pw = Playwright::launch().await.expect("launch playwright");
    let browser = pw.chromium().launch().await.expect("launch chromium");
    let context = browser.new_context().await.expect("new context");

    // Trace the whole run; published as a downloadable receipt.
    let tracing = context.tracing().await.expect("tracing handle");
    tracing
        .start(Some(
            TracingStartOptions::default()
                .name("playwright-rust.dev dogfood")
                .screenshots(true)
                .snapshots(true),
        ))
        .await
        .expect("start trace");

    // Also record a HAR of the run; published as a downloadable receipt so
    // visitors can see exactly what the page loaded. Real network traffic, no
    // contrived surface needed.
    tracing
        .start_har(
            receipts.join("dogfood.har").to_string_lossy().into_owned(),
            Some(StartHarOptions::default()),
        )
        .await
        .expect("start HAR recording");

    let page = context.new_page().await.expect("new page");
    page.goto(&format!("http://{addr}"), None)
        .await
        .expect("navigate to site");

    // Step 1: the SPA renders. The locator auto-waits for the WASM app to mount
    // and paint the hero, so there is no sleep or readiness polling.
    expect(page.locator("#hero-title").await)
        .to_have_text("Playwright for Rust")
        .await
        .expect("hero renders once the WASM app boots");
    // The primary CTA must point at the docs (a navigation contract: catches a
    // broken or wrong docs link).
    expect(page.locator("#cta-docs").await)
        .to_have_attribute("href", "https://docs.rs/playwright-rs")
        .await
        .expect("the Docs button links to docs.rs");
    // Accessibility guard: assert the page's key landmarks via the page-level
    // ARIA snapshot (Playwright 1.60). Partial/template matching keeps it robust
    // to unrelated copy changes while catching structural a11y regressions (the
    // hero stops being a level-1 heading in a `banner`, a section heading loses
    // its level, etc.).
    expect_page(&page)
        .to_match_aria_snapshot(
            "- banner:\n  - heading \"Playwright for Rust\" [level=1]\n- heading \"Install\" [level=2]\n- heading \"What you get\" [level=2]",
        )
        .await
        .expect("the page's accessibility landmarks are present");
    // Publish the full accessibility tree as a downloadable receipt, with each
    // element's bounding box appended (the 1.60 `boxes` option).
    let aria_tree = page
        .aria_snapshot(Some(AriaSnapshotOptions::default().boxes(true)))
        .await
        .expect("aria snapshot");
    std::fs::write(receipts.join("aria-snapshot.txt"), aria_tree).expect("write aria receipt");
    shot(&page, &steps, "01.png", "#hero").await;

    // Step 2: switch the comparison language and assert the resulting state.
    // The default tab is Python; clicking Java must swap the snippet and mark
    // the Java tab selected.
    let comparison = page.locator("#comparison").await;
    expect(comparison.clone())
        .to_contain_text("sync_playwright")
        .await
        .expect("comparison defaults to Python");
    page.locator("[data-lang='Java']")
        .await
        .click(None)
        .await
        .expect("click the Java tab");
    expect(page.locator("[data-lang='Java']").await)
        .to_have_attribute("aria-selected", "true")
        .await
        .expect("the Java tab becomes selected");
    expect(comparison.clone())
        .to_contain_text("Playwright.create()")
        .await
        .expect("the Java snippet is shown");
    expect(comparison)
        .not()
        .to_contain_text("sync_playwright")
        .await
        .expect("the Python snippet is replaced");
    shot(&page, &steps, "02.png", "#comparison").await;

    // Step 3: a second interactive widget. Switch the cross-browser tile from
    // Chromium to Firefox, scoping the locator to that card.
    page.locator("#feature-cross-browser [data-lang='Firefox']")
        .await
        .click(None)
        .await
        .expect("click the Firefox engine tab");
    expect(
        page.locator("#feature-cross-browser [data-lang='Firefox']")
            .await,
    )
    .to_have_attribute("aria-selected", "true")
    .await
    .expect("the Firefox tab becomes selected");
    expect(
        page.locator("#feature-cross-browser [data-lang='Chromium']")
            .await,
    )
    .to_have_attribute("aria-selected", "false")
    .await
    .expect("the Chromium tab deselects");
    expect(page.locator("#feature-cross-browser").await)
        .to_contain_text("firefox")
        .await
        .expect("the Firefox snippet is shown");
    shot(&page, &steps, "03.png", "#feature-cross-browser").await;

    // Step 4: every feature card renders its own snippet, actually highlighted.
    // For each card assert it is visible, shows a token unique to its snippet
    // (so we are not testing one shared constant), and that its code contains
    // colored <span>s. The color check is what proves the build-time syntect
    // HTML rendered as markup: a broken pipeline (escaped text, empty const, no
    // highlighting) would show the same text but zero colored spans.
    for (id, token) in [
        ("#feature-locators", "page.locator"),
        ("#feature-assertions", "to_have_text"),
        ("#feature-cross-browser", "launch"),
        ("#feature-routing", "route"),
        ("#feature-tracing", "tracing_subscriber"),
        ("#feature-responsive", "set_viewport_size"),
    ] {
        expect(page.locator(id).await)
            .to_be_visible()
            .await
            .unwrap_or_else(|e| panic!("feature card {id} should render: {e:?}"));
        expect(page.locator(id).await)
            .to_contain_text(token)
            .await
            .unwrap_or_else(|e| panic!("feature card {id} should show its snippet: {e:?}"));
        let colored = page
            .locator(&format!("{id} span[style*='color']"))
            .await
            .count()
            .await
            .unwrap_or_else(|e| panic!("count colored spans in {id}: {e:?}"));
        assert!(
            colored > 0,
            "feature card {id} should render highlighted (colored) code, found {colored} colored spans"
        );
    }
    shot(&page, &steps, "04.png", "#features").await;

    // Step 5: the footer is up front about being an unofficial binding.
    let disclaimer = page.locator("#disclaimer").await;
    expect(disclaimer.clone())
        .to_contain_text("unofficial")
        .await
        .expect("footer discloses unofficial status");
    expect(disclaimer)
        .to_contain_text("Microsoft")
        .await
        .expect("footer names the Microsoft trademark");
    shot(&page, &steps, "05.png", "#footer").await;

    // Step 6: demonstrate masking. Capture the hero with its badges redacted
    // behind a solid rust-colored box. This consumes the mask / mask_color
    // screenshot options that completed screenshot parity in playwright-rs.
    let masked = ScreenshotOptions::builder()
        .animations(Animations::Disabled)
        .mask(vec![page.locator("#hero-badges img").await])
        .mask_color("#ce422b")
        .build();
    let bytes = page
        .locator("#hero")
        .await
        .screenshot(Some(masked))
        .await
        .expect("masked hero screenshot");
    std::fs::write(steps.join("06.png"), bytes).expect("write step 06 screenshot");

    // The walkthrough is itself an interactive stepper. Driving it covers the
    // third interactive widget on the page.
    page.locator("#walk-next")
        .await
        .click(None)
        .await
        .expect("click the walkthrough Next button");
    expect(page.locator("#walkthrough").await)
        .to_contain_text("Step 2 of 6")
        .await
        .expect("the walkthrough advances to the next step");

    // Write the HAR receipt (every request the run made).
    tracing.stop_har().await.expect("write HAR receipt");

    // Save the trace zip as the deep-dive receipt.
    tracing
        .stop(Some(TracingStopOptions::default().path(
            receipts.join("trace.zip").to_string_lossy().into_owned(),
        )))
        .await
        .expect("write trace receipt");

    browser.close().await.ok();
    server.abort();
}
