#!/usr/bin/env bash
#
# Several crates sit outside the workspace but depend on playwright-rs by
# path, so each carries its own Cargo.lock:
#
#   crates/site, crates/site-e2e   wasm target, conflicting features
#   crates/playwright/fuzz         cargo-fuzz needs its own excluded workspace
#
# A workspace dependency change silently staleness all of them, and they only
# catch up whenever someone happens to run a cargo command against those
# manifests — leaving a dirty tree that looks like unrelated noise. The fuzz
# lockfile is the worst of the three for this, since nothing routine touches
# it: it had drifted from playwright-rs 0.13.0 to 0.15.1 before anyone
# noticed.
#
# This checks rather than rewrites: refreshing a lockfile is a dependency
# resolution change, and it should be an explicit command you ran, not
# something that happened during a commit.
#
# Nothing is broken when this fails — no CI job uses --locked against these
# crates. It is a hygiene gate, so the lockfiles reflect reality and the drift
# does not get rediscovered as a mystery.
set -uo pipefail

status=0
# Every tracked Cargo.lock below the root is one of these trees.
for lockfile in $(git ls-files '*/Cargo.lock'); do
  manifest=$(dirname "$lockfile")
  if ! cargo metadata --manifest-path "$manifest/Cargo.toml" --locked --format-version 1 \
       >/dev/null 2>&1; then
    echo "stale lockfile: $manifest/Cargo.lock is behind the workspace."
    echo "  refresh with: cargo metadata --manifest-path $manifest/Cargo.toml >/dev/null"
    echo "  then stage $manifest/Cargo.lock"
    status=1
  fi
done
exit $status
