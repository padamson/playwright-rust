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
# --locked so a stale lockfile fails instead of being rewritten mid-commit;
# CI passes nothing, since pages.yml builds these trees unlocked too.
set -euo pipefail
cd "$(dirname "$0")/.."

status=0
echo "==> cargo deny check bans licenses sources (workspace)"
cargo deny "$@" check bans licenses sources || status=1
for manifest in crates/site crates/site-e2e crates/playwright/fuzz; do
    echo "==> cargo deny check bans licenses sources ($manifest)"
    cargo deny "$@" --config "$PWD/deny.toml" --manifest-path "$manifest/Cargo.toml" \
        check bans licenses sources || status=1
done
exit "$status"
