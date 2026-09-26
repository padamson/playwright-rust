#!/usr/bin/env bash
#
# Fail when the shipped skill changes without a version bump.
#
# The skill reaches consumers through `npx skills add`, which copies it and
# has no version concept of its own. The frontmatter's `metadata.version` is
# the only version an installed copy carries, so a content change that
# leaves it alone is invisible to everyone who has the skill installed.
#
# The skill versions on its own cadence, not the crate's: the crate version
# on the default branch is the last release, while the skill there describes
# the next one, so pinning them made the skill claim a release whose binary
# it did not match.
#
# The rule: whenever content under `skills/` differs from a base revision,
# `metadata.version` must differ from it too. The pre-commit hook applies it
# per commit, index against HEAD. CI applies it with `--base <ref>` to
# everything since the merge base with that ref, the aggregate that actually
# lands, because the hook is per-clone and `--no-verify` skips it. The two
# can disagree on one commit (a bump in one commit and an unbumped edit in
# the next passes CI and fails the hook on the second), and that is the
# intent: the hook keeps history honest, CI keeps what lands honest.
#
# One fixed-shape read: the version is the double-quoted string on the
# `version:` line under `metadata:`. A bare number or single quotes read as
# no version and fail here, so there is exactly one spelling that passes.
#
# A ref that does not resolve or shares no history with HEAD is an error
# here. A caller that would rather skip in that case checks before calling.
set -euo pipefail

usage() {
  echo "usage: $(basename "$0") [--base <ref>]" >&2
  exit 2
}

base=HEAD
while [ $# -gt 0 ]; do
  case "$1" in
    --base)
      [ $# -ge 2 ] || usage
      base="$2"
      shift 2
      ;;
    *) usage ;;
  esac
done

skill=skills/playwright-rs-usage/SKILL.md

# Initial commit: nothing to compare against.
git rev-parse -q --verify HEAD >/dev/null 2>&1 || exit 0

# Everything below compares the index against `$base`. That serves both
# callers with one code path: the hook wants exactly the index, and a fresh
# CI checkout leaves the index equal to HEAD.
if [ "$base" != HEAD ]; then
  base=$(git merge-base "$base" HEAD 2>/dev/null) || {
    echo "skill version guard: --base does not resolve or shares no history with HEAD." >&2
    exit 2
  }
fi

# Nothing under skills/ moved, so there is nothing to guard. pre-commit's
# `files:` filter already implies this on a real commit, but `run
# --all-files` runs every hook regardless of what changed. Without this the
# guard fails a clean full-tree run with "content changed but the version
# did not", and the obvious response to that message is a version bump
# describing no change, so the false positive has a plausible wrong fix.
# `--quiet` exits 1 for "changed"; anything else is git failing.
changed=0
git diff --cached --quiet "$base" -- skills/ || changed=$?
case "$changed" in
  0) exit 0 ;;
  1) ;;
  *)
    echo "skill version guard: git diff against $base failed (exit $changed)." >&2
    exit 2
    ;;
esac

# The one shape: inside the frontmatter, under `metadata:`, an indented
# `version:` whose value is double-quoted. Anything else prints nothing,
# which reads as "no version", including a file that is absent at the
# revision, so the commit that first adds the skill compares against empty.
# CRLF is stripped first: a Windows checkout can hand back \r\n through the
# index while the guard reads bytes with `git show`.
read_skill_version() {
  tr -d '\r' | awk '
    /^---$/ { fence++; if (fence >= 2) exit; next }
    fence == 1 && /^metadata:[[:space:]]*$/ { inmeta = 1; next }
    fence == 1 && inmeta && /^[^[:space:]]/ { inmeta = 0 }
    fence == 1 && inmeta && /^[[:space:]]+version:[[:space:]]*"[^"]*"/ {
      s = $0
      sub(/^[[:space:]]+version:[[:space:]]*"/, "", s)
      sub(/".*$/, "", s)
      print s
      exit
    }
  '
}

old=$(git show "$base:$skill" 2>/dev/null | read_skill_version) || old=""
new=$(git show ":$skill" 2>/dev/null | read_skill_version) || new=""

if [ -z "$new" ]; then
  echo "skill version guard: $skill has no readable metadata.version." >&2
  echo "Write it as a double-quoted string under metadata: it is the only" >&2
  echo "version an installed copy of the skill carries." >&2
  exit 1
fi
if [ "$old" = "$new" ]; then
  echo "skill version guard: skill content changed but $skill still says $new." >&2
  echo "Bump metadata.version so installed consumers can see the update." >&2
  exit 1
fi
