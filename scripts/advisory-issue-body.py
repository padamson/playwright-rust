#!/usr/bin/env python3
"""Renders the advisory monitor's tracking-issue body.

Usage: advisory-issue-body.py OLD_BODY REPORT OUT

OLD_BODY is the open issue's body (an empty file when there is none) and
REPORT is check-advisories.sh's output. Writes the new body to OUT and
prints what an open issue needs:

  changed    the findings changed; edit the body and comment, so watchers
             are told
  refreshed  the same findings, reported differently; edit the body only
  unchanged  leave the issue alone

With no open issue, the caller opens one with OUT whatever this prints.

The report sits between two markers, so an update rewrites only that
section and triage notes around it survive. Findings are compared by
identity, not text: cargo deny quotes Cargo.lock line numbers, which move
with every dependency bump, and a comment for each would notify watchers
about nothing.
"""

import re
import sys

START, END = "<!-- advisory-report -->", "<!-- /advisory-report -->"

INTRO = """An advisory that is not in an ignore list is affecting a tree one of
the Security & Quality gates checks, so the next push or scheduled run
will go red.

Triage each one:
- **Fix available**: bump the crate (`cargo update -p <crate>`, plus
  `--manifest-path` for `crates/site`, `crates/site-e2e` or
  `crates/playwright/fuzz`), or bump the parent that pins it.
- **No fix / upstream-blocked**: accept the risk and add the
  `RUSTSEC-…` id, with a rationale, to the ignore list of each gate
  that reports it: `deny.toml` `[advisories].ignore` for
  `cargo deny`, and `.cargo/audit.toml` `[advisories].ignore` (create
  it if absent) for `scripts/audit-lockfiles.sh`. A vulnerability in
  the workspace tree is reported by both.

"""

FOOTER = "\n\n_Maintained automatically by the Security Advisory Monitor workflow._\n"

# A cargo deny diagnostic's first line names the check and the crate or
# advisory title; a RUSTSEC id identifies each cargo audit entry.
FINDING = re.compile(r"^(?:error|warning)\[[a-z-]+\]: .*$|RUSTSEC-\d{4}-\d{4}", re.M)


def render(old, report):
    report = report.rstrip("\n")
    section = f"{START}\n```\n{report}\n```\n{END}"
    if not old.strip():
        return INTRO + section + FOOTER, "changed"
    if START not in old or END not in old:
        return old.rstrip("\n") + "\n\n" + section + "\n", "changed"
    i, j = old.index(START), old.index(END) + len(END)
    if old[i:j] == section:
        return old, "unchanged"
    same = set(FINDING.findall(old[i:j])) == set(FINDING.findall(section))
    return old[:i] + section + old[j:], "refreshed" if same else "changed"


def main(old_path, report_path, out_path):
    # Text mode turns the CRLF endings of a body saved from GitHub's web
    # editor into LF, so an untouched report still compares equal.
    with open(old_path) as f:
        old = f.read()
    with open(report_path) as f:
        report = f.read()
    body, status = render(old, report)
    with open(out_path, "w") as f:
        f.write(body)
    print(status)


if __name__ == "__main__":
    main(*sys.argv[1:4])
