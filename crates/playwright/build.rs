//! Build script for playwright-core
//!
//! Downloads and extracts the Playwright driver from Azure CDN during build time.
//! This matches the approach used by playwright-python, playwright-java, and playwright-dotnet.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Playwright driver version to download
const PLAYWRIGHT_VERSION: &str = "1.56.1";

/// Azure CDN base URL for Playwright drivers
const DRIVER_BASE_URL: &str = "https://playwright.azureedge.net/builds/driver";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Get the appropriate drivers directory using robust workspace detection
    let drivers_dir = get_drivers_dir();

    // Detect platform
    let platform = detect_platform();
    let driver_dir = drivers_dir.join(format!("playwright-{}-{}", PLAYWRIGHT_VERSION, platform));

    // Check if driver already exists
    if driver_dir.exists() {
        // Driver already downloaded, silently use it
        set_output_env_vars(&driver_dir, platform);
        return;
    }

    // Download and extract driver
    println!(
        "cargo:warning=Downloading Playwright driver {} for {}...",
        PLAYWRIGHT_VERSION, platform
    );

    match download_and_extract_driver(&drivers_dir, platform) {
        Ok(extracted_dir) => {
            println!(
                "cargo:warning=Playwright driver downloaded to {}",
                extracted_dir.display()
            );
            set_output_env_vars(&extracted_dir, platform);
        }
        Err(e) => {
            println!("cargo:warning=Failed to download Playwright driver: {}", e);
            println!("cargo:warning=The driver will need to be installed manually or via npm.");
            println!(
                "cargo:warning=You can set PLAYWRIGHT_DRIVER_PATH to specify driver location."
            );
        }
    }
}

/// Get the drivers directory using robust workspace detection
///
/// This function handles multiple scenarios:
/// 1. Development within playwright-rust workspace
/// 2. Used as a dependency from crates.io in a workspace project
/// 3. Used as a dependency from crates.io in a non-workspace project
///
/// The detection strategy:
/// 1. Try CARGO_WORKSPACE_DIR (available in Rust 1.73+) - gets the dependent project's workspace
/// 2. Walk up directory tree looking for Cargo.toml with [workspace]
/// 3. Fallback to platform-specific cache directory (like playwright-python)
fn get_drivers_dir() -> PathBuf {
    // Strategy 1: Use CARGO_WORKSPACE_DIR if available (Rust 1.73+)
    // This points to the workspace root of the project being built (not playwright-core)
    if let Ok(workspace_dir) = env::var("CARGO_WORKSPACE_DIR") {
        let drivers_dir = PathBuf::from(workspace_dir).join("drivers");
        println!(
            "cargo:warning=Using workspace drivers directory: {}",
            drivers_dir.display()
        );
        return drivers_dir;
    }

    // Strategy 2: Walk up the directory tree to find a workspace Cargo.toml
    // This handles cases where CARGO_WORKSPACE_DIR isn't available
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let mut current = manifest_dir.as_path();
    while let Some(parent) = current.parent() {
        let cargo_toml = parent.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(contents) = fs::read_to_string(&cargo_toml) {
                if contents.contains("[workspace]") {
                    let drivers_dir = parent.join("drivers");
                    println!("cargo:warning=Found workspace at: {}", parent.display());
                    println!(
                        "cargo:warning=Using drivers directory: {}",
                        drivers_dir.display()
                    );
                    return drivers_dir;
                }
            }
        }
        current = parent;
    }

    // Strategy 3: Fallback to platform-specific cache directory
    // This matches playwright-python's approach and works in all scenarios
    let cache_dir = dirs::cache_dir()
        .expect("Could not determine cache directory")
        .join("playwright-rust")
        .join("drivers");

    println!(
        "cargo:warning=No workspace found, using cache directory: {}",
        cache_dir.display()
    );
    println!("cargo:warning=This matches playwright-python's approach for system-wide driver installation");

    cache_dir
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

    // Download the file
    let response = reqwest::blocking::get(&url)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Download failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Download failed with status: {}", response.status()),
        ));
    }

    // Read response bytes
    let bytes = response.bytes().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to read response: {}", e),
        )
    })?;

    println!("cargo:warning=Downloaded {} bytes", bytes.len());

    // Extract ZIP file
    let cursor = io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to open ZIP: {}", e)))?;

    let extract_dir = drivers_dir.join(format!("playwright-{}-{}", PLAYWRIGHT_VERSION, platform));

    println!("cargo:warning=Extracting to: {}", extract_dir.display());

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to read ZIP entry: {}", e),
            )
        })?;

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
