use leptos::prelude::*;

use crate::snippets;

/// Steps mirroring the dogfood test, each with its code excerpt and the
/// screenshot the test captured at that point (written to dist/receipts/steps/).
fn steps() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "Wait for the app to render",
            snippets::WALK_01_RS,
            "/receipts/steps/01.png",
        ),
        (
            "Switch the comparison language",
            snippets::WALK_02_RS,
            "/receipts/steps/02.png",
        ),
        (
            "Switch the browser engine",
            snippets::WALK_03_RS,
            "/receipts/steps/03.png",
        ),
        (
            "Check every feature card",
            snippets::WALK_04_RS,
            "/receipts/steps/04.png",
        ),
        (
            "Verify the disclaimer",
            snippets::WALK_05_RS,
            "/receipts/steps/05.png",
        ),
        (
            "Redact elements with a mask",
            snippets::WALK_06_RS,
            "/receipts/steps/06.png",
        ),
    ]
}

#[component]
pub fn Walkthrough() -> impl IntoView {
    let steps = StoredValue::new(steps());
    let n = steps.with_value(|s| s.len());
    let (active, set_active) = signal(0usize);

    let labels: Vec<&'static str> = steps.with_value(|s| s.iter().map(|t| t.0).collect());
    // Rebuild the step list in one reactive closure (no per-button reactive
    // bindings), so the active highlight always tracks the state.
    let buttons = move || {
        let act = active.get();
        labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let label = *label;
                let base = "rounded-md px-3 py-2 text-left text-sm font-semibold transition";
                let class = if act == i {
                    format!("{base} bg-rust-500/15 text-rust-300")
                } else {
                    format!("{base} text-rust-50/55 hover:text-rust-50/85")
                };
                view! {
                    <button
                        type="button"
                        data-step=i
                        aria-current=if act == i { "step" } else { "false" }
                        class=class
                        on:click=move |_| set_active.set(i)
                    >
                        {format!("{}. {label}", i + 1)}
                    </button>
                }
            })
            .collect_view()
    };

    let code = move || steps.with_value(|s| s[active.get()].1);
    let shot = move || steps.with_value(|s| s[active.get()].2);
    let caption = move || steps.with_value(|s| s[active.get()].0);

    view! {
        <section id="walkthrough" class="mx-auto max-w-5xl px-6 py-12">
            <h2 class="mb-2 text-2xl font-bold text-rust-300">"Watch the test drive the page"</h2>
            <p class="mb-6 max-w-2xl text-sm text-rust-50/70">
                "Every deploy, playwright-rs runs this test against the page. Step through it to see "
                "the code and what the browser saw at each step."
            </p>
            <div class="grid grid-cols-1 gap-6 md:grid-cols-2 md:items-start">
                <div class="flex flex-col gap-3">
                    <div role="tablist" class="flex flex-col gap-1">{buttons}</div>
                    // Fixed height so varying snippet lengths don't resize the
                    // section (which would shift the footer and controls).
                    <pre
                        class="h-72 overflow-auto rounded-lg border border-rust-700/40 bg-ink-800 p-4 text-sm leading-relaxed"
                        inner_html=code
                    ></pre>
                </div>
                <div class="flex flex-col gap-3 md:sticky md:top-6">
                    // Fixed-height, object-contain box so screenshots of
                    // different sizes occupy a constant frame (controls below
                    // stay stationary).
                    <img
                        src=shot
                        alt=caption
                        class="h-72 w-full rounded-lg border border-rust-700/40 bg-ink-800 object-contain shadow-lg"
                        loading="lazy"
                    />
                    <div class="flex items-center justify-between text-sm text-rust-50/60">
                        <button
                            type="button"
                            id="walk-prev"
                            class="rounded-md px-3 py-1.5 font-semibold text-rust-300 transition hover:text-rust-500 disabled:opacity-40"
                            prop:disabled=move || active.get() == 0
                            on:click=move |_| set_active.update(|a| *a = a.saturating_sub(1))
                        >
                            "Prev"
                        </button>
                        <span>{move || format!("Step {} of {n}", active.get() + 1)}</span>
                        <button
                            type="button"
                            id="walk-next"
                            class="rounded-md px-3 py-1.5 font-semibold text-rust-300 transition hover:text-rust-500 disabled:opacity-40"
                            prop:disabled=move || active.get() + 1 == n
                            on:click=move |_| set_active.update(|a| {
                                if *a + 1 < n {
                                    *a += 1;
                                }
                            })
                        >
                            "Next"
                        </button>
                    </div>
                </div>
            </div>
        </section>
    }
}
