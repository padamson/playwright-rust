#!/usr/bin/env bash
#
# Compare the driver digests pinned in driver_urls.rs with the ones npm and
# nodejs.org publish now, so a republished artifact shows up here before a
# build refuses it. Run weekly by upstream-playwright-check.yml.
#
# Usage: scripts/check-driver-digests.sh
#
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
src="$root/crates/playwright/src/build_support/driver_urls.rs"

# The quoted strings of one `const NAME ... = ( ... );` block, one per line.
pins() {
  awk -v name="$1" '$0 ~ "const " name ":" { on = 1 } on { print } on && /^\);/ { exit }' "$src" \
    | grep -oE '"[^"]+"' | tr -d '"' || true
}

# The value of a `const NAME: &str = "...";` line.
const_str() {
  sed -nE "s/^const $1: &str = \"([^\"]+)\";/\1/p" "$2"
}

playwright_version=$(const_str PLAYWRIGHT_VERSION "$root/crates/playwright/build.rs")
node_version=$(const_str NODE_VERSION "$src")
if [ -z "$playwright_version" ] || [ -z "$node_version" ]; then
  echo "::error::could not read PLAYWRIGHT_VERSION from build.rs or NODE_VERSION from $src"
  exit 1
fi

failed=0
report() { # what, pinned, published
  if [ -n "$2" ] && [ "$2" = "$3" ]; then
    echo "ok    $1"
  else
    echo "::error::$1 is pinned as '${2}' but published as '${3}'"
    failed=1
  fi
}

fetch() {
  curl -sf --max-time 30 --retry 2 "$1" || { echo "::error::could not fetch $1" >&2; return 1; }
}

npm_pins=$(pins NPM_CORE_INTEGRITY)
npm_version=$(sed -n 1p <<<"$npm_pins")
npm_sri=$(sed -n 2p <<<"$npm_pins")
node_pins=$(pins NODE_ARCHIVE_SHA256)
node_pin_version=$(sed -n 1p <<<"$node_pins")
node_archives=$(tail -n +2 <<<"$node_pins" | paste - -)
if [ "$npm_version" != "$playwright_version" ] || [ "$node_pin_version" != "$node_version" ]; then
  echo "::error::the pins in $src are for playwright-core '$npm_version' and Node '$node_pin_version', not $playwright_version and $node_version"
  exit 1
fi
if [ "$(wc -l <<<"$node_archives")" -ne 6 ]; then
  echo "::error::expected six Node triples with digests in NODE_ARCHIVE_SHA256 in $src"
  exit 1
fi

npm_doc=$(fetch "https://registry.npmjs.org/playwright-core/$npm_version")
report "playwright-core $npm_version" "$npm_sri" "$(jq -r '.dist.integrity // empty' <<<"$npm_doc")"

shasums=$(fetch "https://nodejs.org/dist/v$node_version/SHASUMS256.txt")
while read -r triple hex; do
  case "$triple" in
    win-*) file="node-v$node_version-$triple.zip" ;;
    *) file="node-v$node_version-$triple.tar.gz" ;;
  esac
  report "$file" "$hex" "$(awk -v f="$file" '$2 == f { print $1 }' <<<"$shasums")"
done <<<"$node_archives"

exit "$failed"
