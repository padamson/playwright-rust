//! Build script for playwright-rs
//!
//! Assembles the Playwright Node.js driver from two artifacts — the
//! `playwright-core` npm tarball (registry.npmjs.org) and a pinned Node.js
//! binary (nodejs.org) — into the same `node` + `package/` layout the old
//! prebuilt CDN zip provided (see ADR 0006; the prebuilt zips were
//! discontinued when the azureedge CDN shut down). The runtime side
//! (`src/server/driver.rs`) picks the path up via compile-time
//! `option_env!()` lookups.
//!
//! By default the driver lands in the user cache, at the same version-keyed
//! path `playwright-rs install` writes and the runtime lookup probes
//! (`<cache>/playwright-rust/<version>/playwright-<version>-<platform>`; see
//! ADR 0007). One download then serves every target directory, every
//! cargo-mutants copy, and every workspace on the machine, and a consumer
//! that has built once is offline-safe. The assembled files are watched, so
//! a cleaned cache reassembles on the next build. Two env knobs change it:
//!
//! - `PLAYWRIGHT_DRIVER_CACHE_DIR` puts the driver somewhere specific, for a
//!   CI job that caches an explicit path or a machine whose home is not
//!   the place for it.
//! - `PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD` skips the download entirely for
//!   compile-only jobs (e.g. the MSRV `cargo check`) that never launch a
//!   browser, saving a ~90 MB fetch.
//!
//! `$OUT_DIR` is the fallback for a process with no resolvable home
//! directory. Cargo's convention is that build scripts write only there;
//! this one writes to the cache on purpose, and the trade is recorded in
//! the ADR.

use std::env;
use std::path::{Path, PathBuf};

const PLAYWRIGHT_VERSION: &str = "1.63.0";

// Download + assembly logic shared with the cli binary (`src/bin/
// playwright_rs.rs`); pulls in the pure URL/platform mapping from
// `driver_urls.rs`, which the lib test suite unit-tests.
include!("src/build_support/driver_assembly.rs");

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/build_support/driver_assembly.rs");
    println!("cargo:rerun-if-changed=src/build_support/driver_urls.rs");
    println!("cargo:rerun-if-env-changed=PLAYWRIGHT_DRIVER_CACHE_DIR");
    println!("cargo:rerun-if-env-changed=PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD");

    let platform = detect_platform();

    // Skip the download on docs.rs (no network) and for compile-only jobs
    // (e.g. MSRV `cargo check`, mutation testing) that never launch a browser.
    if env::var_os("DOCS_RS").is_some() || env::var_os("PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD").is_some()
    {
        set_absent_env_vars(platform, "skipped");
        return;
    }

    let versioned = format!("playwright-{PLAYWRIGHT_VERSION}-{platform}");

    // The source is recorded so the runtime can say what became of the driver
    // when a launch finds none, and so tests assert a layout only where it
    // applies.
    let (driver_dir, source) = match env::var_os("PLAYWRIGHT_DRIVER_CACHE_DIR") {
        Some(dir) => (PathBuf::from(dir).join(&versioned), "cache_dir"),
        None => match dirs::cache_dir() {
            Some(root) => (
                cached_driver_dir(&root, PLAYWRIGHT_VERSION, platform),
                "user_cache",
            ),
            None => {
                let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR set by Cargo"));
                (
                    out_dir.join("playwright-driver").join(&versioned),
                    "out_dir",
                )
            }
        },
    };

    let node_exe = driver_dir.join(node_exe_name(platform));
    let cli_js = driver_dir.join("package").join("cli.js");

    // Old driver versions linger beside the current one; a bump of
    // PLAYWRIGHT_VERSION changes build.rs and reruns this, so the new version
    // is always written, but prior versions are not garbage-collected.
    if node_exe.exists() && cli_js.exists() {
        set_present_env_vars(&driver_dir, platform, source);
        watch(&[&node_exe, &cli_js]);
        return;
    }

    println!(
        "cargo:warning=Assembling Playwright driver {PLAYWRIGHT_VERSION} (Node {NODE_VERSION}) for {platform}..."
    );

    match assemble_driver(&driver_dir, PLAYWRIGHT_VERSION, platform) {
        Ok(()) => {
            println!(
                "cargo:warning=Playwright driver assembled at {}",
                driver_dir.display()
            );
            predate_invocation(&[&node_exe, &cli_js]);
            set_present_env_vars(&driver_dir, platform, source);
            watch(&[&node_exe, &cli_js]);
        }
        Err(e) => {
            // Compile anyway (same shape as the skip path): the runtime
            // resolution chain can still find a driver via PLAYWRIGHT_DRIVER_PATH
            // or an npm-installed playwright, and a build without one fails at
            // launch with ServerNotFound instead of a cryptic missing-env-var
            // compile error in downstream crates. Nothing is watched on this
            // path, so the failure is sticky until a knob changes or
            // `cargo clean -p playwright-rs`; watching files that do not exist
            // would rerun this script, and retry the download, on every cargo
            // invocation.
            println!("cargo:warning=Failed to assemble Playwright driver: {e}");
            println!(
                "cargo:warning=Set PLAYWRIGHT_DRIVER_PATH to a driver directory, or install one via npm."
            );
            set_absent_env_vars(platform, "failed");
        }
    }
}

/// Watch the two files the runtime needs. The driver lives outside the
/// target directory, where nothing else would notice it going; cargo treats
/// a missing watched path as changed, so a cleaned cache reruns this script
/// and reassembles on the next build. Only called once the files exist.
fn watch(files: &[&Path]) {
    for file in files {
        println!("cargo:rerun-if-changed={}", file.display());
    }
}

/// Give freshly assembled files the mtime of cargo's invocation stamp.
///
/// Cargo decides whether a watched file changed by comparing its mtime to
/// `invoked.timestamp`, which it writes beside `OUT_DIR` before this script
/// starts. A file written during the run is newer than that stamp, so
/// without this the very next build reruns the script and recompiles the
/// crate and everything above it once more. The tar path preserves the
/// archive's old mtimes, but the zip path (Windows) writes fresh files, so
/// both are set. The stamp is a cargo internal: if it is not where expected,
/// nothing is changed and the cost is that one extra rebuild.
fn predate_invocation(files: &[&Path]) {
    let Ok(out_dir) = env::var("OUT_DIR") else {
        return;
    };
    let stamp = Path::new(&out_dir)
        .parent()
        .map(|dir| dir.join("invoked.timestamp"));
    let Some(mtime) = stamp
        .and_then(|stamp| std::fs::metadata(stamp).ok())
        .and_then(|meta| meta.modified().ok())
    else {
        return;
    };
    for file in files {
        if let Ok(handle) = std::fs::File::options().write(true).open(file) {
            let _ = handle.set_modified(mtime);
        }
    }
}

/// Detect the current platform and return the Playwright platform identifier
fn detect_platform() -> &'static str {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    playwright_platform(os, arch).unwrap_or_else(|| {
        println!("cargo:warning=Unsupported platform: {os} {arch}");
        println!("cargo:warning=Defaulting to linux platform");
        "linux"
    })
}

/// Env vars for builds that have no driver on disk (skipped or failed
/// download): the crate still compiles, and the runtime falls back through
/// its resolution chain. `reason` is what the runtime reports if that chain
/// also comes up empty.
fn set_absent_env_vars(platform: &str, reason: &str) {
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_DIR=");
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_VERSION={PLAYWRIGHT_VERSION}");
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_PLATFORM={platform}");
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_DIR_SOURCE={reason}");
}

/// Env vars for a build with a driver on disk. `source` names which default
/// or knob chose the directory.
fn set_present_env_vars(driver_dir: &Path, platform: &str, source: &str) {
    println!(
        "cargo:rustc-env=PLAYWRIGHT_DRIVER_DIR={}",
        driver_dir.display()
    );
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_VERSION={PLAYWRIGHT_VERSION}");
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_PLATFORM={platform}");
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_DIR_SOURCE={source}");
    // Deliberately not PLAYWRIGHT_NODE_EXE / PLAYWRIGHT_CLI_JS: those names are
    // the user's runtime overrides, and cargo injects every rustc-env value
    // into the environment of this crate's own test and bin processes, so
    // emitting them here would make the bundled paths look like overrides.
    // The runtime derives both files from PLAYWRIGHT_DRIVER_DIR instead.
}
