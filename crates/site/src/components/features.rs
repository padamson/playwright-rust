use leptos::prelude::*;

use super::{CodeBlock, CodeTabs, FeatureCard};
use crate::snippets;
use crate::version::{SITE_VERSION, is_dev};

/// Where the in-process serving card sends the reader: the walkthrough on
/// this page, and the module reference. docs.rs only has the module once the
/// release that carries it ships, so the dev build links to the source.
fn route_service_links() -> Vec<(&'static str, String)> {
    let reference = if is_dev() {
        "https://github.com/padamson/playwright-rust/blob/main/crates/playwright/src/protocol/route_service.rs"
            .to_string()
    } else {
        format!(
            "https://docs.rs/playwright-rs/{SITE_VERSION}/playwright_rs/protocol/route_service/index.html"
        )
    };
    vec![
        ("Walkthrough", "#serve-walkthrough".to_string()),
        ("Reference", reference),
    ]
}

#[component]
pub fn Features() -> impl IntoView {
    view! {
        <section id="features" class="mx-auto max-w-5xl px-6 py-12">
            <h2 class="mb-6 text-2xl font-bold text-rust-300">"What you get"</h2>
            <div class="grid grid-cols-1 gap-5 md:grid-cols-2">
                <FeatureCard
                    id="feature-locators"
                    title="Auto-waiting locators"
                    blurb="Locators wait for elements to be actionable, so no sleeps and no flakes."
                >
                    <CodeBlock html=snippets::CARD_LOCATORS_RS/>
                </FeatureCard>
                <FeatureCard
                    id="feature-assertions"
                    title="Auto-retrying assertions"
                    blurb="expect() retries until the DOM matches or the timeout elapses."
                >
                    <CodeBlock html=snippets::CARD_ASSERTIONS_RS/>
                </FeatureCard>
                <FeatureCard
                    id="feature-cross-browser"
                    title="All three engines"
                    blurb="Chromium, Firefox, and WebKit run the same code. Pick an engine:"
                >
                    <CodeTabs tabs=vec![
                        ("Chromium", snippets::ENGINE_CHROMIUM_RS),
                        ("Firefox", snippets::ENGINE_FIREFOX_RS),
                        ("WebKit", snippets::ENGINE_WEBKIT_RS),
                    ]/>
                </FeatureCard>
                <FeatureCard
                    id="feature-routing"
                    title="Network interception"
                    blurb="Mock, block, or inspect any request from Rust."
                >
                    <CodeBlock html=snippets::CARD_ROUTING_RS/>
                </FeatureCard>
                <FeatureCard
                    id="feature-route-service"
                    title="Serve your app from the test"
                    blurb="Hand an axum router or a wasm bundle to route_service. No port, no server, any origin."
                    unreleased=true
                    links=route_service_links()
                >
                    <CodeBlock html=snippets::CARD_ROUTE_SERVICE_RS/>
                </FeatureCard>
                <FeatureCard
                    id="feature-tracing"
                    title="Built-in observability"
                    blurb="Wire up tracing and every call emits structured spans."
                >
                    <CodeBlock html=snippets::CARD_TRACING_RS/>
                </FeatureCard>
                <FeatureCard
                    id="feature-responsive"
                    title="Responsive testing"
                    blurb="Drive any viewport to test responsive layouts."
                >
                    <CodeBlock html=snippets::CARD_RESPONSIVE_RS/>
                </FeatureCard>

                // Web storage / WebAuthn / File System Access shipped in 0.15.0
                // (Playwright 1.61 parity); wait-for-function, Rust closures in
                // the page, and session save & replay shipped in 0.16.0 (1.62.1).
                // To preview a not-yet-released feature, add a card with
                // `unreleased=true` — it renders only on the dev build with an
                // "Unreleased" badge, and the flag is dropped once it ships.
                <FeatureCard
                    id="feature-webstorage"
                    title="Web storage"
                    blurb="Read and write the page's localStorage / sessionStorage directly."
                >
                    <CodeBlock html=snippets::CARD_WEBSTORAGE_RS/>
                </FeatureCard>
                <FeatureCard
                    id="feature-webauthn"
                    title="WebAuthn passkeys"
                    blurb="Install a virtual authenticator and manage credentials for auth tests."
                >
                    <CodeBlock html=snippets::CARD_WEBAUTHN_RS/>
                </FeatureCard>
                <FeatureCard
                    id="feature-wait-for-function"
                    title="Wait for anything"
                    blurb="Wait on arbitrary page state, or on a matched element, with a JS predicate."
                >
                    <CodeBlock html=snippets::CARD_WAIT_FOR_FUNCTION_RS/>
                </FeatureCard>
                <FeatureCard
                    id="feature-evaluate-callback"
                    title="Rust closures in the page"
                    blurb="Pass a Rust closure into evaluate; the page calls back into your test and awaits the result."
                >
                    <CodeBlock html=snippets::CARD_EVALUATE_CALLBACK_RS/>
                </FeatureCard>
                <FeatureCard
                    id="feature-session-state"
                    title="Session save & replay"
                    blurb="Capture cookies, storage, IndexedDB and passkeys; restore them into a fresh context."
                >
                    <CodeBlock html=snippets::CARD_SESSION_STATE_RS/>
                </FeatureCard>
                <FeatureCard
                    id="feature-fake-fs"
                    title="File System Access testing"
                    blurb="Fake showSaveFilePicker / showOpenFilePicker to test save/open flows with no native dialog."
                >
                    <CodeBlock html=snippets::CARD_FAKE_FS_RS/>
                </FeatureCard>
            </div>
        </section>
    }
}
