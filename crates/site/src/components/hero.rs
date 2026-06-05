use leptos::prelude::*;

use super::icons::{self, Icon};

const CRATES_IO: &str = "https://crates.io/crates/playwright-rs";
const DOCS_RS: &str = "https://docs.rs/playwright-rs";
const GITHUB: &str = "https://github.com/padamson/playwright-rust";

#[component]
pub fn Hero() -> impl IntoView {
    view! {
        <header id="hero" class="flex flex-col items-center px-6 pt-24 pb-16 text-center">
            <h1
                id="hero-title"
                class="text-5xl font-bold tracking-tight text-rust-500 sm:text-6xl"
            >
                "Playwright for Rust"
            </h1>
            <p id="hero-tagline" class="mt-5 max-w-2xl text-lg text-rust-50/80">
                "Cross-browser end-to-end testing for Rust. The same Playwright API you "
                "already know from Python, Java, and .NET."
            </p>
            <p id="unofficial" class="mt-3 text-xs text-rust-50/50">
                "Unofficial, community-maintained Rust bindings for Microsoft Playwright."
            </p>

            <div
                id="hero-badges"
                class="mt-7 flex flex-wrap items-center justify-center gap-2"
            >
                <a href=CRATES_IO>
                    <img alt="crates.io" src="https://img.shields.io/crates/v/playwright-rs.svg"/>
                </a>
                <a href=DOCS_RS>
                    <img alt="docs.rs" src="https://docs.rs/playwright-rs/badge.svg"/>
                </a>
                <img
                    alt="Playwright 1.60.0"
                    src="https://img.shields.io/badge/Playwright-1.60.0-45ba4b"
                />
            </div>

            <div class="mt-9 flex flex-wrap items-center justify-center gap-3">
                <a
                    id="cta-docs"
                    href=DOCS_RS
                    class="inline-flex items-center gap-2 rounded-lg bg-rust-500 px-5 py-2.5 font-semibold text-rust-50 transition hover:bg-rust-600"
                >
                    <Icon path=icons::DOCS_RS/>
                    "Docs"
                </a>
                <a
                    href=GITHUB
                    class="inline-flex items-center gap-2 rounded-lg border border-rust-700/50 px-5 py-2.5 font-semibold text-rust-50 transition hover:border-rust-500"
                >
                    <Icon path=icons::GITHUB/>
                    "GitHub"
                </a>
                <a
                    href=CRATES_IO
                    class="inline-flex items-center gap-2 rounded-lg border border-rust-700/50 px-5 py-2.5 font-semibold text-rust-50 transition hover:border-rust-500"
                >
                    <img src="/crates-io.png" alt="" class="h-5 w-auto"/>
                    "crates.io"
                </a>
            </div>
        </header>
    }
}
