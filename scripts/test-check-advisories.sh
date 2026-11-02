#!/usr/bin/env bash
#
# Self-test for check-advisories.sh and advisory-issue-body.py, the two
# halves of advisory-monitor.yml.
#
# The monitor runs on a schedule nobody watches, so a classifier that calls
# a hit "broken" fails one job and files nothing, and one that calls a
# fetch error a hit files an issue about the network. A stub `cargo` on
# PATH replays real output from cargo-deny 0.20 and cargo-audit 0.22, and
# that output is what pins the classifier to the formats it was written
# against. The renderer cases pin when watchers get a comment.

set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
checker="$here/check-advisories.sh"
renderer="$here/advisory-issue-body.py"
# An explicit template: BSD mktemp does not take TMPDIR into account for a
# bare `-d`, so leaving it out puts the fixtures somewhere macOS may refuse.
work=$(mktemp -d "${TMPDIR:-/tmp}/check-advisories-test.XXXXXX")
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin" "$work/fx"

cat > "$work/bin/cargo" <<'STUB'
#!/usr/bin/env bash
echo "$*" >> "$STUB_DIR/calls"
case "$1" in
    deny)  cat "$STUB_DIR/fx/$STUB_DENY.out";  exit "$(cat "$STUB_DIR/fx/$STUB_DENY.code")" ;;
    audit)
        # With STUB_AUDIT_LOCKS set, only the lockfiles it names have a hit.
        if [ -n "${STUB_AUDIT_LOCKS:-}" ]; then
            case " $STUB_AUDIT_LOCKS " in *" $3 "*) cat "$STUB_DIR/fx/audit-hit.out"; exit 1 ;; esac
            exit 0
        fi
        cat "$STUB_DIR/fx/$STUB_AUDIT.out"; exit "$(cat "$STUB_DIR/fx/$STUB_AUDIT.code")" ;;
esac
echo "stub cargo: unexpected subcommand: $*" >&2
exit 99
STUB
chmod +x "$work/bin/cargo"

fixture() {
    printf '%s\n' "$2" > "$work/fx/$1.code"
    cat > "$work/fx/$1.out"
}

fixture deny-clean 0 <<'OUT'
advisories ok
OUT

fixture deny-hit 1 <<'OUT'
error[unmaintained]: Bincode is unmaintained
   ┌─ /home/runner/work/playwright-rust/playwright-rust/Cargo.lock:13:1
   │
13 │ bincode 1.3.3 registry+https://github.com/rust-lang/crates.io-index
   │ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ unmaintained advisory detected
   │
   ├ ID: RUSTSEC-2025-0141
   ├ Advisory: https://rustsec.org/advisories/RUSTSEC-2025-0141
   ├ Solution: No safe upgrade is available!
   ├ bincode v1.3.3
     └── syntect v5.3.0

advisories FAILED
OUT

fixture deny-broken 1 <<'OUT'
2026-10-10 14:12:54 [ERROR] failed to acquire advisory database lock: failed to obtain lock file '/nonexistent/advisory-dbs': failed to create parent directories for lock path
OUT

# A typo in deny.toml: same diagnostic shape as a finding, but no advisory.
fixture deny-config-error 1 <<'OUT'
error[unexpected-keys]: found 1 unexpected keys, expected: ["version", "db-path", "db-urls", "vulnerability", "notice", "unmaintained", "unsound", "yanked", "ignore", "severity-threshold", "git-fetch-with-cli", "disable-yank-checking", "maximum-db-staleness", "unused-ignored-advisory"]
  ┌─ /home/runner/work/playwright-rust/playwright-rust/deny.toml:2:1
  │
2 │ ignroe = []
  │ ──────

2026-10-10 14:28:34 [ERROR] failed to deserialize config from '/home/runner/work/playwright-rust/playwright-rust/deny.toml'
OUT

fixture audit-clean 0 <<'OUT'
OUT

fixture audit-warnings 0 <<'OUT'
Crate:     bincode
Version:   1.3.3
Warning:   unmaintained
Title:     Bincode is unmaintained
Date:      2025-12-16
ID:        RUSTSEC-2025-0141
URL:       https://rustsec.org/advisories/RUSTSEC-2025-0141
Dependency tree:
bincode 1.3.3
└── syntect 5.3.0
    └── playwright-rs-site 0.1.0

OUT

fixture audit-hit 1 <<'OUT'
Crate:     smallvec
Version:   1.6.0
Title:     Buffer overflow in SmallVec::insert_many
Date:      2021-01-08
ID:        RUSTSEC-2021-0003
URL:       https://rustsec.org/advisories/RUSTSEC-2021-0003
Severity:  9.8 (critical)
Solution:  Upgrade to >=0.6.14, <1.0.0 OR >=1.6.1
Dependency tree:
smallvec 1.6.0
└── probe 0.1.0

error: 1 vulnerability found!
OUT

fixture audit-broken 1 <<'OUT'
error: error loading advisory database: I/O operation failed: failed to read directory `/nonexistent/db/crates`: No such file or directory (os error 2)
Caused by:
  -> failed to read directory `/nonexistent/db/crates`: No such file or directory (os error 2)
OUT

failures=0

# fail NAME [CONTEXT_FILE]: report a failed check, indenting the context.
fail() {
    echo "FAIL: $1"
    if [ -f "${2:-}" ]; then sed 's/^/    /' "$2"; fi
    failures=$((failures + 1))
}

# run DENY AUDIT, then expect NAME STATUS TEXT against its result. An empty
# TEXT asserts the report is empty.
run() {
    : > "$work/calls"
    status=0
    PATH="$work/bin:$PATH" STUB_DIR="$work" STUB_DENY="$1" STUB_AUDIT="$2" STUB_AUDIT_LOCKS="${AUDIT_LOCKS:-}" \
        "$checker" > "$work/out" 2> "$work/stderr" || status=$?
}

expect() {
    local name="$1" status_want="$2" text="$3"
    if [ "$status" -ne "$status_want" ]; then
        cat "$work/out" "$work/stderr" > "$work/context"
        fail "$name: exit $status, expected $status_want" "$work/context"
    elif [ -z "$text" ] && [ -s "$work/out" ]; then
        fail "$name: expected an empty report" "$work/out"
    elif [ -n "$text" ] && ! grep -qF -- "$text" "$work/out"; then
        fail "$name: report did not contain '$text'" "$work/out"
    else
        echo "ok: $name"
    fi
}

run deny-clean audit-clean
expect "both clean is clean" 0 ""
if grep -q -- '--log-level error --color never check advisories' "$work/calls" &&
    grep -q -- '-q --color never' "$work/calls"; then
    echo "ok: the quiet flags that keep the report stable are passed"
else
    fail "the quiet flags that keep the report stable were not passed" "$work/calls"
fi
if [ "$(grep -c '^audit ' "$work/calls")" -eq 4 ]; then
    echo "ok: one cargo audit per lockfile"
else
    fail "expected one cargo audit per lockfile" "$work/calls"
fi

run deny-clean audit-warnings
expect "an unmaintained warning from audit is not a hit" 0 ""

run deny-hit audit-clean
expect "a deny diagnostic is a hit" 1 "RUSTSEC-2025-0141"
expect "and the report names the gate" 1 "## cargo deny check advisories"

run deny-clean audit-hit
expect "an audit vulnerability is a hit" 1 "RUSTSEC-2021-0003"
expect "and the report names the gate" 1 "## scripts/audit-lockfiles.sh"

run deny-broken audit-clean
expect "a deny failure without a diagnostic is broken, with no report" 2 ""

run deny-config-error audit-clean
expect "a deny.toml error is broken, not an advisory" 2 ""

run deny-clean audit-broken
expect "an audit failure without a summary is broken, with no report" 2 ""

run deny-hit audit-broken
expect "a hit beside a broken check is still filed, and the job told" 3 "RUSTSEC-2025-0141"

# render OLD_BODY_FILE REPORT_FILE, then rendered NAME STATUS TEXT. TEXT
# is a line the new body must contain.
render() {
    rm -f "$work/comment.txt"
    status_out=$(RUN_URL=https://example.test/run "$renderer" "$1" "$2" "$work/body.txt" "$work/comment.txt")
}

# contains NAME FILE TEXT
contains() {
    if grep -qF -- "$3" "$2" 2>/dev/null; then
        echo "ok: $1"
    else
        fail "$1: $(basename "$2") did not contain '$3'" "$2"
    fi
}

rendered() {
    if [ "$status_out" != "$2" ]; then
        fail "$1: rendered as '$status_out', expected '$2'"
    else
        contains "$1" "$work/body.txt" "$3"
    fi
}

commented() {
    contains "$1" "$work/comment.txt" "$2"
}

# lacks NAME FILE TEXT: FILE exists and does not contain TEXT.
lacks() {
    if [ ! -f "$2" ]; then
        fail "$1: $(basename "$2") was not written"
    elif grep -qF -- "$3" "$2"; then
        fail "$1: $(basename "$2") contained '$3'" "$2"
    else
        echo "ok: $1"
    fi
}

# one_report NAME: the body holds exactly one report section.
one_report() {
    if [ "$(grep -c -- '<!-- advisory-report -->' "$work/body.txt")" -eq 1 ]; then
        echo "ok: $1"
    else
        fail "$1: the body holds more than one report" "$work/body.txt"
    fi
}

# GitHub counts characters, not bytes, and the report's box drawing is
# multibyte.
chars() {
    python3 -I -c 'import sys; print(len(open(sys.argv[1], encoding="utf-8").read()))' "$1"
}

# fits NAME: the body is under GitHub's limit, says it was cut, and cut
# nothing mid-line.
fits() {
    local name="$1" size
    size=$(chars "$work/body.txt")
    if [ "$size" -gt 65536 ]; then
        fail "$name: body is $size characters, over GitHub's 65536"
    elif ! grep -qF "the run log has the full report" "$work/body.txt"; then
        fail "$name: body does not say the report was cut" "$work/body.txt"
    elif grep -E 'RUSTSEC-[0-9]|^error\[' "$work/body.txt" |
        grep -qvE '^RUSTSEC-[0-9]{4}-[0-9]{4}$|^error\[unmaintained\]: Bincode [0-9]{4} is unmaintained$'; then
        fail "$name: a kept line was cut short" "$work/body.txt"
    else
        echo "ok: $name"
    fi
}

# many N: a cargo deny report with N distinct advisories, inclusion graphs
# and all.
many() {
    echo '## cargo deny check advisories (workspace)'
    echo
    awk -v n="$1" '{ a[NR] = $0 } END {
        for (k = 1; k <= n; k++) for (i = 1; i <= NR; i++) {
            l = a[i]
            gsub(/RUSTSEC-2025-0141/, sprintf("RUSTSEC-2025-%04d", k), l)
            gsub(/Bincode is/, sprintf("Bincode %04d is", k), l)
            print l
        }
    }' "$work/fx/deny-hit.out"
}

# notes N: N lines of triage notes.
notes() {
    awk -v n="$1" 'BEGIN { for (i = 1; i <= n; i++)
        printf "Triage note %d: still waiting on an upstream release of this crate.\n", i }'
}

# audit_report LOCK...: the checker's report when only the named lockfiles
# carry the vulnerability.
audit_report() {
    AUDIT_LOCKS="$*" run deny-clean audit-clean
    cat "$work/out"
}

: > "$work/empty.txt"
cp "$work/fx/deny-hit.out" "$work/report.txt"
render "$work/empty.txt" "$work/report.txt"
rendered "a first report carries the triage steps" changed "Triage each one:"
cp "$work/body.txt" "$work/opened.txt"

render "$work/opened.txt" "$work/report.txt"
rendered "the same report leaves the issue alone" unchanged "RUSTSEC-2025-0141"

sed 's/$/\r/' "$work/opened.txt" > "$work/crlf.txt"
render "$work/crlf.txt" "$work/report.txt"
rendered "a body saved with CRLF endings is still unchanged" unchanged "RUSTSEC-2025-0141"

sed 's/Cargo\.lock:13:1/Cargo.lock:17:1/; s/^13 │/17 │/' "$work/fx/deny-hit.out" > "$work/moved.txt"
render "$work/opened.txt" "$work/moved.txt"
rendered "a finding whose lockfile line moved refreshes without a comment" refreshed "Cargo.lock:17:1"

cat "$work/fx/deny-hit.out" "$work/fx/audit-hit.out" > "$work/more.txt"
render "$work/opened.txt" "$work/more.txt"
rendered "a new finding is a change" changed "RUSTSEC-2021-0003"
commented "and the comment names it" "RUSTSEC-2021-0003"
commented "and links the run log" "(https://example.test/run)"
cp "$work/body.txt" "$work/more-opened.txt"
render "$work/more-opened.txt" "$work/report.txt"
commented "a comment for a fixed finding lists it as resolved" "Resolved:"

printf 'Triage: waiting on syntect.\n\n%s\n' "$(cat "$work/opened.txt")" > "$work/noted.txt"
render "$work/noted.txt" "$work/more.txt"
rendered "triage notes outside the report survive an update" changed "Triage: waiting on syntect."

printf 'Hand-written body with no markers.\n' > "$work/bare.txt"
render "$work/bare.txt" "$work/report.txt"
rendered "a body without markers keeps its text and gains the report" changed "Hand-written body with no markers."

audit_report Cargo.lock > "$work/one-lock.txt"
audit_report Cargo.lock crates/site/Cargo.lock > "$work/two-locks.txt"
render "$work/empty.txt" "$work/one-lock.txt"
cp "$work/body.txt" "$work/one-lock-opened.txt"
render "$work/one-lock-opened.txt" "$work/two-locks.txt"
rendered "an advisory reaching a second lockfile is a change" changed "crates/site/Cargo.lock"
commented "and the comment names that lockfile" "cargo audit --file crates/site/Cargo.lock: RUSTSEC-2021-0003"

printf 'Note: the report ends at <!-- /advisory-report -->.\n\n%s\n' "$(cat "$work/opened.txt")" > "$work/quoted.txt"
render "$work/quoted.txt" "$work/report.txt"
rendered "an end marker quoted above the report does not duplicate it" unchanged "RUSTSEC-2025-0141"
one_report "and the body holds one report"

printf 'Note: <!-- advisory-report --> marks the report; keep this note.\n\n%s\n' "$(cat "$work/opened.txt")" > "$work/quoted-start.txt"
render "$work/quoted-start.txt" "$work/more.txt"
rendered "a start marker quoted above the report is not taken for it" changed "marks the report; keep this note."

# Both markers kept around a report someone reshaped by hand.
awk '{ print } /^<!-- advisory-report -->$/ { print "<details>"; print "" }' "$work/opened.txt" > "$work/reshaped.txt"
render "$work/reshaped.txt" "$work/report.txt"
rendered "a reshaped report between intact markers is replaced, not duplicated" refreshed "RUSTSEC-2025-0141"
one_report "and holds one report, not two"

sed 's/^error\[unmaintained\]: Bincode is unmaintained$/error[unmaintained]: ``` @someone/' "$work/fx/deny-hit.out" > "$work/backticks.txt"
render "$work/opened.txt" "$work/backticks.txt"
if grep -qx '````' "$work/body.txt" && grep -qx '````' "$work/comment.txt"; then
    echo "ok: a title holding a code fence cannot close the report's"
else
    fail "a title holding a code fence closes the report's" "$work/body.txt"
fi

many 300 > "$work/big.txt"
render "$work/empty.txt" "$work/big.txt"
fits "a report past GitHub's limit keeps only its findings"
rendered "and keeps every one" changed "RUSTSEC-2025-0300"
if [ "$(grep -c 'RUSTSEC-2025-0300' "$work/body.txt")" -eq 1 ]; then
    echo "ok: once each, though the report names each id twice"
else
    fail "a kept finding is listed more than once" "$work/body.txt"
fi
cp "$work/body.txt" "$work/big-opened.txt"
render "$work/big-opened.txt" "$work/big.txt"
rendered "and the same report again leaves the issue alone" unchanged "RUSTSEC-2025-0300"

{ notes 600; cat "$work/opened.txt"; } > "$work/long-notes.txt"
render "$work/long-notes.txt" "$work/big.txt"
fits "triage notes count toward the limit"

many 1000 > "$work/huge.txt"
render "$work/empty.txt" "$work/huge.txt"
fits "findings alone past the limit are cut at a line"
cp "$work/body.txt" "$work/huge-opened.txt"
render "$work/opened.txt" "$work/huge.txt"
commented "a comment past the limit is cut and says so" "more not shown)"

# A new finding at the front pushes the last ones past the cut; they are
# still live, so nothing is announced as resolved.
cat "$work/one-lock.txt" "$work/huge.txt" > "$work/huge-more.txt"
render "$work/huge-opened.txt" "$work/huge-more.txt"
rendered "a new finding in a cut report is a change" changed "RUSTSEC-2021-0003"
commented "and the comment names it" "RUSTSEC-2021-0003"
lacks "and findings pushed past the cut are not called resolved" "$work/comment.txt" "Resolved:"

# Notes that leave no room for a single finding, on a body GitHub still
# accepts: the findings are live, so this is no change, whatever the body
# can show.
{ notes 870; cat "$work/opened.txt"; } > "$work/longer-notes.txt"
render "$work/longer-notes.txt" "$work/report.txt"
rendered "notes too long to leave room for the report do not resolve its findings" refreshed "Triage note 870:"
fits "and the body still fits"
lacks "and shows no finding" "$work/body.txt" "RUSTSEC-2025-0141"

if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed"
    exit 1
fi
echo "all checks passed"
