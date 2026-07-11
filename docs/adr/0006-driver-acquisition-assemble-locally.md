# ADR 0006: Assemble the driver from npm + Node instead of the prebuilt CDN zip

**Status:** Proposed

**Date:** 2026-07-08

**Related Documents:**
- [ADR 0001: Protocol architecture](0001-protocol-architecture.md) — why a Node driver at all
- Implementation Plan: [driver-cdn-migration.md](../implementation-plans/driver-cdn-migration.md)

---

## Context and Problem Statement

`crates/playwright/build.rs` (and, duplicated, the `cli`-feature binary
`src/bin/playwright_rs.rs`) downloads a single prebuilt archive,
`playwright-<version>-<platform>.zip`, from
`https://playwright.azureedge.net/builds/driver`. That archive bundles a
Node.js runtime plus the `playwright-core` JavaScript driver; the build script
extracts it and the runtime (`src/server/driver.rs`) launches
`node package/cli.js`.

That artifact no longer exists. Verified 2026-07-08:

- `playwright.azureedge.net/builds/driver/...` → **404** (the Azure CDN was
  shut down).
- The replacement CDN `cdn.playwright.dev/dbazure/download/playwright/builds/`
  is live and serves **browsers** (a real Chromium build returns 200 + a
  127 MB zip) but **not the driver**: `builds/driver/playwright-*.zip` returns
  the gateway's not-found response (HTTP 400 `GatewayExceptionResponse`, the
  same response a bogus Chromium revision gets) for **every** driver version
  tried, current and old. The prebuilt driver zips were never migrated.
- Upstream issues microsoft/playwright#38273 and #40084 (both open) report the
  same breakage across the ecosystem.
- playwright-python (and the other language bindings) responded by **building
  the driver bundle locally** — `scripts/build_driver.py` downloads
  `playwright-core-<version>.tgz` from the npm registry plus a pinned Node.js
  binary from `nodejs.org/dist`, then packages `node` + `package/` into the
  same `playwright-<version>-<platform>.zip` layout.

The prebuilt-zip distribution is effectively discontinued.

**Impact:** every fresh build (cold `target/`, new machine, CI without a warm
cache, cargo-mutants' copied tree) fails — either the compile error
`environment variable PLAYWRIGHT_DRIVER_VERSION not defined` (download failed,
so `build.rs` set no rustc-env) or, with `PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD=1`, a
runtime `ServerNotFound`. **v0.14.0 on crates.io is currently uninstallable for
fresh users.** Surfaced during downstream dogfooding and independently reported.

### Requirements Summary

- **Functional:** restore working fresh builds on all six supported platforms
  (mac x64, mac-arm64, linux x64, linux-arm64, win x64, win-arm64).
- **Compatibility:** preserve the ADR-0001 architecture — JSON-RPC to the Node
  driver; do not reimplement browser protocols.
- **Build toolchain:** stays pure Rust — no `npm`/`node` required to
  `cargo build` (today `build.rs` only downloads + unzips; keep it that way).
- **Runtime contract:** unchanged — a bundled `node` (or `node.exe`) alongside
  `package/cli.js`, at the layout `src/server/driver.rs` already expects, so no
  runtime code changes.
- **Maintainability:** community project; minimize moving parts and per-release
  manual steps.

### Current Architecture Context

- **Codebase:** `playwright-rs` single crate; `build.rs` acquires the driver,
  `src/server/driver.rs` + `playwright_server.rs` launch it.
- **Current build-deps:** `ureq` (HTTP GET), `zip` (extract).
- **Duplication:** the download/extract routine exists twice —
  `build.rs::download_and_extract_driver` and
  `src/bin/playwright_rs.rs::ensure_driver_in_user_cache` (flagged by an
  existing TODO). Both are broken; both must change. We'll take this opportunity to refactor.

---

## Decision Drivers

1. **Restore installability** — this is a release blocker; the published crate
   is broken.
2. **Match the ecosystem** — replicate playwright-python's proven model rather
   than invent one.
3. **Minimize toolchain surface** — the build must stay pure Rust.
4. **Preserve the runtime contract** — same on-disk layout ⇒ zero runtime
   change ⇒ small, low-risk diff.
5. **Robustness & supply-chain clarity** — fail loudly on download errors;
   be explicit about the new download sources (npm registry, nodejs.org).

---

## Options Considered

### Option 1: Repoint `DRIVER_BASE_URL` at the new CDN

Swap the azureedge base for `cdn.playwright.dev/dbazure/download/playwright`.

**Rejected.** Empirically the driver zips are not hosted there (not-found for
all versions). This is not a URL typo — the artifact is gone. A base swap
changes nothing.

### Option 2: Assemble the driver locally (npm core + Node binary) — **chosen**

Replicate `build_driver.py` in `build.rs`: download the `playwright-core`
tarball from the npm registry and the pinned Node.js binary from `nodejs.org`,
and extract both directly into the driver directory as `node` +
`package/` — the exact layout the old zip produced.

**Key details:**
- npm tarball: `https://registry.npmjs.org/playwright-core/-/playwright-core-<ver>.tgz`
  (`.tar.gz`; extract its `package/` subtree).
- Node binary: `https://nodejs.org/dist/v<node>/node-v<node>-<node-platform>.<ext>`
  (`.tar.gz` on Unix → `bin/node`; `.zip` on Windows → `node.exe`). Requires a
  pinned `NODE_VERSION` tracked next to `PLAYWRIGHT_VERSION`, sourced from
  Playwright's own pin for the release.
- No intermediate zip: unlike `build_driver.py` (which produces a
  redistributable archive) we only need the extracted layout on disk.
- Build stays pure Rust: adds `.tar.gz` extraction (`flate2` + `tar`) alongside
  the existing `zip`.

**Pros:** matches upstream; no CDN we don't control for the driver (npm and
nodejs.org are first-tier, stable); runtime code untouched; build stays
Rust-only.

**Cons:** two downloads and a `NODE_VERSION` pin to track; a
platform→Node-triple mapping; a slightly larger build-dep tree.

**Build-deps added:** `flate2`, `tar` (both under the existing `cli` feature +
the build-script deps).

### Option 3: Require a system Node.js (stop bundling)

Detect `node` on `PATH`; drop the bundled runtime.

**Rejected.** Regresses DX — consumers must install a version-compatible Node;
breaks "works out of the box" and the `cargo install playwright-rs` story;
introduces version-skew failures. The bundled runtime is a deliberate feature.

### Option 4: Vendor the driver in-repo / in-crate

Commit prebuilt bundles.

**Rejected.** ~40 MB × 6 platforms bloats the repo and the published crate, and
still needs a manual refresh every driver bump.

### Option 5: Go Node-free (speak browser protocols directly from Rust)

Eliminate Node entirely.

**Out of scope here.** This is the parked full-rust-native track; its blocker
is WebKit (no Rust client for the WebKit Inspector Protocol; BiDi's WebKit
support lags), which would forfeit Playwright's tri-engine parity. If ever
pursued it is its own ADR, not a fix for this outage.

---

## Decision Outcome

**Chosen option: Option 2 — assemble the driver locally from npm + Node.**

**Rationale:**

1. **It is the only option that restores installability** while keeping the
   ADR-0001 architecture — the driver is intrinsically Node/JS, and this is how
   the whole binding ecosystem now acquires it.
2. **Smallest blast radius.** Reproducing the exact `node` + `package/` layout
   means `src/server/driver.rs` and the launch path are unchanged; the diff is
   confined to the two download routines.
3. **No worse on toolchains.** The build stays pure Rust (download + extract);
   the runtime Node dependency is unchanged from today (it was always bundled).
4. **More resilient than before.** The single point of failure (one Microsoft
   CDN) is replaced by two first-tier sources (npm, nodejs.org), plus loud
   failure and documented escape hatches.

**Trade-offs accepted:**

- A `NODE_VERSION` pin to maintain per Playwright release.
- A new download source (`nodejs.org`) — a supply-chain surface to acknowledge.
- `build.rs` grows a platform→Node-triple map and `.tar.gz` handling.

---

## Consequences

### Positive

- Fresh builds work again on all supported platforms; the crate is installable.
- Driver acquisition matches upstream, so future upstream changes are easier to
  track.
- Removes reliance on a Microsoft CDN we don't control for the driver.
- Opportunity to de-duplicate the two download routines (the standing TODO).

### Negative

- `build.rs` is more complex (two artifacts, platform mapping, two archive
  formats).
- A second version to track (`NODE_VERSION`) and keep in step with the driver.
- Larger build-dependency tree (`flate2`, `tar`) → more `cargo vet` surface.

### Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| Node platform-string mapping wrong on some target | Build fails on that platform | Medium | Exercise a real download + browser launch on all six platforms in CI |
| npm / nodejs.org transient outage or move | Fresh build fails | Low | Fail loudly with the attempted URL; document `PLAYWRIGHT_DRIVER_CACHE_DIR` / `PLAYWRIGHT_DRIVER_PATH` escape hatches; consider a mirror fallback |
| `NODE_VERSION` drifts from Playwright's pin | Subtle driver/runtime mismatch | Medium | Extend `xtask verify-driver-version` to also assert the Node pin; source it from Playwright's release |
| Assembled layout subtly differs from the old zip | Runtime `ServerNotFound` | Low | Assert the `node` + `package/cli.js` paths exist at the end of `build.rs`; the integration suite launching a real browser is the end-to-end check |

---

## Validation

### How This Decision Will Be Validated

- [ ] Fresh build (empty `target/`) downloads, assembles, and launches a
      browser on each of the six platforms in CI.
- [ ] The existing cross-browser integration suite passes against the assembled
      driver (proves functional identity with the old bundle).
- [ ] `PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD` and `PLAYWRIGHT_DRIVER_CACHE_DIR` still
      behave as documented.

### Success Criteria

- A `cargo build` on a clean checkout produces a working driver with no network
  source other than npm + nodejs.org.
- No runtime code change required.
- A driver-version bump touches `PLAYWRIGHT_VERSION` + `NODE_VERSION` and
  nothing else structural.

### Benchmark Needed?

**No.** Acquisition is a build-time one-off; correctness and cross-platform
coverage are what matter, and those are validated by CI, not benchmarks.

---

## Implementation Notes

Full sequencing lives in the implementation plan. Key points:

- Extract the shared download/assemble logic so `build.rs` and the `cli` binary
  stop duplicating it (resolves the standing TODO).
- Keep the escape hatches (`PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD`,
  `PLAYWRIGHT_DRIVER_CACHE_DIR`, `PLAYWRIGHT_DRIVER_PATH`) intact.
- Ship as a patch on the v0.14.x line as well, since the published crate is
  broken — not only folded into the next minor.

### Rollback Plan

If assembly proves unworkable on a platform, that platform can fall back to the
documented `PLAYWRIGHT_DRIVER_PATH` override while the mapping is fixed; the
change is isolated to the acquisition layer, so reverting is low-risk.

---

## References

- [ADR 0001: Protocol architecture](0001-protocol-architecture.md)
- Upstream: microsoft/playwright#38273, microsoft/playwright#40084
- playwright-python `scripts/build_driver.py` (the reference assembly)

---

**Author:** Paul Adamson

**Last Updated:** 2026-07-08
