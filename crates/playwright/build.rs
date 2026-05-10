//! Build script for playwright-rs
//!
//! Downloads and extracts the Playwright Node.js driver from Azure CDN
//! into Cargo's `$OUT_DIR`. The runtime side (`src/server/driver.rs`)
//! picks the path up via compile-time `option_env!()` lookups. Because
//! `$OUT_DIR` lives inside `target/`, the driver is automatically cached
//! by any `target/`-cache configuration in CI (e.g. `Swatinem/rust-cache`).
//! No manual driver-cache step is needed.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const PLAYWRIGHT_VERSION: &str = "1.59.1";
const DRIVER_BASE_URL: &str = "https://playwright.azureedge.net/builds/driver";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Skip driver download on docs.rs — it has no network access and doesn't need drivers
    if std::env::var("DOCS_RS").is_ok() {
        println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_DIR=");
        println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_VERSION={PLAYWRIGHT_VERSION}");
        println!("cargo:rustc-env=PLAYWRIGHT_DRIVER_PLATFORM=docs-rs");
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR set by Cargo"));
    let drivers_dir = out_dir.join("playwright-driver");

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

    println!("cargo:warning=Downloading Playwright driver {PLAYWRIGHT_VERSION} for {platform}...");

    match download_and_extract_driver(&drivers_dir, platform) {
        Ok(extracted_dir) => {
            println!(
                "cargo:warning=Playwright driver downloaded to {}",
                extracted_dir.display()
            );
            set_output_env_vars(&extracted_dir, platform);
        }
        Err(e) => {
            println!("cargo:warning=Failed to download Playwright driver: {e}");
            println!("cargo:warning=The driver will need to be installed manually or via npm.");
            println!(
                "cargo:warning=You can set PLAYWRIGHT_DRIVER_PATH to specify driver location."
            );
        }
    }
}

/// Detect the current platform and return the Playwright platform identifier
fn detect_platform() -> &'static str {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match (os, arch) {
        ("macos", "x86_64") => "mac",
        ("macos", "aarch64") => "mac-arm64",
        ("linux", "x86_64") => "linux",
        ("linux", "aarch64") => "linux-arm64",
        ("windows", "x86_64") => "win32_x64",
        ("windows", "aarch64") => "win32_arm64",
        _ => {
            println!("cargo:warning=Unsupported platform: {} {}", os, arch);
            println!("cargo:warning=Defaulting to linux platform");
            "linux"
        }
    }
}

/// Download and extract the Playwright driver
fn download_and_extract_driver(drivers_dir: &Path, platform: &str) -> io::Result<PathBuf> {
    // Create drivers directory
    fs::create_dir_all(drivers_dir)?;

    // Download URL
    let filename = format!("playwright-{}-{}.zip", PLAYWRIGHT_VERSION, platform);
    let url = format!("{}/{}", DRIVER_BASE_URL, filename);

    println!("cargo:warning=Downloading from: {}", url);

    // Download the file via ureq (synchronous, minimal dep tree).
    let mut response = ureq::get(&url)
        .call()
        .map_err(|e| io::Error::other(format!("Download failed: {}", e)))?;

    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(io::Error::other(format!(
            "Download failed with status: {}",
            status
        )));
    }

    // The driver archive is ~50 MB; ureq's default body limit is 10 MB.
    // Lift the limit for this trusted Microsoft CDN download.
    let bytes: Vec<u8> = response
        .body_mut()
        .with_config()
        .limit(u64::MAX)
        .read_to_vec()
        .map_err(|e| io::Error::other(format!("Failed to read response: {}", e)))?;

    println!("cargo:warning=Downloaded {} bytes", bytes.len());

    // Extract ZIP file
    let cursor = io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| io::Error::other(format!("Failed to open ZIP: {}", e)))?;

    let extract_dir = drivers_dir.join(format!("playwright-{}-{}", PLAYWRIGHT_VERSION, platform));

    println!("cargo:warning=Extracting to: {}", extract_dir.display());

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| io::Error::other(format!("Failed to read ZIP entry: {}", e)))?;

        let outpath = extract_dir.join(file.name());

        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;

            // Set executable permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                // Make executable: node binary and any shell scripts
                if outpath.ends_with("node")
                    || outpath.extension().and_then(|s| s.to_str()) == Some("sh")
                {
                    let mut perms = fs::metadata(&outpath)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&outpath, perms)?;
                }
            }
        }
    }

    println!(
        "cargo:warning=Successfully extracted {} files",
        archive.len()
    );

    Ok(extract_dir)
}

/// Set environment variables for use at runtime
fn set_output_env_vars(driver_dir: &Path, platform: &str) {
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
