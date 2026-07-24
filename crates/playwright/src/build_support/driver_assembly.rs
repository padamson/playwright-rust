// Driver assembly: download the `playwright-core` npm tarball plus a pinned
// Node.js binary and lay them out exactly like the discontinued prebuilt zip
// (`<dir>/node[.exe]` + `<dir>/package/cli.js`), so the runtime launch path
// is unchanged (see ADR 0006).
//
// `include!`d by `build.rs` and the cli binary — the two places that acquire
// a driver. Effectful (ureq / zip / flate2 / tar); the pure mapping lives in
// `driver_urls.rs`, included below, and is unit-tested from the lib suite.

include!("driver_urls.rs");

#[cfg(not(any(feature = "aws-lc", feature = "ring")))]
compile_error!("enable either the `aws-lc` or `ring` feature to select a TLS crypto backend");

/// Builds the crypto provider used by the driver's HTTP client.
///
/// `aws-lc` wins when both features are present so `--all-features` and
/// dependency feature unification remain usable. Consumers that need to keep
/// `ring` out of their dependency graph must disable default features before
/// enabling `aws-lc`.
#[cfg(feature = "aws-lc")]
fn tls_crypto_provider() -> rustls::crypto::CryptoProvider {
    rustls::crypto::aws_lc_rs::default_provider()
}

#[cfg(all(not(feature = "aws-lc"), feature = "ring"))]
fn tls_crypto_provider() -> rustls::crypto::CryptoProvider {
    rustls::crypto::ring::default_provider()
}

/// Downloads and assembles the driver into `driver_dir` for a Playwright
/// `platform` identifier (e.g. `mac-arm64`).
///
/// Assembles into a sibling temp directory and renames into place, so an
/// interrupted run can't leave a half-populated `driver_dir` that a later
/// exists()-check mistakes for a complete driver. Errors name the URL or
/// archive entry that failed, so an upstream move is a one-line diagnosis.
#[allow(dead_code)]
fn assemble_driver(
    driver_dir: &std::path::Path,
    driver_version: &str,
    platform: &str,
) -> std::io::Result<()> {
    use std::io;

    let triple = node_triple(platform).ok_or_else(|| {
        io::Error::other(format!("unsupported driver platform: {platform}"))
    })?;

    let parent = driver_dir
        .parent()
        .ok_or_else(|| io::Error::other("driver dir has no parent"))?;
    std::fs::create_dir_all(parent)?;

    // Temp dir next to the target so the final rename stays on one filesystem.
    let tmp = parent.join(format!(
        ".{}.partial-{}",
        driver_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("driver"),
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp)?;

    let result = (|| {
        extract_npm_package(&http_get(&npm_core_url(driver_version))?, &tmp)?;
        extract_node_binary(
            &http_get(&node_archive_url(NODE_VERSION, triple))?,
            NODE_VERSION,
            triple,
            &tmp,
        )?;

        // The layout contract the runtime launch path depends on.
        let node_name = if is_windows_platform(platform) {
            "node.exe"
        } else {
            "node"
        };
        for required in [tmp.join(node_name), tmp.join("package").join("cli.js")] {
            if !required.exists() {
                return Err(io::Error::other(format!(
                    "assembled driver is missing {}",
                    required.display()
                )));
            }
        }
        Ok(())
    })();

    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }

    // A concurrent run may have completed first; its result is equally good.
    if driver_dir.exists() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Ok(());
    }
    std::fs::rename(&tmp, driver_dir)
}

/// GET `url` fully into memory (driver artifacts are tens of MB; ureq's
/// default 10 MB body cap is lifted). Errors carry the URL.
#[allow(dead_code)]
fn http_get(url: &str) -> std::io::Result<Vec<u8>> {
    use std::io;
    use std::sync::Arc;

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::Rustls)
                .unversioned_rustls_crypto_provider(Arc::new(tls_crypto_provider()))
                .build(),
        )
        .build()
        .into();

    let mut response = agent
        .get(url)
        .call()
        .map_err(|e| io::Error::other(format!("download of {url} failed: {e}")))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(io::Error::other(format!(
            "download of {url} failed with status {status}"
        )));
    }
    response
        .body_mut()
        .with_config()
        .limit(u64::MAX)
        .read_to_vec()
        .map_err(|e| io::Error::other(format!("reading {url} failed: {e}")))
}

/// Extracts the npm tarball's `package/` subtree into `dest/package/`.
#[allow(dead_code)]
fn extract_npm_package(tgz: &[u8], dest: &std::path::Path) -> std::io::Result<()> {
    use std::io;

    let gz = flate2::read::GzDecoder::new(tgz);
    let mut archive = tar::Archive::new(gz);
    let mut entries = 0usize;
    for entry in archive.entries()? {
        let mut entry = entry?;
        if entry.path()?.starts_with("package") {
            // unpack_in guards against path traversal outside `dest`.
            entry.unpack_in(dest)?;
            entries += 1;
        }
    }
    if entries == 0 {
        return Err(io::Error::other(
            "npm tarball contained no package/ entries",
        ));
    }
    Ok(())
}

/// Extracts the `node` executable (plus Node's LICENSE) out of the Node dist
/// archive into `dest/node[.exe]`, matching the old bundle layout.
#[allow(dead_code)]
fn extract_node_binary(
    archive_bytes: &[u8],
    node_version: &str,
    triple: &str,
    dest: &std::path::Path,
) -> std::io::Result<()> {
    use std::io;
    use std::io::Read;

    let exe_inner = node_archive_exe_path(node_version, triple);
    let license_inner = format!("node-v{node_version}-{triple}/LICENSE");

    let mut found_exe = false;
    if triple.starts_with("win-") {
        let cursor = io::Cursor::new(archive_bytes.to_vec());
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| io::Error::other(format!("Node archive is not a valid zip: {e}")))?;
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| io::Error::other(format!("zip read failed: {e}")))?;
            let name = file.name().to_string();
            let out = if name == exe_inner {
                found_exe = true;
                dest.join("node.exe")
            } else if name == license_inner {
                dest.join("LICENSE")
            } else {
                continue;
            };
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            std::fs::write(&out, bytes)?;
        }
    } else {
        let gz = flate2::read::GzDecoder::new(archive_bytes);
        let mut archive = tar::Archive::new(gz);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();
            let out = if path == std::path::Path::new(&exe_inner) {
                found_exe = true;
                dest.join("node")
            } else if path == std::path::Path::new(&license_inner) {
                dest.join("LICENSE")
            } else {
                continue;
            };
            entry.unpack(&out)?;
            #[cfg(unix)]
            if out.ends_with("node") {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&out)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&out, perms)?;
            }
        }
    }

    if !found_exe {
        return Err(io::Error::other(format!(
            "Node archive did not contain {exe_inner}"
        )));
    }
    Ok(())
}
