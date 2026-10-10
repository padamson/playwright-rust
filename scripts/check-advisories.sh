#!/usr/bin/env bash
#
# Runs the advisory half of both supply-chain gates and says whether a
# failure is a finding or a broken tool. advisory-monitor.yml files an
# issue from the report.
#
# It reads the trees the gates read, and no others, so the monitor never
# reports something no gate fails on. `cargo deny check` (security.yml)
# covers the workspace and fails on unmaintained crates as well as
# vulnerabilities. scripts/audit-lockfiles.sh covers all four lockfiles and
# fails on vulnerabilities only. Pointing cargo deny at crates/site instead
# would flag three unmaintained crates in syntect's build tree that no gate
# fails on and that have no fix.
#
# Prints the report of each check that found something.
#   0  both clean
#   1  an advisory a gate would fail on
#   2  a check failed without reporting one: an advisory-db fetch, cargo
#      metadata, a bad config. There is nothing to triage, and an issue
#      about a network error would bury the next real one.
#   3  both: one check found an advisory and the other broke. The finding
#      still gets filed, and the job still fails, so a dead half of the
#      monitor cannot hide behind a live one.
set -uo pipefail
cd "$(dirname "$0")/.."

found=0
broken=0

# The quiet flags keep the advisory-db fetch chatter (deny) and the
# advisory count (audit) out of the report. Only the codes below are
# findings: a bad deny.toml reports in the same shape
# (`error[unexpected-keys]`) and is a broken check, not an advisory.
deny=$(cargo deny --log-level error --color never check advisories 2>&1)
code=$?
if [ "$code" -ne 0 ]; then
    if grep -qE '^(error|warning)\[(vulnerability|unmaintained|unsound|notice|yanked)\]' <<<"$deny"; then
        found=1
        printf '## cargo deny check advisories (workspace)\n\n%s\n\n' "$deny"
    else
        broken=1
        printf 'cargo deny exited %s without reporting an advisory:\n%s\n' "$code" "$deny" >&2
    fi
fi

audit=$(scripts/audit-lockfiles.sh -q --color never 2>&1)
code=$?
if [ "$code" -ne 0 ]; then
    if grep -qE '^error: [0-9]+ vulnerabilit(y|ies) found!$' <<<"$audit"; then
        found=1
        printf '## scripts/audit-lockfiles.sh (every lockfile)\n\n%s\n\n' "$audit"
    else
        broken=1
        printf 'cargo audit exited %s without reporting a vulnerability:\n%s\n' "$code" "$audit" >&2
    fi
fi

exit $((found + 2 * broken))
