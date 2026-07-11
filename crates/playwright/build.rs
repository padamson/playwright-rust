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
//! By default the driver lands in Cargo's `$OUT_DIR` (inside `target/`),
//! which is fine for local dev. Two env knobs tune it for CI:
//!
//! - `PLAYWRIGHT_DRIVER_CACHE_DIR` relocates the download to a stable,
//!   version-keyed path so CI can cache it on its own key. This is needed
//!   because `Swatinem/rust-cache` prunes workspace build-script output —
//!   a driver left in `$OUT_DIR` is NOT cached by it and re-downloads every
//!   run. The driver belongs in its own `actions/cache`, like the browsers.
//! - `PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD` skips the download entirely for
//!   compile-only jobs (e.g. the MSRV `cargo check`) that never launch a
//!   browser, saving a ~90 MB fetch.

use std::env;
use std::path::PathBuf;

const PLAYWRIGHT_VERSION: &str = "1.60.0";

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

    // Skip the download on docs.rs (no network) and for compile-only jobs
    // (e.g. MSRV `cargo check`, mutation testing) that never launch a browser.
    if env::var_os("DOCS_RS").is_some() || env::var_os("PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD").is_some()
    {
        set_skipped_env_vars("skipped");
        return;
    }

    // Default to OUT_DIR; PLAYWRIGHT_DRIVER_CACHE_DIR relocates the driver to a
    // stable, version-keyed path that CI can cache on its own key (see header).
    // The source is recorded so tests can assert the OUT_DIR layout only when
    // it actually applies.
    let (drivers_dir, source) = match env::var_os("PLAYWRIGHT_DRIVER_CACHE_DIR") {
        Some(dir) => (PathBuf::from(dir), "cache_dir"),
        None => {
            let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR set by Cargo"));
            (out_dir.join("playwright-driver"), "out_dir")
        }
    };
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_DIR_SOURCE={source}");

    let platform = detect_platform();
    let driver_dir = drivers_dir.join(format!("playwright-{PLAYWRIGHT_VERSION}-{platform}"));

    // Old driver versions linger in OUT_DIR until `cargo clean` — Cargo's
    // build-script fingerprint reruns this on PLAYWRIGHT_VERSION bumps
    // (which change build.rs) so the new version is always written, but
    // we don't garbage-collect prior versions. Disk bloat only.
    if driver_dir.exists() {
        set_output_env_vars(&driver_dir, platform);
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
            set_output_env_vars(&driver_dir, platform);
        }
        Err(e) => {
            // Compile anyway (same shape as the skip path): the runtime
            // resolution chain can still find a driver via PLAYWRIGHT_DRIVER_PATH
            // or an npm-installed playwright, and a build without one fails at
            // launch with ServerNotFound instead of a cryptic missing-env-var
            // compile error in downstream crates.
            println!("cargo:warning=Failed to assemble Playwright driver: {e}");
            println!(
                "cargo:warning=Set PLAYWRIGHT_DRIVER_PATH to a driver directory, or install one via npm."
            );
            set_skipped_env_vars("failed");
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
/// its resolution chain.
fn set_skipped_env_vars(reason: &str) {
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_DIR=");
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_VERSION={PLAYWRIGHT_VERSION}");
    println!(
        "cargo:rustc-env=PLAYWRIGHT_DRIVER_PLATFORM={}",
        detect_platform()
    );
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_DIR_SOURCE={reason}");
}

/// Set environment variables for use at runtime
fn set_output_env_vars(driver_dir: &std::path::Path, platform: &str) {
    // Set the driver directory for runtime
    println!(
        "cargo:rustc-env=PLAYWRIGHT_DRIVER_DIR={}",
        driver_dir.display()
    );
    println!(
        "cargo:rustc-env=PLAYWRIGHT_DRIVER_VERSION={}",
        PLAYWRIGHT_VERSION
    );
    println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_PLATFORM={}", platform);

    // Node executable path
    let node_exe = if cfg!(windows) {
        driver_dir.join("node.exe")
    } else {
        driver_dir.join("node")
    };

    if node_exe.exists() {
        println!("cargo:rustc-env=PLAYWRIGHT_NODE_EXE={}", node_exe.display());
    }

    // CLI.js path
    let cli_js = driver_dir.join("package").join("cli.js");
    if cli_js.exists() {
        println!("cargo:rustc-env=PLAYWRIGHT_CLI_JS={}", cli_js.display());
    }
}
