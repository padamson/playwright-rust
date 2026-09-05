use leptos::prelude::*;

/// One feature highlight: title, one-line blurb, and a code area supplied as
/// children (a `CodeBlock` for a single snippet, or `CodeTabs` for variants).
///
/// Set `unreleased` for a feature that's on the dev (main HEAD) build but not
/// yet on crates.io: the card shows an "Unreleased" badge and renders **only**
/// on the dev build (`SITE_VERSION == "dev"`), so release snapshots omit it.
/// When the feature ships, drop `unreleased` and the card becomes permanent.
///
/// `links` are `(label, href)` pairs rendered under the blurb, for a card that
/// has somewhere deeper to send the reader: a walkthrough anchor on this page,
/// the reference on docs.rs.
#[component]
pub fn FeatureCard(
    /// Stable id so the dogfood test can assert the card rendered.
    id: &'static str,
    title: &'static str,
    blurb: &'static str,
    /// Mark a not-yet-released feature (dev-only, badged).
    #[prop(optional)]
    unreleased: bool,
    /// `(label, href)` pairs shown under the blurb.
    #[prop(optional)]
    links: Vec<(&'static str, String)>,
    children: Children,
) -> impl IntoView {
    // An unreleased card only appears on the dev build.
    if unreleased && !crate::version::is_dev() {
        return ().into_any();
    }

    let links = (!links.is_empty()).then(|| {
        view! {
            <p class="mb-4 flex gap-4 text-xs font-semibold">
                {links
                    .into_iter()
                    .map(|(label, href)| {
                        view! {
                            <a href=href class="text-rust-300 underline hover:text-rust-500">
                                {label}
                            </a>
                        }
                    })
                    .collect_view()}
            </p>
        }
    });

    view! {
        <div id=id class="flex flex-col rounded-xl border border-rust-700/30 bg-ink-800 p-5">
            <div class="flex items-center gap-2">
                <h3 class="text-lg font-semibold text-rust-300">{title}</h3>
                {unreleased.then(|| view! { <super::UnreleasedBadge /> })}
            </div>
            <p class="mt-1 mb-4 text-sm text-rust-50/70">{blurb}</p>
            {links}
            <div class="mt-auto">{children()}</div>
        </div>
    }
    .into_any()
}
