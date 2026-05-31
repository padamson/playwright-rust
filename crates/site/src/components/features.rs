use leptos::prelude::*;

use super::FeatureCard;

#[component]
pub fn Features() -> impl IntoView {
    view! {
        <section id="features" class="mx-auto max-w-5xl px-6 py-12">
            <h2 class="mb-6 text-2xl font-bold text-rust-300">"What you get"</h2>
            <div class="grid grid-cols-1 gap-5 md:grid-cols-2 lg:grid-cols-3">
                <FeatureCard
                    id="feature-locators"
                    title="Auto-waiting locators"
                    blurb="Locators wait for elements to be actionable — no sleeps, no flakes."
                    code="page.locator(\"button#save\")\n    .await\n    .click()\n    .await?;"
                />
                <FeatureCard
                    id="feature-assertions"
                    title="Auto-retrying assertions"
                    blurb="expect() retries until the DOM matches or the timeout elapses."
                    code="expect(page.locator(\"#status\").await)\n    .to_have_text(\"Ready\")\n    .await?;"
                />
                <FeatureCard
                    id="feature-cross-browser"
                    title="All three engines"
                    blurb="Chromium, Firefox, and WebKit — the same code across every browser."
                    code="let browser = pw.webkit()\n    .launch()\n    .await?;"
                />
                <FeatureCard
                    id="feature-routing"
                    title="Network interception"
                    blurb="Mock, block, or inspect any request from Rust."
                    code="page.route(\"**/*.png\", |route| async move {\n    route.abort().await\n}).await?;"
                />
                <FeatureCard
                    id="feature-tracing"
                    title="Built-in observability"
                    blurb="Wire up tracing and every call emits structured spans."
                    code="tracing_subscriber::fmt().init();\npage.goto(url, None).await?; // emits a span"
                />
                <FeatureCard
                    id="feature-responsive"
                    title="Responsive testing"
                    blurb="Drive any viewport to test responsive layouts."
                    code="page.set_viewport_size(Viewport {\n    width: 375,\n    height: 667,\n}).await?;"
                />
            </div>
        </section>
    }
}
