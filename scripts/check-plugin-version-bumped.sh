#!/usr/bin/env bash
#
# Fail the commit when plugin content (the packaged skill or .claude-plugin/)
# changes without a plugin.json version bump.
#
# The explicit version in plugin.json is the consumer-facing update gate:
# /plugin update reports "already at the latest version" until it moves, so
# a skill edit without a bump silently freezes updates for every installed
# consumer. The bump is pure bookkeeping, which makes it easy to forget --
# this hook is what makes forgetting impossible.
#
# Two modes, one rule:
#   (no args)      pre-commit: the staged tree against HEAD, so it checks
#                  what is actually being committed.
#   --base <ref>   CI: HEAD against <ref>, the base of a PR or the tip
#                  before a push, so a hookless clone or --no-verify commit
#                  is still caught.
set -euo pipefail

manifest=.claude-plugin/plugin.json
guarded=(skills/ .claude-plugin/)

base=""
if [ "${1:-}" = "--base" ]; then
  base="${2:?--base needs a ref}"
  shift 2
fi

if [ -n "$base" ]; then
  old_spec="$base:$manifest"
  new_spec="HEAD:$manifest"
  skill_spec_prefix="HEAD:"
  content_unchanged() { git diff --quiet "$base" HEAD -- "${guarded[@]}"; }
else
  # Initial commit: nothing to compare against.
  git rev-parse -q --verify HEAD >/dev/null 2>&1 || exit 0
  old_spec="HEAD:$manifest"
  new_spec=":$manifest"
  skill_spec_prefix=":"
  content_unchanged() { git diff --cached --quiet -- "${guarded[@]}"; }
fi

# Nothing changed under the guarded paths: nothing to guard. The hook's
# `files:` filter already implies this on a real commit, but `run
# --all-files` runs every hook regardless of what changed, and without this
# the guard would fail any full-tree run that is not itself a version bump.
if content_unchanged 2>/dev/null; then
  exit 0
fi

# Empty input (the commit that first adds the manifest) is "no version",
# not a JSON parse error.
read_version() {
  python3 -c '
import json, sys
text = sys.stdin.read()
print(json.loads(text).get("version", "") if text.strip() else "")
'
}

old=$(git show "$old_spec" 2>/dev/null | read_version || echo "")
new=$(git show "$new_spec" 2>/dev/null | read_version || echo "")

if [ -z "$new" ]; then
  echo "plugin version guard: $manifest has no version field." >&2
  echo "Add one -- it is the update gate for installed consumers." >&2
  exit 1
fi
if [ "$old" = "$new" ]; then
  echo "plugin version guard: plugin content changed but $manifest is still $new." >&2
  echo "Bump the version so installed consumers see the update." >&2
  exit 1
fi

# The same skill reaches consumers through two channels. /plugin gates on
# plugin.json's version; the Agent Skills format has no version concept and
# just re-pulls, so metadata.version in the frontmatter is the only version a
# non-plugin consumer can read. They have to agree or the two channels
# disagree about what is installed.
skill=skills/playwright-rs-usage/SKILL.md

skill_version=$(git show "$skill_spec_prefix$skill" 2>/dev/null | python3 -c '
import re, sys
text = sys.stdin.read()
m = re.match(r"^---\n(.*?)\n---\n", text, re.S)
if not m:
    sys.exit("no frontmatter")
m = re.search(r"^metadata:\n(?:[ \t]+.*\n)*?[ \t]+version:[ \t]*\"?([^\"\n]+)\"?", m.group(1), re.M)
print(m.group(1).strip() if m else "")
') || {
  echo "plugin version guard: could not read frontmatter from $skill." >&2
  exit 1
}

if [ -z "$skill_version" ]; then
  echo "plugin version guard: $skill has no metadata.version." >&2
  echo "Add one matching $manifest ($new) -- it is the only version a" >&2
  echo "consumer installing outside /plugin can see." >&2
  exit 1
fi
if [ "$skill_version" != "$new" ]; then
  echo "plugin version guard: $manifest is $new but $skill says $skill_version." >&2
  echo "Move them together." >&2
  exit 1
fi
