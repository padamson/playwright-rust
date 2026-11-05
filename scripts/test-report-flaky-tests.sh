#!/usr/bin/env bash
#
# Self-test for report-flaky-tests.sh.
#
# The reporter's job is to be the only thing standing between a flaky test
# and a green job nobody looks at twice, so a regression in it is silent by
# construction. The fixtures below are real nextest output, captured from a
# test rigged to fail its first attempt, and they are what pins the parser
# to the format it was written against.

set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
reporter="$here/report-flaky-tests.sh"
# An explicit template: BSD mktemp does not take TMPDIR into account for a
# bare `-d`, so leaving it out puts the fixtures somewhere macOS may refuse.
work=$(mktemp -d "${TMPDIR:-/tmp}/flaky-report-test.XXXXXX")
trap 'rm -rf "$work"' EXIT

cat > "$work/with-flake.log" <<'LOG'
    Starting 2 tests across 1 binary
        PASS [   0.005s] playwright-rs::probe probe_always_passes
   RETRY 2/2 [         ] (───) playwright-rs::probe probe_fails_once_then_passes
  TRY 2 PASS [   0.006s] (2/2) playwright-rs::probe probe_fails_once_then_passes
────────────
     Summary [   0.015s] 2 tests run: 2 passed (1 flaky), 0 skipped
   FLAKY 2/2 [   0.006s] (2/2) playwright-rs::probe probe_fails_once_then_passes
LOG

cat > "$work/clean.log" <<'LOG'
    Starting 5 tests across 1 binary
        PASS [   0.013s] playwright-rs protocol::in_flight::tests::settle_default_neither_waits_nor_flags
────────────
     Summary [   0.115s] 5 tests run: 5 passed, 303 skipped
LOG

cat > "$work/second-flake.log" <<'LOG'
    Starting 3 tests across 1 binary
────────────
     Summary [   1.200s] 3 tests run: 3 passed (1 flaky), 0 skipped
   FLAKY 2/2 [   0.900s] (2/2) playwright-rs::integration websocket::test_frame
LOG

# nextest right-aligns the progress counter to the width of the total, so a
# run of more than nine tests pads it: "( 753/1077)". From a Windows job.
cat > "$work/padded-counter.log" <<'LOG'
     Summary [ 821.781s] 1077 tests run: 1077 passed (1 flaky), 58 skipped
     FLAKY 2/2 [   2.037s] ( 753/1077) playwright-rs::integration navigation::test_page_navigation_methods
LOG

# nextest folds leaky tests into the same parenthetical. From a Windows job.
cat > "$work/flaky-and-leaky.log" <<'LOG'
     Summary [ 310.931s] 1089 tests run: 1089 passed (1 flaky, 2 leaky), 60 skipped
   FLAKY 2/2 [   6.766s] ( 567/1089) playwright-rs::integration connection::test_connect_over_cdp_real_chrome
LOG

cat > "$work/malformed.log" <<'LOG'
error: could not compile `playwright-rs`
LOG

cat > "$work/truncated-stress.log" <<'LOG'
    Starting 58 tests across 1 binary
        PASS [   1.004s] playwright-rs::integration checkbox::test_check_firefox
LOG

# A FLAKY line without the (attempt/total) counter nextest prints today.
cat > "$work/drifted-line.log" <<'LOG'
     Summary [   0.015s] 2 tests run: 2 passed (1 flaky), 0 skipped
   FLAKY 2/2 [   0.006s] playwright-rs::probe probe_fails_once_then_passes
LOG

# A drifted format: the summary counts a flake the FLAKY lines do not name.
cat > "$work/drifted.log" <<'LOG'
     Summary [   0.015s] 2 tests run: 2 passed (1 flaky), 0 skipped
LOG

failures=0
check() {
    local name="$1" expected_status="$2" expected_text="$3"
    shift 3
    local out status
    set +e
    out=$("$reporter" "$@" 2>&1)
    status=$?
    set -e
    if [ "$status" -ne "$expected_status" ]; then
        echo "FAIL: $name: exit $status, expected $expected_status"
        echo "$out" | sed 's/^/    /'
        failures=$((failures + 1))
        return
    fi
    if ! printf '%s\n' "$out" | grep -qF "$expected_text"; then
        echo "FAIL: $name: output did not contain '$expected_text'"
        echo "$out" | sed 's/^/    /'
        failures=$((failures + 1))
        return
    fi
    echo "ok: $name"
}

check "names the flaky test" 0 \
    "playwright-rs::probe probe_fails_once_then_passes" "$work/with-flake.log"
check "names a flaky test whose counter is padded" 0 \
    "playwright-rs::integration navigation::test_page_navigation_methods" "$work/padded-counter.log"
check "names a flaky test in a run that also leaked" 0 \
    "playwright-rs::integration connection::test_connect_over_cdp_real_chrome" "$work/flaky-and-leaky.log"
check "warns for the annotation" 0 \
    "::warning title=Flaky test::" "$work/with-flake.log"
check "clean run reports none" 0 \
    "No flaky tests" "$work/clean.log"
check "counts flakes across several runs" 0 \
    "2 flaky test result(s)" "$work/clean.log" "$work/with-flake.log" "$work/second-flake.log"
# A build break leaves a log with no summary. That must not paint this
# step red on top of the one that actually failed, and must not be silent.
check "a log that never summarized is reported, not fatal" 0 \
    "has no nextest summary" "$work/malformed.log"
check "and it says so as an annotation" 0 \
    "::warning title=No test summary::" "$work/malformed.log"
# A stress lane killed by its timeout must not be absorbed by a healthy
# sibling log and vanish.
check "a truncated log is named even beside a healthy one" 0 \
    "truncated-stress.log has no nextest summary" \
    "$work/clean.log" "$work/truncated-stress.log"
check "a missing log is an error" 1 \
    "none of these nextest logs exist" "$work/absent.log"
check "a log that never ran is skipped when another has output" 0 \
    "playwright-rs::probe probe_fails_once_then_passes" "$work/absent.log" "$work/with-flake.log"
check "a count that disagrees with the names is an error" 1 \
    "but 0 FLAKY line(s) parsed" "$work/drifted.log"
# Without this the sed falls through and the whole raw log line gets
# published as if it were the test's name.
check "a FLAKY line that will not parse is an error" 1 \
    "cannot parse the test name out of this FLAKY line" "$work/drifted-line.log"
check "a flake beside a broken log is still reported" 0 \
    "playwright-rs::probe probe_fails_once_then_passes" \
    "$work/malformed.log" "$work/with-flake.log"

if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed"
    exit 1
fi
echo "all checks passed"
