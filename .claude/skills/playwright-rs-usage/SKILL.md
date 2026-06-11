---
name: playwright-rs-usage
description: Procedural reference for using playwright-rs in Rust browser-automation code — object model (Browser/Context/Page/Locator), the `locator!()` macro, builder pattern for options, auto-wait semantics, and how to capture / inspect traces for failure diagnosis. Use when writing tests or scripts with playwright-rs as a dependency. Loaded automatically when the current repo has playwright-rs in its Cargo.toml; can also be copied into downstream projects at `.claude/skills/playwright-rs-usage/`.
---

# Using playwright-rs

Rust bindings for [Microsoft Playwright](https://playwright.dev). This
crate is a thin JSON-RPC client to the upstream Playwright server, so
the API mirrors playwright-python / java / .NET semantics. When
unsure about a method's behavior, the
[upstream Playwright docs](https://playwright.dev/docs/api) are the
authoritative reference.

**Drift discipline.** The canonical API tour lives in the crate-level
rustdoc at <https://docs.rs/playwright-rs> — that's compile-checked
against the actual code. This skill is the "what to reach for, what
to avoid" overlay: durable conventions, not a method-by-method
reference. The one Rust code block below is compile-checked by
`cargo xtask verify-agent-docs`; the rest is prose that ages well
because it talks in concepts (auto-wait, builder pattern) rather
than specific method names.

## Object model

```text
Playwright            start here — Playwright::launch().await?
  └── BrowserType     .chromium() / .firefox() / .webkit()
        └── Browser   .launch().await? → owns the browser process
              └── BrowserContext       isolated cookies / storage
                    └── Page            one tab
                          └── Locator   selector with auto-wait
```

`Locator` is the workhorse. Build with `page.locator(...)` or the
semantic `get_by_*` helpers (`get_by_role`, `get_by_text`, etc. —
see docs.rs for the full list) and chain action / assertion methods.

## Conventions to follow

- **`Result<T>` and `async/await` on `tokio`.** One error type:
  `playwright_rs::Error`. Use `?` to propagate.
- **Builders / setters for option-heavy methods.** `goto`, `click`,
  `screenshot`, `fill`, `tracing().start`, etc. take an `Options`
  struct. These are `#[non_exhaustive]` (so upstream option additions
  stay non-breaking) — struct literals won't compile. Construct with
  the type's `builder()` where it has one, otherwise chain setters off
  `Default`/`new()`: `GetByRoleOptions::default().name("OK").exact(true)`,
  `Cookie::new(name, value).domain("example.com")`. The exact method
  names live on docs.rs; don't memorize them.
- **Auto-wait + auto-retry.** Locator-based actions wait until the
  element is actionable; `expect()` assertions retry until they hold
  or time out. **Never insert `tokio::time::sleep` between an action
  and a check** — that's a smell. If a wait feels necessary, you
  probably want `expect(locator).to_be_visible().await` or similar.
- **`locator!()` macro for literal selectors.** Compile-time validation
  catches typos and structural errors. Fall back to `&str` only for
  selectors computed at runtime.
- **No reimplemented browser protocols.** This crate is intentionally
  thin over the Playwright server. Anything you can't do via Playwright
  itself, you can't do here.

## Minimal test skeleton

This block is compile-checked by `cargo xtask verify-agent-docs` —
if the API drifts, the verifier fails:

```rust,no_run
use anyhow::Result;
use playwright_rs::{Playwright, locator, expect};

#[tokio::test]
async fn login_flow() -> Result<()> {
    let pw = Playwright::launch().await?;
    let browser = pw.chromium().launch().await?;
    let context = browser.new_context().await?;
    let page = context.new_page().await?;

    page.goto("https://example.com/login", None).await?;
    page.locator(locator!("input[name='user']")).await.fill("alice", None).await?;
    page.locator(locator!("input[name='pass']")).await.fill("hunter2", None).await?;
    page.locator(locator!("text=Sign in")).await.click(None).await?;

    expect(page.locator(locator!(".welcome")).await).to_be_visible().await?;

    browser.close().await?;
    Ok(())
}
```

## Capabilities worth reaching for

Concept-level pointers; the exact options live on docs.rs.

- **Stable / redacted screenshots.** `ScreenshotOptions` carries
  `animations(Disabled)` for flake-free shots (freeze CSS animations
  before capture) and `mask`/`mask_color` to overpaint dynamic or
  sensitive elements. Reach for `animations(Disabled)` whenever a
  screenshot races an animation.
- **Context-level events.** Beyond per-page handlers, `BrowserContext`
  observes activity across *all* its pages — `on_download`,
  `on_page_load` / `on_page_close`,
  `on_frame_attached` / `_detached` / `_navigated` — and
  `Browser::on_context` fires for each new context. Use these for
  multi-tab fixtures instead of wiring every page individually.
- **HAR network capture.** `tracing().start_har(path, ..)` /
  `stop_har()` records all network traffic to a HAR — inspect it in
  browser devtools or replay it deterministically with
  `route_from_har`. A sibling to trace capture.
- **External drag-and-drop.** `Locator::drop` simulates dragging files
  or data in from outside the page (upload zones), distinct from
  `drag_to`, which drags one element onto another within the page.
- **Accessibility-tree assertions.** `expect_page(&page)
  .to_match_aria_snapshot(..)` (and the locator form) guard the page's
  ARIA structure as a regression check; `aria_snapshot` can emit
  `[box=..]` bounding boxes for visual/agent reasoning.

## Debugging failures with traces

Rust has no async `Drop`, so trace cleanup is **explicit**. The
canonical pattern: capture the test result, run cleanup
unconditionally, pass the trace path only on failure.

See [`examples/trace_on_failure.rs`](https://github.com/padamson/playwright-rust/blob/main/crates/playwright/examples/trace_on_failure.rs)
for the runnable end-to-end version — it's compiled by
`cargo check --examples` so it can't silently rot.

To view a captured trace: `playwright show-trace trace.zip`. The
viewer is language-agnostic — same UI JS / Python users see. The
hosted version is at <https://trace.playwright.dev>.

## Programmatic trace inspection

For CI bots, agent feedback loops, or any code that wants to read what
happened in a trace without re-running the test, add the companion
crate as a `[dev-dependencies]` entry: `playwright-rs-trace = "0.1"`.

Use `TraceReader::actions()` to walk the reassembled action stream,
`TraceReader::network()` for HTTP traffic. The crate's `//!`
rustdoc on <https://docs.rs/playwright-rs-trace> has a runnable
example.

## Things that look like playwright-python but aren't quite

- **No `sync_playwright`.** Async only; everything awaits on `tokio`.
- **`Result`, not exceptions.** Use `?` to propagate.
- **No keyword arguments.** Options come through `Options` structs with
  `..Default::default()`, not `name=value` in method calls.
- **No async `Drop`.** Always close browsers / stop tracing explicitly
  in a cleanup block — don't rely on RAII for I/O.
- **Locators are values, not lazy proxies.** `page.locator(...)`
  returns a `Locator` you can `.clone()` and re-use cheaply.

## Common pitfalls

- **Manual sleeps before assertions.** Use `expect(..)` and let it
  auto-retry.
- **Closing the browser before `tracing.stop()`** — traces are written
  on stop, so order matters: stop tracing first, then close.
- **Hardcoding selectors as `&str`.** Switch to `locator!()` for any
  selector you write as a literal.

## References

- Full API: <https://docs.rs/playwright-rs>
- Runnable examples: <https://github.com/padamson/playwright-rust/tree/main/crates/playwright/examples>
- Upstream Playwright docs: <https://playwright.dev/docs/api>
- Trace format / parser: <https://docs.rs/playwright-rs-trace>
- `locator!()` macro: <https://docs.rs/playwright-rs-macros>
