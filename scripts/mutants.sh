#!/usr/bin/env bash
#
# Run cargo-mutants locally over the lines changed in HEAD (or any base
# ref you choose), so the runtime is in the "useful while iterating"
# range rather than "leave on overnight."
#
# Usage:
#   ./scripts/mutants.sh                      # diff HEAD~1..HEAD
#   ./scripts/mutants.sh main                 # diff main..HEAD
#   ./scripts/mutants.sh 0bb7329              # diff <sha>..HEAD
#   ./scripts/mutants.sh HEAD~5               # diff last 5 commits
#   ./scripts/mutants.sh --working            # diff uncommitted edits (working tree vs HEAD)
#   ./scripts/mutants.sh --working main       # working tree vs main
#   ./scripts/mutants.sh -- --jobs 4          # default base + extra cargo-mutants args
#   ./scripts/mutants.sh main --jobs 4        # explicit base + extra args
#
# The first non-dash argument is the base ref; anything else (and
# everything after the first dash-prefixed arg) passes through to
# cargo-mutants. `--working` is the one flag this script consumes
# itself: it diffs the working tree against the base (HEAD by default)
# instead of a ref range, so a review that pauses before committing can
# still gate its edits. `git diff HEAD` sees tracked files only; `git
# add -N <file>` first if the change adds a new file. See
# https://mutants.rs/ for the full CLI surface.
#
# Why `--in-diff`: an unscoped `cargo mutants` run grows linearly with
# codebase size and routinely runs many hours. `--in-diff` narrows
# mutation to just the lines in the supplied diff: typically seconds
# to minutes for a normal commit.
#
# Scope: `.cargo/mutants.toml` lists the files we mutate at all
# (`examine_globs`) plus mutants we exclude as integration-only. The
# `--in-diff` filter intersects with that scope; diffs touching files
# outside `examine_globs` produce no mutants, which is intentional.
#
# Don't run two of these at once: every cargo-mutants process writes
# `mutants.out/`, so a second run overwrites the first one's counts. The
# binary is `cargo-mutants` (hyphen), so a stray run is stopped with
# `pkill -f cargo-mutants`.
#
# Prerequisites: `cargo install cargo-mutants` (once per machine).
#
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Mutation testing runs the unit tests and the browser-free test binaries
# only (see .cargo/mutants.toml), never a browser, so skip the ~42 MB
# Playwright driver download build.rs would otherwise do in every scratch
# build.
export PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD=1

# Incremental is disabled globally (.cargo/config.toml) because it bloated
# target/ with no payoff for normal builds, but cargo-mutants rebuilds once
# per mutant and genuinely benefits from it, so re-enable it for mutation runs.
export CARGO_INCREMENTAL=1

# --- Project-specific pre-setup ----------------------------------------
# cargo-mutants copies the source tree to a tempdir before building.
# Anything the build expects but that isn't checked in (wasm-pack
# artifacts referenced via include_str!/include_bytes!, generated FFI
# bindings, etc.) won't follow. Mutation testing doesn't exercise those
# bytes at runtime, so empty placeholders are enough.
#
# playwright-rust currently has no such artifacts (the Playwright driver
# is skipped above). Add placeholders here if a future change starts
# include!()-ing checked-out-only files.
# -----------------------------------------------------------------------

# Pull `--working` out of the args wherever it sits; everything else is
# left in place for the base-ref / passthrough handling below.
WORKING=0
REST=()
for arg in "$@"; do
  if [[ "$arg" == "--working" ]]; then
    WORKING=1
  else
    REST+=("$arg")
  fi
done
set -- ${REST[@]+"${REST[@]}"}

# git's empty tree: diffing a root commit against it yields the whole
# initial scaffold rather than exiting 128 on a missing parent.
EMPTY_TREE="$(git hash-object -t tree /dev/null)"

# Resolve the base ref: first non-dash positional arg. Anything starting
# with `-` is treated as a cargo-mutants arg. A ref the user supplied
# must resolve; a silent skip here would let a required CI check pass
# without mutating anything (typo, force-pushed-away `before` SHA,
# shallow clone).
if [[ $# -gt 0 && "$1" != -* && "$1" != "--" ]]; then
  BASE="$1"
  shift
  if ! git rev-parse --verify --quiet "${BASE}^{commit}" >/dev/null; then
    echo "error: base ref '${BASE}' does not resolve." >&2
    exit 1
  fi
elif [[ "$WORKING" -eq 1 ]]; then
  # Working tree vs HEAD; vs the empty tree before the first commit.
  if git rev-parse --verify --quiet HEAD >/dev/null; then
    BASE="HEAD"
  else
    BASE="$EMPTY_TREE"
  fi
else
  # HEAD~1, or the empty tree when HEAD is the root commit.
  if ! git rev-parse --verify --quiet HEAD >/dev/null; then
    echo "nothing committed yet; nothing to mutate (use --working to gate uncommitted edits)."
    exit 0
  fi
  if git rev-parse --verify --quiet 'HEAD^1' >/dev/null; then
    BASE="HEAD~1"
  else
    BASE="$EMPTY_TREE"
  fi
fi

# `--` separator is allowed for clarity; consume it so it doesn't pass
# through to cargo-mutants as a positional.
if [[ $# -gt 0 && "$1" == "--" ]]; then
  shift
fi

LABEL="$BASE"
if [[ "$BASE" == "$EMPTY_TREE" ]]; then
  LABEL="empty tree (root commit)"
fi

# Under $TMPDIR explicitly: `mktemp -t` ignores it on some platforms, and
# a fixed path would be shared by two concurrent runs.
DIFF="$(mktemp "${TMPDIR:-/tmp}/mutants.XXXXXX")"
trap 'rm -f "$DIFF"' EXIT

# One `git diff` for both modes: `<base> HEAD` for a ref range, `<base>`
# alone for the working tree. `--no-renames` because git renders a pure
# `git mv` as a rename with zero content lines, which `--in-diff` reads
# as "nothing changed" and passes green without mutating the moved code.
DIFF_ARGS=(--no-renames "$BASE")
RANGE="working tree vs ${LABEL}"
if [[ "$WORKING" -eq 0 ]]; then
  DIFF_ARGS+=(HEAD)
  RANGE="${LABEL}..HEAD"
fi
git diff "${DIFF_ARGS[@]}" > "$DIFF"

if [[ ! -s "$DIFF" ]]; then
  echo "no diff for ${RANGE}; nothing to mutate."
  if [[ "$WORKING" -eq 1 ]]; then
    echo "tip: new files are invisible to 'git diff' until 'git add -N <file>'."
  else
    echo "tip: commit your changes locally first, then re-run (or use --working)."
  fi
  exit 0
fi

echo "mutating changes in ${RANGE} ($(wc -l < "$DIFF") diff lines)"
# Not `exec`: that would replace the shell and skip the EXIT trap, leaking
# the diff file on every run. The exit status propagates via `set -e`.
cargo mutants --in-diff "$DIFF" "$@"
