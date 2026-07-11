//! playwright-rs CLI — bootstrap the Playwright driver into a stable
//! user cache and install browsers.
//!
//! For downstream binaries distributed via `cargo install`, the
//! compile-time `$OUT_DIR` driver path is invalidated when Cargo cleans
//! up the build's `target/`. Running `playwright-rs install` populates
//! `dirs::cache_dir()/playwright-rust/<version>/`, which the library's
//! runtime resolution chain probes after the bundled lookup.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

// Download + assembly logic shared with build.rs (see ADR 0006: the driver
// is assembled from the npm `playwright-core` tarball + a pinned Node binary).
include!("../build_support/driver_assembly.rs");

#[derive(Parser)]
#[command(name = "playwright-rs", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Download the Playwright driver to the user cache (if missing) and install browsers.
    Install {
        /// Browser names to install (e.g. `chromium firefox webkit`). Omit to install all.
        browsers: Vec<String>,
        /// Pass `--with-deps` to the Playwright CLI (forced on Linux regardless).
        #[arg(long)]
        with_deps: bool,
        /// Populate the user-cache driver but skip the browser-install step.
        /// Intended for CI smoke tests and `cargo install` post-install bootstrap.
        #[arg(long)]
        driver_only: bool,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.cmd {
        Cmd::Install {
            browsers,
            with_deps,
            driver_only,
        } => match run_install(browsers, with_deps, driver_only).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("playwright-rs: {e}");
                ExitCode::FAILURE
            }
        },
    }
}

async fn run_install(
    browsers: Vec<String>,
    with_deps: bool,
    driver_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let version = env!("PLAYWRIGHT_DRIVER_VERSION");
    let platform = env!("PLAYWRIGHT_DRIVER_PLATFORM");

    let driver_dir = ensure_driver_in_user_cache(version, platform)?;
    eprintln!("Driver ready at: {}", driver_dir.display());

    if driver_only {
        return Ok(());
    }

    // The library's `install_browsers_with_deps` calls `get_driver_executable()`,
    // which probes the bundled path first. Override via `PLAYWRIGHT_DRIVER_PATH`
    // so the user-cache driver is used instead of whatever the compile-time
    // bundled path happens to be (which may not exist post-`cargo install`).
    // SAFETY: set_var is unsafe in Rust 2024 because env mutation isn't
    // thread-safe; we run before spawning any worker threads.
    unsafe {
        std::env::set_var("PLAYWRIGHT_DRIVER_PATH", &driver_dir);
    }

    let browser_refs: Vec<&str> = browsers.iter().map(String::as_str).collect();
    let browsers_arg: Option<&[&str]> = if browser_refs.is_empty() {
        None
    } else {
        Some(&browser_refs)
    };

    if with_deps {
        playwright_rs::install_browsers_with_deps(browsers_arg).await?;
    } else {
        playwright_rs::install_browsers(browsers_arg).await?;
    }

    Ok(())
}

/// Ensure the Playwright driver exists at
/// `<cache>/playwright-rust/<version>/playwright-<version>-<platform>/`.
/// Assembles it from npm + nodejs.org if absent. Returns the driver dir.
fn ensure_driver_in_user_cache(
    version: &str,
    platform: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cache_root = dirs::cache_dir().ok_or("could not determine user cache directory")?;
    let driver_dir = cache_root
        .join("playwright-rust")
        .join(version)
        .join(format!("playwright-{version}-{platform}"));

    let cli_js = driver_dir.join("package").join("cli.js");
    if cli_js.exists() {
        return Ok(driver_dir);
    }

    eprintln!("Assembling driver {version} (Node {NODE_VERSION}) for {platform}...");
    assemble_driver(&driver_dir, version, platform)?;

    Ok(driver_dir)
}
