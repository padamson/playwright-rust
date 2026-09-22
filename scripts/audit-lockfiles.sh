#!/usr/bin/env bash
# Run cargo audit against every lockfile in the repo, not just the root one.
#
# crates/site, crates/site-e2e and crates/playwright/fuzz are excluded from
# the workspace and carry their own Cargo.lock, which a bare `cargo audit`
# at the root never opens. Dependabot lists all four directories, so the
# pins keep moving, but a root-only audit would still miss an advisory
# against a crate pinned only in one of those files.
set -euo pipefail
cd "$(dirname "$0")/.."

status=0
for lock in Cargo.lock crates/site/Cargo.lock crates/site-e2e/Cargo.lock crates/playwright/fuzz/Cargo.lock; do
    echo "==> cargo audit --file $lock"
    cargo audit --file "$lock" "$@" || status=1
done
exit "$status"
