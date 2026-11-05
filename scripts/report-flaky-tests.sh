#!/usr/bin/env bash
#
# Surfaces the flaky tests a green CI job hides.
#
# `profile.ci` sets `retries = 1`, so a test that fails once and passes on
# the retry exits 0. The job goes green and the only trace is one FLAKY
# line in the middle of a few thousand lines of log, which nobody reads.
# That is why the flaky-test tracker only ever recorded tests that failed
# *both* attempts, and why "the tracker is empty" could not be told apart
# from "nothing was looking".
#
# Reads the nextest logs captured by the test steps, writes the flaky test
# names to the job summary and as warning annotations, and exits 0: a
# flake is something to record and watch, not something to fail a build
# over.
#
# It distinguishes three things, because conflating them either cries wolf
# on every broken build or goes quiet exactly when it matters:
#
#   * A log that never reached a summary. The step failed to compile, or
#     was killed by its timeout. Warns and moves on, per log, so a stress
#     lane that died is visible rather than absorbed by its healthy
#     sibling, but an ordinary build break does not also paint this step
#     red.
#   * A log whose summary disagrees with its FLAKY lines, or a FLAKY line
#     this script cannot parse. That is nextest's output format moving
#     under us, and it exits non-zero, because the alternative is silence
#     that reads like good news.
#   * Everything else: report and exit 0.
#
# Usage: scripts/report-flaky-tests.sh <nextest-log> [<nextest-log>...]

set -euo pipefail

if [ "$#" -eq 0 ]; then
    echo "usage: $0 <nextest-log> [<nextest-log>...]" >&2
    exit 2
fi

label="${FLAKY_REPORT_LABEL:-this job}"

note() {
    echo "$1"
    if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
        echo "$1" >> "$GITHUB_STEP_SUMMARY"
    fi
}

present=""
for log in "$@"; do
    # A step that never ran, because it belongs to the other OS's branch or
    # because an earlier step failed, leaves no log at all. That is normal
    # and says nothing about flakiness.
    if [ -f "$log" ]; then
        present="$present $log"
    fi
done

if [ -z "$present" ]; then
    echo "error: none of these nextest logs exist:$(printf ' %s' "$@")" >&2
    echo "The step is wired to logs that were never written." >&2
    exit 1
fi

expected=0
runs=0
unsummarized=""
flaky_lines=""

for log in $present; do
    # nextest turns color off when it is not writing to a terminal, so these
    # logs normally carry no escapes; strip them anyway so a runner that
    # forces color does not silently defeat every pattern below. The
    # padding nextest prints sits inside the escapes, which is why the
    # patterns still anchor on leading whitespace afterwards.
    # shellcheck disable=SC1003
    text=$(sed -E $'s/\033\\[[0-9;]*[a-zA-Z]//g' "$log")

    if ! printf '%s\n' "$text" | grep -qE '^[[:space:]]*Summary \['; then
        unsummarized="$unsummarized $log"
        continue
    fi
    runs=$((runs + 1))

    # A run's own summary counts its flaky tests: "9 tests run: 9 passed
    # (1 flaky), 0 skipped", or "(1 flaky, 2 leaky)" when tests also leaked.
    # The count is absent when none were, and `|| true` keeps that from
    # taking the script down under `pipefail`.
    count=$(printf '%s\n' "$text" |
        grep -E '^[[:space:]]*Summary \[' |
        grep -oE '[0-9]+ flaky' |
        grep -oE '[0-9]+' |
        awk '{ total += $1 } END { print total + 0 }' || true)
    expected=$((expected + count))

    matched=$(printf '%s\n' "$text" | grep -E '^[[:space:]]*FLAKY [0-9]+/[0-9]+ ' || true)
    if [ -n "$matched" ]; then
        flaky_lines="${flaky_lines}${matched}
"
    fi
done

if [ -n "$unsummarized" ]; then
    for log in $unsummarized; do
        note "Note: ${log} has no nextest summary, so nothing from that run is reported here."
        echo "::warning title=No test summary::${log} never reached a nextest summary in ${label}; the step failed to build or was killed, so its flaky results are unknown"
    done
fi

if [ "$runs" -eq 0 ]; then
    echo "No nextest run in${present} got as far as a summary; nothing to report."
    exit 0
fi

# "  FLAKY 2/2 [   0.008s] (2/2) <binary> <test name>". The counter is padded
# to the width of the total, "( 753/1077)", once a run has ten tests or more.
# Parsed one line at a time and with `-n`, so a line that does not match
# yields nothing rather than passing through as though the whole raw line
# were a test name.
names=""
found=0
while IFS= read -r line; do
    [ -n "$line" ] || continue
    found=$((found + 1))
    name=$(printf '%s\n' "$line" |
        sed -nE 's|^[[:space:]]*FLAKY [0-9]+/[0-9]+ \[[^]]*\] \([[:space:]]*[0-9]+/[0-9]+\) (.+)$|\1|p')
    if [ -z "$name" ]; then
        echo "error: cannot parse the test name out of this FLAKY line:" >&2
        echo "  $line" >&2
        echo "nextest's output format has changed and this script needs updating." >&2
        exit 1
    fi
    names="${names}${name}
"
done <<EOF
$flaky_lines
EOF

if [ "$found" -ne "$expected" ]; then
    echo "error: nextest reported $expected flaky test(s) but $found FLAKY line(s) parsed." >&2
    echo "The two disagree, so this script can no longer be trusted to name them;" >&2
    echo "nextest's output format has probably changed." >&2
    exit 1
fi

if [ "$expected" -eq 0 ]; then
    echo "No flaky tests across $runs nextest run(s)."
    exit 0
fi

unique=$(printf '%s' "$names" | sort -u)

echo "$expected flaky test result(s) in $label:"
printf '%s\n' "$unique" | sed 's/^/  /'

while IFS= read -r test; do
    [ -n "$test" ] || continue
    echo "::warning title=Flaky test::${test} passed only on retry in ${label}"
done <<EOF
$unique
EOF

if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        echo "### Flaky tests in ${label}"
        echo
        echo "Passed only after a retry, so the job is green. Record each one in the"
        echo "flaky-test tracking issue with this run's URL, and harden it when it recurs."
        echo
        printf '%s\n' "$unique" | sed 's/^/- `/; s/$/`/'
        echo
    } >> "$GITHUB_STEP_SUMMARY"
fi
