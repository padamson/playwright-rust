use leptos::prelude::*;

use crate::snippets;

/// One walkthrough step: label, highlighted code, and the receipt the dogfood
/// test captured at that point (a path under `dist/receipts/`).
pub type Step = (&'static str, &'static str, &'static str);

/// Steps mirroring the dogfood test that drives the page over a socket.
fn dogfood_steps() -> Vec<Step> {
    vec![
        (
            "Wait for the app to render",
            snippets::WALK_01_RS,
            "receipts/steps/01.png",
        ),
        (
            "Switch the comparison language",
            snippets::WALK_02_RS,
            "receipts/steps/02.png",
        ),
        (
            "Switch the browser engine",
            snippets::WALK_03_RS,
            "receipts/steps/03.png",
        ),
        (
            "Check every feature card",
            snippets::WALK_04_RS,
            "receipts/steps/04.png",
        ),
        (
            "Verify the disclaimer",
            snippets::WALK_05_RS,
            "receipts/steps/05.png",
        ),
        (
            "Redact elements with a mask",
            snippets::WALK_06_RS,
            "receipts/steps/06.png",
        ),
    ]
}

/// Steps mirroring the test that serves this site's wasm bundle to the
/// browser from inside the test process, through `route_service`, then
/// changes the backend's answers under the running app.
fn serve_steps() -> Vec<Step> {
    vec![
        (
            "Serve it from the test. Nothing listens",
            snippets::SERVE_01_RS,
            "receipts/serve/01.png",
        ),
        (
            "The router answered the app's fetch",
            snippets::SERVE_02_RS,
            "receipts/serve/02.png",
        ),
        (
            "Change the backend's answer",
            snippets::SERVE_03_RS,
            "receipts/serve/03.png",
        ),
        (
            "Take the backend down",
            snippets::SERVE_04_RS,
            "receipts/serve/04.png",
        ),
    ]
}

/// The walkthrough of the dogfood test that drives this page.
#[component]
pub fn Walkthrough() -> impl IntoView {
    view! {
        <StepPanel
            id="walkthrough"
            nav="walk"
            title="Watch the test drive the page"
            intro="Every deploy, playwright-rs runs this test against the page. Step through it to see the code and what the browser saw at each step."
            steps=dogfood_steps()
        />
    }
}

/// The walkthrough of the test that serves this page in-process; unreleased
/// until the release that carries `route_service` ships.
#[component]
pub fn ServeWalkthrough() -> impl IntoView {
    view! {
        <StepPanel
            id="serve-walkthrough"
            nav="serve"
            title="Serve the app from inside the test"
            intro="This page is a wasm app. The same deploy also serves its bundle to the browser from inside the test process: an axum router, no socket, and a backend the test rewrites while the app is running. Step through the code and what the browser saw."
            steps=serve_steps()
            unreleased=true
        />
    }
}

/// A stepper: numbered step buttons, the active step's code, and its receipt.
///
/// `nav` prefixes the prev/next control ids (`{nav}-prev`, `{nav}-next`) so two
/// panels on one page stay addressable.
#[component]
pub fn StepPanel(
    /// The section id, also the anchor the feature cards link to.
    id: &'static str,
    /// Prefix for the prev/next control ids.
    nav: &'static str,
    title: &'static str,
    intro: &'static str,
    steps: Vec<Step>,
    /// Mark a walkthrough for a not-yet-released feature: badged, and
    /// rendered only on the dev build, the same rule as an unreleased card.
    #[prop(optional)]
    unreleased: bool,
) -> impl IntoView {
    if unreleased && !crate::version::is_dev() {
        return ().into_any();
    }
    let steps = StoredValue::new(steps);
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
    let prev_id = format!("{nav}-prev");
    let next_id = format!("{nav}-next");

    view! {
        <section id=id class="mx-auto max-w-5xl px-6 py-12">
            <div class="mb-2 flex items-center gap-3">
                <h2 class="text-2xl font-bold text-rust-300">{title}</h2>
                {unreleased.then(|| view! { <super::UnreleasedBadge /> })}
            </div>
            <p class="mb-6 max-w-2xl text-sm text-rust-50/70">{intro}</p>
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
                            id=prev_id
                            class="rounded-md px-3 py-1.5 font-semibold text-rust-300 transition hover:text-rust-500 disabled:opacity-40"
                            prop:disabled=move || active.get() == 0
                            on:click=move |_| set_active.update(|a| *a = a.saturating_sub(1))
                        >
                            "Prev"
                        </button>
                        <span>{move || format!("Step {} of {n}", active.get() + 1)}</span>
                        <button
                            type="button"
                            id=next_id
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
    .into_any()
}
