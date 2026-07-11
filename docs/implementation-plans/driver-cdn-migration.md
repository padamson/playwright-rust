# Driver acquisition: assemble locally after the CDN shutdown

**Status:** 🚧 Planned

**Companion ADR:** [0006-driver-acquisition-assemble-locally.md](../adr/0006-driver-acquisition-assemble-locally.md)

**Goal:** Restore fresh-build installability after Microsoft discontinued the
prebuilt `playwright-<ver>-<platform>.zip` driver artifact, by assembling the
driver from the npm `playwright-core` package plus a pinned Node.js binary —
producing the same on-disk layout the old zip did, so no runtime code changes.

**Why now.** Every fresh build fails (cold cache → `PLAYWRIGHT_VERSION not
defined`, or `ServerNotFound` at runtime). v0.14.0 on crates.io is
uninstallable for fresh users. See the ADR for the verified diagnosis.

---

## What we're replicating

playwright-python's `scripts/build_driver.py`, minus the final re-zip:

1. Download `playwright-core-<PLAYWRIGHT_VERSION>.tgz` from
   `https://registry.npmjs.org/playwright-core/-/playwright-core-<ver>.tgz`.
   Extract its `package/` subtree → `<driver_dir>/package/`.
2. Download the pinned Node binary from
   `https://nodejs.org/dist/v<NODE_VERSION>/node-v<NODE_VERSION>-<node-platform>.<ext>`.
   Extract `bin/node` (Unix) → `<driver_dir>/node`, or `node.exe` (Windows) →
   `<driver_dir>/node.exe`. Set 0o755 on Unix.

Target layout (what `src/server/driver.rs` already expects — unchanged):

```
<driver_dir>/
  node            (or node.exe on Windows)
  package/
    cli.js
    lib/...
```

## Platform mapping

`build.rs::detect_platform()` already yields the Playwright platform string.
Add the Node-dist mapping (Node uses a different naming scheme):

| Rust (os, arch)      | Playwright   | Node dist triple      | Node archive |
|----------------------|--------------|-----------------------|--------------|
| macos, x86_64        | mac          | darwin-x64            | tar.gz       |
| macos, aarch64       | mac-arm64    | darwin-arm64          | tar.gz       |
| linux, x86_64        | linux        | linux-x64             | tar.gz       |
| linux, aarch64       | linux-arm64  | linux-arm64           | tar.gz       |
| windows, x86_64      | win32_x64    | win-x64               | zip          |
| windows, aarch64     | win32_arm64  | win-arm64             | zip          |

(The Playwright platform string is no longer needed for the *download* — it
only labeled the old zip — but keep it for the driver-dir name / cache key and
`PLAYWRIGHT_DRIVER_PLATFORM`.)

## The NODE_VERSION pin

- Add `const NODE_VERSION: &str = "<x.y.z>"` next to `PLAYWRIGHT_VERSION` in
  `build.rs` (and wherever the shared module lands).
- Source the value from Playwright's own pin for the `v<PLAYWRIGHT_VERSION>`
  release (playwright-python commits a `NODE_VERSION` file at the matching
  tag). **Action:** fetch it for 1.61.1 when implementing — GitHub rate-limited
  the lookup during planning, so it's deliberately left as a step, not guessed.
- Extend `xtask verify-driver-version` to also assert the `NODE_VERSION`
  constant is consistent wherever it appears (mirrors how it guards
  `PLAYWRIGHT_VERSION`).

---

## Work slices

### Slice 1 — Shared acquisition module (resolve the duplication first)

The download/extract logic is duplicated in `build.rs` and
`src/bin/playwright_rs.rs` (standing TODO). Both are broken and both must
change identically, so factor the new logic into one place before rewriting it.

- Options: a small module `include!()`d by both, or a tiny internal path. Match
  whatever the TODO's "once the architecture stabilizes" intent prefers.
- The shared code exposes: given `(drivers_dir, versions, platform)`, download +
  assemble the `node` + `package/` layout, returning the driver dir.

### Slice 2 — Assemble-locally implementation

- Add `flate2` + `tar` build-deps (and to the `cli` feature deps) for `.tar.gz`;
  keep `zip` for the Windows Node archive and existing use.
- Implement the two-download assembly per "What we're replicating" above.
- **Fail loudly:** on any non-2xx or missing expected entry, error with the
  attempted URL and which artifact (npm vs Node) failed — so a future move is a
  one-line diagnosis, not a downstream `PLAYWRIGHT_VERSION not defined` mystery.
- At the end, assert `<driver_dir>/node[.exe]` and
  `<driver_dir>/package/cli.js` exist before setting the rustc-env vars.
- Preserve all env knobs: `PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD`,
  `PLAYWRIGHT_DRIVER_CACHE_DIR`, `PLAYWRIGHT_DRIVER_PATH`,
  `PLAYWRIGHT_DRIVER_DIR` — unchanged behavior.

### Slice 3 — Version guard + supply chain

- `xtask verify-driver-version`: add the `NODE_VERSION` pin to its checks.
- `cargo vet` / `cargo deny`: certify or exempt the new build-deps
  (`flate2`, `tar` + transitives) per the supply-chain skill.
- Note `nodejs.org` and `registry.npmjs.org` as the driver download sources in
  the build.rs header (replacing the azureedge reference).

### Slice 4 — Validate, without a per-push cross-platform build fan-out

Three checks with very different costs; run each where it belongs rather than
fanning out a six-platform build on every push.

- **Every push — pure unit test (near-free).** The platform→Node-triple map and
  the npm/nodejs.org URL construction are pure functions. Unit-test that all six
  targets produce the right triple + URL. This is what actually catches a
  mapping typo; it needs no runner for the target platform and no network.
- **Every push — the existing matrix (no new cost).** `test.yml`'s current
  Linux/macOS/Windows × engine matrix already exercises the real download →
  assemble → launch on the hosted-runner platforms. The fix rides on it for
  free. (The arch variants without hosted runners — linux-arm64, win-arm64,
  mac-x64 — can't launch a browser in CI anyway, so a true six-platform launch
  matrix isn't achievable and isn't the goal.)
- **Weekly cron — resolve all six URLs (the real insurance).** A job that
  HEAD/GETs the npm tarball + Node binary for all six triples and asserts 200.
  One runner checks all six; cheap. **This is the guard that matters:** the
  azureedge shutdown broke us with *zero* code change, so a code-path-filtered
  job would never have fired. Fold it into the existing weekly
  `upstream-playwright-check` cron (same spirit: catch upstream drift on a
  cadence, not on our commits).
- **Path-filtered to the acquisition code + release tags — any heavier real
  download validation** beyond the matrix, if wanted. This is the only place
  code-change gating is appropriate, and it's optional given the matrix already
  covers the runner platforms.

Also: confirm `PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD` (compile-only) still
short-circuits, and that the runtime fallback chain (`try_npm_global` etc.)
is unaffected — it's the escape hatch users can rely on until the fix ships.

### Slice 5 — Release

- **Ship a v0.14.x patch first**, ahead of 0.15.0 — the published crate is
  broken for fresh installs, so the fix should reach existing `= "0.14"`
  consumers. Land the acquisition fix and cut `0.14.1` (driver stays 1.60.0,
  the version 0.14.0 pins). 0.15.0 (with the accumulated changes + driver
  1.61.1) then rides on the same fix later.
- Scope the patch tightly: the acquisition change + its guard/tests only — do
  **not** pull the unreleased 0.15.0 surface into the 0.14.x line.
- CHANGELOG `### Fixed`: driver now assembled from npm + Node after the
  azureedge CDN shutdown; note the new `NODE_VERSION`.
- Downstream follow-up: consumers pinning `= "0.14"` update to the patch once
  it ships and drop any interim `PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD=1` workaround.

**Backport mechanics.** `main` has already moved past v0.14.0 (driver 1.61.1,
crate renamed to `crates/playwright/`, plus the unreleased 0.15.0 work), so the
patch is a **backport branch off the `v0.14.0` tag**, not a cut of `main`:

- Branch from `v0.14.0`; apply only the acquisition change (shared module +
  build.rs/CLI-bin rewrite + guard/tests). Driver stays 1.60.0.
- Pin the `NODE_VERSION` that **Playwright 1.60.0** used (not 1.61.1's) — source
  it from Playwright's pin at the `v1.60.0` tag.
- Release `0.14.1` from that branch. Develop the fix on `main` first (against
  1.61.1) so it lands in 0.15.0 too, then backport the same logic to the 0.14.x
  branch with the 1.60.0 driver + Node pins — keeping the two in sync.

---

## Testing strategy

- **Unit (host, in the shared module):** platform→Node-triple mapping is
  exhaustively correct for all six targets; URL construction for a known
  (version, platform) matches the expected npm + nodejs.org URLs; the
  loud-failure path produces an error naming the URL. These are pure functions —
  no network.
- **Build/integration:** the fresh-build + browser-launch check per platform in
  CI is the real validation (network + assembly + runtime), as today.
- Do **not** add a network download to the unit tests — keep the pure mapping/
  URL logic separable and test that; leave the actual fetch to the build + the
  integration suite.

## Open questions

- Exact `NODE_VERSION` for 1.61.1 — resolve at implementation time from
  Playwright's pin (see above).
- Mirror fallback for Node/npm: worth it, or is loud-fail + escape hatches
  enough for v0.x? Lean "loud-fail + document" now; revisit if flakiness shows.
- Shared-module mechanism (`include!()` vs internal crate) — pick during
  Slice 1 to match the repo's preference.

## Status

Becomes a historical reference once the fix ships; the companion ADR captures
the durable decision.
