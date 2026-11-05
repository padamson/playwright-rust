// Pure platform mapping, URL construction, and pinned artifact digests for
// driver acquisition.
//
// The prebuilt driver zips died with the azureedge CDN, so the driver is
// assembled from two artifacts: the `playwright-core` npm tarball and a
// pinned Node.js binary (see ADR 0006). This file is std-only so it can be
// `include!`d by `build.rs` and the cli binary, and unit-tested from the
// library suite without pulling the download deps into the lib.

/// Node.js runtime bundled with the driver. Must match the upstream
/// Playwright release's own pin for `PLAYWRIGHT_VERSION` (playwright-python
/// commits it as `NODE_VERSION` at the matching tag).
const NODE_VERSION: &str = "24.21.0";

/// npm's `dist.integrity` for the pinned `playwright-core` tarball, with the
/// version it belongs to: `npm view playwright-core@<version> dist.integrity`.
const NPM_CORE_INTEGRITY: (&str, &str) = (
    "1.64.1",
    "sha512-BC4s8y6SEVA6Q1rSxZTto8hI1KUI3xXlHGZgvGqP4IqPdI/a6FX9H29IDNz7vKo9l21b6LfYosScPL6M3xxyDQ==",
);

/// SHA-256 of each Node dist archive, per Node triple, with the Node version
/// they belong to, from `https://nodejs.org/dist/v<version>/SHASUMS256.txt`.
const NODE_ARCHIVE_SHA256: (&str, [(&str, &str); 6]) = (
    "24.21.0",
    [
        (
            "darwin-x64",
            "1462cb3b3046b815cf8ea436d3da450ec1a9f11dac7e5a46b0ada5305d7e8097",
        ),
        (
            "darwin-arm64",
            "bed7eea5325e1108f32ce5228ddd6a5f0f08a499ee42aa7442aea583702f6057",
        ),
        (
            "linux-x64",
            "6e1db87ef58b8819e5d5402eff1536491b18edd8eb7bee5ef7897876e88dc5ff",
        ),
        (
            "linux-arm64",
            "724282c3b43aec998aa9527380465b45d229e021b58035f5f4f63095eabfe5d5",
        ),
        (
            "win-x64",
            "158f7685b44de51f6c0df1d153526cbcd3e1bc739a8dfc607721cef75de9e541",
        ),
        (
            "win-arm64",
            "8779b1bde1d39f8d420e3b57aa657b39891af434d3de44a919044cec06785921",
        ),
    ],
);

/// The digest a downloaded artifact must have before it is unpacked, in
/// the form its publisher prints it.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PinnedDigest {
    /// An npm `dist.integrity`: `sha512-` and the base64 digest.
    Sha512Integrity(&'static str),
    /// Lowercase hex, as `SHASUMS256.txt` prints it.
    Sha256Hex(&'static str),
}

/// The pinned digest of the `playwright-core` tarball, or `None` when the pin
/// belongs to a different driver version.
#[allow(dead_code)]
fn npm_core_digest(driver_version: &str) -> Option<PinnedDigest> {
    let (version, sri) = NPM_CORE_INTEGRITY;
    (version == driver_version).then_some(PinnedDigest::Sha512Integrity(sri))
}

/// The pinned digest of a Node dist archive, or `None` when the pins belong
/// to a different Node version or name no such triple.
#[allow(dead_code)]
fn node_archive_digest(node_version: &str, triple: &str) -> Option<PinnedDigest> {
    let (version, archives) = NODE_ARCHIVE_SHA256;
    if version != node_version {
        return None;
    }
    archives
        .iter()
        .find(|(t, _)| *t == triple)
        .map(|(_, hex)| PinnedDigest::Sha256Hex(hex))
}

/// Playwright platform identifier for an (os, arch) pair, e.g.
/// `("macos", "aarch64")` → `mac-arm64`. `None` for unsupported pairs.
#[allow(dead_code)]
pub(crate) fn playwright_platform(os: &str, arch: &str) -> Option<&'static str> {
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
pub(crate) fn node_triple(playwright_platform: &str) -> Option<&'static str> {
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
pub(crate) fn is_windows_platform(playwright_platform: &str) -> bool {
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

/// Name of the runtime binary inside a driver directory for a Playwright
/// platform identifier: `node.exe` on Windows targets, `node` elsewhere.
///
/// The one place this is spelled out. The build script, the assembler, and
/// the runtime lookup all probe for this file, and they must agree.
pub(crate) fn node_exe_name(playwright_platform: &str) -> &'static str {
    if is_windows_platform(playwright_platform) {
        "node.exe"
    } else {
        "node"
    }
}

/// Name of an assembled driver's directory,
/// `playwright-<version>-<platform>-node<node>`, wherever it is put. The Node
/// version is part of it because the driver bundles its runtime: without it,
/// moving [`NODE_VERSION`] would leave every directory already assembled
/// running the old Node.
pub(crate) fn driver_dir_name(version: &str, platform: &str) -> String {
    format!("playwright-{version}-{platform}-node{NODE_VERSION}")
}

/// Where an assembled driver lives under a cache root:
/// `<root>/playwright-rust/<version>/<driver_dir_name>`.
///
/// One layout shared by the build script (which assembles there by
/// default), the CLI (`playwright-rs install`), and the runtime lookup, so
/// a driver produced by any of them is found by the others.
pub(crate) fn cached_driver_dir(
    cache_root: &std::path::Path,
    version: &str,
    platform: &str,
) -> std::path::PathBuf {
    cache_root
        .join("playwright-rust")
        .join(version)
        .join(driver_dir_name(version, platform))
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
    fn the_runtime_binary_is_node_exe_only_on_windows_targets() {
        assert_eq!(node_exe_name("win32_x64"), "node.exe");
        assert_eq!(node_exe_name("win32_arm64"), "node.exe");
        assert_eq!(node_exe_name("mac-arm64"), "node");
        assert_eq!(node_exe_name("linux"), "node");
    }

    #[test]
    fn cached_driver_dir_is_keyed_by_version_platform_and_node() {
        assert_eq!(
            cached_driver_dir(std::path::Path::new("/c"), "1.63.0", "mac-arm64"),
            std::path::PathBuf::from(format!(
                "/c/playwright-rust/1.63.0/playwright-1.63.0-mac-arm64-node{NODE_VERSION}"
            ))
        );
    }

    #[test]
    fn every_node_triple_has_a_pinned_sha256() {
        for (_, _, _, triple) in ALL {
            let Some(PinnedDigest::Sha256Hex(hex)) = node_archive_digest(NODE_VERSION, triple)
            else {
                panic!("no pinned SHA-256 for the {triple} Node archive");
            };
            assert_eq!(hex.len(), 64, "{triple}: {hex}");
            assert!(
                hex.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "{triple}: {hex}"
            );
        }
        assert_eq!(node_archive_digest(NODE_VERSION, "solaris-x64"), None);
        assert_eq!(node_archive_digest("0.0.0", "linux-x64"), None);
    }

    #[test]
    fn the_pinned_driver_has_an_npm_integrity() {
        let Some(PinnedDigest::Sha512Integrity(sri)) =
            npm_core_digest(env!("PLAYWRIGHT_DRIVER_VERSION"))
        else {
            panic!("no pinned integrity for the pinned playwright-core");
        };
        let b64 = sri.strip_prefix("sha512-").expect(sri);
        assert_eq!(b64.len(), 88, "{sri}");
        assert!(b64.ends_with("=="), "{sri}");
        assert_eq!(npm_core_digest("0.0.0"), None);
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
