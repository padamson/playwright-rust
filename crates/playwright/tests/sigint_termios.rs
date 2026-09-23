// Reproducer for issue #59: Ctrl-C while Playwright is running breaks the
// shell's terminal mode (non-canonical, line editing destroyed).
//
// Strategy:
//   1. Build the `sigint_repro` example as a release binary.
//   2. Drive it from `expect(1)` inside a real pty (see
//      `sigint_termios_harness.exp`), which:
//        - snapshots `stty -a` before the run,
//        - launches the binary, waits for its READY marker,
//        - sends Ctrl-C,
//        - snapshots `stty -a` afterwards.
//   3. Diff the two snapshots — any change is the bug.
//
// Skipped on Windows (no expect/pty pair model). The CI workflow installs
// expect on Linux; macOS ships it.

#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn expect_available() -> bool {
    Command::new("expect")
        .arg("-v")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// Heavy: drives four chromium-launching scenarios through an expect/pty
// harness (~25s, dominated by the per-scenario browser launches, not the
// build). Kept out of the default dev-loop run; CI exercises it in the
// `--run-ignored` lane (single-threaded, which suits a terminal test).
// Run locally with:
//   cargo nextest run -p playwright-rs --run-ignored ignored-only -E 'test(sigint)'
#[test]
#[ignore = "heavy pty/expect + chromium harness; runs in CI's --run-ignored lane"]
fn sigint_does_not_break_terminal_termios() {
    // macOS ships expect and the Linux job installs it; a missing binary is
    // a broken lane, not a reason to report a pass.
    assert!(
        expect_available(),
        "`expect` is not on PATH; install it (apt-get install expect) to run this lane"
    );

    let root = workspace_root();

    // Build the reproducer example in debug, not release: the integration
    // suite already compiles playwright-rs in debug, so this reuses that
    // rlib and only links the small example (seconds) instead of a cold
    // release build of the whole crate (~30s). The terminal-termios
    // behavior under SIGINT is independent of optimization level.
    let build_status = Command::new(env!("CARGO"))
        .args([
            "build",
            "--example",
            "sigint_repro",
            "--package",
            "playwright-rs",
        ])
        .current_dir(&root)
        .status()
        .expect("failed to invoke cargo build");
    assert!(build_status.success(), "cargo build of sigint_repro failed");

    let bin = root.join("target/debug/examples/sigint_repro");
    assert!(bin.is_file(), "expected built example at {}", bin.display());

    // Kept when the test panics on any path (harness exit, missing snapshot,
    // termios diff), since the captures are what a reader of the failure
    // needs; a passing run leaves nothing behind.
    struct KeepOnPanic(Option<tempfile::TempDir>);
    impl Drop for KeepOnPanic {
        fn drop(&mut self) {
            if std::thread::panicking()
                && let Some(dir) = self.0.take()
            {
                let kept = dir.keep();
                eprintln!("[sigint_termios] raw captures kept in {}", kept.display());
            }
        }
    }
    let scratch_dir = KeepOnPanic(Some(
        tempfile::Builder::new()
            .prefix("playwright-rs-sigint-termios")
            .tempdir()
            .expect("create scratch dir"),
    ));
    let scratch = scratch_dir
        .0
        .as_ref()
        .expect("scratch dir is set until drop")
        .path()
        .to_path_buf();

    let harness =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/sigint_termios_harness.exp");
    assert!(harness.is_file(), "harness not at {}", harness.display());

    let status = Command::new("expect")
        .arg(&harness)
        .arg(&bin)
        .arg(&scratch)
        .status()
        .expect("failed to invoke expect");
    assert!(status.success(), "expect harness exited non-zero");

    let baseline = scratch.join("stty-baseline");
    let baseline_txt =
        normalize_stty(&std::fs::read_to_string(&baseline).expect("missing baseline"));
    let scenarios = [
        "A_before_ready",
        "B_after_ready",
        "C_double_ctrlc",
        "D_with_profile",
    ];

    let mut bugs = Vec::new();
    for name in scenarios {
        let stty_path = scratch.join(format!("stty-{name}"));
        match std::fs::read_to_string(&stty_path) {
            Ok(raw) => {
                let txt = normalize_stty(&raw);
                if txt != baseline_txt {
                    eprintln!("[sigint_termios] {name}: TERMIOS CHANGED — bug reproduced.");
                    eprintln!("--- baseline\n{baseline_txt}");
                    eprintln!("--- {name}\n{txt}");
                    bugs.push(name);
                } else {
                    eprintln!("[sigint_termios] {name}: termios unchanged");
                }
            }
            Err(e) => eprintln!("[sigint_termios] {name} stty snapshot missing ({e})"),
        }
    }

    assert!(
        bugs.is_empty(),
        "termios changed across Ctrl-C in scenarios: {bugs:?} (issue #59)"
    );
}

/// Strips transient stty flags that flap based on observation timing
/// rather than reflecting real terminal-mode changes. `pendin` toggles
/// when there is unread input in the kernel buffer; `flusho` indicates
/// output is being discarded mid-flush. Both are observer effects, not
/// the icanon/echo state we care about for #59.
fn normalize_stty(s: &str) -> String {
    s.split_whitespace()
        .filter(|tok| !matches!(*tok, "pendin" | "-pendin" | "flusho" | "-flusho"))
        .collect::<Vec<_>>()
        .join(" ")
}
