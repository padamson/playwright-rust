// Install browsers matching the bundled Playwright driver
//
// Copy this file into your own examples/ directory and run it in CI
// (in a workspace, add `-p <your-package>`):
//   cargo run --example install-browsers -- chromium firefox webkit
// The driver version rides Cargo.lock, so a dependabot bump of the crate
// moves the crate, driver, and browsers together with no workflow edit.
// Requires tokio with the `macros` and `rt-multi-thread` features.
//
// Pass `--with-deps` to also install the system libraries the browsers
// need (Linux CI typically wants this; it runs the package manager under
// sudo). Without it, only browsers install — matching `npx playwright
// install` on every platform.

use playwright_rs::{install_browsers, install_browsers_with_deps};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let with_deps = args.iter().any(|arg| arg == "--with-deps");
    let browsers: Vec<&str> = args
        .iter()
        .filter(|arg| *arg != "--with-deps")
        .map(String::as_str)
        .collect();
    let selection = (!browsers.is_empty()).then_some(browsers.as_slice());
    if with_deps {
        install_browsers_with_deps(selection).await?;
    } else {
        install_browsers(selection).await?;
    }
    Ok(())
}
