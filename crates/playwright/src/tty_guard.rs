// Defensive Ctrl-C handling for stdin termios state.
//
// Background: when a Playwright session is interrupted with SIGINT (Ctrl-C),
// the spawned Node driver and its chromium subprocesses can clobber the
// controlling terminal's `termios` state in subtle ways and never restore
// it, leaving the user's shell in non-canonical mode where arrow keys
// echo as raw `^[[D` sequences instead of moving the cursor (issue #59).
//
// The exact subprocess responsible has not been pinpointed — the symptom
// reproduces in some terminal environments but not others, and we have
// not been able to recreate it inside an `expect`-allocated pty across
// macOS/Linux ARM64/Linux x64 CI runners. The defensive fix here is
// agnostic to which process is the offender:
//
//   1. At `Playwright::launch`, snapshot stdin's termios if stdin is a tty.
//   2. Install a one-shot SIGINT handler that restores the snapshot before
//      letting the process die with the conventional 130 exit code.
//   3. The `Drop` impl on `Playwright` also restores the snapshot so the
//      same protection applies to graceful exits and panics.
//
// The signal handler can be disabled by setting the
// `PLAYWRIGHT_NO_SIGNAL_HANDLER` environment variable (any non-empty
// value) — for users who manage their own SIGINT handlers and don't
// want this library overriding them.
//
// Stub on Windows: the symptom in #59 is Unix-specific and Windows uses
// a different console-mode model. The whole module compiles to no-ops
// on Windows.

#[cfg(unix)]
mod imp {
    use parking_lot::Mutex;
    use std::sync::OnceLock;

    static SAVED: OnceLock<Mutex<Option<libc::termios>>> = OnceLock::new();
    static HANDLER_INSTALLED: OnceLock<()> = OnceLock::new();

    /// Snapshot stdin's termios if stdin is a tty. Idempotent — only the
    /// first call records a snapshot; later calls are no-ops so we never
    /// overwrite the original "clean" state with a possibly-already-clobbered
    /// one.
    pub(crate) fn save_if_tty() {
        let cell = SAVED.get_or_init(|| Mutex::new(None));
        let mut guard = cell.lock();
        if guard.is_some() {
            return;
        }
        // SAFETY: tcgetattr writes to the termios pointer on success and
        // ignores it on failure. Stdin (fd 0) is always a valid file
        // descriptor in a hosted environment.
        unsafe {
            if libc::isatty(0) != 1 {
                return;
            }
            let mut t: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(0, &mut t) == 0 {
                *guard = Some(t);
            }
        }
    }

    /// Restore the saved termios to stdin. No-op if nothing was saved
    /// (stdin wasn't a tty or save_if_tty was never called).
    pub(crate) fn restore() {
        let Some(cell) = SAVED.get() else { return };
        let guard = cell.lock();
        let Some(t) = *guard else { return };
        // SAFETY: tcsetattr reads from the termios pointer and applies
        // it. Failure is acceptable (e.g. stdin no longer a tty) and we
        // ignore the return code.
        unsafe {
            let _ = libc::tcsetattr(0, libc::TCSANOW, &t);
        }
    }

    /// Install a one-shot SIGINT handler that restores termios and exits
    /// with code 130. Returns immediately if already installed or if the
    /// `PLAYWRIGHT_NO_SIGNAL_HANDLER` env var is set.
    pub(crate) fn install_signal_handler() {
        if std::env::var_os("PLAYWRIGHT_NO_SIGNAL_HANDLER").is_some() {
            return;
        }
        if HANDLER_INSTALLED.set(()).is_err() {
            return;
        }

        tokio::spawn(async move {
            // tokio::signal::ctrl_c registers a SIGINT listener via the
            // tokio runtime; multiple listeners can coexist with user
            // handlers, but our task gets to act first and exit the
            // process cleanly.
            if tokio::signal::ctrl_c().await.is_ok() {
                restore();
                // 128 + SIGINT(2) = 130, the conventional exit code for
                // Ctrl-C interrupted programs.
                std::process::exit(130);
            }
        });
    }
}

#[cfg(not(unix))]
mod imp {
    pub(crate) fn save_if_tty() {}
    pub(crate) fn restore() {}
    pub(crate) fn install_signal_handler() {}
}

pub(crate) use imp::{install_signal_handler, restore, save_if_tty};
