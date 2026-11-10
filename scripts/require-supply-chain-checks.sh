#!/usr/bin/env bash
#
# Fails unless cargo vet, cargo deny, and cargo-lockpatrol's advisory check
# (security.yml's Supply Chain Review, License & Dependency Check, and
# Security Audit jobs) passed
# on a commit. release.yml runs it before publishing, beside a check that
# the commit is on main, so a tag on a commit where one of them failed or
# never ran publishes nothing.
#
# Usage: scripts/require-supply-chain-checks.sh <check-runs.json>
#
# The file is the output of
# `gh api --paginate repos/<owner>/<repo>/commits/<sha>/check-runs`, one
# JSON page after another.

set -euo pipefail

runs=$(cat "$1")
failed=0
for check in "Supply Chain Review" "License & Dependency Check" "Security Audit"; do
  # The newest run of the check by GitHub Actions: a re-run supersedes the
  # run before it, and a check run from any other app does not count.
  result=$(jq -rs --arg name "$check" '
    [.[].check_runs[] | select(.app.slug == "github-actions" and .name == $name)]
    | max_by(.id) // empty
    | "\(.status) \(.conclusion // "none")"' <<<"$runs")
  if [ -z "$result" ]; then
    echo "::error::$check has not run on this commit. Push it to main and tag once Security & Quality passes there."
    failed=1
  elif [ "$result" != "completed success" ]; then
    echo "::error::$check is '$result' on this commit. Publishing waits for it to pass; re-run this workflow's failed jobs once it has."
    failed=1
  else
    echo "ok    $check"
  fi
done
exit "$failed"
