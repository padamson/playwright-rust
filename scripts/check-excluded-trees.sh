#!/usr/bin/env bash
#
# A tree outside the workspace keeps its own Cargo.lock, and three files
# have to name it or it goes unchecked without a word:
#
#   .github/workflows/security.yml   deny-manifests, the push and PR gate
#   .github/workflows/release.yml    deny-manifests, the tag-time gate
#   .github/dependabot.yml           directories, or its lockfile never moves
#
# scripts/deny-lockfiles.sh and scripts/check-external-lockfiles.sh find the
# trees through git, as cargo-lockpatrol does, so they need no list. This
# checks that each tracked Cargo.lock below the root is named in all three
# files. The other direction needs no check: the shared deny job fails on a
# listed directory that has no tracked Cargo.lock.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

status=0
for lockfile in $(git ls-files '*/Cargo.lock'); do
  dir=$(dirname "$lockfile")
  for workflow in .github/workflows/security.yml .github/workflows/release.yml; do
    if ! grep -Eq "^[[:space:]]+${dir}[[:space:]]*$" "$workflow"; then
      echo "$dir has its own Cargo.lock but is not in $workflow's deny-manifests."
      status=1
    fi
  done
  if ! grep -Fq "\"/$dir\"" .github/dependabot.yml; then
    echo "$dir has its own Cargo.lock but is not in .github/dependabot.yml's cargo directories."
    status=1
  fi
done
exit $status
