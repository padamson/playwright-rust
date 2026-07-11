// Pure platform-mapping and URL construction for driver acquisition.
//
// The prebuilt driver zips died with the azureedge CDN, so the driver is
// assembled from two artifacts: the `playwright-core` npm tarball and a
// pinned Node.js binary (see ADR 0006). This file is std-only so it can be
// `include!`d by `build.rs` and the cli binary, and unit-tested from the
// library suite without pulling the download deps into the lib.

/// Node.js runtime bundled with the driver. Must match the upstream
/// Playwright release's own pin for `PLAYWRIGHT_VERSION` (playwright-python
/// commits it as `NODE_VERSION` at the matching tag).
#[allow(dead_code)]
const NODE_VERSION: &str = "24.17.0";

/// Playwright platform identifier for an (os, arch) pair, e.g.
/// `("macos", "aarch64")` → `mac-arm64`. `None` for unsupported pairs.
#[allow(dead_code)]
fn playwright_platform(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("macos", "x86_64") => Some("mac"),
        ("macos", "aarch64") => Some("mac-arm64"),
        ("linux", "x86_64") => Some("linux"),
        ("linux", "aarch64") => Some("linux-arm64"),
        ("windows", "x86_64") => Some("win32_x64"),
        ("windows", "aarch64") => Some("win32_arm64"),
        _ => None,
    }
}

/// Node dist triple for a Playwright platform identifier, e.g.
/// `mac-arm64` → `darwin-arm64` (Node names platforms differently).
#[allow(dead_code)]
fn node_triple(playwright_platform: &str) -> Option<&'static str> {
    match playwright_platform {
        "mac" => Some("darwin-x64"),
        "mac-arm64" => Some("darwin-arm64"),
        "linux" => Some("linux-x64"),
        "linux-arm64" => Some("linux-arm64"),
        "win32_x64" => Some("win-x64"),
        "win32_arm64" => Some("win-arm64"),
        _ => None,
    }
}

/// Whether a Playwright platform identifier is a Windows target (Node ships
/// a `.zip` with `node.exe` there; `.tar.gz` with `bin/node` elsewhere).
#[allow(dead_code)]
fn is_windows_platform(playwright_platform: &str) -> bool {
    playwright_platform.starts_with("win32")
}

/// npm registry URL of the `playwright-core` tarball for a driver version.
#[allow(dead_code)]
fn npm_core_url(driver_version: &str) -> String {
    format!("https://registry.npmjs.org/playwright-core/-/playwright-core-{driver_version}.tgz")
}

/// nodejs.org URL of the Node binary archive for a Node dist triple.
#[allow(dead_code)]
fn node_archive_url(node_version: &str, triple: &str) -> String {
    let ext = if triple.starts_with("win-") {
        "zip"
    } else {
        "tar.gz"
    };
    format!("https://nodejs.org/dist/v{node_version}/node-v{node_version}-{triple}.{ext}")
}

/// Path of the `node` executable inside the Node dist archive.
#[allow(dead_code)]
fn node_archive_exe_path(node_version: &str, triple: &str) -> String {
    if triple.starts_with("win-") {
        format!("node-v{node_version}-{triple}/node.exe")
    } else {
        format!("node-v{node_version}-{triple}/bin/node")
    }
}

#[cfg(test)]
mod driver_urls_tests {
    use super::*;

    const ALL: [(&str, &str, &str, &str); 6] = [
        ("macos", "x86_64", "mac", "darwin-x64"),
        ("macos", "aarch64", "mac-arm64", "darwin-arm64"),
        ("linux", "x86_64", "linux", "linux-x64"),
        ("linux", "aarch64", "linux-arm64", "linux-arm64"),
        ("windows", "x86_64", "win32_x64", "win-x64"),
        ("windows", "aarch64", "win32_arm64", "win-arm64"),
    ];

    #[test]
    fn every_supported_target_maps_to_platform_and_triple() {
        for (os, arch, platform, triple) in ALL {
            assert_eq!(playwright_platform(os, arch), Some(platform));
            assert_eq!(node_triple(platform), Some(triple));
        }
        assert_eq!(playwright_platform("freebsd", "x86_64"), None);
        assert_eq!(node_triple("solaris"), None);
    }

    #[test]
    fn windows_platforms_are_detected() {
        assert!(is_windows_platform("win32_x64"));
        assert!(is_windows_platform("win32_arm64"));
        assert!(!is_windows_platform("mac-arm64"));
        assert!(!is_windows_platform("linux"));
    }

    #[test]
    fn npm_url_matches_registry_scheme() {
        assert_eq!(
            npm_core_url("1.61.1"),
            "https://registry.npmjs.org/playwright-core/-/playwright-core-1.61.1.tgz"
        );
    }

    #[test]
    fn node_urls_and_exe_paths_per_family() {
        assert_eq!(
            node_archive_url("24.17.0", "darwin-arm64"),
            "https://nodejs.org/dist/v24.17.0/node-v24.17.0-darwin-arm64.tar.gz"
        );
        assert_eq!(
            node_archive_url("24.17.0", "win-x64"),
            "https://nodejs.org/dist/v24.17.0/node-v24.17.0-win-x64.zip"
        );
        assert_eq!(
            node_archive_exe_path("24.17.0", "linux-x64"),
            "node-v24.17.0-linux-x64/bin/node"
        );
        assert_eq!(
            node_archive_exe_path("24.17.0", "win-arm64"),
            "node-v24.17.0-win-arm64/node.exe"
        );
    }

    #[test]
    fn node_pin_is_a_bare_semver() {
        // The xtask drift guard and the nodejs.org URL scheme both assume a
        // bare MAJOR.MINOR.PATCH (no leading 'v').
        let parts: Vec<&str> = NODE_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "NODE_VERSION must be MAJOR.MINOR.PATCH");
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
    }
}
