use leptos::prelude::*;

use crate::components::{Comparison, Features, Hero, Install};

/// Root of the landing page. Each section is a reusable component so the view
/// code carries over unchanged if the build ever moves from CSR/Trunk to
/// SSR/cargo-leptos.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-ink-900 text-rust-50 antialiased">
            <Hero/>
            <Install/>
            <Comparison/>
            <Features/>
        </div>
    }
}
