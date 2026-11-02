#!/usr/bin/env python3
"""Renders the advisory monitor's tracking-issue body.

Usage: advisory-issue-body.py OLD_BODY REPORT OUT COMMENT

OLD_BODY is the open issue's body (an empty file when there is none) and
REPORT is check-advisories.sh's output. Writes the new body to OUT and
prints what an open issue needs:

  changed    the findings changed; edit the body and post COMMENT, which
             lists the findings that came and went and links the run log
             named by $RUN_URL, so watchers are told
  refreshed  the same findings, reported differently; edit the body only
  unchanged  leave the issue alone

With no open issue, the caller opens one with OUT whatever this prints.

The report sits between two markers, so an update rewrites only that
section and triage notes around it survive. Findings are compared by
identity, not text: cargo deny quotes Cargo.lock line numbers, which move
with every dependency bump, and a comment for each would notify watchers
about nothing.
"""

import os
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

# GitHub refuses a body or comment over 65536 characters. This leaves
# room for the fence, the markers and the run-log line.
LIMIT = 60000
CUT = "(Cut to fit GitHub's size limit; the run log has the full report.)"

# A `## ` heading names a gate and a `==> ` line the lockfile audited
# under it; a cargo deny diagnostic's first line names the check and the
# crate or advisory title, and a RUSTSEC id identifies each advisory.
TREE = re.compile(r"^(?:## |==> )(.*)$")
ITEM = re.compile(r"^(?:error|warning)\[[a-z-]+\]: .*$|RUSTSEC-\d{4}-\d{4}")

# A marker counts only as a whole line, so a triage note that quotes one
# inline is not taken for the report's edge.
START_LINE = re.compile(rf"^{re.escape(START)}$", re.M)
END_LINE = re.compile(rf"^{re.escape(END)}$", re.M)


def entries(text):
    """Each finding paired with the tree it was found in, in report
    order, so an advisory that reaches another lockfile is a new one."""
    tree = None
    for line in text.splitlines():
        heading = TREE.match(line)
        if heading:
            tree = heading.group(1)
            continue
        for item in ITEM.findall(line):
            yield tree, item


def findings(text):
    return set(entries(text))


def compact(report):
    """The report's findings alone, under their trees, in a form that
    parses back to the same findings."""
    lines, tree, seen = [], None, set()
    for entry in entries(report):
        if entry in seen:
            continue
        seen.add(entry)
        if entry[0] != tree:
            tree = entry[0]
            lines += ["", f"## {tree}"] if lines else [f"## {tree}"]
        lines.append(entry[1])
    return lines


def fit(lines, room):
    """The leading lines that fit in `room` characters."""
    kept, size = [], 0
    for line in lines:
        size += len(line) + 1
        if size > room:
            break
        kept.append(line)
    return kept


def fenced(text):
    """`text` in a code fence longer than any backtick run inside it.
    Advisory text is community-authored, and a title that closed the
    fence would render as markdown, @-mentions and all."""
    longest = max((len(run) for run in re.findall(r"`+", text)), default=0)
    fence = "`" * max(3, longest + 1)
    return f"{fence}\n{text}\n{fence}"


def section(report, room):
    """The report between the markers, in at most `room` characters: what
    is left beside the triage notes, which count toward the limit too.
    Past that, only the findings are kept, so identity survives the cut."""
    report = report.rstrip("\n")
    if len(report) > room:
        report = "\n".join(fit(compact(report), room - len(CUT) - 2) + ["", CUT])
    return f"{START}\n{fenced(report)}\n{END}"


def report_span(old):
    """Where the report section sits in `old`, or None: the last start
    marker before the last end marker, whatever lies between, so a report
    someone reshaped (wrapped in <details>, a fence line deleted) is
    replaced rather than left under a second one."""
    ends = list(END_LINE.finditer(old))
    if not ends:
        return None
    starts = list(START_LINE.finditer(old, 0, ends[-1].start()))
    return (starts[-1].start(), ends[-1].end()) if starts else None


def render(old, report):
    """The new body, the findings resolved and new since the old one, and
    the status line."""
    span = report_span(old)
    if not old.strip():
        before, old_section, after = INTRO, "", FOOTER
    elif span:
        before, old_section, after = old[: span[0]], old[span[0] : span[1]], old[span[1] :]
    else:
        before, old_section, after = old.rstrip("\n") + "\n\n", "", "\n"
    new = section(report, LIMIT - len(before) - len(after))
    if new == old_section:
        return old, set(), set(), "unchanged"
    was, shown, live = findings(old_section), findings(new), findings(report)
    # Resolved means gone from the whole report, not pushed past the cut.
    # New means shown: one past the cut cannot be told from one that was
    # always there.
    resolved, found = was - live, shown - was
    return before + new + after, resolved, found, "changed" if resolved or found else "refreshed"


def comment(resolved, new, run_url):
    def listing(heading, items):
        if not items:
            return []
        lines = [f"{tree}: {item}" if tree else item for tree, item in sorted(items, key=str)]
        kept = fit(lines, LIMIT // 2)
        if len(kept) < len(lines):
            kept += ["", f"({len(lines) - len(kept)} more not shown)"]
        return [f"{heading}:", fenced("\n".join(kept)), ""]

    log = f"The issue body has the new report, and [the run log]({run_url}) has it in full." if run_url else ""
    return "\n".join(["The advisory findings changed.", "", *listing("New", new), *listing("Resolved", resolved), log])


def main(old_path, report_path, out_path, comment_path):
    # Text mode turns the CRLF endings of a body saved from GitHub's web
    # editor into LF, so an untouched report still compares equal.
    with open(old_path) as f:
        old = f.read()
    with open(report_path) as f:
        report = f.read()
    body, resolved, new, status = render(old, report)
    with open(out_path, "w") as f:
        f.write(body)
    with open(comment_path, "w") as f:
        f.write(comment(resolved, new, os.environ.get("RUN_URL", "")))
    print(status)


if __name__ == "__main__":
    main(*sys.argv[1:5])
