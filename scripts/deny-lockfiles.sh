#!/usr/bin/env bash
# Run cargo deny's bans, licenses and sources checks against the workspace
# and every tree excluded from it.
#
# crates/site, crates/site-e2e and crates/playwright/fuzz carry their own
# Cargo.lock, which a bare `cargo deny check` at the root never opens. The
# site is deployed, and site-e2e builds inside pages.yml's deploy job, which
# can push to the repo, so their licenses and sources get the root's checks.
# Advisories are cargo-lockpatrol's, which covers all four lockfiles.
#
# Arguments go to cargo deny before `check`. The pre-commit hook passes
# --locked so a stale lockfile fails instead of being rewritten mid-commit.
# CI does not run this script. The shared supply-chain.yml checks the
# workspace --locked and these trees unlocked, through the `deny-manifests`
# list in security.yml and release.yml. The trees here are every tracked
# Cargo.lock below the root, and scripts/check-excluded-trees.sh holds
# those lists to the same set.
set -euo pipefail
cd "$(dirname "$0")/.."

status=0
echo "==> cargo deny check bans licenses sources (workspace)"
cargo deny "$@" check bans licenses sources || status=1
for lockfile in $(git ls-files '*/Cargo.lock'); do
    manifest=$(dirname "$lockfile")
    echo "==> cargo deny check bans licenses sources ($manifest)"
    cargo deny "$@" --config "$PWD/deny.toml" --manifest-path "$manifest/Cargo.toml" \
        check bans licenses sources || status=1
done
exit "$status"
