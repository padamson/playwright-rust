# ADR 0007: Assemble the driver into the user cache by default

**Status:** Accepted

**Date:** 2026-09-22

**Related Documents:**
- [ADR 0006: Driver acquisition, assemble locally](0006-driver-acquisition-assemble-locally.md)

---

## Context and Problem Statement

ADR 0006 has `build.rs` assemble the Playwright driver (the `playwright-core`
npm tarball plus a pinned Node binary, about 128 MB) at compile time. It
assembled into Cargo's `$OUT_DIR`, inside `target/`, unless
`PLAYWRIGHT_DRIVER_CACHE_DIR` pointed elsewhere.

`$OUT_DIR` is keyed by the dependency hash, so every change of version,
feature set, or profile produced a fresh directory and a fresh download, and
nothing collected the old ones. A consumer measured the cost: fourteen
assembled drivers, 2.2 GB, under one `target/`; four downloads per
`cargo mutants --jobs 4` run, since each job builds in its own copy of the
tree; and a lint job that compiled the crate and launched nothing still
paid for the download. The knobs that avoid all of that were documented only
in `build.rs`, so consumers did not know they existed.

Meanwhile the crate already had a second, stable location: `playwright-rs
install` writes to `dirs::cache_dir()/playwright-rust/<version>/`, and the
runtime lookup probes that path after the bundled one. The build script was
the one acquisition path not using it.

### Requirements Summary

- **Functional:** one download per machine per driver version; a build that
  has run once works offline; a cleaned cache does not leave a build that
  looks fresh but launches nothing.
- **Compatibility:** the runtime contract is unchanged (a `node` binary and
  `package/cli.js` in one directory); the existing knobs keep working.
- **CI:** consumers cache one directory beside the browsers, with no
  crate-specific environment variable to learn.
- **Maintainability:** one layout definition shared by the build script, the
  CLI, and the runtime.

---

## Decision Drivers

1. Measured disk and network cost in consumer builds.
2. Three acquisition paths that should agree on one location.
3. Offline-safety for a machine that has built once.
4. Cargo's convention that build scripts write only to `$OUT_DIR`.

---

## Options Considered

### Option 1: Keep `$OUT_DIR`, document the knobs

Leave the default alone and teach consumers `PLAYWRIGHT_DRIVER_CACHE_DIR`
and `PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD`.

**Pros:** no behavior change; honors the cargo convention exactly.

**Cons:** every consumer pays the cost until they read the docs and wire two
environment variables into every job and script; the mutation-run
multiplication stays; the CLI and runtime keep a location the build never
uses.

### Option 2: Assemble into the user cache by default

`build.rs` assembles into `<cache>/playwright-rust/<version>/playwright-
<version>-<platform>/`, the path the CLI and runtime already use, and emits
`cargo:rerun-if-changed` on the assembled `node` and `package/cli.js`.
`PLAYWRIGHT_DRIVER_CACHE_DIR` overrides the location; `$OUT_DIR` is the
fallback when no cache root resolves (no home directory).

**Pros:** one download per machine per version; offline after one build;
consumer CI caches the same directory it caches for browsers; the three
acquisition paths share one layout helper. Cargo treats a missing watched
file as changed, so a wiped cache reruns the script and reassembles.

**Cons:** a build script writing outside `$OUT_DIR`, which cargo's
documentation advises against and some hermetic-build setups forbid. Two
concurrent first builds on one machine both download.

### Option 3: Keep `$OUT_DIR`, add a lock-free copy from the user cache

Assemble into the user cache once, then copy into each `$OUT_DIR`.

**Pros:** each target directory ends up self-contained.

**Cons:** still writes to the cache, so it carries Option 2's convention
cost, and adds 128 MB per target hash back on top. It solves nothing Option
2 does not.

---

## Decision Outcome

**Chosen Option:** Option 2.

**Rationale:**

1. The cost is measured, not hypothetical, and it lands on every consumer
   rather than on this repo.
2. The hermetic-build objection is already moot: sandboxed builds have no
   network, so the `$OUT_DIR` default fails in them too, and they use the
   skip knob plus `PLAYWRIGHT_DRIVER_PATH` either way.
3. The CLI has written to this location since it existed, so the crate
   already made this trade once.

**Trade-offs Accepted:**

- A deliberate exception to the `$OUT_DIR` convention, named in the build
  script header and the changelog so nobody mistakes it for an oversight.
- Concurrent first builds can download twice. The temp-then-rename assembly
  makes that safe; a lock file can be added if it is ever more than
  wasteful.
- A cache root that does not resolve falls back to `$OUT_DIR`, so the old
  behavior is still reachable and still tested.

---

## Consequences

### Positive

- Consumers' `target/` no longer accumulates drivers; mutation runs stop
  multiplying downloads.
- `playwright-rs install`, `build.rs`, and the runtime resolve to one path
  through one helper, `cached_driver_dir`.
- Consumer CI drops the cache-dir knob and adds one path to the browser
  cache step.

### Negative

- Anyone auditing build scripts for out-of-tree writes will flag this one.
  The header comment and this ADR are the answer.
- The user-cache directory grows by one driver per version ever built,
  until the user clears it. That is what a cache directory is for, and it
  is bounded by versions rather than by target hashes.

### Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| A wiped cache leaves a fresh-looking build with no driver | Launch fails | Medium | `rerun-if-changed` on the assembled files; verified by deleting the cached driver and rebuilding |
| Home directory unresolvable (minimal container, service account) | Falls back to `$OUT_DIR` | Low | The fallback is kept and its layout test still runs where it applies |
| Two builds race on a first download | Wasted bandwidth | Low | Atomic rename; a rename that loses the race is treated as success when the destination holds a complete driver |
| Files written during the run are newer than cargo's invocation stamp | One extra full rebuild of the crate and its dependents after every assembly | Certain without mitigation (confirmed with a probe crate) | The script sets the assembled files' mtime to cargo's `invoked.timestamp`; a time taken inside the script is already too late |
| Watched files that do not exist after a failed download | The script reruns and retries the download on every cargo invocation | Certain without mitigation | Watch lines are emitted only once the files exist, so a failure stays sticky until a knob changes or `cargo clean -p playwright-rs` |
| Consumer CI keyed its cache on the old explicit path | Re-downloads until updated | Medium | The knob still works; changelog names the simpler form |

---

## Validation

- [x] A clean build with no cached driver assembles into the user cache and
      records `user_cache` as the source.
- [x] Deleting the cached driver and building again reassembles it without
      `cargo clean`, and the build after that is fresh.
- [x] A build whose download fails is not rerun by the next build.
- [x] The runtime's user-cache lookup resolves the directory the build
      script wrote (unit test gated on the `user_cache` source).
- [x] The `$OUT_DIR` fallback and the `PLAYWRIGHT_DRIVER_CACHE_DIR` override
      still behave as before.
- [ ] CI on all three platforms restores the driver from the browser cache
      key and makes no download on a warm run.

---

## References

- [ADR 0006](0006-driver-acquisition-assemble-locally.md)
- cargo book, build scripts: "Build scripts should not modify any files
  outside of `OUT_DIR`."
- XDG Base Directory Specification, `$XDG_CACHE_HOME`: non-essential data
  the application can regenerate.
