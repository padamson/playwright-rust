use leptos::prelude::*;

use crate::components::{
    Comparison, DogfoodBanner, Features, Footer, Hero, Install, ServeWalkthrough, VersionSwitcher,
    Walkthrough,
};

/// Root of the landing page. Each section is a reusable component so the view
/// code carries over unchanged if the build ever moves from CSR/Trunk to
/// SSR/cargo-leptos.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-ink-900 text-rust-50 antialiased">
            <VersionSwitcher/>
            <Hero/>
            <Install/>
            <Comparison/>
            <Features/>
            <DogfoodBanner/>
            <Walkthrough/>
            <ServeWalkthrough/>
            <Footer/>
        </div>
    }
}
