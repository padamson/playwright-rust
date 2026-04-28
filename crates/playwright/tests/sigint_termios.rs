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
// Skipped on Windows (no expect/pty pair model) and when `expect` isn't
// installed on the runner. The CI workflow installs expect on Linux/macOS.

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

#[test]
fn sigint_does_not_break_terminal_termios() {
    if !expect_available() {
        eprintln!("[sigint_termios] `expect` not on PATH — skipping");
        return;
    }

    let root = workspace_root();

    // Build the reproducer example in release mode.
    let build_status = Command::new(env!("CARGO"))
        .args([
            "build",
            "--release",
            "--example",
            "sigint_repro",
            "--package",
            "playwright-rs",
        ])
        .current_dir(&root)
        .status()
        .expect("failed to invoke cargo build");
    assert!(build_status.success(), "cargo build of sigint_repro failed");

    let bin = root.join("target/release/examples/sigint_repro");
    assert!(bin.is_file(), "expected built example at {}", bin.display());

    let scratch = std::env::temp_dir().join("playwright-rs-sigint-termios");
    let _ = std::fs::remove_dir_all(&scratch);

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
        "termios changed across Ctrl-C in scenarios: {bugs:?} (issue #59)\n\
         (raw captures in {})",
        scratch.display()
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
