# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking changes

Public-API type-consistency sweep — within the crate, the same conceptual quantity was typed several different ways. These align the types with each other and with upstream Playwright. All are mechanical call-site migrations.

- **`Mouse::move_to`, `click`, `dblclick`, and `wheel` now take `f64` coordinates instead of `i32`.** Every other pixel-coordinate surface in the crate is already `f64` (`Position`, `BoundingBox`, `Touchscreen::tap`), matching upstream Playwright's floating-point pointer coords; the mouse API was the lone `i32` outlier, which capped precision and forced a lossy `f64 → i32` cast on coordinates derived from `bounding_box()`. To migrate, pass float literals: `mouse.click(150, 200, None)` → `mouse.click(150.0, 200.0, None)`. Integer-typed variables need `as f64`.
- **`DeviceViewport` and `ScreencastSize` `width`/`height` changed from `i32` to `u32`,** matching `Viewport` (the type used to *set* a context viewport). The three "pixel dimensions" structs now agree, so a `DeviceViewport` read from a device descriptor can be handed to a `Viewport` field without a cast. `BoundingBox` stays `f64` (it is measured geometry, where sub-pixel values are real).
- **`ScreencastStartOptions::quality` changed from `i32` to `u8`,** matching `ScreenshotOptions::quality`. Both are JPEG quality in `0..=100`; `u8` is range-honest. Migrate by dropping any wider integer type at the call site (e.g. `.quality(80i32)` → `.quality(80)`).
- **`locator(selector)` now takes `impl Into<String>` instead of `&str`** on `Page`, `Frame`, `Locator`, and `FrameLocator`. A dynamically-built selector can be passed as an owned `String` without a `&format!(...)` borrow at the call site. Existing `&str` and `&String` call sites are unaffected (`&str: Into<String>`); the only break is for code that relied on the method being non-generic (e.g. storing it as a `fn` pointer or using turbofish).
- **`Page::locator`, `Page::frame_locator`, and the `Page::get_by_*` helpers are now synchronous.** Locators are lazy handles: construction does no I/O (the old `async fn` bodies contained no await points) and all resolution happens at action time, which is upstream Playwright's contract in every binding. `Frame`, `Locator`, and `FrameLocator` already exposed these constructors as plain `fn`; `Page` was the odd one out, producing the `page.locator("x").await.click(None).await` double-await that consumers flagged while dogfooding. Migrate by deleting the `.await` after the constructor: `page.locator("x").await.click(None).await?` becomes `page.locator("x").click(None).await?`; same for `get_by_text`, `get_by_role`, `frame_locator`, etc. Action and assertion calls are unchanged and still await.
- **Option-taking methods now accept `impl Into<Option<O>>` instead of `Option<O>`.** Every public action / navigation / config method that took a trailing `options: Option<SomeOptions>` (e.g. `click`, `fill`, `hover`, `check`, `tap`, `drag_to`, `screenshot`, `press`, `select_option` on `Locator`; `goto`, `pdf`, `emulate_media`, `screenshot` on `Page`; mouse/keyboard/tracing/coverage/credentials/API-request methods) now takes `impl Into<Option<SomeOptions>>`. This lets you pass options directly without wrapping: `click(ClickOptions::builder()…build())` instead of `click(Some(…))`. Existing `click(None)` and `click(Some(opts))` call sites are **unaffected** — both `Option<O>` and `O` implement `Into<Option<O>>`. The only break is for code that relied on these methods being non-generic (storing one as a `fn` pointer, or turbofishing it).

### Added

- **`Page::fake_file_system()` — opt-in File System Access API fake** (`crate::testing::FakeFileSystem`). Replaces `window.showSaveFilePicker` / `showOpenFilePicker` with deterministic in-memory implementations so save/open flows are testable without a native dialog: seed openable files with `set_open_file`, read back saves with `last_saved_bytes()`/`last_saved_name()`, and control `queryPermission`/`requestPermission` via `set_permission`. Requested through downstream use, where the ~40-line hand-rolled `add_init_script` shim was being re-derived per project. This is a playwright-rs convenience with no upstream Playwright equivalent (upstream cannot drive the native pickers either; microsoft/playwright#11288). The fake handles are persistable to IndexedDB (the standard "reopen last file on startup" pattern) — it swaps a serializable placeholder for the method-bearing handle on `put` and rehydrates it on read, so `indexedDB.put(handle)` does not throw `DataCloneError`. For apps that read their last file on *startup*, `FakeFileSystem::seed_on_navigation(name, bytes, permission)` re-establishes file content and permission before the app mounts on the next navigation (a plain reload otherwise clears the in-memory content and resets permission, too late for `set_open_file` to help). Scope notes: IndexedDB persistence requires a real origin (`goto` an `http://` page, not `set_content`); `showDirectoryPicker` is not faked.
- **`APIResponse::security_details()` and `APIResponse::server_addr()`** (Playwright 1.61.0) — mirror the browser-side `Response` accessors on API responses, returning the TLS certificate details and resolved remote IP/port when the server provides them (HTTPS / when available). New parity surface; reuses the existing `SecurityDetails` / `RemoteAddr` types.
- **`ConnectOverCdpOptions::artifacts_dir`** (Playwright 1.61.0) — set the directory where traces and downloads captured over a CDP connection are stored, via `BrowserType::connect_over_cdp`.
- **Screencast `cursor` action decoration + frame `timestamp`** (Playwright 1.61.0) — `ShowActionsOptions::cursor(ActionCursor::Pointer)` draws a pointer cursor at each action point; `ScreencastFrame::timestamp` exposes the frame presentation time (seconds) when the driver supplies it.
- **`WebStorage` via `Page::local_storage()` / `Page::session_storage()`** (Playwright 1.61.0) — read/write the current origin's `localStorage` and `sessionStorage` with `get_item` / `set_item` / `remove_item` / `clear` / `items`, talking to the Page channel directly (no `evaluate` round-trip). `WebStorage` and `WebStorageKind` are re-exported at the crate root.
- **WebAuthn virtual credentials via `BrowserContext::credentials()`** (Playwright 1.61.0) — `Credentials::install` adds a virtual authenticator; `create` / `get` / `delete` register, list (with optional `rp_id`/`id` filters), and remove passkeys for `navigator.credentials` testing without hardware. `Credentials`, `VirtualCredential`, `CredentialsCreateOptions`, and `CredentialsGetOptions` are re-exported at the crate root.

### Changed

- **Bundled Playwright driver bumped to 1.61.1** (from 1.60.0). New server surface includes WebAuthn virtual-authenticator credentials, a `WebStorage` API (`page.localStorage`/`sessionStorage`), `apiResponse.securityDetails()`/`serverAddr()`, and Ubuntu 26.04 support. Browser builds: Chromium 149, Firefox 151, WebKit 26.5. Install matching browsers with `npx playwright@1.61.1 install`.

### Fixed

- **`to_have_text` / `to_contain_text` now normalize whitespace, matching upstream `toHaveText`/`toContainText`.** Runs of whitespace — including newlines — in both the element's text and the expected string collapse to single spaces before comparison, so an element whose text renders across multiple lines (`"Scan\n→\nGroup"`) matches a single-line expectation (`"Scan → Group"`). Previously only outer whitespace was trimmed, so such assertions burned the full retry timeout and failed with a confusing mismatch. The regex variants (`to_have_text_regex` / `to_contain_text_regex`) are unchanged and still match the raw text, per upstream. Surfaced through downstream use.
- **Known limitation (Chromium 149 / headless Linux): a `Mouse::move` while a button is held hangs.** The browser bundled with 1.61.0 treats a button-held move as a native drag whose move RPC never returns under headless Linux. Use `Locator::drag_to` for drag-and-drop; discrete `Mouse::down` / `Mouse::up` (without an intervening move) are unaffected.
- **`to_match_aria_snapshot` adapted to the 1.61.0 `expect` protocol change.** The driver's `expect` channel method no longer returns a `{ matches }` result; it returns nothing on success and reports a failed assertion through a top-level `errorDetails` object (carrying `timedOut`, `customErrorMessage`, and `received`). The connection layer now reads `errorDetails` and maps it to `AssertionTimeout`/`AssertionFailed`. Because the driver attaches `errorDetails` to *every* `expect` error — including infrastructure failures such as the target closing mid-retry, for which it is an empty `{}` — classification keys on the object's *content*, not its presence: an empty details object is treated as a genuine protocol error (e.g. `Error::ProtocolError` for a closed target), not a failed assertion. Without this, every aria-snapshot assertion failed to deserialize the now-`null` response, and a page closed during a retrying assertion was misreported as an assertion failure. `errorDetails` is held as raw JSON and interpreted best-effort, so a future driver's wire-shape change degrades to a surfaced error rather than a dropped message that would hang the caller (which has no client-side timeout).

## [0.14.1] - 2026-07-10

### Security

- **Bumped `crossbeam-epoch` 0.9.18 → 0.9.20 ([RUSTSEC-2026-0204](https://rustsec.org/advisories/RUSTSEC-2026-0204)) and `anyhow` 1.0.102 → 1.0.103 ([RUSTSEC-2026-0190](https://rustsec.org/advisories/RUSTSEC-2026-0190)) in the repository lockfile.** Both reach this repo only through dev-dependencies (benchmarks and tests), so no consumer of `playwright-rs` was exposed.

### Fixed

- **Driver acquisition survives the shutdown of Microsoft's `playwright.azureedge.net` CDN.** The prebuilt `playwright-<version>-<platform>.zip` driver archives were discontinued with that CDN (they now 404), which broke every fresh build — cold caches, new machines, and CI failed with `environment variable PLAYWRIGHT_DRIVER_VERSION not defined` (issue [#116](https://github.com/padamson/playwright-rust/issues/116)). The build script (and the `cli` feature's `playwright-rs install`) now assemble the driver from two artifacts instead: the `playwright-core` npm tarball (registry.npmjs.org) and a pinned Node.js binary (nodejs.org, currently 24.17.0) — the same model playwright-python uses — laid out identically to the old bundle so runtime behavior is unchanged ([ADR 0006](https://github.com/padamson/playwright-rust/blob/main/docs/adr/0006-driver-acquisition-assemble-locally.md)). The bundled driver stays at 1.60.0; no API changes. A failed download now compiles anyway (the runtime falls back to `PLAYWRIGHT_DRIVER_PATH` or an npm-installed Playwright, failing at launch with `ServerNotFound`) instead of poisoning downstream builds with the missing-env-var compile error, and download errors name the exact URL that failed.

## [0.14.0] - 2026-06-13

### Added

- **Tracing + HAR types are now re-exported at the crate root** — `Tracing`, `TracingStartOptions`, `TracingStopOptions`, `StartHarOptions`, `HarMode`, and `HarContent` join every other consumer-facing type at `playwright_rs::*` (previously reachable only via `playwright_rs::protocol::*`, which still works). Surfaced by dogfooding: a consumer following the obvious `playwright_rs::Viewport` pattern hit a compile error on `playwright_rs::StartHarOptions`.

- **Playwright 1.60 API polish** — the remaining 1.60 surface: `to_have_css_pseudo(name, value, pseudo)` asserts the computed style of a `::before`/`::after` pseudo-element; a `description` option on `get_by_role`; a `boxes` option on `aria_snapshot` (appends each element's `[box=x,y,width,height]`, useful for AI/visual reasoning); `WebError::location()` returning the uncaught exception's script URL/line/column; `WebSocketRoute::protocols()` (the subprotocols the page requested); a `no_defaults` option on `connect_over_cdp` (attach to a running browser without Playwright's default overrides); and `Page::hide_highlight()`. Internally, `ConsoleMessage` location now reads the 1.60 `line`/`column` keys, falling back to the deprecated `lineNumber`/`columnNumber`. With this, the crate covers the full Playwright 1.60 API surface.
- **`BrowserContext` lifecycle events + `Browser::on_context`** — the Playwright 1.60 context-level event surface. `Browser::on_context` fires when a new context is created on the browser. `BrowserContext::on_download`, `on_frame_attached`, `on_frame_detached`, `on_frame_navigated`, `on_page_load`, and `on_page_close` observe the corresponding page events across *all* pages of a context (forwarded from each current and future page) — the idiomatic way to write multi-tab fixtures without subscribing on every page individually.
- **Page-level `to_match_aria_snapshot`** — `expect_page(&page).to_match_aria_snapshot(...)` asserts the whole document's accessibility tree (rooted at `:root`), the page counterpart of the existing locator assertion. Matching is partial (the expected acts as a template), so it makes a robust accessibility regression guard. New API surface in Playwright 1.60.
- **HAR recording: `Tracing::start_har` / `Tracing::stop_har`** — record a HAR (HTTP Archive) of network traffic to a file. `StartHarOptions` controls `content` (`omit`/`embed`/`attach`), `mode` (`full`/`minimal`), a `url_filter` glob, and `resources_dir`. A `.zip` path bundles bodies as attachments; any other path extracts the `.har` JSON (via the new `LocalUtils::har_unzip`). The result opens in browser devtools or replays in tests through `route_from_har`. New API surface in Playwright 1.60.
- **`Locator::drop`** — simulate external drag-and-drop of files and/or MIME-typed data onto an element (e.g. an upload drop zone), the canonical primitive for upload-zone tests. Distinct from `drag_to` (intra-page element drag). Pass files as `FilePayload`s and `data` as `(mime_type, value)` pairs via `DropOptions`, with optional `position` and `timeout`. New API surface in Playwright 1.60.
- **Screenshot options: `animations`, `caret`, `scale`, `style`, `mask`, `mask_color`** on `Page::screenshot` and `Locator::screenshot` (via `ScreenshotOptions`). A pre-existing playwright-python parity gap, surfaced while dogfooding the landing site (an animated tab indicator raced an element screenshot; `animations(Animations::Disabled)` is the fix). `Animations` is now always available (previously only with the `screenshot-diff` feature) and shared between `ScreenshotOptions` and the `to_have_screenshot` assertions; new `Caret` and `Scale` enums are re-exported from the crate root. `mask(Vec<Locator>)` overpaints the given locators with a solid box (color set by `mask_color`) to redact dynamic or sensitive content; each locator is serialized to the protocol's `{ frame, selector }` channel-ref shape. This completes screenshot-options parity with playwright-python.
- **Trace-level `tracing` spans on the internal JSON-RPC layer** — Phase 2 of the observability work. `Connection::send_message` and inbound message `dispatch`, plus the pipe and websocket transport send paths, now carry `#[tracing::instrument(level = "trace")]` spans with structural fields only (`method`, `guid`, request `id`, frame `bytes_len`); `skip_all` keeps arbitrary-JSON RPC params out of the spans. Opt in with `playwright_rs=trace` for per-request protocol timing and event sequencing. Closes [#98](https://github.com/padamson/playwright-rust/issues/98) (Phase 2 sub-task of [#83](https://github.com/padamson/playwright-rust/issues/83)).

### Changed

- **Breaking: `FilePayload::builder()` replaced by `FilePayload::new(name, mime_type, buffer)`.** All three fields are required, so the builder's only failure mode was a runtime panic on a missing field; the constructor makes them required at compile time. `from_path` / `from_file` are unchanged.
- **Breaking: public option/data structs and enums are now `#[non_exhaustive]`** ([ADR 0005](../../docs/adr/0005-non-exhaustive-public-types.md)). Struct-literal construction (including the `..Default::default()` form) no longer compiles downstream; construct via the existing builders (`ScreenshotOptions::builder()`, ...), the new chainable setters (`GetByRoleOptions::default().name("OK").exact(true)`, `Cookie::new(name, value).domain(...)`, `ProxySettings::new(server)`, `TracingStartOptions::default().screenshots(true)`, ...), or `Default` + field mutation. Matching on public enums (including `Error`) now needs a wildcard arm. This makes upstream Playwright option/field additions semver-minor after 1.0 instead of breaking. Geometrically closed types (`Position`, `ScreenshotClip`, `PdfMargin`, `ScreencastSize`, `Viewport`, `DeviceViewport`, `Geolocation`) stay exhaustive and keep literal construction.
- **Breaking: `Locator::highlight()` now takes `Option<HighlightOptions>`** (previously no arguments), to carry the new 1.60 `style` option and to match Playwright's `highlight(options)` plus this crate's option-method convention. Existing calls migrate `.highlight()` → `.highlight(None)`.
- **`build.rs` driver download is now relocatable and skippable via env.** `PLAYWRIGHT_DRIVER_CACHE_DIR` downloads the driver to a stable, version-keyed path instead of `$OUT_DIR` so CI can cache it on its own key — necessary because `Swatinem/rust-cache` prunes workspace build-script output, so a driver left in `$OUT_DIR` is *not* cached by it and re-downloads (~42 MB) every run (correcting the v0.13.0 note that claimed `$OUT_DIR` was automatically cached). `PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD` skips the download entirely for compile-only consumers (e.g. `cargo check`) that never launch a browser. Both default off; local builds are unchanged (the driver still lands in `$OUT_DIR`).

### Fixed

- **Rustdoc examples are now actually compile-checked.** The previous convention marked browser doctests ` ```ignore ` and "executed" them in CI with `--ignored` — but rustdoc never compiles `ignore` blocks at all, so that job had always been a green no-op and the examples had no drift protection. All 90+ doc examples are now ` ```no_run ` (compiled on every `cargo test --doc`, not executed), with hidden scaffolding so rendered docs are unchanged. The conversion surfaced and fixed ~10 silently rotted examples, including a removed method (`expect_websocket`), a sync→async drift (`BrowserContext::tracing`), and a wrong accessor (`ConsoleMessage::type_`).
- **Connection-layer concurrency hardening.** The transport-to-dispatch message channel is now bounded (256), so a dispatch loop that falls behind exerts backpressure on the driver instead of buffering memory without limit. `wait_for_object` blocks on a `Notify` instead of polling every 10ms. `dispose()` unregisters objects synchronously instead of spawning a task, closing a window where a disposed object could still receive events (and removing a hidden tokio-runtime requirement from a sync path). The pending-callback map uses a sync lock (it was never held across an await), and the transport-writer mutex now documents that holding it across the send is the write-serialization mechanism.
- **`Frame` no longer keeps its `Page` alive after disposal.** The frame's back-reference is cleared when the frame is disposed, breaking the `Page`↔`Frame` reference cycle that previously kept both objects in memory after a page closed (`frame.page()` returns `None` once the frame is disposed).
- **`Page::locator()` / `frame_locator()` can no longer panic.** They previously `.expect()`-ed a per-call registry lookup of the main frame, which panicked if the page had been closed. The main frame is now resolved once at `Page` construction (matching playwright-python), making locator creation genuinely infallible; actions on a closed page still fail with a normal `Error`. `Page::url()` now always reads through the main frame too, instead of a fallback cache. The remaining flagged panics on public paths (`FilePayload` builder, root-object init, a CDP params invariant) were converted to errors or made unreachable.

## [0.13.0] - 2026-05-23

### Security

- Bumped transitive `openssl` from 0.10.79 to 0.10.80 to pick up the fix for [GHSA-phqj-4mhp-q6mq](https://github.com/advisories/GHSA-phqj-4mhp-q6mq) — potential out-of-bounds write in `CipherCtxRef::cipher_update_inplace` for AES-KW-PAD ciphers. We don't use AES-KW-PAD anywhere; the path enters the tree transitively via `tokio-tungstenite` → `native-tls` → `openssl`. Affected the workspace `Cargo.lock` and the fuzz target's lockfile; both now resolve to 0.10.80+. Detected by Dependabot (alerts #17 / #18).
- Bumped transitive `openssl` from 0.10.78 to 0.10.79 to pick up the fix for [GHSA-xp3w-r5p5-63rr](https://github.com/advisories/GHSA-xp3w-r5p5-63rr) / [CVE-2026-42327](https://www.cve.org/CVERecord?id=CVE-2026-42327) — undefined behaviour in `X509Ref::ocsp_responders` for certificates with non-UTF-8 OCSP URLs. Affected the workspace `Cargo.lock` and the fuzz target's lockfile; both now resolve to 0.10.79+. We pull `openssl` transitively via `native-tls` (the default TLS feature); no public-API impact. Detected by Dependabot (alerts #15 / #16); `cargo audit` had not yet picked up the advisory at fix time.
- Refreshed `supply-chain/imports.lock` exemptions for the bumped `openssl` and `openssl-sys` versions via `cargo vet regenerate exemptions`. `cargo vet`, `cargo audit`, `cargo deny check` all green post-bump.
- Adopted transitively-trusted publishers (`cargo vet trust --all`) for BurntSushi, Lokathor, epage, seanmonstar, and kennykerr (the latter three with `--allow-multiple-publishers` for flagship multi-maintainer crates like `clap`, `tower`, `windows-*`). Also added the fermyon trust import. Net effect: **215 → 168 exemptions** (-47, ~22% reduction) with zero first-hand audit work — every dropped exemption is now backed by a trusted publisher already vetted by 2+ trust roots we import (Mozilla, Google, ISRG, etc.). `cargo vet`, `cargo audit`, `cargo deny check` all green.

### Breaking changes

- **`Locator::aria_snapshot()` now takes `Option<AriaSnapshotOptions>`.** The Playwright 1.59 `ariaSnapshot` RPC accepts `mode` (Ai or Default), `track`, `depth`, and `timeout`; we now surface those through a struct passed to `aria_snapshot(...)`. To migrate, change `locator.aria_snapshot()` → `locator.aria_snapshot(None)`. The newly-added `Page::aria_snapshot(options)` lands in this release with the same shape, so it's not a migration concern there. See the matching entry under **Added** for the full motivation.
- **`ScreencastFrame::data` changed from `Vec<u8>` to `bytes::Bytes`.** With high-FPS streams and multiple registered handlers, the previous shape allocated a fresh `Vec<u8>` for every handler on every frame; `Bytes` makes the decoded JPEG allocate exactly once per frame and lets each handler-clone be a refcount bump rather than a memcpy. Downstream sinks that pipe to `axum`, `tonic`, `tokio_util::codec`, or any `Bytes`-aware API now hand off without copying. `Bytes` implements `Deref<Target = [u8]>`, so most existing reads (`frame.data.len()`, `&frame.data[..]`, `tokio::fs::write(path, &frame.data)`) compile unchanged; the only migration is for code that consumed the buffer by value (e.g. `let v: Vec<u8> = frame.data` → `let v = frame.data.to_vec()`). Closes [#85](https://github.com/padamson/playwright-rust/issues/85) (sub-task of [#55](https://github.com/padamson/playwright-rust/issues/55), v0.13.0).

### Changed

- **Playwright driver upgraded to 1.60.0** (from 1.59.1) — picks up current Chromium 148 / Firefox 150 / WebKit 26.4 binaries and removes long-deprecated upstream APIs (`Locator.ariaRef`, the `handle` option on `exposeBinding`, the `logger` option on `connect`/`connectOverCDP`, and the `videosPath`/`videoSize` context options) — none of which were exposed in this crate, so the bump is source-compatible. The `route.fulfill()` body-transmission reverse-canary still passes against 1.60.0, so the documented limitation range is extended to 1.49.0 – 1.60.0. New 1.60 surface (HAR tracing API, `Locator::drop`, expanded BrowserContext lifecycle events, `description` on `get_by_role`, `pseudo` on `to_have_css`, etc.) is tracked separately under the v1.0 gap analysis for incremental rollout pre-1.0.
- **`build.rs` writes the Playwright driver into Cargo's `$OUT_DIR` instead of `<repo>/drivers/`.** The previous strategy walked up from `CARGO_MANIFEST_DIR` looking for a workspace `Cargo.toml`, which misbehaved when playwright-rs was consumed as a git dependency: the walk wandered into `$CARGO_HOME/git/checkouts/...`, and the resolved `drivers/` directory did not survive Cargo's CI cache restore (the embedded driver path in the cached `.rlib` pointed at a directory that no longer existed; runtime fell through to `Error::ServerNotFound`). `$OUT_DIR` is the idiomatic Rust location for build-script artifacts and is automatically cached by any `target/`-cache CI configuration. Dropped the `dirs` build-dependency. Runtime resolution is unchanged for library consumers — `option_env!()` picks up wherever build.rs writes — so this is a behaviour change for shell scripts / CI configs that hardcoded `<repo>/drivers/`, not a Rust API break.
- **CI workflows no longer cache `<repo>/drivers/`.** `Cache Playwright Drivers` steps in `.github/workflows/test.yml` and `.github/workflows/release.yml` are removed; the existing `target/` cache covers `$OUT_DIR`. The three OS-conditional hardcoded `drivers/playwright-<version>-<platform>/node ... cli.js install` shell invocations are replaced with a single cross-platform `cargo run --release --bin playwright-rs --features cli -- install chromium firefox webkit` step.
- **Integration tests use the public `get_driver_executable()` API.** Three tests in `crates/playwright/tests/integration/connection.rs` previously hardcoded `env!("CARGO_MANIFEST_DIR").join("drivers")` and probed the filesystem; they now call `crate::common::playwright_package_dir()` which wraps the public driver-resolution API. Removes implicit dependence on the legacy `<repo>/drivers/` layout.
- **`build.rs`: replaced `reqwest` with `ureq`** for the one blocking driver download from Azure CDN. `reqwest` pulled in hyper, tower, h2, and ~220 lines of transitive build-time dependencies for a single GET; `ureq` is a thin synchronous client that covers the use case directly. Cuts our total `cargo tree` size from ~619 → ~507 lines (~18%). Build-only change, zero user-facing impact. Closes [#63](https://github.com/padamson/playwright-rust/issues/63) (sub-task of [#62](https://github.com/padamson/playwright-rust/issues/62)).
- **Narrowed `tokio` features from `"full"` to the explicit subset we use** (`fs`, `io-util`, `macros`, `net`, `process`, `rt`, `rt-multi-thread`, `signal`, `sync`, `time`). Drops the unused `parking_lot`, `tracing`, and `io-std` features. No public API change; this just documents the runtime surface we depend on. Closes [#64](https://github.com/padamson/playwright-rust/issues/64) (sub-task of [#62](https://github.com/padamson/playwright-rust/issues/62)).
- **Replaced `mime_guess` with a small hand-rolled extension-to-MIME map.** `mime_guess` ships a compile-time lookup table for hundreds of MIME types we'd never encounter for browser-automation file uploads. The new helper covers the ~40 extensions actually relevant (images, documents, archives, common text/data formats) and falls back to `application/octet-stream`. Internal-use only; SemVer-compatible. Closes [#65](https://github.com/padamson/playwright-rust/issues/65) (sub-task of [#62](https://github.com/padamson/playwright-rust/issues/62)).

### Added

- **`playwright-rs` binary (feature `cli`).** clap-based CLI shipped behind the new `cli` feature. Initial subcommand: `playwright-rs install [BROWSERS]... [--with-deps] [--driver-only]`. The binary downloads the Playwright driver into a stable user-cache directory (`~/Library/Caches/playwright-rust/<version>/` on macOS, `~/.cache/playwright-rust/<version>/` on Linux, `%LOCALAPPDATA%\playwright-rust\<version>\` on Windows) and then invokes the browser installer. Discoverable via `cargo install playwright-rs --features cli`. Future-proofed for `playwright-rs driver path` / `driver version` subcommands.
- **`get_driver_executable()` runtime resolution adds a user-cache lookup.** Between the compile-time bundled lookup and the existing env-var/npm fallbacks, the resolver now probes `dirs::cache_dir()/playwright-rust/<version>/playwright-<version>-<platform>/`. This is the location that `playwright-rs install` populates, so downstream binaries distributed via `cargo install` can rely on a stable runtime driver path after the user runs the installer once.
- **`Page::aria_snapshot(options)`** — page-level shorthand for `page.locator("body").aria_snapshot(options)`. Returns the ARIA accessibility tree for the entire page as a YAML string. Closes [#70](https://github.com/padamson/playwright-rust/issues/70) (sub-task of [#55](https://github.com/padamson/playwright-rust/issues/55), v0.13.0).
- **`AriaSnapshotOptions` on both `Locator::aria_snapshot` and `Page::aria_snapshot`** — surfaces the Playwright 1.59 `mode`, `track`, `depth`, and `timeout` parameters. The headline knob is `mode: AriaSnapshotMode::Ai` for AI-friendly snapshots intended for LLM/codegen consumption (the default mode is the human-readable form). Re-exported as `AriaSnapshotMode` and `AriaSnapshotOptions` from the crate root. **Breaking:** the `Locator::aria_snapshot()` no-arg signature is gone; the new shape is `aria_snapshot(options: Option<AriaSnapshotOptions>)`. Migration is one character per call site — pass `None` to keep the prior behaviour.
- **`TracingStartOptions::live`** — new field; when `Some(true)`, enables Playwright's live-trace mode so a viewer can attach to an in-progress recording. Forwarded as `{"live": true}` to the `tracingStart` RPC. Closes [#71](https://github.com/padamson/playwright-rust/issues/71) (sub-task of [#55](https://github.com/padamson/playwright-rust/issues/55), v0.13.0).
- **`Locator::normalize()`** — returns a new locator whose selector has been resolved to a best-practices canonical form (test-ids, ARIA roles, accessible text). Useful as a building block for codegen helpers. Calls Playwright's `resolveSelector` RPC under the hood. Closes [#69](https://github.com/padamson/playwright-rust/issues/69) (sub-task of [#55](https://github.com/padamson/playwright-rust/issues/55), v0.13.0).
- **`LaunchOptions::artifacts_dir`** — surfaces Playwright's `artifactsDir` launch option so consumers can choose where traces, video, and downloads land instead of letting Playwright pick a temp directory. Builder method `LaunchOptions::artifacts_dir(path)`. Closes [#72](https://github.com/padamson/playwright-rust/issues/72) (sub-task of [#55](https://github.com/padamson/playwright-rust/issues/55), v0.13.0).
- **`Page::clear_console_messages()` / `Page::clear_page_errors()`** — drop everything in the respective accumulators so the next `page.console_messages()` / `page.page_errors()` call starts from empty. Useful between test phases when you want to assert against only messages from a specific phase. The JS-style `predicate` filter argument from Playwright was deliberately not added — Rust users can chain `.into_iter().filter(...)` on the existing getter result, which is more idiomatic. Closes [#73](https://github.com/padamson/playwright-rust/issues/73) (sub-task of [#55](https://github.com/padamson/playwright-rust/issues/55), v0.13.0).
- **`Page::pick_locator()` / `Page::cancel_pick_locator()`** — programmatic entry to the Playwright Inspector's interactive element picker. `pick_locator()` blocks until the user clicks an element and returns a `Locator` for it; `cancel_pick_locator()` aborts an in-progress pick from a different async context. Synthetic clicks (e.g. `page.mouse.click`) do **not** complete the picker — the resolution requires a real DOM click — so test coverage focuses on the cancel-then-release pathway rather than a happy-path pick. Closes [#76](https://github.com/padamson/playwright-rust/issues/76) (sub-task of [#55](https://github.com/padamson/playwright-rust/issues/55), v0.13.0).
- **`CDPSession::on(method, handler)` / `CDPSession::on_close(handler)`** — the missing event side of the CDP API. `on(method, ...)` registers a handler keyed by CDP event method name (e.g. `"Network.requestWillBeSent"`); the Playwright server forwards every CDP event so no Playwright-side subscription is needed, but you typically still need to enable the CDP domain itself first (e.g. `session.send("Network.enable", None).await?`). `on_close(...)` registers a handler that fires when the session is detached. Closes [#74](https://github.com/padamson/playwright-rust/issues/74) (sub-task of [#55](https://github.com/padamson/playwright-rust/issues/55), v0.13.0).
- **`Page::screencast()` — full Playwright 1.59 screencast surface.** `Page::screencast()` returns a `Screencast` handle exposing the live-streaming, recording, and annotation surfaces of the 1.59 protocol:
  - **Live streaming** — `start(options)` + `on_frame(handler)` deliver JPEG frames as `ScreencastFrame { data: bytes::Bytes }` as the browser renders. `ScreencastStartOptions` covers `size`, `quality`, and `path`.
  - **Recording-to-disk** — when `ScreencastStartOptions::path` is set, the screencast is also recorded to that file; `stop()` saves and closes it via the same `Artifact` mechanism `Video` uses. Distinct lifecycle from `Video`: `Video` (via `BrowserContextOptions::record_video`) captures the entire page session automatically; screencast `path` captures only during the user-initiated start/stop window.
  - **Action labels** — `show_actions(ShowActionsOptions { duration, position, font_size })` overlays each action as it's invoked; `hide_actions()` stops.
  - **Chapter cards** — `show_chapter(title, ChapterOptions { description, duration })` marks named phases for an agent's video log.
  - **HTML overlays** — `show_overlay(html, options) -> OverlayId`, `remove_overlay(id)`, `set_overlay_visible(bool)`. Arbitrary HTML rendered over the streamed frames; useful for agent annotations.

  Useful for "agent receipts": an LLM-driven flow can observe the browser as it works, produce an annotated video log alongside the action log, and split the recording into named phases. Closes [#79](https://github.com/padamson/playwright-rust/issues/79) (sub-task of [#55](https://github.com/padamson/playwright-rust/issues/55), v0.13.0).
- **`Debugger` class + `BrowserContext::debugger()` accessor** — programmatic control of Playwright Inspector's "PAUSED" overlay (IDE / inspector tooling, distinct from the MCP / agent codegen path). New `Debugger` exposes `request_pause()`, `resume()`, `next()`, `run_to(location)`, `paused_details()` getter, `is_paused()` convenience, and `on_paused_state_changed(handler)` event. `PausedDetails` and `PausedLocation` re-exported from the crate root. Closes [#75](https://github.com/padamson/playwright-rust/issues/75) (sub-task of [#55](https://github.com/padamson/playwright-rust/issues/55), v0.13.0).
- **`screenshot-diff` feature flag (opt-in).** Gates `Expectation::to_have_screenshot` / `PageExpectation::to_have_screenshot` and the related types (`Animations`, `ScreenshotAssertionOptions`, `ScreenshotAssertionOptionsBuilder`) along with the `image` dependency. Enable with `features = ["screenshot-diff"]` to add the `image` crate and pixel-diff support; default-off keeps cold builds lean for the majority of consumers, who don't use pixel-diff assertions. Closes [#66](https://github.com/padamson/playwright-rust/issues/66) and [#67](https://github.com/padamson/playwright-rust/issues/67) (sub-tasks of [#62](https://github.com/padamson/playwright-rust/issues/62)).
- **`tracing` instrumentation across the public async surface** — every public async method on `Browser`, `BrowserContext`, `BrowserType`, `Page`, `Frame`, `Locator`, `ElementHandle`, `Tracing`, `CDPSession`, `Debugger`, `Screencast`, `Request`, `Response`, `Dialog`, `Download`, `Worker`, and `FileChooser` now carries a `#[tracing::instrument]` attribute. Top-level user operations (`goto`, `click`, `fill`, `screenshot`, `pdf`, `evaluate`, `tracing.start/stop`, `browser.close`, `browser_type.launch`) emit at `info`; most public-API spans at `debug`. **Wire-protocol chatter** (per-message `Sending message` / `Request JSON` / `Dispatching message` / per-object `__create__` / `Disposed object` / per-event `Event on {guid}`) emits at `trace` so `playwright_rs=debug` stays quiet and readable; opt in to the firehose with `playwright_rs=trace`. Spans expose cardinality-bounded identifiers (`guid`, `selector`, `url`, `name`, `method`) and selected completion-time fields (`status` on `goto`, `bytes_len` on `screenshot`/`pdf`/`body`/`stop_tracing`, `count` on `Locator::count`, `version` on `http_version`); sensitive payloads (input values, eval expressions, request/response bodies) are deliberately excluded. Internal `tokio::spawn` sites in `Page`, `BrowserType`, `CDPSession`, `Connection`, `ChannelOwnerImpl`, `PlaywrightServer`, and `tty_guard` are wrapped with `Instrument::in_current_span()` so user-registered handlers and event fan-out tasks inherit the caller's span context. Wire up any `tracing_subscriber` and you get rich spans for free; see the new "Observability" section in the crate root rustdoc. Phase 2 follow-ups (internal-RPC `trace`-level spans, OTel semantic-convention alignment, sensitive-data spans) tracked in [#83](https://github.com/padamson/playwright-rust/issues/83). Closes [#84](https://github.com/padamson/playwright-rust/issues/84) and [#91](https://github.com/padamson/playwright-rust/issues/91) (sub-task of [#55](https://github.com/padamson/playwright-rust/issues/55), v0.13.0).
- **`playwright_rs::locator!()` compile-time-validated selector macro** — re-exported from the new companion crate [`playwright-rs-macros`](../playwright-rs-macros/CHANGELOG.md) (independently versioned). Gated behind a default-on `macros` feature; opt out with `default-features = false` to skip the proc-macro dependency. Closes [#81](https://github.com/padamson/playwright-rust/issues/81). See the macros crate's CHANGELOG for the macro's own release notes.
- **`Page::set_url_fragment(hash)` / `Page::clear_url_fragment()`** — typed helpers wrapping `history.replaceState` so test suites that click anchor links can mutate the URL fragment without dropping to an `evaluate_value` JS string. `set_url_fragment` accepts a fragment with or without a leading `#`; `clear_url_fragment` strips any trailing `#...`. Neither fires a real navigation. Closes [#89](https://github.com/padamson/playwright-rust/issues/89).
- **Debug-build runtime-binding assertion on `Browser`.** A `Browser` (and every protocol object descended from it) is bound to the tokio runtime that launched it; sharing one across runtimes (e.g. caching via `OnceCell<Browser>` between `#[tokio::test]` invocations whose runtimes have already exited) silently deadlocks because the protocol channels point at a dead runtime. Debug builds now `panic!` on the wire path with a clear message when this happens; release builds skip the check (zero overhead). Pairs with a new "Runtime binding" section in the [`Browser`](crate::Browser) rustdoc explaining the constraint and the workaround (launch a fresh `Playwright` + `Browser` per test). Closes [#90](https://github.com/padamson/playwright-rust/issues/90).

### Removed

- **`<repo>/drivers/` directory is no longer created or read.** If you relied on this path (e.g. shell scripts pointing `PLAYWRIGHT_DRIVER_PATH` at `./drivers/playwright-<version>-<platform>`), the driver now lives under `target/<profile>/build/playwright-rs-<hash>/out/playwright-driver/` for in-workspace builds and under `dirs::cache_dir()/playwright-rust/<version>/` after running `playwright-rs install`. The runtime resolution chain finds both automatically; `PLAYWRIGHT_DRIVER_PATH` remains supported as an explicit user override.

## [0.12.3] - 2026-04-30

### Fixed

- **Ctrl-C no longer breaks the user's shell terminal (Unix)** — running a Playwright program and hitting Ctrl-C used to leave the shell with arrow keys echoing as raw `^[[A` instead of recalling history; recovering required `stty sane` or `reset`. Two layers fix it: (1) the Node driver is now spawned in its own process group via `process_group(0)`, so SIGINT only signals our Rust process and Node runs `gracefullyProcessExitDoNotHang` instead of crashing on EPIPE while chromium events are in flight; (2) the driver's stderr is piped and drained into `tracing::debug!` on target `playwright_rs::driver_stderr` instead of being inherited from our tty, so any terminal-capability queries Node writes during shutdown don't pollute the user's terminal. Closes [#59](https://github.com/padamson/playwright-rust/issues/59). A defensive tty-snapshot/restore SIGINT handler is also installed as belt-and-suspenders; opt-out via `PLAYWRIGHT_NO_SIGNAL_HANDLER=1`.

### Added

- **`trace_on_failure` example + README "Testing & Debugging" section** — idiomatic Rust pattern for ensuring `tracing.stop()` / `browser.close()` run on test failure (since `?` and `unwrap()` skip subsequent cleanup) and for getting line-pinpointed failures despite `?` hiding source location. Closest to the Java/.NET pattern (explicit try-finally) since Rust has no first-party Playwright test runner and no async `Drop`. Closes [#61](https://github.com/padamson/playwright-rust/issues/61).

### Changed

- **Release workflow: stripped dead CLI-binary scaffolding** — we're a library-only crate, so the build/strip/archive/upload-asset/SLSA-attestation steps were no-ops guarded by `if [ -f playwright ]`. Removing them eliminates the cosmetic "Could not find subject" red annotations that appeared on every release. Closes [#56](https://github.com/padamson/playwright-rust/issues/56).

## [0.12.2] - 2026-04-27

### Added

- **Locator assertions** — `to_have_attribute(name, value)`, `to_have_class(expected)`, `to_have_css(name, value)`, `to_have_count(count)`, plus `_regex` variants for the first three. Closes [#58](https://github.com/padamson/playwright-rust/issues/58).

## [0.12.1] - 2026-04-25

### Security

- **`RUSTSEC-2026-0104`** — bump `rustls-webpki` from `0.103.12` to `0.103.13`. Reachable panic in CRL parsing via `BorrowedCertRevocationList::from_der` when handling a syntactically valid empty `BIT STRING` in the `onlySomeReasons` element of an `IssuingDistributionPoint` extension. Reachable prior to signature verification. Applications that don't use CRLs are not affected. Advisory: <https://rustsec.org/advisories/RUSTSEC-2026-0104>

## [0.12.0] - 2026-04-19

### Added

- **`context.set_storage_state(state)`** — replaces cookies and localStorage on an existing context without recreation (implemented client-side: clear cookies, add new cookies, restore per-origin localStorage via JS evaluation)
- **`context.is_closed()`** — returns `true` after `close()` or a server-initiated "close" event
- **`context.clock()` / `page.clock()`** — Clock API for fake timer control in deterministic tests
  - `Clock::install(options)` — install fake timers, optionally setting an initial epoch timestamp (`clockInstall` RPC on BrowserContext channel)
  - `Clock::fast_forward(ticks)` — advance the clock by milliseconds, firing due timers (`clockFastForward` RPC with `ticksNumber`)
  - `Clock::pause_at(time)` — jump to an epoch timestamp and pause; no timers fire until resumed (`clockPauseAt` RPC with `timeNumber`)
  - `Clock::resume()` — resume the clock after `pause_at` (`clockResume` RPC)
  - `Clock::set_fixed_time(time)` — freeze `Date.now()` at a fixed epoch without affecting timer scheduling (`clockSetFixedTime` RPC with `timeNumber`)
  - `Clock::set_system_time(time)` — update system time without freezing the clock (`clockSetSystemTime` RPC with `timeNumber`)
  - `ClockInstallOptions` — `time: Option<u64>` for setting the install epoch
  - `page.clock()` delegates to the parent `BrowserContext::clock()` — all RPCs go on the context channel
  - `Clock` and `ClockInstallOptions` exported from crate root
  - See: <https://playwright.dev/docs/api/class-clock>

- **`page.route_from_har(har_path, options)` / `context.route_from_har(har_path, options)`** — replays network requests from a HAR archive; uses client-side `HarRouter` pattern (calls `local_utils.har_open()` then `local_utils.har_lookup()` per request); accepts `RouteFromHarOptions` with `url` (glob filter), `not_found` (`"abort"` or `"fallback"`), `update`, `update_content`, and `update_mode` fields; maps to `harOpen`/`harLookup` RPCs on the `LocalUtils` channel
- **`Touchscreen` class** — `page.touchscreen()` returns a `Touchscreen` handle; `touchscreen.tap(x, y)` simulates a single touch event at viewport coordinates (`touchscreenTap` RPC on Page channel); requires `has_touch: true` in `BrowserContextOptions`
- **`page.drag_and_drop(source, target, options)`** — performs drag and drop between two CSS selectors on the main frame; delegates to `Frame::locator_drag_to` (`dragAndDrop` RPC); accepts the same `DragToOptions` as `Locator::drag_to`
- **`page.console_messages()`** — returns all console messages accumulated since page creation (`Vec<ConsoleMessage>`); console subscription enabled by default on every BrowserContext so no handler registration required
- **`page.page_errors()`** — returns all uncaught JS error messages accumulated since page creation (`Vec<String>`); populated automatically via `pageError` events
- **`page.opener()`** — returns the page that opened this popup (`Option<Page>`), or `None` for non-popup pages; reads the opener GUID from the page initializer
- **`Video` class** — `page.video()` returns `Some(Video)` when the context was created with `record_video`; `Video::path()` returns the recording path, `Video::save_as(path)` copies the file, `Video::delete()` removes it; all methods wait internally for the artifact (fired via `"video"` event on page close), so no manual sleep is required
- **`page.request_gc()`** — forces garbage collection (Chromium only)
- **`page.workers()`** — returns all active web workers in the page (`Vec<Worker>`); accumulated from `worker` events as workers are created
- **`context.service_workers()`** — returns all active service workers in the browser context (`Vec<Worker>`); accumulated from `serviceWorker` events
- **`expect_event()`** — generic event waiting on Page and BrowserContext, returning typed `EventValue` enum
- **`playwright.request()`** — headless API testing without a browser (`get`, `post`, `put`, `delete`, `patch`, `head`, `fetch`, `APIResponse`)
- **`to_match_aria_snapshot(expected)`** — ARIA accessibility tree assertion with auto-retry
- **`Locator::aria_snapshot()`** — returns the ARIA accessibility tree as a YAML string (`ariaSnapshot` RPC on Frame)
- **`Locator::describe(description)`** — returns a new Locator with a human-readable label appended to the selector (`internal:describe=...`) for cleaner trace/error output; client-side only
- **`Locator::highlight()`** — highlights the matched element in the browser for visual debugging (`highlight` RPC on Frame)
- **`Locator::content_frame()`** — returns a `FrameLocator` for the content of an `<iframe>` element; client-side only
- **`ElementHandle::content_frame()`** — returns the `Frame` for an `<iframe>` element, or `None` if not an iframe (`contentFrame` RPC on ElementHandle channel)
- **`ElementHandle::owner_frame()`** — returns the `Frame` that owns this element (`ownerFrame` RPC on ElementHandle channel)
- **`ElementHandle::wait_for_element_state(state, timeout)`** — waits until the element reaches the given state (`"visible"`, `"hidden"`, `"stable"`, `"enabled"`, `"disabled"`, `"editable"`) (`waitForElementState` RPC on ElementHandle channel)
- **`Accessibility` class** — `page.accessibility()` returns an `Accessibility` handle; `accessibility.snapshot(options)` returns the page's ARIA accessibility tree as a YAML string wrapped in `serde_json::Value`; implemented via `FrameAriaSnapshot` RPC (the modern Playwright equivalent — the legacy `accessibilitySnapshot` RPC was removed in current Playwright versions)
- **`Coverage` class** — `page.coverage()` returns a `Coverage` handle (Chromium only); `start_js_coverage(options)` / `stop_js_coverage()` collect V8 JS coverage (`JSCoverageEntry` with `url`, `script_id`, `source`, `functions: Vec<JSFunctionCoverage>`); `start_css_coverage(options)` / `stop_css_coverage()` collect CSS coverage (`CSSCoverageEntry` with `url`, `text`, `ranges: Vec<CoverageRange>`); maps to `startJSCoverage` / `stopJSCoverage` / `startCSSCoverage` / `stopCSSCoverage` RPCs on the Page channel
- **`page.add_locator_handler()` / `page.remove_locator_handler()`** — registers an async handler that Playwright calls whenever a matching element appears and blocks an actionability check (e.g. cookie banners, permission dialogs); accepts `AddLocatorHandlerOptions` with `no_wait_after` and `times`; maps to `registerLocatorHandler` / `resolveLocatorHandler` / `unregisterLocatorHandler` RPCs; handler receives the matching `Locator` as argument
- **`page.route_web_socket(url, handler)` / `context.route_web_socket(url, handler)`** — intercepts WebSocket connections matching a URL glob pattern; handler receives a `WebSocketRoute` object with `connect_to_server()`, `close(options)`, `send(message)`, `on_message(handler)`, and `on_close(handler)`; maps to `setWebSocketInterceptionPatterns` RPC with `webSocketRoute` events; `WebSocketRoute` and `WebSocketRouteCloseOptions` exported from crate root
- **`ConsoleMessage::timestamp()`** — epoch milliseconds (`f64`) when the message was emitted
- **`Response::http_version()`** — HTTP version string (e.g. `"HTTP/1.1"`, `"HTTP/2.0"`) via the `httpVersion` RPC added in Playwright 1.59
- **`Request::existing_response()`** — synchronous `Option<Response>` for the already-received response, complementing the async `response()` getter
- **`browser.bind(title, options)` / `browser.unbind()`** — expose a playwright-rs-launched browser over WebSocket so external clients (`@playwright/mcp`, Playwright CLI, agent tooling) can attach via `BrowserType::connect()`; `BindOptions` has `host`/`port`/`workspace_dir`/`metadata`; maps to `startServer` / `stopServer` RPCs (Playwright 1.59+)
- **Playwright driver upgraded to 1.59.1** (from 1.58.2) — required for `Response::http_version()` and picks up current Chromium/Firefox/WebKit binaries

### Breaking Changes

- **`BrowserContextOptions::accept_downloads` type changed** (closes #49) — field type is now `Option<AcceptDownloads>` instead of `Option<bool>`, matching the protocol's three-state `"accept"`/`"deny"`/`"internal"` string. The builder method accepts `impl Into<AcceptDownloads>`, so `bool` callers still work (`true` → `Accept`, `false` → `Deny`). Direct struct-literal construction or field pattern-matching must migrate.
- **macOS 14 WebKit no longer supported** — Playwright 1.59 dropped macOS 14 ("Sonoma") support for WebKit. Users on macOS 14 must upgrade to macOS 15+ or pin to `playwright-rs = "0.11"` (which ships driver 1.58.2). Chromium and Firefox are unaffected on macOS 14.

## [0.11.0] - 2026-04-16

### Added

- **New classes**: `JSHandle`, `Worker`, `WebError`, `WebSocket` (completed), `ConsoleMessage`, `FileChooser`, `Selectors`
- **`playwright.devices()`** — device descriptor map for browser emulation (`DeviceDescriptor`, `DeviceViewport`)
- **`ConsoleMessage::args()`** — returns `&[Arc<JSHandle>]` for console method arguments
- **Browser methods** — `contexts()`, `browser_type()`, `on_disconnected()`, `start_tracing()`/`stop_tracing()`, `new_browser_cdp_session()`
- **Page event handlers** — `on_close`, `on_load`, `on_crash`, `on_pageerror`, `on_popup`, `on_frameattached`, `on_framedetached`, `on_framenavigated`, `on_worker`, `on_console`, `on_filechooser`
- **Page expect methods** — `expect_popup`, `expect_download`, `expect_response`, `expect_request`, `expect_console_message`, `expect_file_chooser`
- **BrowserContext events** — `on_console`, `on_weberror`, `on_serviceworker`, `on_dialog`, `expect_console_message`

### Breaking Changes

- **`ConnectionLike` trait gains `selectors()` method** — internal server infrastructure, not user-facing API. Any code implementing `ConnectionLike` directly must add the new method.

### Fixed

- **unwrap() audit (closes #48)** — replaced bare `unwrap()` calls in library code with `expect()` (for infallible operations) or proper error handling (for protocol data). Remaining `unwrap()` calls are only mutex locks (`lock().unwrap()`) and test code.
- **15 broken rustdoc links** — all intra-doc links now resolve correctly (qualified paths for cross-module references)

## [0.10.0] - 2026-04-11

### Added

- **`BrowserContext::new_cdp_session(page)`** — creates a Chrome DevTools Protocol session (Chromium only)
  - `CDPSession::send(method, params)` — send any CDP command and receive the result as JSON
  - `CDPSession::detach()` — detach from the CDP session
  - `CDPSession` registered in the object factory for server-created sessions
  - See: <https://playwright.dev/docs/api/class-cdpsession>
- **`BrowserContext::tracing()`** — access the per-context Tracing object
  - `Tracing::start(options)` — begin trace recording (`tracingStart` + `tracingStartChunk`)
  - `Tracing::stop(options)` — stop trace recording (`tracingStopChunk` + `tracingStop`)
  - `TracingStartOptions` — `name`, `screenshots`, `snapshots` fields
  - `TracingStopOptions` — `path` field to export trace as a `.zip` archive
  - Artifact export wired through `Artifact::save_as` for path-based stop
  - See: <https://playwright.dev/docs/api/class-tracing>

- **`install_browsers()` / `install_browsers_with_deps()`** — programmatic browser installation (closes #46)
  - `install_browsers(None)` — install all browsers
  - `install_browsers(Some(&["chromium", "firefox"]))` — install specific browsers
  - `install_browsers_with_deps(browsers)` — also installs system dependencies (useful for CI)
  - Reuses the bundled Playwright driver — no `npx` required
- **Frame public API completion** — Frame now at **100%** coverage
  - `frame.locator(selector)` — create Locator scoped to a frame (was internal-only)
  - All 7 `get_by_*` methods (`get_by_text`, `get_by_label`, `get_by_role`, etc.)
  - `evaluate_handle(expression)` — returns ElementHandle from JS evaluation
  - `child_frames()` — returns child frames by scanning the connection registry
  - Properties: `name()`, `page()`, `parent_frame()`, `is_detached()`
  - `frame.page()` back-reference set lazily when Page accesses its main frame
- **`FrameLocator` class** — locate elements inside iframes using Playwright's `internal:control=enter-frame` selector engine
  - `page.frame_locator(selector)` / `locator.frame_locator(selector)` — entry points
  - `frame_locator.locator(selector)` — create Locator inside iframe
  - `frame_locator.frame_locator(selector)` — nested iframes
  - All 7 `get_by_*` methods (`get_by_text`, `get_by_label`, `get_by_role`, etc.)
  - `first()`, `last()`, `nth(index)` — composition for multiple matching iframes
  - `owner()` — Locator for the iframe element itself
- **`ConnectionExt` extension trait** — typed object retrieval via `connection.get_typed::<T>(guid).await?`, eliminating boilerplate `get_object` + `as_any().downcast_ref::<T>()` pattern
- **`downcast_parent<T>()` helper** — one-line parent type resolution replacing manual parent + downcast chains
- **`Error::TypeMismatch`** — structured error variant with guid, expected type, and actual type for better diagnostics

### Breaking Changes

- **MSRV bumped from 1.85 to 1.88** — transitive dependencies (`icu_*`, `image`, `time`, `zip`) now require Rust 1.88
- **`ConnectionLike` trait uses `#[async_trait]`** — methods migrated from manual `Pin<Box<dyn Future>>` returns to idiomatic `async fn`. Any code implementing `ConnectionLike` directly must update method signatures (internal server infrastructure, not user-facing API).

### Changed

- **Security & quality CI** — `cargo audit` and `cargo deny` run on every push to main and weekly; mutation testing moved from `test.yml` to dedicated `security.yml` with weekly schedule + release tag triggers; MSRV check (Rust 1.85) in `test.yml`
- **`deny.toml`** — license compliance (Apache-2.0 compatible), crate bans, source restrictions, duplicate detection
- **`cargo vet`** — supply chain review with trusted imports from 7 organizations; new dependencies require audit
- **SLSA provenance** — release artifacts include signed build attestations via `actions/attest-build-provenance`
- **Fuzz targets** — `cargo-fuzz` targets for `parse_value`, `serialize_argument`, `parse_result` (protocol parsing layer)
- **BrowserContext event handlers** — context-level event subscriptions (fire before page handlers)
  - `on_page(handler)` — fires when new page created in context
  - `on_close(handler)` — fires when context is closed
  - `on_request(handler)` / `on_request_finished(handler)` / `on_request_failed(handler)` — network events from any page
  - `on_response(handler)` — response events from any page
- **`expect_page()` / `expect_close()`** — promise-based event waiting with timeout
  - `expect_page(timeout)` — returns `EventWaiter<Page>` that resolves when a new page is created
  - `expect_close(timeout)` — returns `EventWaiter<()>` that resolves when the context closes
  - `EventWaiter<T>` — generic one-shot waiter backed by `tokio::sync::oneshot` with configurable timeout (default 30s)
- **`on_dialog(handler)`** — context-level dialog handler, fires before page handlers
- **`expose_function()` / `expose_binding()`** — JS→Rust callback bridge on both BrowserContext and Page
  - `expose_function(name, callback)` — inject a Rust function callable from JS as `window[name](...)`
  - `expose_binding(name, callback)` — same with source info (note: `needsHandle: true` not yet supported)
  - BindingCall protocol object for handling JS→Rust invocations
- Added `async-trait` as a dependency

## [0.9.0] - 2026-03-27

### Added

- **Back-reference properties** — navigate the protocol object hierarchy from child to parent
  - `dialog.page()` — returns the `Page` that owns the dialog (via protocol parent)
  - `download.page()` — returns the `Page` that triggered the download (stored at construction)
  - `response.request()` — returns the `Request` that triggered the response (via ResponseObject parent)
  - `response.frame()` — returns the `Frame` that initiated the request (delegates to `request.frame()`)
  - `request.frame()` — returns the `Frame` that initiated the request (eagerly resolved from initializer GUID)
- **Response server info** — inspect connection and TLS details
  - `response.security_details()` — TLS/SSL certificate info (`SecurityDetails`: issuer, protocol, subject_name, valid_from, valid_to); returns `None` for HTTP
  - `response.server_addr()` — server IP address and port (`RemoteAddr`); returns `None` for cached responses
  - `response.finished()` — wait for response to finish loading (currently returns immediately for goto/reload responses)
- **Request completion methods** — full request lifecycle access
  - `request.redirected_from()` / `request.redirected_to()` — navigate the redirect chain (eagerly resolved from initializer)
  - `request.response()` — get the matching `Response` via RPC
  - `request.sizes()` — resource size info (`RequestSizes`: request_body_size, request_headers_size, response_body_size, response_headers_size)
- **New types**: `SecurityDetails`, `RemoteAddr`, `RequestSizes` (exported from `playwright_rs::protocol`)
- **Page assertions** — `expect_page(&page)` now supports title and URL assertions
  - `to_have_title(expected)` / `to_have_title_regex(pattern)` — assert page title with auto-retry
  - `to_have_url(expected)` / `to_have_url_regex(pattern)` — assert page URL with auto-retry
  - `.not()` negation and `.with_timeout()` supported (matching locator assertion pattern)

### Breaking Changes

- **Response struct fields are now private** — `response.url`, `response.status`, `response.status_text`, `response.ok`, `response.headers` are no longer accessible as public fields. Use the existing accessor methods instead: `response.url()`, `response.status()`, `response.status_text()`, `response.ok()`, `response.headers()`. These methods were already available; only direct field access is removed.
- **`Download::from_artifact` is now `pub(crate)`** — this was an internal constructor not intended for public use.

### Fixed

- **Request parent type corrected** — Request's parent in the Playwright protocol is Page (not Frame as previously assumed). The `request.frame()` method now correctly resolves the frame from the initializer's `frame` GUID via the connection registry.

## [0.8.7] - 2026-03-24

### Added

- **Locator advanced methods** — `tap()`, `evaluate()`, `evaluate_all()`, `drag_to()`, `wait_for()`, `dispatch_event()`, `bounding_box()`, `scroll_into_view_if_needed()`
  - `tap(options)` — touch-tap on an element (requires `has_touch: true` context); `TapOptions` builder with `force`, `modifiers`, `position`, `timeout`, `trial`
  - `evaluate(expression, arg)` — run a JavaScript function with the element as first argument, returns typed `R: DeserializeOwned`
  - `evaluate_all(expression, arg)` — run a JavaScript function with all matching elements as an array, returns typed `R: DeserializeOwned`
  - `drag_to(target, options)` — drag this element to another; `DragToOptions` builder with `force`, `source_position`, `target_position`, `timeout`, `trial`
  - `wait_for(options)` — wait for element to reach a state (`Visible`, `Hidden`, `Attached`, `Detached`); `WaitForOptions` with `state` and `timeout`
  - `dispatch_event(type, event_init)` — fire DOM events with optional initialization data
  - `bounding_box()` — get element dimensions and position (x, y, width, height)
  - `scroll_into_view_if_needed()` — scroll element into viewport
  - `page` property — back-reference to the owning Page from any Locator
- **TLS backend features** — Expose `native-tls`, `rustls-tls-native-roots`, and `rustls-tls-webpki-roots` features for choosing TLS implementation (PR #41). Defaults to `native-tls`.
- **Locator filtering & composition** — `filter()`, `and_()`, `or_()` methods for narrowing and combining locators
  - `filter(FilterOptions)` — narrow by `has_text`, `has_not_text`, `has` (child locator), `has_not`
  - `and_(locator)` — match elements satisfying both locators
  - `or_(locator)` — match elements satisfying either locator
- **Locator interaction methods** — `focus()`, `blur()`, `press_sequentially()`, `all_inner_texts()`, `all_text_contents()`
  - `focus()` / `blur()` — set or remove keyboard focus on an element
  - `press_sequentially(text, options)` — type characters one by one with optional delay
  - `all_inner_texts()` / `all_text_contents()` — bulk text retrieval from all matching elements
  - `dispatch_event(type, event_init)` — fire DOM events with optional initialization data
  - `bounding_box()` — get element dimensions and position (x, y, width, height)
  - `scroll_into_view_if_needed()` — scroll element into viewport
- **BrowserContext runtime setters** — configure context after creation
  - `cookies(urls)` — retrieve cookies, optionally filtered by URL
  - `clear_cookies(options)` — remove cookies with optional name/domain/path filters
  - `set_extra_http_headers(headers)` — add HTTP headers to all requests
  - `grant_permissions(permissions, options)` — grant browser permissions (geolocation, camera, etc.)
  - `clear_permissions()` — revoke all granted permissions
  - `set_geolocation(geolocation)` — override device geolocation, or pass None to clear
  - `set_offline(offline)` — toggle offline mode
- **Page methods** — `bring_to_front()`, `viewport_size()`, `set_extra_http_headers()`, `emulate_media()`, `pdf()`, `add_script_tag()`
  - `bring_to_front()` — activate the page tab
  - `viewport_size()` — get current viewport dimensions (returns None if no_viewport context)
  - `set_extra_http_headers(headers)` — add HTTP headers to all page requests
  - `emulate_media(options)` — override CSS media type, color scheme, reduced motion, forced colors
  - `pdf(options)` — generate PDF (Chromium only), with full options (margins, scale, landscape, etc.)
  - `add_script_tag(options)` — inject JavaScript via URL, file path, or inline content
- **Page timeout & state** — `set_default_timeout()`, `set_default_navigation_timeout()`, `is_closed()`, `frames()`
  - `set_default_timeout(ms)` / `set_default_navigation_timeout(ms)` — configure default timeouts for actions and navigation
  - `is_closed()` — check if page has been closed (tracks close events from server)
  - `frames()` — list page frames (currently main frame only; iframe enumeration planned)
- **BrowserContext timeout defaults** — `set_default_timeout()`, `set_default_navigation_timeout()`
  - Propagates to all existing pages and newly created pages in the context
- **Response body access** — `body()`, `text()`, `json()`, `all_headers()`, `header_value()`, `headers_array()`
  - `body()` — response body as raw bytes
  - `text()` — response body as UTF-8 string
  - `json::<T>()` — parse response body as typed JSON (`T: DeserializeOwned`)
  - `all_headers()` — all response headers as HashMap (merges duplicates)
  - `header_value(name)` — get a single header value by name
  - `headers_array()` — all headers as `Vec<HeaderEntry>` preserving duplicates
- **Request properties** — `headers()`, `post_data()`, `post_data_buffer()`, `post_data_json()`, `failure()`, `all_headers()`, `header_value()`, `headers_array()`, `timing()`
  - `headers()` — request headers as HashMap (from initializer)
  - `post_data()` / `post_data_buffer()` — request body as string or bytes (base64-decoded)
  - `post_data_json::<T>()` — parse request body as typed JSON
  - `failure()` — error text if request failed (set on `requestFailed` event)
  - `all_headers()` / `header_value()` / `headers_array()` — full raw headers via RPC
  - `timing()` — `ResourceTiming` with 9 timing fields (extracted from Response on `requestFinished`)

### Changed

- **Playwright driver upgraded to 1.58.2** (from 1.56.1) — includes WebKit 26.0, Chromium 133, Firefox 135

### Fixed

- **WebKit `launchPersistentContext` now works** — Closes #39. Upgraded Playwright driver resolves "Browser started with no default context" error on macOS ARM64
- **docs.rs build** — Pin docs.rs to `nightly-2025-05-01` to work around `generic-array` 0.14 incompatibility with Rust 1.92+ (`doc_auto_cfg` removal)

## [0.8.6] - 2026-03-14

### Fixed

- **docs.rs build** — Skip Playwright driver download when building on docs.rs (no network access needed for documentation)
- **Imprecise dependency versions** — Pin workspace dependencies to minor versions (e.g., `serde = "1.0"` instead of `"1"`)

## [0.8.5] - 2026-03-14

### Added

- **`ignore_default_args` for persistent contexts** - Added `ignore_default_args` option to `BrowserContextOptions` for use with `launch_persistent_context_with_options()` (Issue #38)
  - `IgnoreDefaultArgs::Bool(true)` - Playwright does not pass its own default args
  - `IgnoreDefaultArgs::Array(vec)` - Filters out specific default arguments
  - Applies same `ignoreDefaultArgs` → `ignoreAllDefaultArgs` protocol normalization as `LaunchOptions`
  - Matches Playwright's official `launchPersistentContext` API
- **Page network event listeners** - Subscribe to network events on individual pages (PR #37)
  - `page.on_request(handler)` - Fires when a request is issued
  - `page.on_response(handler)` - Fires when a response is received
  - `page.on_request_finished(handler)` - Fires when a request completes successfully
  - `page.on_request_failed(handler)` - Fires when a request fails
  - Lazy subscription: events are only subscribed when a handler is registered
  - Works with iframes and sub-resources
- **Response accessor methods** - `response.status()`, `response.status_text()`, `response.url()` (PR #37)
- **`page.go_back()` / `page.go_forward()`** - History navigation with optional timeout and wait_until options
- **`page.set_content(html)`** - Set page HTML content directly, with optional timeout and wait_until options
- **`page.wait_for_load_state(state)`** - Wait for `load`, `domcontentloaded`, or `networkidle` states
- **`page.wait_for_url(url)`** - Wait for navigation to a matching URL (exact string or glob pattern)
- **`locator.is_hidden()` / `locator.is_disabled()`** - Negative state checks complementing `is_visible()` and `is_enabled()`
- **`to_have_screenshot()` visual regression assertion** (Issue #35)
  - `expect(locator).to_have_screenshot(path, options)` — compare locator screenshot against baseline
  - `expect_page(&page).to_have_screenshot(path, options)` — page-level screenshot comparison
  - Auto-creates baseline on first run, compares on subsequent runs
  - `max_diff_pixels` / `max_diff_pixel_ratio` — configurable tolerance
  - `threshold` — per-pixel color distance tolerance (default 0.2)
  - `animations: Disabled` — freeze CSS animations/transitions before capture
  - `mask` — overlay locators with pink (#FF00FF) to exclude dynamic content
  - `update_snapshots` — force baseline update
  - Generates diff image on failure highlighting differences in red
  - Auto-retry with timeout (default 5s), matching Playwright's assertion pattern

### Fixed

- Replace `unwrap()` with graceful error handling in network event dispatch (Issue #40)

## [0.8.4] - 2026-03-01

### Added

- **`get_by_*` locators** - Modern Playwright locator methods for finding elements by user-facing attributes
  - `get_by_text(text, exact)` - Find by text content
  - `get_by_label(text, exact)` - Find form controls by associated label
  - `get_by_placeholder(text, exact)` - Find inputs by placeholder text
  - `get_by_alt_text(text, exact)` - Find images by alt text
  - `get_by_title(text, exact)` - Find elements by title attribute
  - `get_by_test_id(test_id)` - Find elements by `data-testid` attribute (always exact)
  - `get_by_role(role, options)` - Find elements by ARIA role with optional name, checked, disabled, expanded, selected, level, pressed, include_hidden filters
  - All methods available on both `Page` and `Locator` (chainable)
  - Case-insensitive substring matching by default (`exact=false`), case-sensitive exact with `exact=true`
  - `AriaRole` enum with 81 ARIA roles for compile-time safety
  - `GetByRoleOptions` struct for role-based filtering
- **`connect_over_cdp`** - Connect to Chrome DevTools Protocol endpoints (Issue #32)
  - `browser_type.connect_over_cdp(endpoint_url, options)` - Connect to remote Chrome via CDP
  - Supports browserless, Chrome with `--remote-debugging-port`, and other CDP services
  - Accepts optional headers, timeout, and slow_mo options
  - Chromium-only (returns error for Firefox/WebKit)
- **`Locator.all()`** - Iterate over all matching elements (Issue #33)
  - `locator.all()` returns `Vec<Locator>`, one per matching element
  - Empty vec for non-matching selectors (no error)
  - Matches Playwright's `locator.all()` API
- **Improved error messages** - All locator methods now include the selector in error messages (Issue #33)
  - Timeout errors show `[selector: div.page-number > span:last-child]` instead of generic messages
  - Applied to all query methods (`text_content`, `get_attribute`, etc.) and action methods (`click`, `fill`, etc.)
- **BrowserContext proxy support** - Added `proxy` option to `BrowserContextOptions` for per-context proxy configuration (PR #29, Issue #28)
  - Enables rotating proxies without creating new browser instances
  - Supports HTTP and SOCKS proxies with optional authentication
- **Complete Route API** - Full network interception parity with Playwright (Issue #36)
  - `route.fallback(overrides)` - Continue to next matching handler (handler chaining)
  - `route.fetch(options)` - Fetch actual response for inspection/modification before fulfilling
  - `FetchResponse` type with `status()`, `ok()`, `headers()`, `body()`, `text()`, `json()` methods
  - `FetchOptions` builder for customizing fetch requests (method, headers, post_data, timeout)
- **Context-level routing** - `BrowserContext.route()`, `unroute()`, `unroute_all()` for routing across all pages in a context
- **Page unroute methods** - `page.unroute(pattern)` and `page.unroute_all()` for removing route handlers
- **APIRequestContext** - Internal implementation for `route.fetch()` via BrowserContext's request context
  - Handles fetch → fetchResponseBody → disposeAPIResponse protocol flow
  - Automatic base64 encoding/decoding for request and response bodies
- **`UnrouteBehavior` enum** - Control behavior when removing route handlers

### Fixed

- **`no_viewport(true)` / `--start-maximized` not working** - Fixed protocol field name for viewport disabling (Issue #34)
  - `no_viewport` now correctly serializes as `noDefaultViewport` (matching the Playwright protocol)
  - Previously serialized as `noViewport` which the server silently ignored
  - Enables `--start-maximized` with `no_viewport(true)` to produce maximized browser windows

## [0.8.3] - 2026-01-25

### Added

- **PLAYWRIGHT_VERSION constant** - Exposes bundled Playwright driver version (`1.56.1`) as a public constant for version-aware browser installation (Issue #27)
- **Helpful browser installation errors** - Detects missing browser errors and provides actionable guidance (Issue #27)
- **Page.content()** - Returns full HTML content of the page including DOCTYPE (Issue #23)
  - `page.content()` - Retrieves complete HTML markup
  - `frame.content()` - Frame-level implementation for consistency with Playwright API
- **Page.set_viewport_size()** - Dynamically resize viewport for responsive testing (Issue #24)
  - `page.set_viewport_size(viewport)` - Set viewport to specific width/height
  - Enables testing mobile, tablet, and desktop layouts within a single page session

### Fixed

- **page.url() hash navigation** - URL now correctly includes hash fragment after anchor clicks (Issue #26)
  - Frame now handles "navigated" events to track URL changes including hash updates
  - Page delegates to main frame for URL (matches playwright-python/JS behavior)

### Changed

- **Rust Edition 2024** - Upgraded to Rust Edition 2024, requiring Rust 1.85+
- **README documentation** - Added comprehensive browser installation section (Issue #25)

## [0.8.2] - 2026-01-19

### Added

- **Protocol Stubs** - Explicit protocol types for `Android`, `Electron`, `Tracing`, `APIRequestContext`, and `LocalUtils` to support valid registration and prevent "Unknown protocol type" warnings. (Implemented as stubs for future expansion)
- **Cookie & Storage Management** - Implemented `BrowserContext::storage_state()` and `BrowserContext::add_cookies()` (Issue #10)
- **Remote Connection** - Support for connecting to remote browsers via WebSocket
  - `BrowserType::connect(url, options)` implementation
  - `ConnectOptions` builder for connection configuration (headers, slow_mo, timeout)
  - WebSocket transport using `tokio-tungstenite`
  - Internal transport abstraction supporting both options (Pipe and WebSocket)
- **WebSocket Event Handling** - `Page::on_websocket()` for intercepting WebSocket connections (Slice 2)
  - `WebSocket` protocol object with events: `on_frame_sent`, `on_frame_received`, `on_close`, `on_error`
  - Access to WebSocket URL and state
- **File Upload Helpers** - `FilePayload::from_path` and `from_file` for automatic MIME type detection and easier file uploads.
- **Browser Context Options** - Added support for `RecordHar` and `RecordVideo` configuration (paths, dimensions, filters).
- **Service Worker Control** - Added `service_workers` option to `BrowserContextOptions`.
- **Error Handling** - Added `Error::context()` for richer error reporting.

### Breaking Changes

- **Error Enum**: Added `Error::Context` variant. Exhaustive matches on `Error` will need to handle this new variant.
- **BrowserContextOptions**: Added new public fields (`record_har`, `service_workers`, etc.). Code constructing this struct via struct literal syntax (e.g. `BrowserContextOptions { ... }`) will break; use `BrowserContextOptions::builder()` instead.

### Fixed

- **Event Deserialization** - Fixed `ProtocolError` when parsing `__dispose__` events by correctly handling optional `params` field (Issue #11)

## [0.8.1] - 2026-01-04

### Added

- **Persistent Contexts & App Mode** - Support for `launchPersistentContext` (Issue #9)
  - `BrowserType::launch_persistent_context(user_data_dir)`
  - `BrowserType::launch_persistent_context_with_options(user_data_dir, options)`
  - Full support for `--app=url` argument for standalone application windows
  - Persistent user data directories for saving session state (cookies, local storage) across runs
  - Initial page handling for app mode (automatically tracked in `context.pages()`)

## [0.8.0] - 2025-12-30

### Added

- **Typed Evaluate API** - Generic `Page::evaluate()` method with argument serialization and typed results (PR #8)
  - `Page::evaluate<T: Serialize, U: DeserializeOwned>(expression, arg)` - Fully typed JavaScript evaluation
  - `Frame::evaluate<T: Serialize>(expression, arg)` - Frame-level evaluation returning `serde_json::Value`
  - Argument serialization: Pass any `Serialize` type to JavaScript (primitives, structs, arrays, objects)
  - Result deserialization: Receive typed results with compile-time validation
  - Backward compatible: Original `evaluate_expression()` and `evaluate_value()` methods preserved
  - Comprehensive serialization module with Playwright protocol support
  - Special value handling: Infinity, NaN, -0, circular references, TypedArrays, Dates, BigInt
  - Example: `evaluate_typed.rs` demonstrating usage patterns

### Community Credit

- Implementation by @douglasob (Douglas Braga)

## [0.7.2] - 2025-12-24

### Added

- **Storage State Support** - `BrowserContextOptions` now supports `storageState` for session persistence (Issue #6)
  - `storage_state(StorageState)` - Load cookies and localStorage from inline object
  - `storage_state_path(String)` - Load storage state from JSON file
  - New types: `Cookie`, `LocalStorageItem`, `Origin`, `StorageState`
  - Enables authentication state persistence without re-login
  - Async file reading with proper error handling
- `Page::pause()` method for manual debugging (Issue #5)
  - Opens Playwright Inspector and pauses script execution
  - Delegates to new `BrowserContext::pause()` method

### Fixed

- Protocol serialization for methods with no arguments (fixed `ProtocolError` on `pause`)
- **Consistent Test Logging** - Refactored all integration tests to explicitly initialize tracing, ensuring protocol errors are captured and visible (Issue #4)

## [0.7.1] - 2025-12-24

### Added

- **Script Injection** - `BrowserContext.add_init_script()` for context-level script injection before page load
- **Script Injection** - `Page.add_init_script()` for page-level script injection before page load
- **Style Injection** - `Page.add_style_tag()` for injecting CSS into pages
  - `AddStyleTagOptions` struct with builder pattern
  - Support for inline `content` (CSS string)
  - Support for external `url` (stylesheet URL)
  - Support for `path` (load CSS from file)
  - Returns `ElementHandle` to the injected style tag

## [0.7.0] - 2025-11-16

### Added

- `Browser::is_connected()` method to check if the browser is still connected to the server (Issue #2)

### Changed

- **[BREAKING] Single-crate architecture** - Consolidated `playwright-core` into `playwright-rs` to match all official Playwright implementations (Python, Java, .NET, Node.js)
  - Merged all code from `playwright-core` into `playwright-rs` under `src/protocol/` and `src/server/`
  - Removed `playwright-core` crate from workspace
  - Updated all internal imports from `playwright_core::` to `playwright_rs::`
  - Server module now marked `#[doc(hidden)]` - exposed only for integration testing
  - **Migration**: Users of `playwright-rs` v0.6.x should see no API changes. Users of `playwright-core` should switch to `playwright-rs` (see deprecation notice in playwright-core v0.6.2)

### Fixed

- Resolved root cause of Issue #3 by eliminating two-crate complexity that caused workspace detection issues
- Updated all doctests to use consolidated crate structure
- Fixed integration test imports to use new module paths

### Internal

- All 248+ tests passing (library + integration + doctests)
- Maintained backward compatibility for public API
- Release workflow updated to publish single crate

**Related**: Issue #3, ADR 0003 (Single-Crate Architecture Decision)

## [0.6.1] - 2025-11-15

### Fixed

- **[Critical] Build script workspace detection** - Fixed issue #3 where `build.rs` failed to detect the correct workspace root when playwright-core is used as a crates.io dependency
  - Implemented robust three-tier detection strategy:
    1. Use `CARGO_WORKSPACE_DIR` (Rust 1.73+) to detect dependent project's workspace
    2. Walk up directory tree to find `Cargo.toml` with `[workspace]`
    3. Fallback to platform-specific cache directory (matches playwright-python behavior)
  - This fix unblocks usage of playwright-rust in downstream projects like Folio
  - Drivers now download to the correct location in all scenarios (workspace development, crates.io dependency, non-workspace projects)

## [0.6.0] - 2025-11-14

**First public release** of `playwright-rs` - Production-ready Rust bindings for Microsoft Playwright.

### Platform Support

- **Cross-platform**: Full support for Windows, macOS, and Linux
- **Cross-browser**: Chromium, Firefox, and WebKit
- **Windows-specific**: Platform-specific lifecycle management with Drop handler for proper stdio cleanup
- **CI-ready**: Automated cross-platform testing on all three operating systems

### Browser & Page Management

- Launch browsers (Chromium, Firefox, WebKit) in headless or headed mode
- Create isolated browser contexts
- Page lifecycle management (create, navigate, close)
- Automatic resource cleanup

### Page Navigation & Content

- Navigate to URLs with `page.goto()` (returns `Option<Response>` - `None` for data URLs and `about:blank`)
- Reload pages, get page title and URL
- Response status and metadata access

### Element Location & Interaction

- **Locators** with CSS selectors and auto-waiting
- **Locator chaining**: `first()`, `last()`, `nth()`, nested locators
- **Element queries**: `text_content()`, `inner_text()`, `inner_html()`, `get_attribute()`, `input_value()`
- **State queries**: `is_visible()`, `is_enabled()`, `is_checked()`, `is_editable()`
- **Actions**: `click()`, `dblclick()`, `fill()`, `clear()`, `press()`, `check()`, `uncheck()`, `hover()`
- **Advanced actions**: `select_option()` (by value/label/index), `set_input_files()`
- **Checkbox convenience**: `set_checked(bool)` for boolean-based check/uncheck

### Action Options (Builder Pattern)

- **Click options**: button, modifiers, position, force, trial, timeout, delay
- **Fill options**: force, timeout
- **Press options**: delay, timeout
- **Check options**: force, position, timeout, trial
- **Hover options**: force, modifiers, position, timeout, trial
- **Select options**: force, timeout

### Low-Level Input Control

- **Keyboard API**: `type_text()`, `press()`, `down()`, `up()`, `insert_text()` with delay options
- **Mouse API**: `move_to()`, `click()`, `dblclick()`, `down()`, `up()`, `wheel()` with button, click_count, delay, steps options

### Screenshots

- Page screenshots with `page.screenshot()`
- Element screenshots with `locator.screenshot()`
- **Screenshot options**: PNG/JPEG format, quality control, full-page, clip region, omit background
- Save to file or get bytes

### Assertions with Auto-Retry

- **Visibility**: `expect(locator).to_be_visible()`, `to_be_hidden()`
- **Text**: `to_have_text()`, `to_contain_text()` with regex pattern support
- **Value**: `to_have_value()` with regex pattern support
- **State**: `to_be_enabled()`, `to_be_disabled()`, `to_be_checked()`, `to_be_unchecked()`, `to_be_editable()`, `to_be_focused()`
- **Negation**: `.not()` for all assertions
- **Custom timeouts**: `.with_timeout()` configuration
- Default 5-second timeout with 100ms polling interval

### Network Interception

- **Route registration**: `page.route()` with async closure handlers
- **Request blocking**: `route.abort()`
- **Request modification**: `route.continue_()` with header/method/postData overrides
- **Response mocking**: `route.fulfill()` with custom status, headers, body (works for API/fetch requests)
- **JSON helpers**: Automatic serialization with `.json()`
- **Pattern matching**: Glob patterns (`**/*.png`, `**/*`, etc.)
- **Request inspection**: Access URL, method, headers in route handlers
- **Cross-browser**: Works on Chromium, Firefox, WebKit

### Event Handling

- **Downloads**: `page.on_download()` with `download.save_as()` and metadata access
- **Dialogs**: `page.on_dialog()` for alert/confirm/prompt with `accept()`/`dismiss()`

### Advanced Features

- **Browser context options**: viewport, user agent, locale, timezone, geolocation, mobile emulation, JavaScript control, offline mode
- **File uploads**: Basic PathBuf upload and advanced FilePayload with name/mimeType/buffer control
- **JavaScript evaluation**: `page.evaluate_value()` with return values

### Developer Experience

- **Comprehensive documentation**: 100% rustdoc coverage with examples, error docs, and Playwright links
- **cargo-nextest integration**: Faster test execution
- **Performance benchmarks**: Criterion.rs suite for regression detection
- **Cross-platform CI**: Tests run on Linux, macOS, and Windows

### Performance Optimizations

- **GUID storage**: Arc<str> optimization (5.5x faster clones, 2.0x faster lookups)
- **Message transport**: Chunked reading for large messages (>32KB) reduces memory pressure
- **Test suite**: 68% reduction in test count through combining related tests

### Known Limitations

- **route.fulfill() body transmission**: Response bodies are not transmitted to browsers due to a Playwright server limitation (tested with Playwright 1.56.1). Workaround: Mock at HTTP server level or wait for Playwright server update. The Rust implementation is correct and will work when Playwright fixes this issue.

### Migration Notes

- Navigation methods (`goto()`, `reload()`) return `Option<Response>` instead of `Response`
  - Playwright returns null for data URLs and `about:blank` (valid behavior, not an error)
  - Migration: `page.goto("https://example.com").await?.expect("response")` or use `if let Some(response) = page.goto(...).await? { ... }`

[Unreleased]: https://github.com/padamson/playwright-rust/compare/v0.13.0...HEAD
[0.13.0]: https://github.com/padamson/playwright-rust/compare/v0.12.3...v0.13.0
[0.12.3]: https://github.com/padamson/playwright-rust/compare/v0.12.2...v0.12.3
[0.12.2]: https://github.com/padamson/playwright-rust/compare/v0.12.1...v0.12.2
[0.12.1]: https://github.com/padamson/playwright-rust/compare/v0.12.0...v0.12.1
[0.12.0]: https://github.com/padamson/playwright-rust/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/padamson/playwright-rust/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/padamson/playwright-rust/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/padamson/playwright-rust/compare/v0.8.7...v0.9.0
[0.8.7]: https://github.com/padamson/playwright-rust/compare/v0.8.6...v0.8.7
[0.8.6]: https://github.com/padamson/playwright-rust/compare/v0.8.5...v0.8.6
[0.8.5]: https://github.com/padamson/playwright-rust/compare/v0.8.4...v0.8.5
[0.8.4]: https://github.com/padamson/playwright-rust/compare/v0.8.3...v0.8.4
[0.8.3]: https://github.com/padamson/playwright-rust/compare/v0.8.2...v0.8.3
[0.8.2]: https://github.com/padamson/playwright-rust/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/padamson/playwright-rust/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/padamson/playwright-rust/compare/v0.7.2...v0.8.0
[0.7.2]: https://github.com/padamson/playwright-rust/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/padamson/playwright-rust/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/padamson/playwright-rust/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/padamson/playwright-rust/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/padamson/playwright-rust/releases/tag/v0.6.0
