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
    audit) cat "$STUB_DIR/fx/$STUB_AUDIT.out"; exit "$(cat "$STUB_DIR/fx/$STUB_AUDIT.code")" ;;
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
    if [ -n "${2:-}" ]; then sed 's/^/    /' "$2"; fi
    failures=$((failures + 1))
}

# run DENY AUDIT, then expect NAME STATUS TEXT against its result. An empty
# TEXT asserts the report is empty.
run() {
    : > "$work/calls"
    status=0
    PATH="$work/bin:$PATH" STUB_DIR="$work" STUB_DENY="$1" STUB_AUDIT="$2" \
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
    status_out=$("$renderer" "$1" "$2" "$work/body.txt")
}

rendered() {
    local name="$1" want="$2" text="$3"
    if [ "$status_out" != "$want" ]; then
        fail "$name: rendered as '$status_out', expected '$want'"
    elif ! grep -qF -- "$text" "$work/body.txt"; then
        fail "$name: body did not contain '$text'" "$work/body.txt"
    else
        echo "ok: $name"
    fi
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

printf 'Triage: waiting on syntect.\n\n%s\n' "$(cat "$work/opened.txt")" > "$work/noted.txt"
render "$work/noted.txt" "$work/more.txt"
rendered "triage notes outside the report survive an update" changed "Triage: waiting on syntect."

printf 'Hand-written body with no markers.\n' > "$work/bare.txt"
render "$work/bare.txt" "$work/report.txt"
rendered "a body without markers keeps its text and gains the report" changed "Hand-written body with no markers."

if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed"
    exit 1
fi
echo "all checks passed"
