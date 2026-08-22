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
# Compares the staged manifest against HEAD's, so it checks what is actually
# being committed.
set -euo pipefail

manifest=.claude-plugin/plugin.json

# Initial commit: nothing to compare against.
git rev-parse -q --verify HEAD >/dev/null 2>&1 || exit 0

read_version() {
  python3 -c 'import json,sys; print(json.load(sys.stdin).get("version",""))'
}

old=$(git show "HEAD:$manifest" 2>/dev/null | read_version || echo "")
new=$(git show ":$manifest" 2>/dev/null | read_version || echo "")

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

skill_version=$(git show ":$skill" 2>/dev/null | python3 -c '
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
