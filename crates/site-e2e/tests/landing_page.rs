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
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::http::{StatusCode, header::CONTENT_TYPE};
use axum::response::IntoResponse;
use playwright_rs::protocol::{
    ActionCursor, Animations, AriaSnapshotOptions, Page, Playwright, ScreencastStartOptions,
    ScreenshotOptions, ShowActionsOptions, StartHarOptions, TracingStartOptions,
    TracingStopOptions, Viewport,
};
use playwright_rs::{expect, expect_page};
use tower_http::services::ServeDir;

fn dist_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../site/dist")
}

/// Serve `dist` on an ephemeral port. `overlay` routes are merged ahead of the
/// static fallback, so a test can stub an endpoint the built site fetches (the
/// switcher's `/versions.json`, say) without hand-rolling a second server.
async fn serve_with(
    dist: &PathBuf,
    overlay: Option<Router>,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = overlay
        .unwrap_or_else(Router::new)
        .fallback_service(ServeDir::new(dist));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind site server");
    let addr = listener.local_addr().expect("local addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve site");
    });
    (addr, handle)
}

async fn serve(dist: &PathBuf) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    serve_with(dist, None).await
}

/// What the in-process backend answers for `/versions.json`: the JSON, or
/// `None` for an outage. Shared with the test, which rewrites it mid-run.
type Backend = Arc<Mutex<Option<String>>>;

fn backend_answering(manifest: &str) -> Backend {
    Arc::new(Mutex::new(Some(manifest.to_string())))
}

/// A router answering `/versions.json` from `backend`, for stubbing what the
/// version switcher fetches.
fn versions_manifest(backend: &Backend) -> Router {
    let answers = backend.clone();
    Router::new().route(
        "/versions.json",
        axum::routing::get(move || {
            let answer = answers.lock().expect("backend lock").clone();
            async move {
                match answer {
                    Some(json) => ([(CONTENT_TYPE, "application/json")], json).into_response(),
                    None => StatusCode::SERVICE_UNAVAILABLE.into_response(),
                }
            }
        }),
    )
}

/// A fresh Chromium page. The `Playwright` and `Browser` handles come back
/// too: dropping either tears down the browser.
async fn launch_page() -> (Playwright, playwright_rs::protocol::Browser, Page) {
    let pw = Playwright::launch().await.expect("launch playwright");
    let browser = pw.chromium().launch().await.expect("launch chromium");
    let page = browser.new_page().await.expect("new page");
    (pw, browser, page)
}

/// The built site, or `None` when it is absent — these tests skip rather than
/// fail so `cargo test` is useful without a prior `trunk build`.
fn dist_or_skip(what: &str) -> Option<PathBuf> {
    let dist = dist_dir();
    if dist.join("index.html").exists() {
        return Some(dist);
    }
    eprintln!(
        "skipping {what}: {} not built. Run `trunk build` in crates/site first.",
        dist.display()
    );
    None
}

/// Serve the site and open its landing page in a fresh browser.
///
/// Returns the `Playwright` and `Browser` handles alongside the page: dropping
/// either tears down the browser, so the caller must hold them for the life of
/// the test. Tests that need a `BrowserContext` (tracing, HAR, video) build
/// their own rather than using this.
async fn open_site(
    dist: &PathBuf,
    overlay: Option<Router>,
) -> (
    Playwright,
    playwright_rs::protocol::Browser,
    Page,
    tokio::task::JoinHandle<()>,
) {
    let (addr, server) = serve_with(dist, overlay).await;
    let (pw, browser, page) = launch_page().await;
    page.goto(&format!("http://{addr}"), None)
        .await
        .expect("navigate to site");
    (pw, browser, page, server)
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
        .screenshot(opts)
        .await
        .unwrap_or_else(|e| panic!("screenshot {selector}: {e:?}"));
    std::fs::write(steps.join(file), bytes)
        .unwrap_or_else(|e| panic!("write step screenshot {file}: {e:?}"));
}

#[tokio::test]
async fn landing_page_works_as_advertised() {
    let Some(dist) = dist_or_skip("dogfood test") else {
        return;
    };
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

    // Asset paths (receipts, images) must be RELATIVE so they resolve under the
    // version subpath (/vX.Y.Z/ or /dev/) on the deployed site, not the domain
    // root. Root-absolute "/receipts/..." 404'd on the versioned deploy. The
    // gate serves at root (where both resolve), so guard the invariant directly.
    let abs_assets = page
        .locator("img[src^='/receipts'], a[href^='/receipts'], img[src^='/crates-io']")
        .count()
        .await
        .expect("count root-absolute asset paths");
    assert_eq!(
        abs_assets, 0,
        "receipt/image paths must be relative so they resolve under the version subpath"
    );

    // Step 1: the SPA renders. The locator auto-waits for the WASM app to mount
    // and paint the hero, so there is no sleep or readiness polling.
    expect(page.locator("#hero-title"))
        .to_have_text("Playwright for Rust")
        .await
        .expect("hero renders once the WASM app boots");
    // The primary CTA must point at the docs (a navigation contract: catches a
    // broken or wrong docs link).
    expect(page.locator("#cta-docs"))
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
    let comparison = page.locator("#comparison");
    expect(comparison.clone())
        .to_contain_text("sync_playwright")
        .await
        .expect("comparison defaults to Python");
    page.locator("[data-lang='Java']")
        .click(None)
        .await
        .expect("click the Java tab");
    expect(page.locator("[data-lang='Java']"))
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
        .click(None)
        .await
        .expect("click the Firefox engine tab");
    expect(page.locator("#feature-cross-browser [data-lang='Firefox']"))
        .to_have_attribute("aria-selected", "true")
        .await
        .expect("the Firefox tab becomes selected");
    expect(page.locator("#feature-cross-browser [data-lang='Chromium']"))
        .to_have_attribute("aria-selected", "false")
        .await
        .expect("the Chromium tab deselects");
    expect(page.locator("#feature-cross-browser"))
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
    // The list must cover every card the page renders, which the count check
    // below enforces. Twice now a release has added cards here with no
    // coverage at all (webstorage/webauthn/fake-fs in 0.15.0, then
    // wait-for-function/evaluate-callback/session-state in 0.16.0), each time
    // by adding to the page and not to this hand-maintained list. An
    // unreleased card renders only on the dev build, which the hero's
    // crates.io badge identifies.
    let is_dev = page
        .locator("#hero-badges img[alt='crates.io: unreleased']")
        .count()
        .await
        .expect("count the unreleased badge")
        == 1;
    let cards = [
        ("#feature-locators", "page.locator", false),
        ("#feature-assertions", "to_have_text", false),
        ("#feature-cross-browser", "launch", false),
        ("#feature-routing", "route", false),
        ("#feature-tracing", "tracing_subscriber", false),
        ("#feature-responsive", "set_viewport_size", false),
        ("#feature-webstorage", "local_storage", false),
        ("#feature-webauthn", "credentials", false),
        ("#feature-wait-for-function", "wait_for_function", false),
        (
            "#feature-evaluate-callback",
            "evaluate_with_callback",
            false,
        ),
        ("#feature-session-state", "storage_state", false),
        ("#feature-fake-fs", "fake_file_system", false),
        ("#feature-route-service", "route_service", true),
    ];
    let cards: Vec<(&str, &str)> = cards
        .into_iter()
        .filter_map(|(id, token, unreleased)| (is_dev || !unreleased).then_some((id, token)))
        .collect();
    let rendered = page
        .locator("[id^='feature-']")
        .count()
        .await
        .expect("count rendered feature cards");
    assert_eq!(
        rendered,
        cards.len(),
        "{rendered} feature cards render but {} are checked; add the new card \
         (id + a token unique to its snippet) to the list above",
        cards.len()
    );
    for (id, token) in cards {
        expect(page.locator(id))
            .to_be_visible()
            .await
            .unwrap_or_else(|e| panic!("feature card {id} should render: {e:?}"));
        expect(page.locator(id))
            .to_contain_text(token)
            .await
            .unwrap_or_else(|e| panic!("feature card {id} should show its snippet: {e:?}"));
        let colored = page
            .locator(format!("{id} span[style*='color']"))
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
    let disclaimer = page.locator("#disclaimer");
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
        .mask(vec![page.locator("#hero-badges img")])
        .mask_color("#ce422b")
        .build();
    let bytes = page
        .locator("#hero")
        .screenshot(Some(masked))
        .await
        .expect("masked hero screenshot");
    std::fs::write(steps.join("06.png"), bytes).expect("write step 06 screenshot");

    // The walkthrough is itself an interactive stepper. Driving it covers the
    // third interactive widget on the page.
    page.locator("#walk-next")
        .click(None)
        .await
        .expect("click the walkthrough Next button");
    expect(page.locator("#walkthrough"))
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

/// The version switcher is fetch-driven (it reads `/versions.json` at runtime),
/// so prove it boots, populates the dropdown from the manifest, and shows the
/// "unreleased" banner on the dev build — served with a fixture manifest.
#[tokio::test]
async fn version_switcher_lists_versions_and_warns_on_dev() {
    let Some(dist) = dist_or_skip("switcher test") else {
        return;
    };

    // Overlay a fixture manifest the dev build can fetch.
    let manifest = versions_manifest(&backend_answering(
        r#"{"latest":"9.9.9","versions":["9.9.9","0.14.0"]}"#,
    ));
    let (_pw, browser, page, server) = open_site(&dist, Some(manifest)).await;

    // The dropdown is always present; once the manifest loads it carries the
    // published versions, and the dev build shows the unreleased banner.
    expect(page.locator("#version-select"))
        .to_be_visible()
        .await
        .expect("version dropdown visible");
    expect(page.locator("#version-select"))
        .to_contain_text("v0.14.0")
        .await
        .expect("dropdown lists published version from manifest");
    expect(page.locator("text=Unreleased dev build"))
        .to_be_visible()
        .await
        .expect("dev build shows the unreleased banner");

    browser.close().await.ok();
    server.abort();
}

/// The dev (main HEAD) build reflects its ahead-of-crates.io state: it installs
/// from git and its hero badges read "unreleased", where a release snapshot pins
/// the published version. The dogfood build is SITE_VERSION=dev, so these
/// dev-only distinctives must render.
#[tokio::test]
async fn dev_build_reflects_unreleased_state() {
    let Some(dist) = dist_or_skip("dev-features test") else {
        return;
    };
    let (_pw, browser, page, server) = open_site(&dist, None).await;

    // The dev build installs from git (main HEAD), not the crates.io version.
    expect(page.locator("#install"))
        .to_contain_text("git = \"https://github.com/padamson/playwright-rust\"")
        .await
        .expect("dev build's install block uses a git dependency");

    // The dev build's hero badges reflect unreleased reality: crates.io shows
    // "unreleased" (not the published version) and the Playwright badge tracks
    // the newer bundled driver. Match on alt text (robust to the external
    // shields image not loading in CI).
    let crates_badge = page
        .locator("#hero-badges img[alt='crates.io: unreleased']")
        .count()
        .await
        .expect("count crates.io badge");
    assert_eq!(
        crates_badge, 1,
        "dev build shows the unreleased crates.io badge"
    );
    let pw_badge = page
        .locator("#hero-badges img[alt='Playwright 1.62.1']")
        .count()
        .await
        .expect("count Playwright badge");
    assert_eq!(pw_badge, 1, "dev build shows the 1.62.1 Playwright badge");

    // The in-process serving card and its walkthrough are `unreleased` until
    // the release that carries `route_service` ships: both render here, badged,
    // and the release-snapshot gate asserts neither does there.
    expect(page.locator("#feature-route-service [data-unreleased-badge]"))
        .to_be_visible()
        .await
        .expect("the unreleased card is badged on the dev build");
    expect(page.locator("#serve-walkthrough [data-unreleased-badge]"))
        .to_be_visible()
        .await
        .expect("the unreleased walkthrough is badged on the dev build");

    // Dogfood the screencast API (shipped in 0.15.0): record the page with
    // cursor decoration and save a frame as the DogfoodBanner's receipt (the
    // snapshot build copies receipts into release snapshots too). This is the
    // live on_frame path, flow-controlled by the driver since 1.62: it only
    // streams because the client acks frames.
    let receipts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../site/public/receipts");
    std::fs::create_dir_all(&receipts).expect("create receipts dir");

    let latest_frame: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    let sink = latest_frame.clone();
    let screencast = page.screencast();
    screencast.on_frame(move |frame| {
        let sink = sink.clone();
        async move {
            *sink.lock().unwrap() = Some(frame.data.to_vec());
            Ok(())
        }
    });
    screencast
        .start(ScreencastStartOptions::default())
        .await
        .expect("start screencast");
    screencast
        .show_actions(ShowActionsOptions::default().cursor(ActionCursor::Pointer))
        .await
        .expect("show_actions with pointer cursor");
    // An interaction makes the cursor overlay appear and drives fresh frames.
    page.locator("#cta-docs")
        .hover(None)
        .await
        .expect("hover the docs CTA");

    // Poll for a streamed frame (no fixed pre-assert sleep).
    let mut captured = None;
    for _ in 0..60 {
        if let Some(bytes) = latest_frame.lock().unwrap().clone() {
            captured = Some(bytes);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    screencast.stop().await.ok();
    let frame = captured.expect("screencast should stream at least one frame");
    std::fs::write(receipts.join("screencast.jpeg"), frame).expect("write screencast receipt");

    browser.close().await.ok();
    server.abort();
}

/// The artifact that actually deploys — not the build the gate above drives.
///
/// The Pages workflow builds `dist/` at `SITE_VERSION=dev` with public-url `/`
/// for the dogfood gate, then builds a *second* time into `dist-snapshot/` at
/// `SITE_VERSION=<ver>` with public-url `/<dest>/`, and ships that. Two classes
/// of breakage are invisible to the gate by construction:
///
/// - **Sub-path resolution.** A root-absolute asset resolves at `/` and 404s
///   under `/vX.Y.Z/`. This already shipped once (`46f12fa`).
/// - **Release-only rendering.** Components branch on `version::is_dev()`, so
///   the release values (`PLAYWRIGHT_RELEASED`, `install.toml`) are unreachable
///   from a dev build. These shipped stale to `/v0.15.0/` and were caught by
///   eye, not by test.
///
/// Driven by env so it exercises the real artifact rather than rebuilding one:
///   `SNAPSHOT_DIST`     path to the snapshot dist dir
///   `SNAPSHOT_BASE`     base path it was built for, e.g. `/v0.15.0/`
///   `SNAPSHOT_VERSION`  the `SITE_VERSION` used, e.g. `0.15.0` or `dev`
///
/// Skips when unset, so a plain `cargo test` stays useful locally.
#[tokio::test]
async fn deployed_snapshot_is_sound() {
    let (Ok(dist), Ok(base), Ok(version)) = (
        std::env::var("SNAPSHOT_DIST"),
        std::env::var("SNAPSHOT_BASE"),
        std::env::var("SNAPSHOT_VERSION"),
    ) else {
        eprintln!("skipping snapshot test: SNAPSHOT_DIST/BASE/VERSION not set.");
        return;
    };
    let dist = PathBuf::from(dist);
    assert!(
        dist.join("index.html").exists(),
        "SNAPSHOT_DIST has no index.html: {}",
        dist.display()
    );

    // Mirror the real gh-pages layout: the snapshot lives under its base path,
    // while the version manifest sits at the *root*, shared by every snapshot.
    // Serving only the sub-path would 404 the switcher's `/versions.json` fetch
    // and misreport it as broken — the first run of this test did exactly that.
    let mount = base.trim_end_matches('/').to_string();
    let manifest = format!(r#"{{"latest":"{version}","versions":["{version}"]}}"#);
    let overlay =
        versions_manifest(&backend_answering(&manifest)).nest_service(&mount, ServeDir::new(&dist));
    let (addr, server) = serve_with(&dist, Some(overlay)).await;

    let (_pw, browser, page) = launch_page().await;

    // Registered before navigating: any 4xx/5xx here is an asset the snapshot
    // build pointed at the wrong place. This is the sub-path guard.
    let broken: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = broken.clone();
    page.on_response(move |resp| {
        let sink = sink.clone();
        let (status, url) = (resp.status(), resp.url().to_string());
        async move {
            if status >= 400 {
                sink.lock().unwrap().push(format!("{status} {url}"));
            }
            Ok(())
        }
    })
    .await
    .expect("register response listener");

    page.goto(&format!("http://{addr}{base}"), None)
        .await
        .expect("navigate to snapshot under its base path");

    // The hero only renders once the WASM bundle boots, which it cannot do if
    // its own assets 404 — so this doubles as the "bundle loads" assertion.
    expect(page.locator("#hero"))
        .to_be_visible()
        .await
        .expect("snapshot renders under its base path");

    if version == "dev" {
        expect(page.locator("#install"))
            .to_contain_text("git = \"https://github.com/padamson/playwright-rust\"")
            .await
            .expect("dev snapshot installs from git");
    } else {
        // Invariants that are only true at release time, which is exactly when
        // this runs. `cargo xtask verify-driver-version` cannot cover the first
        // one: it anchors only PLAYWRIGHT_DEV, because the released value
        // legitimately lags main between releases.
        let badge = format!("Playwright {}", playwright_rs::PLAYWRIGHT_VERSION);
        let count = page
            .locator(format!("#hero-badges img[alt='{badge}']"))
            .count()
            .await
            .expect("count Playwright badge");
        assert_eq!(
            count, 1,
            "release snapshot must advertise the driver it bundles ({badge}); \
             PLAYWRIGHT_RELEASED in hero.rs is stale"
        );

        let minor = version.split('.').take(2).collect::<Vec<_>>().join(".");
        let pin = format!("playwright-rs = \"{minor}\"");
        expect(page.locator("#install"))
            .to_contain_text(&pin)
            .await
            .unwrap_or_else(|e| panic!("release snapshot install pin should be {pin}: {e:?}"));

        let unreleased = page
            .locator("[data-unreleased-badge]")
            .count()
            .await
            .expect("count unreleased badges");
        assert_eq!(
            unreleased, 0,
            "unreleased feature cards are dev-only; they must not ship in a release snapshot"
        );
    }

    let broken = broken.lock().unwrap().clone();
    assert!(
        broken.is_empty(),
        "snapshot requested assets that do not resolve under {base}: {broken:#?}"
    );

    browser.close().await.ok();
    server.abort();
}

/// Serve this site in-process through `route_service` and rewrite its backend
/// under the running app; each step writes the receipt the second walkthrough
/// shows.
#[tokio::test]
async fn site_served_in_process_boots_and_reacts() {
    let Some(dist) = dist_or_skip("in-process serving test") else {
        return;
    };
    let receipts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../site/public/receipts");
    let steps = receipts.join("serve");
    std::fs::create_dir_all(&steps).expect("create receipts/serve dir");

    // Step 1: the app under test, its backend owned by the test. Nothing is
    // listening, and the origin is made up.
    let backend = backend_answering(r#"{"latest":"9.9.9","versions":["9.9.9","0.17.0"]}"#);
    let app = versions_manifest(&backend).fallback_service(ServeDir::new(&dist));

    let (_pw, browser, page) = launch_page().await;
    page.route_service("https://playwright-rust.test/**", app)
        .await
        .expect("register the in-process app");
    page.goto("https://playwright-rust.test/", None)
        .await
        .expect("navigate to the in-process site");
    expect(page.locator("#hero-title"))
        .to_have_text("Playwright for Rust")
        .await
        .expect("the wasm app boots when served in-process");
    let secure = page
        .evaluate_value("String(window.isSecureContext) + ':' + location.origin")
        .await
        .expect("probe the security context");
    assert_eq!(secure, "true:https://playwright-rust.test");
    shot(&page, &steps, "01.png", "#hero").await;

    // The switcher bar is one wide row; a phone-width viewport keeps its
    // receipts legible in the walkthrough's frame.
    page.set_viewport_size(Viewport {
        width: 480,
        height: 640,
    })
    .await
    .expect("narrow the viewport");

    // Step 2: the router answered the app's own fetch with the test's JSON.
    expect(page.locator("#version-select"))
        .to_contain_text("v9.9.9")
        .await
        .expect("the switcher lists the made-up release the in-process endpoint returned");
    expect(page.locator("#version-switcher a"))
        .to_contain_text("v9.9.9")
        .await
        .expect("the banner nudges toward the made-up release");
    shot(&page, &steps, "02.png", "#version-switcher").await;

    // Step 3: rewrite the backend's answer; the app follows on reload.
    *backend.lock().expect("backend lock") =
        Some(r#"{"latest":"0.17.0","versions":["0.17.0"]}"#.to_string());
    page.reload(None)
        .await
        .expect("reload against the new answer");
    expect(page.locator("#version-switcher a"))
        .to_contain_text("v0.17.0")
        .await
        .expect("the banner follows the backend's new latest");
    shot(&page, &steps, "03.png", "#version-switcher").await;

    // Step 4: take the backend down; the app degrades to the current build.
    *backend.lock().expect("backend lock") = None;
    page.reload(None).await.expect("reload against the outage");
    expect(page.locator("#version-select option"))
        .to_have_count(1)
        .await
        .expect("only the current build is offered during the outage");
    expect(page.locator("#version-switcher a"))
        .to_have_count(0)
        .await
        .expect("no nudge without a manifest");
    shot(&page, &steps, "04.png", "#version-switcher").await;

    // The walkthrough that shows these receipts renders on the dev build and
    // steps through them.
    page.set_viewport_size(Viewport {
        width: 1280,
        height: 720,
    })
    .await
    .expect("restore the viewport");
    expect(page.locator("#serve-walkthrough"))
        .to_be_visible()
        .await
        .expect("the in-process walkthrough renders");
    page.locator("#serve-next")
        .click(None)
        .await
        .expect("advance the in-process walkthrough");
    expect(page.locator("#serve-walkthrough img"))
        .to_have_attribute("src", "receipts/serve/02.png")
        .await
        .expect("the second step shows its receipt");

    browser.close().await.expect("close browser");
}
