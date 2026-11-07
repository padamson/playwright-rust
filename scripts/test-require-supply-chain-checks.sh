#!/usr/bin/env bash
#
# Self-test for require-supply-chain-checks.sh.
#
# The gate decides whether a release tag publishes, and it runs only on a
# tag push, so a regression would surface at the next release or not at
# all. Each case below is a set of check runs in the shape the GitHub API
# returns them, and the exit status the gate must give for it.

set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
gate="$here/require-supply-chain-checks.sh"
work=$(mktemp -d "${TMPDIR:-/tmp}/supply-chain-gate-test.XXXXXX")
trap 'rm -rf "$work"' EXIT

# run <id> <name> <status> <conclusion|null> [app]
run() {
  local conclusion=$4
  [ "$conclusion" = null ] || conclusion="\"$conclusion\""
  printf '{"id":%s,"name":"%s","status":"%s","conclusion":%s,"app":{"slug":"%s"}}' \
    "$1" "$2" "$3" "$conclusion" "${5:-github-actions}"
}

# page <run>... : one API page holding the given check runs
page() {
  local IFS=,
  printf '{"total_count":%s,"check_runs":[%s]}\n' "$#" "$*"
}

all_green=(
  "$(run 11 "Supply Chain Review" completed success)"
  "$(run 12 "License & Dependency Check" completed success)"
  "$(run 13 "Security Audit" completed success)"
  "$(run 14 "Test (ubuntu-latest)" completed success)"
)

failures=0
expect() { # name, expected exit status, check-runs file
  local status=0
  "$gate" "$3" >"$work/out" 2>&1 || status=$?
  if [ "$status" -eq "$2" ]; then
    echo "ok    $1"
  else
    echo "FAIL  $1: exit $status, expected $2"
    sed 's/^/      /' "$work/out"
    failures=$((failures + 1))
  fi
}

expect "all three passed" 0 <(page "${all_green[@]}")
expect "the checks span two pages" 0 <(page "${all_green[@]:0:2}"; page "${all_green[@]:2}")
expect "vet never ran" 1 <(page "${all_green[@]:1}")
expect "vet failed" 1 <(page "$(run 11 "Supply Chain Review" completed failure)" "${all_green[@]:1}")
expect "vet still running" 1 <(page "$(run 11 "Supply Chain Review" in_progress null)" "${all_green[@]:1}")

# Newest first, as the API lists them, so taking the first or the last
# match instead of the highest id fails one of the two re-run cases.
expect "a re-run of audit failed after a pass" 1 <(page "$(run 21 "Security Audit" completed failure)" "${all_green[@]}")
expect "a re-run of audit passed after a failure" 0 <(page "$(run 9 "Security Audit" completed failure)" "${all_green[@]}")

expect "vet passed only under another app" 1 <(page "$(run 11 "Supply Chain Review" completed success other-app)" "${all_green[@]:1}")
expect "vet was skipped" 1 <(page "$(run 11 "Supply Chain Review" completed skipped)" "${all_green[@]:1}")
expect "no check runs at all" 1 <(page)

if [ "$failures" -ne 0 ]; then
  echo "$failures case(s) failed"
  exit 1
fi
