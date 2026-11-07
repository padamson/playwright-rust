---
name: driver-bump
description: Runbook for moving the bundled Playwright driver to a new version in playwright-rust. Covers how to establish the real delta (the release notes are not it), parity with playwright-python, which files move, the trace fixture, and the gates. Use when a "Driver bump" issue opens, when asked to bump or roll Playwright, or before touching `PLAYWRIGHT_VERSION` in `build.rs`.
metadata:
  internal: true
---

# Driver bump

The crate pins one Playwright driver (`PLAYWRIGHT_VERSION` in
`crates/playwright/build.rs`) and speaks its JSON-RPC protocol. A bump is
small in the diff and large in what it can break, because the driver
**drops parameters it does not recognize instead of rejecting them**. A
renamed parameter therefore turns a feature off with the whole suite
green. Past releases have done exactly that:

- 1.58 removed the `devtools` launch option, which then did nothing.
- 1.63 renamed every `tracingStart` capture parameter, and traces came out
  empty.
- 1.64 took a request's `id` verbatim as the trace call id, which turned
  the crate's integer ids into `Unknown` trace actions, and moved the trace
  format to v10. Neither was in the release notes.

So the job is to establish the delta first, then move the version.

## 1. Establish the delta

Compare the old and new tags across four sources. Each catches something
the others miss.

1. **The protocol spec.** Move `PLAYWRIGHT_VERSION`, run
   `cargo xtask sync-protocol-spec`, and read `git diff protocol-spec` with
   context, so events, initializers and `internal: true` markers count as
   well as added and removed lines. If only `DRIVER_VERSION` changes, the
   wire contract did not move. The sync needs no driver download.
2. **The public API.** `docs/src/api/*.md` in the Playwright monorepo
   (`since: vX.Y` entries) and `types/types.d.ts` in the published
   `playwright-core`.
3. **The implementation.** `packages/playwright-core/src/client` and
   `src/server`, for behavior the spec does not show: the dispatcher, the
   trace recorder, Chromium switches, selector engines.
4. **playwright-python.** Its roll commit and the fixes around it on the
   matching `release-X.Y` branch (and `main` for a minor). Its new tests are
   the behavior spec for ours. It also pins the Node version the driver
   expects; match it.

For a patch release, `gh api repos/microsoft/playwright/compare/vA...vB`
lists the cherry-picks and files in one call, and is usually the whole
delta. Check whether `packages/playwright-core/browsers.json` is in the file
list: if not, the browser builds are unchanged.

Then sort what you found:

- **Something the crate sends changed** (renamed or removed command or
  parameter): fix it in the bump commit, or the bump ships a silent
  regression. `cargo xtask verify-protocol-methods` catches a removed
  command name, not a renamed parameter.
- **An upstream fix that reaches a surface the crate exposes**: port the
  upstream regression test, and show it fails on the old pin and passes on
  the new one. The red run on the old pin is the evidence the bump fixes
  something for Rust callers; without it the CHANGELOG line is a guess.
  Check reach first: the crate has an `Electron` protocol object but no
  launch API, so Electron fixes do not reach it. Port the upstream test
  server's behavior as it is, fallbacks and fixed URLs included, even
  where one looks unreachable on macOS: upstream's tests run on every
  platform, and Windows Chromium can omit request headers that macOS sends.
  A test that may differ on Windows can be proved before it reaches `main`
  with `gh workflow run test.yml --ref <branch>`.
- **New public API**: a minor bump's new surface goes into a just-in-time
  plan (`docs/implementation-plans/vX.Y-playwright-A.B.md`), one slice per
  feature, landed after the bump. The 1.64 plan is the worked example.
- **Behavior changes callers will notice** (visibility rules, dropped OS
  support such as WebKit on macOS 14 in 1.59, codec changes): name them in
  the CHANGELOG entry, under `### Breaking Changes` if they are.

## 2. Move the version

`cargo xtask verify-driver-version` lists every reference that disagrees
with `build.rs`, so run it after the edit and fix what it names. As of
1.65.0 the set is:

- `crates/playwright/build.rs` (`PLAYWRIGHT_VERSION`)
- `crates/playwright/src/build_support/driver_urls.rs`: `NPM_CORE_INTEGRITY`,
  the version and its `npm view playwright-core@<version> dist.integrity`.
  The assembler refuses a download that does not match, and the unit tests
  fail until the pin names the new version. When `NODE_VERSION` moves too,
  replace `NODE_ARCHIVE_SHA256`'s version and all six rows from that
  release's `SHASUMS256.txt`, after
  `gpgv --keyring pubring.kbx SHASUMS256.txt.sig SHASUMS256.txt` passes with
  the keyring from `nodejs/release-keys` (`gpg/pubring.kbx`).
- `crates/playwright/tests/playwright_version_test.rs`
- `crates/playwright/src/protocol/root.rs`: the note that the
  `sdkLanguage` enum was re-checked. Re-check it (it has never had `rust`)
  before moving the version in the comment.
- `.github/workflows/{test,release,pages}.yml`: browser cache keys
- `crates/site/src/components/hero.rs` (`PLAYWRIGHT_DEV` only) and
  `crates/site-e2e/tests/landing_page.rs`
- `crates/playwright-rs-trace/src/lib.rs` and `tests/parse_fixture.rs`,
  which name the driver that recorded the fixture. Do not just edit the
  string: re-record with `cargo run --package xtask --release --
  regenerate-trace-fixture` (it drives Chromium, so `cargo run` as the first
  token, outside the sandbox) and check the header's `version` and
  `playwrightVersion`. If the trace format moved, keep the old recording as
  `basic-v<N>.trace.zip` and teach the parser the new one, as 1.64 did.
- `docs/architecture/model.c4` picks up the trace crate's doc comment: run
  `asbuilt survey`.

Leave alone: the README badge and `PLAYWRIGHT_RELEASED` in `hero.rs` (both
track crates.io and move at release), and historical mentions in
CHANGELOGs, plans and the site's feature cards.

## 3. Gates

- `cargo xtask verify-driver-version`, `sync-protocol-spec --check`,
  `verify-protocol-methods`, `verify-changelog-links`
- The default lane and doctests locally; the Firefox, WebKit and stress
  lanes (`--run-ignored ignored-only`) locally on a minor bump, since a
  browser build changed
- fmt and clippy for `crates/site` and `crates/site-e2e` by manifest path,
  because the badge lives there and workspace runs miss them
- `asbuilt check`
- Linux and Windows are CI's. A new cache key means a cold browser cache on
  the first run of each workflow.

## 4. Record it

- A `### Changed` entry in `crates/playwright/CHANGELOG.md` `[Unreleased]`:
  old and new version, browser builds, every change that reaches a Rust
  caller, and a sentence on the protocol diff.
- The commit closes the issue `upstream-playwright-check.yml` opened
  (`Closes #N`), and the body says what the delta was, not which files
  moved.
- If the trace format moved, `playwright-rs-trace` releases with
  `playwright-rs`, and the release commit raises both of the crate's
  requirements on it (see the release-process skill).
