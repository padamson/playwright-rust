use std::process::Command;

#[test]
#[ignore = "downloads ~50MB Playwright driver from CDN"]
fn install_driver_only_populates_user_cache() {
    let bin = env!("CARGO_BIN_EXE_playwright-rs");
    let temp = tempfile::tempdir().expect("create tempdir");

    let mut cmd = Command::new(bin);
    cmd.args(["install", "--driver-only"]);
    cmd.env("HOME", temp.path());
    cmd.env("XDG_CACHE_HOME", temp.path().join(".cache"));
    cmd.env("LOCALAPPDATA", temp.path().join("AppData").join("Local"));

    let output = cmd.output().expect("spawn playwright-rs");

    assert!(
        output.status.success(),
        "playwright-rs install --driver-only failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Driver ready at:"),
        "expected 'Driver ready at:' in stderr, got: {stderr}",
    );
    assert!(
        contains_cli_js(temp.path()),
        "no cli.js found anywhere under {}",
        temp.path().display(),
    );
}

#[test]
#[ignore = "downloads ~50MB Playwright driver from CDN"]
fn install_driver_only_second_invocation_is_idempotent() {
    let bin = env!("CARGO_BIN_EXE_playwright-rs");
    let temp = tempfile::tempdir().expect("create tempdir");

    for _ in 0..2 {
        let mut cmd = Command::new(bin);
        cmd.args(["install", "--driver-only"]);
        cmd.env("HOME", temp.path());
        cmd.env("XDG_CACHE_HOME", temp.path().join(".cache"));
        cmd.env("LOCALAPPDATA", temp.path().join("AppData").join("Local"));
        let status = cmd.status().expect("spawn playwright-rs");
        assert!(status.success(), "second invocation failed");
    }
}

fn contains_cli_js(root: &std::path::Path) -> bool {
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if contains_cli_js(&path) {
                return true;
            }
        } else if path.file_name().is_some_and(|n| n == "cli.js") {
            return true;
        }
    }
    false
}
