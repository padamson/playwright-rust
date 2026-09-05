use crate::version::{SITE_VERSION, is_dev};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

/// `/versions.json`, maintained by the deploy: the newest published release and
/// every published version (newest first).
#[derive(Clone, Debug, Default, Deserialize)]
struct Manifest {
    #[serde(default)]
    latest: String,
    #[serde(default)]
    versions: Vec<String>,
}

fn navigate_to(value: &str) {
    let url = match value {
        "dev" => "/dev/".to_string(),
        v => format!("/v{v}/"),
    };
    let _ = window().location().set_href(&url);
}

/// One `<option>` in the switcher: the value posted on change, the visible
/// label, and whether it is the build currently being viewed.
struct VersionOption {
    value: String,
    label: String,
    selected: bool,
}

/// The switcher's options for the given build and (optional) manifest.
///
/// The current build is always present and selected — even before
/// `/versions.json` loads or if the fetch fails — so a release snapshot never
/// falls back to rendering "dev (main)" as the current version. The manifest
/// only contributes the *other* published releases.
fn version_options(current: &str, manifest: Option<&Manifest>) -> Vec<VersionOption> {
    let is_dev = current == "dev";
    let mut opts = vec![VersionOption {
        value: "dev".to_string(),
        label: "dev (main)".to_string(),
        selected: is_dev,
    }];

    let mut listed_current = is_dev;
    if let Some(m) = manifest {
        for v in &m.versions {
            let selected = v == current;
            listed_current |= selected;
            opts.push(VersionOption {
                value: v.clone(),
                label: format!("v{v}"),
                selected,
            });
        }
    }

    // A release build whose version the manifest didn't list (missing/pending
    // fetch, or a snapshot not yet in the manifest) still needs its own option.
    if !listed_current {
        opts.insert(
            1,
            VersionOption {
                value: current.to_string(),
                label: format!("v{current}"),
                selected: true,
            },
        );
    }

    opts
}

/// A nudge banner: a message and, when there's somewhere useful to go, a link.
struct Banner {
    message: String,
    /// `(link label, href)`.
    link: Option<(String, String)>,
}

/// The banner (if any) for the given build and the manifest's `latest` release.
fn banner_for(is_dev: bool, current: &str, latest: &str) -> Option<Banner> {
    if is_dev {
        // Link to the latest release only if one exists — with no release the
        // root redirect points back to /dev/, so a link would just loop.
        let link =
            (!latest.is_empty()).then(|| (format!("Go to v{latest} →"), format!("/v{latest}/")));
        Some(Banner {
            message: "Unreleased dev build (main) — APIs may change.".to_string(),
            link,
        })
    } else if !latest.is_empty() && current != latest {
        Some(Banner {
            message: format!("You're viewing v{current} — a newer release is available."),
            link: Some((format!("Go to v{latest} →"), format!("/v{latest}/"))),
        })
    } else {
        None
    }
}

/// Top bar showing the current docs version, a dropdown to switch between the
/// `dev` (main HEAD) build and every published release, and a banner when the
/// viewer is not on the latest stable version.
#[component]
pub fn VersionSwitcher() -> impl IntoView {
    let manifest = RwSignal::new(None::<Manifest>);

    // Fetch the manifest so a snapshot can list versions released after it was
    // built. Failure (e.g. no manifest yet) leaves just the current version.
    spawn_local(async move {
        if let Ok(resp) = gloo_net::http::Request::get("/versions.json").send().await
            && resp.ok()
            && let Ok(m) = resp.json::<Manifest>().await
        {
            manifest.set(Some(m));
        }
    });

    let is_dev = is_dev();

    // Only nudge once the manifest has loaded (so we know the latest release).
    let banner = move || -> Option<Banner> {
        let m = manifest.get()?;
        banner_for(is_dev, SITE_VERSION, &m.latest)
    };

    view! {
        <div id="version-switcher" class="w-full border-b border-rust-700/30 bg-ink-800/80 text-sm">
            <div class="mx-auto flex max-w-5xl items-center gap-3 px-6 py-2">
                <label for="version-select" class="text-rust-50/60">"Version"</label>
                <select
                    id="version-select"
                    class="rounded border border-rust-700/40 bg-ink-900 px-2 py-1 text-rust-50"
                    on:change=move |ev| navigate_to(&event_target_value(&ev))
                >
                    {move || {
                        version_options(SITE_VERSION, manifest.get().as_ref())
                            .into_iter()
                            .map(|o| {
                                view! {
                                    <option value=o.value selected=o.selected>
                                        {o.label}
                                    </option>
                                }
                            })
                            .collect_view()
                    }}
                </select>

                {move || {
                    banner()
                        .map(|b| {
                            view! {
                                <span class="ml-auto flex items-center gap-2 text-rust-300">
                                    <span>{b.message}</span>
                                    {b.link
                                        .map(|(label, href)| {
                                            view! {
                                                <a
                                                    href=href
                                                    class="font-semibold underline hover:text-rust-500"
                                                >
                                                    {label}
                                                </a>
                                            }
                                        })}
                                </span>
                            }
                        })
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(latest: &str, versions: &[&str]) -> Manifest {
        Manifest {
            latest: latest.to_string(),
            versions: versions.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn selected_values(opts: &[VersionOption]) -> Vec<&str> {
        opts.iter()
            .filter(|o| o.selected)
            .map(|o| o.value.as_str())
            .collect()
    }

    #[test]
    fn release_build_without_manifest_still_selects_its_own_version() {
        // Fetch pending or failed: the current release must still be the
        // selected option, never "dev (main)".
        let opts = version_options("0.14.0", None);
        assert_eq!(selected_values(&opts), ["0.14.0"]);
    }

    #[test]
    fn release_build_selects_current_when_manifest_omits_it() {
        let m = manifest("0.15.0", &["0.15.0"]);
        let opts = version_options("0.14.0", Some(&m));
        assert_eq!(selected_values(&opts), ["0.14.0"]);
    }

    #[test]
    fn release_build_with_manifest_lists_each_version_once() {
        let m = manifest("0.15.0", &["0.15.0", "0.14.0"]);
        let opts = version_options("0.14.0", Some(&m));
        assert_eq!(selected_values(&opts), ["0.14.0"]);
        assert_eq!(
            opts.iter().filter(|o| o.value == "0.14.0").count(),
            1,
            "the current version must not be duplicated"
        );
    }

    #[test]
    fn dev_build_selects_dev() {
        let m = manifest("0.14.0", &["0.14.0"]);
        let opts = version_options("dev", Some(&m));
        assert_eq!(selected_values(&opts), ["dev"]);
    }

    #[test]
    fn dev_banner_has_no_link_when_no_release_exists() {
        // With no published release, the root redirect points back to /dev/, so a
        // "go to the latest release" link would just loop. It must be omitted.
        let b = banner_for(true, "dev", "").expect("dev build always shows a banner");
        assert!(b.link.is_none());
    }

    #[test]
    fn dev_banner_links_to_latest_release_when_one_exists() {
        let b = banner_for(true, "dev", "0.14.0").expect("dev banner");
        assert_eq!(
            b.link,
            Some(("Go to v0.14.0 →".to_string(), "/v0.14.0/".to_string()))
        );
    }

    #[test]
    fn release_banner_nudges_to_newer_release() {
        let b = banner_for(false, "0.13.0", "0.14.0").expect("outdated-release banner");
        assert_eq!(
            b.link,
            Some(("Go to v0.14.0 →".to_string(), "/v0.14.0/".to_string()))
        );
    }

    #[test]
    fn no_banner_on_the_latest_release() {
        assert!(banner_for(false, "0.14.0", "0.14.0").is_none());
        assert!(banner_for(false, "0.14.0", "").is_none());
    }

    // Guards the versions.json contract across the language boundary: the deploy
    // script (producer) and this module's `Manifest` (consumer) must agree. A
    // field rename on either side would silently break the switcher on the live
    // site — nothing else feeds real producer output to the consumer.
    #[cfg(unix)]
    #[test]
    fn update_manifest_output_matches_the_consumer_schema() {
        let script = concat!(env!("CARGO_MANIFEST_DIR"), "/deploy/update-manifest.sh");
        let root = std::env::temp_dir().join(format!(
            "pw-manifest-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for dir in ["v0.13.0", "v0.14.0", "dev"] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }

        let status = std::process::Command::new("bash")
            .arg(script)
            .arg(&root)
            .status()
            .expect("run update-manifest.sh");
        assert!(status.success(), "update-manifest.sh should exit 0");

        let json = std::fs::read_to_string(root.join("versions.json")).unwrap();
        let m: Manifest =
            serde_json::from_str(&json).expect("producer output must deserialize into Manifest");
        assert_eq!(m.latest, "0.14.0");
        assert_eq!(m.versions, vec!["0.14.0".to_string(), "0.13.0".to_string()]);

        let _ = std::fs::remove_dir_all(&root);
    }
}
