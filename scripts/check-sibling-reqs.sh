#!/usr/bin/env bash
# Each publishable crate must require each workspace sibling at that
# sibling's full current version (`0.1.3`, not `0.1` or `~0.1.3`).
#
# A stale line publishes a playwright-rs that a consumer can build against
# the old sibling, which fails if it uses anything the sibling added, and
# cargo refuses the stale line only across a minor bump on 0.x. Checked on
# every commit, so the bump commit fails rather than its release tag, and
# again by release.yml on the tag. Dev-dependencies are left out, since a
# consumer never builds them.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo metadata --no-deps --format-version 1 | jq -r -e '
  (.packages | map({(.name): .version}) | add) as $ver
  | [ .packages[] | select(.publish != []) as $p
      | $p.dependencies[]
      | select(.path != null and .kind != "dev" and $ver[.name] != null)
      | select((.req | ltrimstr("^") | ltrimstr("=")) != $ver[.name])
      | "\($p.name) requires \(.name) \(.req), which is at \($ver[.name])" ]
  | if length > 0 then (.[] | "::error::" + .), false
    else "ok    every sibling is required at its current version" end'
